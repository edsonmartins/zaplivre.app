//! RTP Packetization for Video
//!
//! Implements RTP (Real-time Transport Protocol) packetization and depacketization
//! for video frames, following RFC 3550 (RTP) and codec-specific RFCs:
//! - RFC 6184 for H.264
//! - RFC 7741 for VP8
//!
//! Video frames can be large (tens or hundreds of KB), but UDP packets have
//! MTU limitations (~1500 bytes). This module splits large frames into multiple
//! RTP packets and reassembles them on the receiver side.

use super::video::VideoCodec;
use super::{Result, VoipError};

/// Maximum Transmission Unit for RTP packets
/// Using 1200 bytes to leave room for IP/UDP/RTP headers (~300 bytes overhead)
pub const RTP_MTU: usize = 1200;

/// RTP header size (12 bytes fixed header)
pub const RTP_HEADER_SIZE: usize = 12;

/// Maximum RTP payload size per packet
pub const RTP_MAX_PAYLOAD: usize = RTP_MTU - RTP_HEADER_SIZE;
pub const H264_START_CODE: [u8; 4] = [0x00, 0x00, 0x00, 0x01];

/// RTP packet header (RFC 3550)
///
/// ```text
///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |V=2|P|X|  CC   |M|     PT      |       Sequence Number         |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                           Timestamp                           |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |           Synchronization Source (SSRC) identifier            |
/// +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
/// ```
#[derive(Debug, Clone, Copy)]
pub struct RtpHeader {
    /// RTP version (always 2)
    pub version: u8,

    /// Padding flag
    pub padding: bool,

    /// Extension flag
    pub extension: bool,

    /// CSRC count
    pub csrc_count: u8,

    /// Marker bit (set on last packet of frame)
    pub marker: bool,

    /// Payload type (codec identifier)
    pub payload_type: u8,

    /// Sequence number (increments for each packet)
    pub sequence_number: u16,

    /// Timestamp (in 90kHz clock for video)
    pub timestamp: u32,

    /// Synchronization source identifier
    pub ssrc: u32,
}

impl RtpHeader {
    /// Create a new RTP header
    pub fn new(
        sequence_number: u16,
        timestamp: u32,
        ssrc: u32,
        payload_type: u8,
        marker: bool,
    ) -> Self {
        Self {
            version: 2,
            padding: false,
            extension: false,
            csrc_count: 0,
            marker,
            payload_type,
            sequence_number,
            timestamp,
            ssrc,
        }
    }

    /// Serialize RTP header to bytes (12 bytes)
    pub fn to_bytes(&self) -> [u8; RTP_HEADER_SIZE] {
        let mut bytes = [0u8; RTP_HEADER_SIZE];

        // Byte 0: V(2) + P(1) + X(1) + CC(4)
        bytes[0] = (self.version << 6)
            | ((self.padding as u8) << 5)
            | ((self.extension as u8) << 4)
            | (self.csrc_count & 0x0F);

        // Byte 1: M(1) + PT(7)
        bytes[1] = ((self.marker as u8) << 7) | (self.payload_type & 0x7F);

        // Bytes 2-3: Sequence number
        bytes[2..4].copy_from_slice(&self.sequence_number.to_be_bytes());

        // Bytes 4-7: Timestamp
        bytes[4..8].copy_from_slice(&self.timestamp.to_be_bytes());

        // Bytes 8-11: SSRC
        bytes[8..12].copy_from_slice(&self.ssrc.to_be_bytes());

        bytes
    }

    /// Parse RTP header from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < RTP_HEADER_SIZE {
            return Err(VoipError::SignalingError(format!(
                "RTP packet too short: {} bytes (need at least {})",
                bytes.len(),
                RTP_HEADER_SIZE
            )));
        }

        let version = (bytes[0] >> 6) & 0x03;
        if version != 2 {
            return Err(VoipError::SignalingError(format!(
                "Invalid RTP version: {} (expected 2)",
                version
            )));
        }

        let padding = (bytes[0] & 0x20) != 0;
        let extension = (bytes[0] & 0x10) != 0;
        let csrc_count = bytes[0] & 0x0F;

        let marker = (bytes[1] & 0x80) != 0;
        let payload_type = bytes[1] & 0x7F;

        let sequence_number = u16::from_be_bytes([bytes[2], bytes[3]]);
        let timestamp = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let ssrc = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);

        Ok(Self {
            version,
            padding,
            extension,
            csrc_count,
            marker,
            payload_type,
            sequence_number,
            timestamp,
            ssrc,
        })
    }
}

