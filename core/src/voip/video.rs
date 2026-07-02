//! Video capture and codec support

use super::Result;

/// Video codecs supported
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum VideoCodec {
    /// H.264 - Primary codec (hardware accelerated on most devices)
    H264,
    /// VP8 - Fallback codec (software, good compatibility)
    VP8,
    /// VP9 - Future codec (better compression, hardware support growing)
    VP9,
}

impl VideoCodec {
    /// Get MIME type for the codec
    pub fn mime_type(&self) -> &'static str {
        match self {
            VideoCodec::H264 => "video/H264",
            VideoCodec::VP8 => "video/VP8",
            VideoCodec::VP9 => "video/VP9",
        }
    }

    /// Get SDP fmtp line for the codec
    pub fn fmtp_line(&self) -> String {
        match self {
            VideoCodec::H264 => "profile-level-id=42e01f;packetization-mode=1".to_string(),
            VideoCodec::VP8 | VideoCodec::VP9 => String::new(),
        }
    }

    /// Get RTP payload type (dynamic range 96-127)
    pub fn payload_type(&self) -> u8 {
        match self {
            VideoCodec::H264 => 96,
            VideoCodec::VP8 => 97,
            VideoCodec::VP9 => 98,
        }
    }

    /// Get RTP clock rate (standard for video is 90000 Hz)
    pub fn clock_rate(&self) -> u32 {
        90000 // Standard clock rate for all video codecs
    }
}

/// Video resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoResolution {
    pub width: u32,
    pub height: u32,
}

impl VideoResolution {
    /// VGA resolution (640x480)
    pub const VGA: Self = Self {
        width: 640,
        height: 480,
    };

    /// HD resolution (1280x720)
    pub const HD: Self = Self {
        width: 1280,
        height: 720,
    };

    /// Full HD resolution (1920x1080)
    pub const FHD: Self = Self {
        width: 1920,
        height: 1080,
    };

    /// Create custom resolution
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// Video configuration
#[derive(Debug, Clone)]
pub struct VideoConfig {
    /// Video codec to use
    pub codec: VideoCodec,
    /// Video resolution
    pub resolution: VideoResolution,
    /// Frames per second
    pub fps: u32,
    /// Target bitrate in kbps
    pub bitrate_kbps: u32,
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            codec: VideoCodec::H264,
            resolution: VideoResolution::VGA,
            fps: 24,
            bitrate_kbps: 500,
        }
    }
}

/// Pixel format for video frames
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// YUV 4:2:0 planar format
    YUV420,
    /// NV21 format (YUV 4:2:0 semi-planar)
    NV21,
    /// NV12 format (YUV 4:2:0 semi-planar)
    NV12,
    /// RGB 24-bit
    RGB24,
    /// RGBA 32-bit
    RGBA,
}

/// Video frame data
pub struct VideoFrame {
    /// Raw frame data
    pub data: Vec<u8>,
    /// Frame width in pixels
    pub width: u32,
    /// Frame height in pixels
    pub height: u32,
    /// Timestamp in microseconds
    pub timestamp_us: i64,
    /// Pixel format
    pub format: PixelFormat,
}

impl VideoFrame {
    /// Create a new video frame
    pub fn new(
        data: Vec<u8>,
        width: u32,
        height: u32,
        timestamp_us: i64,
        format: PixelFormat,
    ) -> Self {
        Self {
            data,
            width,
            height,
            timestamp_us,
            format,
        }
    }

    /// Get frame size in bytes
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

/// Camera position
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraPosition {
    /// Front-facing camera
    Front,
    /// Back-facing camera
    Back,
    /// External camera (USB, etc.)
    External,
}

/// Camera information
#[derive(Debug, Clone)]
pub struct CameraInfo {
    /// Camera identifier
    pub id: String,
    /// Camera name
    pub name: String,
    /// Camera position
    pub position: CameraPosition,
}

impl CameraInfo {
    /// Create new camera info
    pub fn new(id: String, name: String, position: CameraPosition) -> Self {
        Self { id, name, position }
    }
}

/// Platform-agnostic video capture trait
pub trait VideoCapture: Send + Sync {
    /// Start camera capture
    fn start(&mut self, config: VideoConfig) -> Result<()>;

    /// Stop camera capture
    fn stop(&mut self) -> Result<()>;

    /// Get next video frame (blocking)
    fn next_frame(&mut self) -> Result<VideoFrame>;

    /// Switch camera (front/back) - mobile only
    fn switch_camera(&mut self) -> Result<()>;

    /// Get available cameras
    fn list_cameras(&self) -> Result<Vec<CameraInfo>>;

    /// Check if currently capturing
    fn is_capturing(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_codec_mime_types() {
        assert_eq!(VideoCodec::H264.mime_type(), "video/H264");
        assert_eq!(VideoCodec::VP8.mime_type(), "video/VP8");
        assert_eq!(VideoCodec::VP9.mime_type(), "video/VP9");
    }

    #[test]
    fn test_video_codec_payload_types() {
        assert_eq!(VideoCodec::H264.payload_type(), 96);
        assert_eq!(VideoCodec::VP8.payload_type(), 97);
        assert_eq!(VideoCodec::VP9.payload_type(), 98);
    }

    #[test]
    fn test_video_resolution_constants() {
        assert_eq!(VideoResolution::VGA.width, 640);
        assert_eq!(VideoResolution::VGA.height, 480);
        assert_eq!(VideoResolution::HD.width, 1280);
        assert_eq!(VideoResolution::HD.height, 720);
    }

    #[test]
    fn test_video_config_default() {
        let config = VideoConfig::default();
        assert_eq!(config.codec, VideoCodec::H264);
        assert_eq!(config.resolution, VideoResolution::VGA);
        assert_eq!(config.fps, 24);
        assert_eq!(config.bitrate_kbps, 500);
    }

    #[test]
    fn test_video_frame_creation() {
        let data = vec![0u8; 640 * 480 * 3];
        let frame = VideoFrame::new(data.clone(), 640, 480, 1000000, PixelFormat::RGB24);
        assert_eq!(frame.width, 640);
        assert_eq!(frame.height, 480);
        assert_eq!(frame.size(), data.len());
    }
}
