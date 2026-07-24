//! Media processing module
//!
//! Image compression, resizing, thumbnail generation, and other media utilities.

pub mod envelope;
pub mod image;

pub use envelope::{MediaEnvelope, MEDIA_ENVELOPE_PREFIX};
pub use image::{compress_image, generate_thumbnail, resize_image, ImageProcessingError};

pub fn media_summary(
    media_type: &str,
    file_name: Option<&str>,
    duration_seconds: Option<i32>,
) -> String {
    match media_type {
        "image" => format!(
            "[Image{}]",
            file_name
                .map(|name| format!(": {}", name))
                .unwrap_or_default()
        ),
        "video" => format!(
            "[Video{}]",
            duration_seconds
                .map(|d| format!(": {}s", d))
                .unwrap_or_default()
        ),
        "voice" | "voice_message" => format!(
            "[Voice{}]",
            duration_seconds
                .map(|d| format!(": {}s", d))
                .unwrap_or_default()
        ),
        "audio" => "[Audio]".to_string(),
        "document" => format!(
            "[File{}]",
            file_name
                .map(|name| format!(": {}", name))
                .unwrap_or_default()
        ),
        other => format!("[{}]", other),
    }
}
