//! WebRTC Peer Connection Management
//!
//! Handles WebRTC peer connections, media tracks (audio + video), and ICE/DTLS.

use super::codec::{OpusConfig, OpusDecoder};
use super::rtp_video::{RtpDepacketizer, RtpPacket, RtpPacketizer};
use super::video::VideoCodec;
use super::VoipError;
use crate::voip::Result;
use interceptor::registry::Registry;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, RwLock};
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::setting_engine::SettingEngine;
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;
use webrtc::track::track_local::{TrackLocal, TrackLocalWriter};

/// WebRTC peer connection wrapper
pub struct WebRTCPeer {
    peer_connection: Arc<RTCPeerConnection>,
    audio_track: Option<Arc<TrackLocalStaticRTP>>,
    video_track: Option<Arc<TrackLocalStaticRTP>>,
    remote_video_callback: Arc<RwLock<Option<Arc<dyn Fn(Vec<u8>, u32, u32) + Send + Sync>>>>,
    remote_audio_callback: Arc<RwLock<Option<Arc<dyn Fn(Vec<u8>, u32, u32) + Send + Sync>>>>,
    video_rtp_state: Mutex<VideoRtpState>,
    video_packetizer: Mutex<Option<RtpPacketizer>>,
}

struct VideoRtpState {
    ssrc: u32,
    ts_base: u32,
    started_at: Instant,
    clock_rate: u32,
}

impl WebRTCPeer {
    /// Create a new WebRTC peer with STUN/TURN configuration
    pub async fn new(ice_servers: Vec<RTCIceServer>) -> Result<Self> {
        // Create a MediaEngine for audio
        let mut media_engine = MediaEngine::default();

        // Register Opus codec (standard for WebRTC audio)
        media_engine
            .register_default_codecs()
            .map_err(|e| VoipError::WebRtcError(format!("Failed to register codecs: {}", e)))?;

        // Create SettingEngine
        let mut setting_engine = SettingEngine::default();

        // Enable detach for data channel
        setting_engine.detach_data_channels();

        // Create API with MediaEngine and SettingEngine
        let mut interceptor_registry = Registry::new();

        // Register default interceptors (RTCP, NACK, etc.)
        interceptor_registry = register_default_interceptors(interceptor_registry, &mut media_engine)
            .map_err(|e| VoipError::WebRtcError(format!("Failed to register interceptors: {}", e)))?;

        let api = APIBuilder::new()
            .with_media_engine(media_engine)
            .with_setting_engine(setting_engine)
            .with_interceptor_registry(interceptor_registry)
            .build();

        // Create peer connection configuration with ICE servers
        let config = RTCConfiguration {
            ice_servers,
            ..Default::default()
        };

        // Create the peer connection
        let peer_connection = Arc::new(
            api.new_peer_connection(config)
                .await
                .map_err(|e| VoipError::WebRtcError(format!("Failed to create peer connection: {}", e)))?,
        );

        let remote_video_callback: Arc<RwLock<Option<Arc<dyn Fn(Vec<u8>, u32, u32) + Send + Sync>>>> =
            Arc::new(RwLock::new(None));
        let remote_audio_callback: Arc<RwLock<Option<Arc<dyn Fn(Vec<u8>, u32, u32) + Send + Sync>>>> =
            Arc::new(RwLock::new(None));

        let video_cb = Arc::clone(&remote_video_callback);
        let audio_cb = Arc::clone(&remote_audio_callback);

        // Register handler for when remote track is added (audio/video)
        peer_connection.on_track(Box::new(move |track, _receiver, _transceiver| {
            let video_cb = Arc::clone(&video_cb);
            let audio_cb = Arc::clone(&audio_cb);

            Box::pin(async move {
                match track.kind() {
                    webrtc::rtp_transceiver::rtp_codec::RTPCodecType::Video => {
                        tracing::info!("📹 Remote video track received");
                        let depacketizer = Arc::new(Mutex::new(RtpDepacketizer::new(VideoCodec::H264)));
                        let mut last_width = 640u32;
                        let mut last_height = 480u32;

                        while let Ok((rtp_packet, _)) = track.read_rtp().await {
                            let header = rtp_packet.header;
                            let packet = RtpPacket::new(
                                super::rtp_video::RtpHeader {
                                    version: header.version,
                                    padding: header.padding,
                                    extension: header.extension,
                                    csrc_count: header.csrc.len() as u8,
                                    marker: header.marker,
                                    payload_type: header.payload_type,
                                    sequence_number: header.sequence_number,
                                    timestamp: header.timestamp,
                                    ssrc: header.ssrc,
                                },
                                rtp_packet.payload.to_vec(),
                            );

                            let mut depacketizer = depacketizer.lock().await;
                            if let Ok(Some(frame)) = depacketizer.depacketize(&packet) {
                                let callback = { video_cb.read().await.clone() };
                                if let Some(callback) = callback {
                                    if let Some((width, height)) = parse_h264_dimensions_from_annexb(&frame) {
                                        last_width = width;
                                        last_height = height;
                                    }
                                    callback(frame, last_width, last_height);
                                }
                            }
                        }
                    }
                    webrtc::rtp_transceiver::rtp_codec::RTPCodecType::Audio => {
                        tracing::info!("🔊 Remote audio track received");
                        let mut decoder = match OpusDecoder::new(OpusConfig::default()) {
                            Ok(decoder) => decoder,
                            Err(err) => {
                                tracing::error!("Failed to initialize Opus decoder: {}", err);
                                return;
                            }
                        };

                        while let Ok((rtp_packet, _)) = track.read_rtp().await {
                            let decoded = match decoder.decode(&rtp_packet.payload) {
                                Ok(samples) => samples,
                                Err(err) => {
                                    tracing::warn!("Failed to decode Opus packet: {}", err);
                                    continue;
                                }
                            };

                            let mut pcm_bytes = Vec::with_capacity(decoded.len() * 2);
                            for sample in decoded {
                                let clamped = sample.clamp(-1.0, 1.0);
                                let value = (clamped * 32767.0) as i16;
                                pcm_bytes.extend_from_slice(&value.to_le_bytes());
                            }

                            let callback = { audio_cb.read().await.clone() };
                            if let Some(callback) = callback {
                                callback(pcm_bytes, 48_000, 1);
                            }
                        }
                    }
                    _ => {}
                }
            })
        }));

        Ok(Self {
            peer_connection,
            audio_track: None,
            video_track: None,
            remote_video_callback,
            remote_audio_callback,
            video_rtp_state: Mutex::new(VideoRtpState {
                ssrc: rand::random::<u32>(),
                ts_base: rand::random::<u32>(),
                started_at: Instant::now(),
                clock_rate: 90_000,
            }),
            video_packetizer: Mutex::new(None),
        })
    }

