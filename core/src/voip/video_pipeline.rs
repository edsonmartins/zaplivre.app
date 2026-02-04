//! Video encoding/decoding pipeline

use super::rtp_video::{RtpDepacketizer, RtpPacket, RtpPacketizer};
use super::video::*;
use super::Result;
use tokio::sync::mpsc;

/// Video encoder pipeline (Camera frames → RTP packets)
pub struct VideoEncoderPipeline {
    config: VideoConfig,
    frame_rx: mpsc::Receiver<VideoFrame>,
    rtp_tx: mpsc::Sender<Vec<u8>>,
    running: bool,
}

impl VideoEncoderPipeline {
    /// Create a new video encoder pipeline
    pub fn new(
        config: VideoConfig,
        frame_rx: mpsc::Receiver<VideoFrame>,
        rtp_tx: mpsc::Sender<Vec<u8>>,
    ) -> Self {
        Self {
            config,
            frame_rx,
            rtp_tx,
            running: false,
        }
    }

    /// Start the encoder pipeline
    pub async fn run(&mut self) -> Result<()> {
        self.running = true;

        // Platform-specific encoder (H.264 via VideoToolbox/MediaCodec)
        // For MVP, assume platform sends pre-encoded frames and we only packetize.
        let mut packetizer = RtpPacketizer::new(rand::random::<u32>(), self.config.codec);
        let clock_rate = self.config.codec.clock_rate() as u64;
        while self.running {
            if let Some(frame) = self.frame_rx.recv().await {
                let ts = if frame.timestamp_us > 0 {
                    ((frame.timestamp_us as u64) * clock_rate / 1_000_000) as u32
                } else {
                    chrono::Utc::now().timestamp_micros() as u32
                };

                for packet in packetizer.packetize(&frame.data, ts) {
                    if let Err(e) = self.rtp_tx.send(packet.to_bytes()).await {
                        tracing::error!("Failed to send RTP packet: {}", e);
                        self.running = false;
                        break;
                    }
                }
            } else {
                // Channel closed
                break;
            }
        }

        self.running = false;
        Ok(())
    }

    /// Stop the encoder pipeline
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// Check if pipeline is running
    pub fn is_running(&self) -> bool {
        self.running
    }
}

/// Video decoder pipeline (RTP packets → Video frames)
pub struct VideoDecoderPipeline {
    config: VideoConfig,
    rtp_rx: mpsc::Receiver<Vec<u8>>,
    frame_tx: mpsc::Sender<VideoFrame>,
    running: bool,
}

impl VideoDecoderPipeline {
    /// Create a new video decoder pipeline
    pub fn new(
        config: VideoConfig,
        rtp_rx: mpsc::Receiver<Vec<u8>>,
        frame_tx: mpsc::Sender<VideoFrame>,
    ) -> Self {
        Self {
            config,
            rtp_rx,
            frame_tx,
            running: false,
        }
    }

    /// Start the decoder pipeline
    pub async fn run(&mut self) -> Result<()> {
        self.running = true;

        // Platform-specific decoder (MVP: depacketize to encoded frame bytes)
        let mut depacketizer = RtpDepacketizer::new(self.config.codec);
        while self.running {
            if let Some(rtp_packet) = self.rtp_rx.recv().await {
                let packet = match RtpPacket::from_bytes(&rtp_packet) {
                    Ok(pkt) => pkt,
                    Err(e) => {
                        tracing::warn!("Failed to parse RTP packet: {}", e);
                        continue;
                    }
                };

                if let Some(frame_data) = depacketizer.depacketize(&packet)? {
                    let frame = VideoFrame::new(
                        frame_data,
                        self.config.resolution.width,
                        self.config.resolution.height,
                        chrono::Utc::now().timestamp_micros(),
                        PixelFormat::YUV420,
                    );

                    if let Err(e) = self.frame_tx.send(frame).await {
                        tracing::error!("Failed to send video frame: {}", e);
                        break;
                    }
                }
            } else {
                // Channel closed
                break;
            }
        }

        self.running = false;
        Ok(())
    }

    /// Stop the decoder pipeline
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// Check if pipeline is running
    pub fn is_running(&self) -> bool {
        self.running
    }
}

/// Video statistics
#[derive(Debug, Clone, Default)]
pub struct VideoStats {
    /// Total frames sent
    pub frames_sent: u64,
    /// Total frames received
    pub frames_received: u64,
    /// Frames dropped
    pub frames_dropped: u64,
    /// Current bitrate in kbps
    pub bitrate_kbps: u32,
    /// Current framerate
    pub fps: u32,
}

impl VideoStats {
    /// Create new video stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Record frame sent
    pub fn record_frame_sent(&mut self) {
        self.frames_sent += 1;
    }

    /// Record frame received
    pub fn record_frame_received(&mut self) {
        self.frames_received += 1;
    }

    /// Record frame dropped
    pub fn record_frame_dropped(&mut self) {
        self.frames_dropped += 1;
    }

    /// Update bitrate
    pub fn update_bitrate(&mut self, bitrate_kbps: u32) {
        self.bitrate_kbps = bitrate_kbps;
    }

    /// Update FPS
    pub fn update_fps(&mut self, fps: u32) {
        self.fps = fps;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_encoder_pipeline_creation() {
        let (frame_tx, frame_rx) = mpsc::channel(10);
        let (rtp_tx, _rtp_rx) = mpsc::channel(10);

        let pipeline = VideoEncoderPipeline::new(VideoConfig::default(), frame_rx, rtp_tx);

        assert!(!pipeline.is_running());
    }

    #[tokio::test]
    async fn test_decoder_pipeline_creation() {
        let (rtp_tx, rtp_rx) = mpsc::channel(10);
        let (frame_tx, _frame_rx) = mpsc::channel(10);

        let pipeline = VideoDecoderPipeline::new(VideoConfig::default(), rtp_rx, frame_tx);

        assert!(!pipeline.is_running());
    }
}