/// RTP packet (header + payload)
#[derive(Debug, Clone)]
pub struct RtpPacket {
    pub header: RtpHeader,
    pub payload: Vec<u8>,
}

impl RtpPacket {
    /// Create a new RTP packet
    pub fn new(header: RtpHeader, payload: Vec<u8>) -> Self {
        Self { header, payload }
    }

    /// Serialize RTP packet to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(RTP_HEADER_SIZE + self.payload.len());
        bytes.extend_from_slice(&self.header.to_bytes());
        bytes.extend_from_slice(&self.payload);
        bytes
    }

    /// Parse RTP packet from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < RTP_HEADER_SIZE {
            return Err(VoipError::SignalingError(
                "RTP packet too short".to_string(),
            ));
        }

        let header = RtpHeader::from_bytes(&bytes[0..RTP_HEADER_SIZE])?;
        let payload = bytes[RTP_HEADER_SIZE..].to_vec();

        Ok(Self { header, payload })
    }
}

/// RTP packetizer for video frames
///
/// Splits large encoded video frames into multiple RTP packets that fit within MTU.
pub struct RtpPacketizer {
    /// Current sequence number (wraps around at 65535)
    sequence_number: u16,

    /// SSRC identifier (remains constant for the stream)
    ssrc: u32,

    /// Codec-specific payload type
    payload_type: u8,

    /// Video codec
    codec: VideoCodec,
}

impl RtpPacketizer {
    /// Create a new RTP packetizer
    pub fn new(ssrc: u32, codec: VideoCodec) -> Self {
        Self {
            sequence_number: rand::random(), // Random initial sequence number
            ssrc,
            payload_type: Self::codec_payload_type(codec),
            codec,
        }
    }

    /// Get RTP payload type for codec
    fn codec_payload_type(codec: VideoCodec) -> u8 {
        match codec {
            VideoCodec::H264 => 96, // Dynamic payload type for H.264
            VideoCodec::VP8 => 97,  // Dynamic payload type for VP8
            VideoCodec::VP9 => 98,  // Dynamic payload type for VP9
        }
    }

    /// Packetize a video frame into multiple RTP packets
    ///
    /// # Arguments
    /// * `frame_data` - Encoded video frame (H.264 NALUs, VP8 frames, etc.)
    /// * `timestamp` - RTP timestamp (in 90kHz clock units)
    ///
    /// # Returns
    /// Vector of RTP packets ready for transmission
    pub fn packetize(&mut self, frame_data: &[u8], timestamp: u32) -> Vec<RtpPacket> {
        match self.codec {
            VideoCodec::H264 => self.packetize_h264(frame_data, timestamp),
            VideoCodec::VP8 => self.packetize_vp8(frame_data, timestamp),
            VideoCodec::VP9 => self.packetize_vp9(frame_data, timestamp),
        }
    }

    /// Packetize H.264 frame (RFC 6184)
    ///
    /// H.264 frames consist of NALUs (Network Abstraction Layer Units).
    /// Each NALU has a header byte followed by payload.
    ///
    /// Packetization modes:
    /// - Mode 0: Single NALU per packet (simplest)
    /// - Mode 1: Fragmentation Units (FU-A) for large NALUs
    fn packetize_h264(&mut self, frame_data: &[u8], timestamp: u32) -> Vec<RtpPacket> {
        let mut packets = Vec::new();

        let nalus = split_annexb_nalus(frame_data);
        if nalus.is_empty() {
            return packets;
        }

        for (index, nalu) in nalus.iter().enumerate() {
            let is_last_nalu = index == nalus.len() - 1;
            packets.extend(self.packetize_h264_nalu(nalu, timestamp, is_last_nalu));
        }

        packets
    }