    /// Register callback for remote video frames
    ///
    /// This should be called after creating the peer but before starting the call
    pub async fn on_remote_video_frame<F>(&self, callback: F) -> Result<()>
    where
        F: Fn(Vec<u8>, u32, u32) + Send + Sync + 'static,
    {
        let mut cb = self.remote_video_callback.write().await;
        *cb = Some(Arc::new(callback));
        Ok(())
    }

    /// Register callback for remote audio frames (decoded PCM16)
    pub async fn on_remote_audio_frame<F>(&self, callback: F) -> Result<()>
    where
        F: Fn(Vec<u8>, u32, u32) + Send + Sync + 'static,
    {
        let mut cb = self.remote_audio_callback.write().await;
        *cb = Some(Arc::new(callback));
        Ok(())
    }

    /// Register callback for local ICE candidates
    pub async fn on_ice_candidate<F>(&self, callback: F) -> Result<()>
    where
        F: Fn(String, Option<String>, Option<u16>) + Send + Sync + 'static,
    {
        let pc = Arc::clone(&self.peer_connection);
        let callback = Arc::new(callback);

        pc.on_ice_candidate(Box::new(move |candidate| {
            let callback = Arc::clone(&callback);
            Box::pin(async move {
                if let Some(candidate) = candidate {
                    if let Ok(json) = candidate.to_json() {
                        callback(json.candidate, json.sdp_mid, json.sdp_mline_index);
                    }
                }
            })
        }));

        Ok(())
    }

