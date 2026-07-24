//! Opus Audio Codec
//!
//! Handles encoding and decoding of audio using Opus codec.

use super::{audio::Sample, Result, VoipError};
use opus::{Application, Channels, Decoder, Encoder};

/// Opus codec configuration
#[derive(Debug, Clone)]
pub struct OpusConfig {
    /// Sample rate (Hz) - Opus supports 8000, 12000, 16000, 24000, 48000
    pub sample_rate: u32,

    /// Number of channels (1 = mono, 2 = stereo)
    pub channels: Channels,

    /// Application type (VoIP optimized)
    pub application: Application,

    /// Bitrate (bps) - 6000 to 510000, recommended 24000-40000 for VoIP
    pub bitrate: i32,

    /// Frame duration (ms) - 2.5, 5, 10, 20, 40, 60
    pub frame_duration_ms: u32,
}

impl Default for OpusConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            channels: Channels::Mono,
            application: Application::Voip,
            bitrate: 24000,        // 24 kbps - good quality for voice
            frame_duration_ms: 20, // 20ms frames (960 samples at 48kHz)
        }
    }
}

impl OpusConfig {
    /// Get frame size in samples
    pub fn frame_size(&self) -> usize {
        (self.sample_rate as usize * self.frame_duration_ms as usize) / 1000
    }

    /// Maximum encoded packet size (recommended by Opus)
    pub fn max_packet_size(&self) -> usize {
        4000 // Opus spec recommends 4000 bytes max
    }
}

/// Opus encoder for compressing audio
pub struct OpusEncoder {
    encoder: Encoder,
    config: OpusConfig,
    frame_buffer: Vec<Sample>,
}

impl OpusEncoder {
    /// Create a new Opus encoder
    pub fn new(config: OpusConfig) -> Result<Self> {
        let mut encoder = Encoder::new(config.sample_rate, config.channels, config.application)
            .map_err(|e| {
                VoipError::CodecError(format!("Failed to create Opus encoder: {:?}", e))
            })?;

        // Set bitrate
        encoder
            .set_bitrate(opus::Bitrate::Bits(config.bitrate))
            .map_err(|e| VoipError::CodecError(format!("Failed to set bitrate: {:?}", e)))?;

        tracing::info!(
            "✅ Opus encoder created: {}Hz, {:?}, {} kbps, {}ms frames",
            config.sample_rate,
            config.channels,
            config.bitrate / 1000,
            config.frame_duration_ms
        );

        Ok(Self {
            encoder,
            config,
            frame_buffer: Vec::new(),
        })
    }

    /// Encode audio samples to Opus packet
    ///
    /// Returns Some(packet) when a full frame is ready, None if buffering
    pub fn encode(&mut self, samples: &[Sample]) -> Result<Option<Vec<u8>>> {
        // Add samples to buffer
        self.frame_buffer.extend_from_slice(samples);

        // Check if we have a full frame
        let frame_size = self.config.frame_size();
        if self.frame_buffer.len() < frame_size {
            return Ok(None); // Not enough samples yet
        }

        // Extract one frame
        let frame: Vec<f32> = self.frame_buffer.drain(..frame_size).collect();

        // Encode frame
        let mut output = vec![0u8; self.config.max_packet_size()];
        let len = self
            .encoder
            .encode_float(&frame, &mut output)
            .map_err(|e| VoipError::CodecError(format!("Opus encoding failed: {:?}", e)))?;

        output.truncate(len);

        tracing::trace!("🎵 Encoded {} samples to {} bytes", frame_size, len);

        Ok(Some(output))
    }

    /// Get number of buffered samples
    pub fn buffered_samples(&self) -> usize {
        self.frame_buffer.len()
    }

    /// Get frame size in samples
    pub fn frame_size(&self) -> usize {
        self.config.frame_size()
    }

    /// Flush any remaining samples (padding with silence if needed)
    pub fn flush(&mut self) -> Result<Option<Vec<u8>>> {
        let frame_size = self.config.frame_size();

        if self.frame_buffer.is_empty() {
            return Ok(None);
        }

        // Pad with silence to complete frame
        while self.frame_buffer.len() < frame_size {
            self.frame_buffer.push(0.0);
        }

        // Encode final frame
        self.encode(&[])
    }
}

/// Opus decoder for decompressing audio
pub struct OpusDecoder {
    decoder: Decoder,
    config: OpusConfig,
}

impl OpusDecoder {
    /// Create a new Opus decoder
    pub fn new(config: OpusConfig) -> Result<Self> {
        let decoder = Decoder::new(config.sample_rate, config.channels).map_err(|e| {
            VoipError::CodecError(format!("Failed to create Opus decoder: {:?}", e))
        })?;

        tracing::info!(
            "✅ Opus decoder created: {}Hz, {:?}",
            config.sample_rate,
            config.channels
        );

        Ok(Self { decoder, config })
    }

    /// Decode Opus packet to audio samples
    pub fn decode(&mut self, packet: &[u8]) -> Result<Vec<Sample>> {
        let frame_size = self.config.frame_size();
        let mut output = vec![0.0f32; frame_size];

        let decoded_samples = self
            .decoder
            .decode_float(packet, &mut output, false)
            .map_err(|e| VoipError::CodecError(format!("Opus decoding failed: {:?}", e)))?;

        output.truncate(decoded_samples);

        tracing::trace!(
            "🎵 Decoded {} bytes to {} samples",
            packet.len(),
            decoded_samples
        );

        Ok(output)
    }