    fn packetize_h264_nalu(
        &mut self,
        nalu: &[u8],
        timestamp: u32,
        is_last_nalu: bool,
    ) -> Vec<RtpPacket> {
        let mut packets = Vec::new();

        if nalu.is_empty() {
            return packets;
        }

        // If NALU fits in one packet, send as single NALU
        if nalu.len() <= RTP_MAX_PAYLOAD {
            let header = RtpHeader::new(
                self.next_sequence_number(),
                timestamp,
                self.ssrc,
                self.payload_type,
                is_last_nalu, // marker bit set only on last NALU of frame
            );

            packets.push(RtpPacket::new(header, nalu.to_vec()));
            return packets;
        }

        // Large NALU: use FU-A fragmentation (RFC 6184 Section 5.8)

        // FU-A Header:
        // +---------------+
        // |0|1|2|3|4|5|6|7|
        // +-+-+-+-+-+-+-+-+
        // |F|NRI|  Type   |  <- NALU header
        // +---------------+
        // |S|E|R|  Type   |  <- FU-A indicator
        // +---------------+

        let nalu_header = nalu[0];
        let nalu_type = nalu_header & 0x1F;
        let nri = nalu_header & 0x60;

        // FU-A indicator: F=0, NRI=original, Type=28 (FU-A)
        let fu_indicator = nri | 28;

        let payload_data = &nalu[1..]; // Skip NALU header
        let mut offset = 0;

        while offset < payload_data.len() {
            let is_first = offset == 0;
            let remaining = payload_data.len() - offset;
            let chunk_size = remaining.min(RTP_MAX_PAYLOAD - 2); // -2 for FU-A headers
            let is_last = offset + chunk_size >= payload_data.len();

            // FU-A header: S(1) E(1) R(1) Type(5)
            let mut fu_header = nalu_type;
            if is_first {
                fu_header |= 0x80; // Set S bit (start)
            }
            if is_last {
                fu_header |= 0x40; // Set E bit (end)
            }

            // Build payload: FU indicator + FU header + fragment
            let mut payload = Vec::with_capacity(2 + chunk_size);
            payload.push(fu_indicator);
            payload.push(fu_header);
            payload.extend_from_slice(&payload_data[offset..offset + chunk_size]);

            let header = RtpHeader::new(
                self.next_sequence_number(),
                timestamp,
                self.ssrc,
                self.payload_type,
                is_last && is_last_nalu, // marker bit only on last fragment of last NALU
            );

            packets.push(RtpPacket::new(header, payload));
            offset += chunk_size;
        }

        packets
    }

    /// Packetize VP8 frame (RFC 7741)
    ///
    /// VP8 uses a simpler packetization scheme than H.264.
    fn packetize_vp8(&mut self, frame_data: &[u8], timestamp: u32) -> Vec<RtpPacket> {
        let mut packets = Vec::new();

        // If frame fits in one packet
        if frame_data.len() <= RTP_MAX_PAYLOAD - 1 {
            // VP8 payload descriptor (1 byte minimal):
            // X=0, R=0, N=0, S=1 (start of partition), PartID=0
            let vp8_descriptor = 0x10; // S=1

            let mut payload = Vec::with_capacity(1 + frame_data.len());
            payload.push(vp8_descriptor);
            payload.extend_from_slice(frame_data);

            let header = RtpHeader::new(
                self.next_sequence_number(),
                timestamp,
                self.ssrc,
                self.payload_type,
                true,
            );

            packets.push(RtpPacket::new(header, payload));
            return packets;
        }

        // Large frame: fragment across multiple packets
        let mut offset = 0;

        while offset < frame_data.len() {
            let is_first = offset == 0;
            let remaining = frame_data.len() - offset;
            let chunk_size = remaining.min(RTP_MAX_PAYLOAD - 1);
            let is_last = offset + chunk_size >= frame_data.len();

            // VP8 payload descriptor: S bit set only on first packet
            let vp8_descriptor = if is_first { 0x10 } else { 0x00 };

            let mut payload = Vec::with_capacity(1 + chunk_size);
            payload.push(vp8_descriptor);
            payload.extend_from_slice(&frame_data[offset..offset + chunk_size]);

            let header = RtpHeader::new(
                self.next_sequence_number(),
                timestamp,
                self.ssrc,
                self.payload_type,
                is_last,
            );

            packets.push(RtpPacket::new(header, payload));
            offset += chunk_size;
        }

        packets
    }