    /// Add audio track to the peer connection
    pub async fn add_audio_track(&mut self) -> Result<()> {
        // Create an audio track (Opus codec)
        let audio_track = Arc::new(TrackLocalStaticRTP::new(
            RTCRtpCodecCapability {
                mime_type: "audio/opus".to_owned(),
                clock_rate: 48000,
                channels: 2,
                sdp_fmtp_line: "".to_owned(),
                rtcp_feedback: vec![],
            },
            "audio".to_owned(),
            "zaplivre-audio".to_owned(),
        ));

        // Add track to peer connection
        let _rtp_sender = self
            .peer_connection
            .add_track(Arc::clone(&audio_track) as Arc<dyn TrackLocal + Send + Sync>)
            .await
            .map_err(|e| VoipError::WebRtcError(format!("Failed to add audio track: {}", e)))?;

        self.audio_track = Some(audio_track);

        tracing::info!("✅ Audio track added to peer connection");
        Ok(())
    }

    /// Add video track to the peer connection
    pub async fn add_video_track(&mut self, codec: VideoCodec) -> Result<()> {
        // Create a video track with specified codec
        let video_track = Arc::new(TrackLocalStaticRTP::new(
            RTCRtpCodecCapability {
                mime_type: codec.mime_type().to_owned(),
                clock_rate: codec.clock_rate(),
                channels: 0, // Video has no channels
                sdp_fmtp_line: codec.fmtp_line(),
                rtcp_feedback: vec![],
            },
            "video".to_owned(),
            "zaplivre-video".to_owned(),
        ));

        // Add track to peer connection
        let _rtp_sender = self
            .peer_connection
            .add_track(Arc::clone(&video_track) as Arc<dyn TrackLocal + Send + Sync>)
            .await
            .map_err(|e| VoipError::WebRtcError(format!("Failed to add video track: {}", e)))?;

        self.video_track = Some(video_track);
        {
            let mut state = self.video_rtp_state.lock().await;
            state.started_at = Instant::now();
            state.clock_rate = codec.clock_rate();
        }
        {
            let state = self.video_rtp_state.lock().await;
            let mut packetizer = self.video_packetizer.lock().await;
            *packetizer = Some(RtpPacketizer::new(state.ssrc, codec));
        }

        tracing::info!("✅ Video track added to peer connection - codec: {:?}", codec);
        Ok(())
    }

    /// Send video frame to remote peer
    ///
    /// Frame data should be pre-encoded (H.264 NALUs or VP8 frames)
    pub async fn send_video_frame(&self, frame: &[u8]) -> Result<()> {
        if let Some(video_track) = &self.video_track {
            let timestamp = {
                let state = self.video_rtp_state.lock().await;
                let elapsed = state.started_at.elapsed();
                let elapsed_ts = (elapsed.as_secs_f64() * state.clock_rate as f64) as u32;
                state.ts_base.wrapping_add(elapsed_ts)
            };

            let packets = {
                let mut packetizer = self.video_packetizer.lock().await;
                let packetizer = packetizer.as_mut().ok_or_else(|| {
                    VoipError::InvalidState("Video packetizer not initialized".to_string())
                })?;
                packetizer.packetize(frame, timestamp)
            };

            for packet in packets {
                let packet = webrtc::rtp::packet::Packet {
                    header: webrtc::rtp::header::Header {
                        version: packet.header.version,
                        padding: packet.header.padding,
                        extension: packet.header.extension,
                        marker: packet.header.marker,
                        payload_type: packet.header.payload_type,
                        sequence_number: packet.header.sequence_number,
                        timestamp: packet.header.timestamp,
                        ssrc: packet.header.ssrc,
                        ..Default::default()
                    },
                    payload: packet.payload.into(),
                };

                video_track
                    .write_rtp(&packet)
                    .await
                    .map_err(|e| VoipError::WebRtcError(format!("Failed to write video frame: {}", e)))?;
            }

            Ok(())
        } else {
            Err(VoipError::InvalidState(
                "Video track not added yet".to_string(),
            ))
        }
    }