    /// Decode with forward error correction (FEC)
    ///
    /// Used when a packet is lost - generates plausible audio from previous packet
    pub fn decode_fec(&mut self, packet: Option<&[u8]>) -> Result<Vec<Sample>> {
        let frame_size = self.config.frame_size();
        let mut output = vec![0.0f32; frame_size];

        let decoded_samples = if let Some(pkt) = packet {
            self.decoder
                .decode_float(pkt, &mut output, true)
                .map_err(|e| VoipError::CodecError(format!("Opus FEC decoding failed: {:?}", e)))?
        } else {
            // Generate silence for lost packet
            frame_size
        };

        output.truncate(decoded_samples);

        Ok(output)
    }
}

/// Opus codec pair (encoder + decoder)
pub struct OpusCodec {
    pub encoder: OpusEncoder,
    pub decoder: OpusDecoder,
}

impl OpusCodec {
    /// Create a new Opus codec pair with default config
    pub fn new() -> Result<Self> {
        Self::with_config(OpusConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(config: OpusConfig) -> Result<Self> {
        let encoder = OpusEncoder::new(config.clone())?;
        let decoder = OpusDecoder::new(config)?;

        Ok(Self { encoder, decoder })
    }
}

impl Default for OpusCodec {
    fn default() -> Self {
        Self::new().expect("Failed to create default Opus codec")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opus_config_default() {
        let config = OpusConfig::default();
        assert_eq!(config.sample_rate, 48000);
        assert_eq!(config.channels, Channels::Mono);
        assert_eq!(config.bitrate, 24000);
        assert_eq!(config.frame_duration_ms, 20);
        assert_eq!(config.frame_size(), 960); // 48000 * 20 / 1000
    }

    #[test]
    fn test_opus_encoder_creation() {
        let result = OpusEncoder::new(OpusConfig::default());
        assert!(result.is_ok());
    }

    #[test]
    fn test_opus_decoder_creation() {
        let result = OpusDecoder::new(OpusConfig::default());
        assert!(result.is_ok());
    }

    #[test]
    fn test_opus_codec_creation() {
        let result = OpusCodec::new();
        assert!(result.is_ok());
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let mut codec = OpusCodec::new().unwrap();

        // Generate test signal (sine wave at 440 Hz)
        let sample_rate = 48000.0;
        let frequency = 440.0;
        let duration = 0.02; // 20ms
        let num_samples = (sample_rate * duration) as usize;

        let mut samples = Vec::new();
        for i in 0..num_samples {
            let t = i as f32 / sample_rate;
            let sample = (2.0 * std::f32::consts::PI * frequency * t).sin() * 0.5;
            samples.push(sample);
        }

        // Encode
        let encoded = codec.encoder.encode(&samples).unwrap();
        assert!(encoded.is_some(), "Should encode full frame");

        let packet = encoded.unwrap();
        assert!(!packet.is_empty(), "Encoded packet should not be empty");
        assert!(packet.len() < 4000, "Packet should be within max size");

        // Decode
        let decoded = codec.decoder.decode(&packet).unwrap();
        assert_eq!(
            decoded.len(),
            num_samples,
            "Should decode same number of samples"
        );

        // Verify decoded signal is similar (not exact due to lossy compression)
        // Just check that we got reasonable values back
        for sample in &decoded {
            assert!(
                sample.abs() <= 1.0,
                "Decoded sample should be in valid range"
            );
        }
    }

    #[test]
    fn test_encoder_buffering() {
        let mut encoder = OpusEncoder::new(OpusConfig::default()).unwrap();

        // Send partial frame
        let samples = vec![0.0f32; 480]; // Half frame (960 / 2)
        let result = encoder.encode(&samples).unwrap();
        assert!(result.is_none(), "Should buffer partial frame");
        assert_eq!(encoder.buffered_samples(), 480);

        // Send rest of frame
        let more_samples = vec![0.0f32; 480];
        let result = encoder.encode(&more_samples).unwrap();
        assert!(result.is_some(), "Should encode complete frame");
        assert_eq!(encoder.buffered_samples(), 0);
    }

    #[test]
    fn test_encoder_flush() {
        let mut encoder = OpusEncoder::new(OpusConfig::default()).unwrap();

        // Send partial frame
        let samples = vec![0.0f32; 100];
        encoder.encode(&samples).unwrap();

        // Flush should pad and encode
        let result = encoder.flush().unwrap();
        assert!(result.is_some(), "Flush should produce packet");
    }

    #[test]
    fn test_decoder_fec() {
        let mut codec = OpusCodec::new().unwrap();

        // Encode a frame
        let samples = vec![0.5f32; 960];
        let packet = codec.encoder.encode(&samples).unwrap().unwrap();

        // Decode with FEC (simulating packet loss)
        let decoded = codec.decoder.decode_fec(Some(&packet)).unwrap();
        assert_eq!(decoded.len(), 960);

        // Decode without packet (pure loss concealment)
        let decoded_loss = codec.decoder.decode_fec(None).unwrap();
        assert_eq!(decoded_loss.len(), 960);
    }

    #[test]
    fn test_different_bitrates() {
        let configs = vec![
            (8000, "8 kbps - minimum"),
            (16000, "16 kbps - low"),
            (24000, "24 kbps - medium"),
            (40000, "40 kbps - high"),
            (64000, "64 kbps - maximum"),
        ];

        for (bitrate, desc) in configs {
            let mut config = OpusConfig::default();
            config.bitrate = bitrate;

            let result = OpusEncoder::new(config);
            assert!(result.is_ok(), "Should create encoder with {}", desc);
        }
    }
}