    /// Packetize VP9 frame (simplified, similar to VP8)
    fn packetize_vp9(&mut self, frame_data: &[u8], timestamp: u32) -> Vec<RtpPacket> {
        // MVP: use the same fragmentation strategy as VP8.
        // This is not a full RFC-compliant VP9 payload descriptor, but keeps
        // frames chunked correctly while we add proper VP9 headers later.
        self.packetize_vp8(frame_data, timestamp)
    }

    /// Get next sequence number (wraps around at 65535)
    fn next_sequence_number(&mut self) -> u16 {
        let seq = self.sequence_number;
        self.sequence_number = self.sequence_number.wrapping_add(1);
        seq
    }
}

/// RTP depacketizer for reassembling video frames
///
/// Receives RTP packets and reassembles them into complete video frames.
pub struct RtpDepacketizer {
    /// Codec being used
    codec: VideoCodec,

    /// Buffer for assembling fragmented frames
    frame_buffer: Vec<u8>,

    /// Expected sequence number (for detecting packet loss)
    expected_sequence: Option<u16>,

    /// Current frame timestamp
    current_timestamp: Option<u32>,
}

impl RtpDepacketizer {
    /// Create a new RTP depacketizer
    pub fn new(codec: VideoCodec) -> Self {
        Self {
            codec,
            frame_buffer: Vec::new(),
            expected_sequence: None,
            current_timestamp: None,
        }
    }

    /// Depacketize an RTP packet
    ///
    /// Returns Some(frame_data) when a complete frame has been assembled,
    /// or None if more packets are needed.
    pub fn depacketize(&mut self, packet: &RtpPacket) -> Result<Option<Vec<u8>>> {
        // Check for sequence number gaps (packet loss)
        if let Some(expected) = self.expected_sequence {
            if packet.header.sequence_number != expected {
                tracing::warn!(
                    "Packet loss detected: expected seq {}, got {}",
                    expected,
                    packet.header.sequence_number
                );

                // Reset frame buffer on packet loss
                self.frame_buffer.clear();
                self.current_timestamp = None;
            }
        }

        self.expected_sequence = Some(packet.header.sequence_number.wrapping_add(1));

        // Check if this is a new frame (different timestamp)
        if let Some(ts) = self.current_timestamp {
            if packet.header.timestamp != ts {
                // New frame started, clear buffer
                self.frame_buffer.clear();
            }
        }

        self.current_timestamp = Some(packet.header.timestamp);

        // Append payload to buffer
        match self.codec {
            VideoCodec::H264 => self.depacketize_h264(packet)?,
            VideoCodec::VP8 => self.depacketize_vp8(packet)?,
            VideoCodec::VP9 => self.depacketize_vp9(packet)?,
        }

        // If marker bit is set, frame is complete
        if packet.header.marker {
            let frame = self.frame_buffer.clone();
            self.frame_buffer.clear();
            self.current_timestamp = None;
            return Ok(Some(frame));
        }

        Ok(None)
    }

    /// Depacketize H.264 RTP packet
    fn depacketize_h264(&mut self, packet: &RtpPacket) -> Result<()> {
        if packet.payload.is_empty() {
            return Ok(());
        }

        let nalu_type = packet.payload[0] & 0x1F;

        if nalu_type == 28 {
            // FU-A fragmented NALU
            if packet.payload.len() < 2 {
                return Err(VoipError::SignalingError("Invalid FU-A packet".to_string()));
            }

            let fu_header = packet.payload[1];
            let is_start = (fu_header & 0x80) != 0;
            let original_nalu_type = fu_header & 0x1F;

            if is_start {
                // Prepend Annex B start code for reconstructed NALU
                self.frame_buffer.extend_from_slice(&H264_START_CODE);
                // First fragment: reconstruct NALU header
                let nri = packet.payload[0] & 0x60;
                let nalu_header = nri | original_nalu_type;
                self.frame_buffer.push(nalu_header);
            }

            // Append fragment payload (skip FU indicator and header)
            self.frame_buffer.extend_from_slice(&packet.payload[2..]);
        } else {
            // Single NALU packet
            self.frame_buffer.extend_from_slice(&H264_START_CODE);
            self.frame_buffer.extend_from_slice(&packet.payload);
        }

        Ok(())
    }