    /// Remove video track (disable camera)
    ///
    /// Triggers renegotiation to inform remote peer
    pub async fn remove_video_track(&mut self) -> Result<()> {
        if self.video_track.is_none() {
            return Ok(()); // Already removed
        }

        self.video_track = None;

        // Trigger renegotiation by creating new offer
        let _ = self.create_offer().await?;

        tracing::info!("🚫 Video track removed from peer connection");
        Ok(())
    }

    /// Check if video track is enabled
    pub fn has_video(&self) -> bool {
        self.video_track.is_some()
    }

    /// Create SDP offer
    pub async fn create_offer(&self) -> Result<String> {
        let offer = self
            .peer_connection
            .create_offer(None)
            .await
            .map_err(|e| VoipError::WebRtcError(format!("Failed to create offer: {}", e)))?;

        // Set local description
        self.peer_connection
            .set_local_description(offer.clone())
            .await
            .map_err(|e| VoipError::WebRtcError(format!("Failed to set local description: {}", e)))?;

        Ok(offer.sdp)
    }

    /// Create SDP answer
    pub async fn create_answer(&self) -> Result<String> {
        let answer = self
            .peer_connection
            .create_answer(None)
            .await
            .map_err(|e| VoipError::WebRtcError(format!("Failed to create answer: {}", e)))?;

        // Set local description
        self.peer_connection
            .set_local_description(answer.clone())
            .await
            .map_err(|e| VoipError::WebRtcError(format!("Failed to set local description: {}", e)))?;

        Ok(answer.sdp)
    }

    /// Set remote SDP description (offer or answer)
    pub async fn set_remote_description(&self, sdp: String, sdp_type: &str) -> Result<()> {
        let remote_desc = match sdp_type {
            "offer" => RTCSessionDescription::offer(sdp)
                .map_err(|e| VoipError::WebRtcError(format!("Invalid SDP offer: {}", e)))?,
            "answer" => RTCSessionDescription::answer(sdp)
                .map_err(|e| VoipError::WebRtcError(format!("Invalid SDP answer: {}", e)))?,
            _ => {
                return Err(VoipError::WebRtcError(format!(
                    "Invalid SDP type: {}",
                    sdp_type
                )))
            }
        };

        self.peer_connection
            .set_remote_description(remote_desc)
            .await
            .map_err(|e| VoipError::WebRtcError(format!("Failed to set remote description: {}", e)))?;

        Ok(())
    }

    /// Add remote ICE candidate
    pub async fn add_ice_candidate(
        &self,
        candidate: String,
        sdp_mid: Option<String>,
        sdp_mline_index: Option<u16>,
    ) -> Result<()> {
        let ice_candidate = RTCIceCandidateInit {
            candidate,
            sdp_mid,
            sdp_mline_index,
            ..Default::default()
        };

        self.peer_connection
            .add_ice_candidate(ice_candidate)
            .await
            .map_err(|e| VoipError::WebRtcError(format!("Failed to add ICE candidate: {}", e)))?;

        Ok(())
    }

    /// Get current connection state
    pub fn connection_state(&self) -> RTCPeerConnectionState {
        self.peer_connection.connection_state()
    }

    /// Close the peer connection
    pub async fn close(&self) -> Result<()> {
        self.peer_connection
            .close()
            .await
            .map_err(|e| VoipError::WebRtcError(format!("Failed to close connection: {}", e)))?;

        tracing::info!("🔌 WebRTC peer connection closed");
        Ok(())
    }

    /// Get reference to the peer connection for event handlers
    pub fn peer_connection(&self) -> Arc<RTCPeerConnection> {
        Arc::clone(&self.peer_connection)
    }

    /// Get audio track for sending audio data
    pub fn audio_track(&self) -> Option<Arc<TrackLocalStaticRTP>> {
        self.audio_track.clone()
    }
}

fn parse_h264_dimensions_from_annexb(data: &[u8]) -> Option<(u32, u32)> {
    for nalu in split_annexb_nalus(data) {
        if nalu.is_empty() {
            continue;
        }
        let nalu_type = nalu[0] & 0x1F;
        if nalu_type == 7 {
            return parse_h264_sps(&nalu);
        }
    }
    None
}

fn split_annexb_nalus(data: &[u8]) -> Vec<Vec<u8>> {
    let mut nalus = Vec::new();
    let mut start: Option<usize> = None;
    let mut i = 0;
    while i + 3 < data.len() {
        let is_start4 = data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 0 && data[i + 3] == 1;
        let is_start3 = data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 1;
        if is_start4 || is_start3 {
            let start_code_len = if is_start4 { 4 } else { 3 };
            if let Some(s) = start {
                if s < i {
                    nalus.push(data[s..i].to_vec());
                }
            }
            start = Some(i + start_code_len);
            i += start_code_len;
            continue;
        }
        i += 1;
    }
    if let Some(s) = start {
        if s < data.len() {
            nalus.push(data[s..].to_vec());
        }
    } else if !data.is_empty() {
        nalus.push(data.to_vec());
    }
    nalus
}

fn parse_h264_sps(nalu: &[u8]) -> Option<(u32, u32)> {
    if nalu.len() < 4 {
        return None;
    }
    let mut reader = BitReader::new(&nalu[1..]); // Skip NAL header

    let profile_idc = reader.read_bits(8)? as u8;
    let _constraints = reader.read_bits(8)?;
    let _level_idc = reader.read_bits(8)?;
    let _sps_id = reader.read_ue()?;

    let mut chroma_format_idc = 1u32;
    let mut separate_colour_plane_flag = false;
    if matches!(
        profile_idc,
        100 | 110 | 122 | 244 | 44 | 83 | 86 | 118 | 128 | 138 | 144
    ) {
        chroma_format_idc = reader.read_ue()?;
        if chroma_format_idc == 3 {
            separate_colour_plane_flag = reader.read_bit()? != 0;
        }
        let _bit_depth_luma_minus8 = reader.read_ue()?;
        let _bit_depth_chroma_minus8 = reader.read_ue()?;
        let _qpprime_y_zero_transform_bypass_flag = reader.read_bit()?;
        let seq_scaling_matrix_present_flag = reader.read_bit()? != 0;
        if seq_scaling_matrix_present_flag {
            let scaling_list_count = if chroma_format_idc != 3 { 8 } else { 12 };
            for i in 0..scaling_list_count {
                let scaling_list_present = reader.read_bit()? != 0;
                if scaling_list_present {
                    let size = if i < 6 { 16 } else { 64 };
                    skip_scaling_list(&mut reader, size)?;
                }
            }
        }
    }

    let _log2_max_frame_num_minus4 = reader.read_ue()?;
    let pic_order_cnt_type = reader.read_ue()?;
    if pic_order_cnt_type == 0 {
        let _log2_max_pic_order_cnt_lsb_minus4 = reader.read_ue()?;
    } else if pic_order_cnt_type == 1 {
        let _delta_pic_order_always_zero_flag = reader.read_bit()?;
        let _offset_for_non_ref_pic = reader.read_se()?;
        let _offset_for_top_to_bottom_field = reader.read_se()?;
        let num_ref_frames_in_pic_order_cnt_cycle = reader.read_ue()?;
        for _ in 0..num_ref_frames_in_pic_order_cnt_cycle {
            let _ = reader.read_se()?;
        }
    }

    let _max_num_ref_frames = reader.read_ue()?;
    let _gaps_in_frame_num_value_allowed_flag = reader.read_bit()?;

    let pic_width_in_mbs_minus1 = reader.read_ue()?;
    let pic_height_in_map_units_minus1 = reader.read_ue()?;
    let frame_mbs_only_flag = reader.read_bit()? != 0;
    if !frame_mbs_only_flag {
        let _mb_adaptive_frame_field_flag = reader.read_bit()?;
    }
    let _direct_8x8_inference_flag = reader.read_bit()?;
    let frame_cropping_flag = reader.read_bit()? != 0;
    let (mut crop_left, mut crop_right, mut crop_top, mut crop_bottom) = (0u32, 0u32, 0u32, 0u32);
    if frame_cropping_flag {
        crop_left = reader.read_ue()?;
        crop_right = reader.read_ue()?;
        crop_top = reader.read_ue()?;
        crop_bottom = reader.read_ue()?;
    }

    let width = (pic_width_in_mbs_minus1 + 1) * 16;
    let height = (pic_height_in_map_units_minus1 + 1) * 16 * if frame_mbs_only_flag { 1 } else { 2 };

    let (crop_unit_x, crop_unit_y) = if separate_colour_plane_flag {
        (1, if frame_mbs_only_flag { 1 } else { 2 })
    } else {
        match chroma_format_idc {
            0 => (1, if frame_mbs_only_flag { 1 } else { 2 }),
            1 => (2, if frame_mbs_only_flag { 2 } else { 4 }),
            2 => (2, if frame_mbs_only_flag { 1 } else { 2 }),
            3 => (1, if frame_mbs_only_flag { 1 } else { 2 }),
            _ => (1, if frame_mbs_only_flag { 1 } else { 2 }),
        }
    };

    let crop_w = (crop_left + crop_right) * crop_unit_x;
    let crop_h = (crop_top + crop_bottom) * crop_unit_y;
    Some((width.saturating_sub(crop_w), height.saturating_sub(crop_h)))
}