    /// Depacketize VP8 RTP packet
    fn depacketize_vp8(&mut self, packet: &RtpPacket) -> Result<()> {
        if packet.payload.is_empty() {
            return Ok(());
        }

        // Skip VP8 payload descriptor (1 byte minimal)
        let vp8_descriptor = packet.payload[0];
        let is_start = (vp8_descriptor & 0x10) != 0;

        if is_start && !self.frame_buffer.is_empty() {
            // New partition started, but we haven't finished previous frame
            // This shouldn't happen with marker bit, but handle gracefully
            tracing::warn!("VP8 frame boundary mismatch");
        }

        // Append payload (skip descriptor)
        self.frame_buffer.extend_from_slice(&packet.payload[1..]);

        Ok(())
    }

    /// Depacketize VP9 RTP packet (simplified)
    fn depacketize_vp9(&mut self, packet: &RtpPacket) -> Result<()> {
        // For now, use same logic as VP8
        self.depacketize_vp8(packet)
    }

    /// Reset depacketizer state
    pub fn reset(&mut self) {
        self.frame_buffer.clear();
        self.expected_sequence = None;
        self.current_timestamp = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rtp_header_serialization() {
        let header = RtpHeader::new(1234, 567890, 0xDEADBEEF, 96, true);
        let bytes = header.to_bytes();

        assert_eq!(bytes.len(), RTP_HEADER_SIZE);

        let parsed = RtpHeader::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.version, 2);
        assert_eq!(parsed.sequence_number, 1234);
        assert_eq!(parsed.timestamp, 567890);
        assert_eq!(parsed.ssrc, 0xDEADBEEF);
        assert_eq!(parsed.payload_type, 96);
        assert!(parsed.marker);
    }

    #[test]
    fn test_rtp_packet_serialization() {
        let header = RtpHeader::new(100, 200, 300, 96, false);
        let payload = vec![1, 2, 3, 4, 5];
        let packet = RtpPacket::new(header, payload.clone());

        let bytes = packet.to_bytes();
        assert_eq!(bytes.len(), RTP_HEADER_SIZE + payload.len());

        let parsed = RtpPacket::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.header.sequence_number, 100);
        assert_eq!(parsed.payload, payload);
    }

    #[test]
    fn test_small_frame_single_packet() {
        let mut packetizer = RtpPacketizer::new(12345, VideoCodec::H264);
        let frame = vec![0x67, 0x42, 0x00, 0x1F]; // Small H.264 SPS NALU
        let timestamp = 90000;

        let packets = packetizer.packetize(&frame, timestamp);

        assert_eq!(packets.len(), 1);
        assert!(packets[0].header.marker);
        assert_eq!(packets[0].payload, frame);
    }

    #[test]
    fn test_large_frame_fragmentation() {
        let mut packetizer = RtpPacketizer::new(12345, VideoCodec::H264);

        // Create a large frame (3000 bytes)
        let mut frame = vec![0x65]; // I-frame NALU header
        frame.extend(vec![0xFF; 2999]);

        let timestamp = 180000;
        let packets = packetizer.packetize(&frame, timestamp);

        // Should be fragmented into multiple packets
        assert!(packets.len() > 1);

        // Only last packet should have marker bit
        for (i, packet) in packets.iter().enumerate() {
            let is_last = i == packets.len() - 1;
            assert_eq!(packet.header.marker, is_last);
            assert_eq!(packet.header.timestamp, timestamp);
        }

        // All packets should fit within MTU
        for packet in &packets {
            let total_size = RTP_HEADER_SIZE + packet.payload.len();
            assert!(total_size <= RTP_MTU);
        }
    }

    #[test]
    fn test_packetize_depacketize_round_trip() {
        let mut packetizer = RtpPacketizer::new(12345, VideoCodec::H264);
        let mut depacketizer = RtpDepacketizer::new(VideoCodec::H264);

        // Create a frame that requires fragmentation
        let mut original_frame = Vec::from(H264_START_CODE);
        original_frame.push(0x65); // NALU header
        original_frame.extend(vec![0xAB; 2500]);

        let timestamp = 270000;
        let packets = packetizer.packetize(&original_frame, timestamp);

        assert!(packets.len() > 1);

        // Depacketize all packets
        let mut reconstructed_frame = None;
        for packet in packets {
            if let Some(frame) = depacketizer.depacketize(&packet).unwrap() {
                reconstructed_frame = Some(frame);
            }
        }

        // Frame should be reconstructed
        assert!(reconstructed_frame.is_some());
        assert_eq!(reconstructed_frame.unwrap(), original_frame);
    }

    #[test]
    fn test_annexb_multi_nalu_round_trip() {
        let mut packetizer = RtpPacketizer::new(12345, VideoCodec::H264);
        let mut depacketizer = RtpDepacketizer::new(VideoCodec::H264);

        let mut original_frame = Vec::from(H264_START_CODE);
        original_frame.extend_from_slice(&[0x67, 0x42, 0x00, 0x1F]); // SPS
        original_frame.extend_from_slice(&H264_START_CODE);
        original_frame.extend_from_slice(&[0x68, 0xCE, 0x06, 0xE2]); // PPS
        original_frame.extend_from_slice(&H264_START_CODE);
        original_frame.extend_from_slice(&[0x65, 0x88, 0x84]); // IDR

        let timestamp = 123000;
        let packets = packetizer.packetize(&original_frame, timestamp);

        let mut reconstructed_frame = None;
        for packet in packets {
            if let Some(frame) = depacketizer.depacketize(&packet).unwrap() {
                reconstructed_frame = Some(frame);
            }
        }

        assert_eq!(reconstructed_frame.unwrap(), original_frame);
    }

    #[test]
    fn test_vp8_packetization() {
        let mut packetizer = RtpPacketizer::new(54321, VideoCodec::VP8);
        let frame = vec![0x10, 0x00, 0x9D, 0x01, 0x2A]; // Small VP8 frame
        let timestamp = 360000;

        let packets = packetizer.packetize(&frame, timestamp);

        assert_eq!(packets.len(), 1);
        assert!(packets[0].header.marker);
        assert_eq!(packets[0].header.payload_type, 97); // VP8 payload type
    }

    #[test]
    fn test_sequence_number_wraparound() {
        let mut packetizer = RtpPacketizer::new(12345, VideoCodec::H264);
        packetizer.sequence_number = 65534; // Near max

        let frame = vec![0x67, 0x42];

        let packets1 = packetizer.packetize(&frame, 1000);
        let packets2 = packetizer.packetize(&frame, 2000);
        let packets3 = packetizer.packetize(&frame, 3000);

        assert_eq!(packets1[0].header.sequence_number, 65534);
        assert_eq!(packets2[0].header.sequence_number, 65535);
        assert_eq!(packets3[0].header.sequence_number, 0); // Wrapped around
    }
}

fn split_annexb_nalus(frame_data: &[u8]) -> Vec<Vec<u8>> {
    let mut nalus = Vec::new();
    let mut start: Option<usize> = None;
    let mut i = 0;

    while i + 3 < frame_data.len() {
        let is_start_code_4 = frame_data[i] == 0
            && frame_data[i + 1] == 0
            && frame_data[i + 2] == 0
            && frame_data[i + 3] == 1;
        let is_start_code_3 =
            frame_data[i] == 0 && frame_data[i + 1] == 0 && frame_data[i + 2] == 1;

        if is_start_code_4 || is_start_code_3 {
            let start_code_len = if is_start_code_4 { 4 } else { 3 };
            if let Some(nalu_start) = start {
                if nalu_start < i {
                    nalus.push(frame_data[nalu_start..i].to_vec());
                }
            }
            start = Some(i + start_code_len);
            i += start_code_len;
            continue;
        }

        i += 1;
    }

    if let Some(nalu_start) = start {
        if nalu_start < frame_data.len() {
            nalus.push(frame_data[nalu_start..].to_vec());
        }
    }

    if nalus.is_empty() && !frame_data.is_empty() {
        nalus.push(frame_data.to_vec());
    }

    nalus
}