fn skip_scaling_list(reader: &mut BitReader, size: usize) -> Option<()> {
    let mut last_scale = 8i32;
    let mut next_scale = 8i32;
    for _ in 0..size {
        if next_scale != 0 {
            let delta_scale = reader.read_se()? as i32;
            next_scale = (last_scale + delta_scale + 256) % 256;
        }
        last_scale = if next_scale == 0 { last_scale } else { next_scale };
    }
    Some(())
}

struct BitReader<'a> {
    data: &'a [u8],
    bit_pos: usize,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, bit_pos: 0 }
    }

    fn read_bit(&mut self) -> Option<u8> {
        if self.bit_pos >= self.data.len() * 8 {
            return None;
        }
        let byte = self.data[self.bit_pos / 8];
        let shift = 7 - (self.bit_pos % 8);
        self.bit_pos += 1;
        Some((byte >> shift) & 0x01)
    }

    fn read_bits(&mut self, count: usize) -> Option<u32> {
        let mut value = 0u32;
        for _ in 0..count {
            value = (value << 1) | (self.read_bit()? as u32);
        }
        Some(value)
    }

    fn read_ue(&mut self) -> Option<u32> {
        let mut zeros = 0usize;
        while self.read_bit()? == 0 {
            zeros += 1;
        }
        let suffix = if zeros > 0 { self.read_bits(zeros)? } else { 0 };
        Some(((1u32 << zeros) - 1) + suffix)
    }

    fn read_se(&mut self) -> Option<i32> {
        let ue = self.read_ue()? as i32;
        let value = if ue % 2 == 0 { -(ue / 2) } else { (ue + 1) / 2 };
        Some(value)
    }
}

/// Build TURN server configuration from credentials
pub fn build_turn_config(
    turn_uris: Vec<String>,
    username: String,
    credential: String,
) -> Vec<RTCIceServer> {
    vec![
        // STUN server (public Google STUN)
        RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        },
        // TURN server (from FASE 10)
        RTCIceServer {
            urls: turn_uris,
            username,
            credential,
            credential_type: webrtc::ice_transport::ice_credential_type::RTCIceCredentialType::Password,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_peer_connection() {
        // Use public STUN server for testing
        let ice_servers = vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        }];

        let peer = WebRTCPeer::new(ice_servers).await;
        assert!(peer.is_ok());
    }

    #[tokio::test]
    async fn test_add_audio_track() {
        let ice_servers = vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        }];

        let mut peer = WebRTCPeer::new(ice_servers).await.unwrap();
        let result = peer.add_audio_track().await;

        assert!(result.is_ok());
        assert!(peer.audio_track().is_some());
    }

    #[test]
    fn test_build_turn_config() {
        let config = build_turn_config(
            vec!["turn:turn.example.com:3478".to_string()],
            "user123".to_string(),
            "pass456".to_string(),
        );

        assert_eq!(config.len(), 2); // STUN + TURN
        assert!(config[0].urls[0].starts_with("stun:"));
        assert!(config[1].urls[0].starts_with("turn:"));
    }
}
