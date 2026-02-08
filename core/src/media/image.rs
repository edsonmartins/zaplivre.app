//! Image processing utilities
//!
//! Compression, resizing, and thumbnail generation for images.

use image::{codecs::jpeg::JpegEncoder, DynamicImage, GenericImageView};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ImageProcessingError {
    #[error("Failed to decode image: {0}")]
    DecodeError(String),

    #[error("Failed to encode image: {0}")]
    EncodeError(String),

    #[error("Unsupported image format")]
    UnsupportedFormat,

    #[error("Invalid dimensions: {0}")]
    InvalidDimensions(String),

    #[error("IO error: {0}")]
    IoError(String),
}

impl From<image::ImageError> for ImageProcessingError {
    fn from(err: image::ImageError) -> Self {
        match err {
            image::ImageError::Decoding(_) => {
                ImageProcessingError::DecodeError(err.to_string())
            }
            image::ImageError::Encoding(_) => {
                ImageProcessingError::EncodeError(err.to_string())
            }
            image::ImageError::Unsupported(_) => ImageProcessingError::UnsupportedFormat,
            _ => ImageProcessingError::IoError(err.to_string()),
        }
    }
}

impl From<std::io::Error> for ImageProcessingError {
    fn from(err: std::io::Error) -> Self {
        ImageProcessingError::IoError(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, ImageProcessingError>;

/// Compress an image to JPEG with specified quality
///
/// # Arguments
/// * `input` - Image data (any supported format: JPEG, PNG, WebP, etc.)
/// * `quality` - JPEG quality (1-100, where 100 is best quality)
///
/// # Returns
/// Compressed JPEG image data
pub fn compress_image(input: &[u8], quality: u8) -> Result<Vec<u8>> {
    if quality == 0 || quality > 100 {
        return Err(ImageProcessingError::InvalidDimensions(
            "Quality must be between 1 and 100".to_string(),
        ));
    }

    // Load image from bytes
    let img = image::load_from_memory(input)?;

    // Encode to JPEG with specified quality
    let mut output = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut output, quality);
    encoder.encode_image(&img)?;

    Ok(output)
}

/// Resize an image to fit within max dimensions while preserving aspect ratio
///
/// # Arguments
/// * `input` - Image data
/// * `max_width` - Maximum width in pixels
/// * `max_height` - Maximum height in pixels
///
/// # Returns
/// Resized image data (JPEG format)
pub fn resize_image(input: &[u8], max_width: u32, max_height: u32) -> Result<Vec<u8>> {
    if max_width == 0 || max_height == 0 {
        return Err(ImageProcessingError::InvalidDimensions(
            "Max dimensions must be greater than 0".to_string(),
        ));
    }

    // Load image
    let img = image::load_from_memory(input)?;

    // Calculate new dimensions preserving aspect ratio
    let (width, height) = img.dimensions();
    let (new_width, new_height) = calculate_resize_dimensions(width, height, max_width, max_height);

    // Resize using Lanczos3 filter (high quality)
    let resized = img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3);

    // Encode to JPEG with good quality (85)
    let mut output = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut output, 85);
    encoder.encode_image(&resized)?;

    Ok(output)
}

/// Generate a square thumbnail from an image
///
/// # Arguments
/// * `input` - Image data
/// * `size` - Thumbnail size (width and height in pixels)
///
/// # Returns
/// Thumbnail image data (JPEG format)
pub fn generate_thumbnail(input: &[u8], size: u32) -> Result<Vec<u8>> {
    if size == 0 {
        return Err(ImageProcessingError::InvalidDimensions(
            "Thumbnail size must be greater than 0".to_string(),
        ));
    }

    // Load image
    let img = image::load_from_memory(input)?;

    // Create square thumbnail by cropping to center
    let thumbnail = create_square_thumbnail(img, size);

    // Encode to JPEG with good quality (80)
    let mut output = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut output, 80);
    encoder.encode_image(&thumbnail)?;

    Ok(output)
}

/// Calculate new dimensions that fit within max dimensions while preserving aspect ratio
fn calculate_resize_dimensions(
    width: u32,
    height: u32,
    max_width: u32,
    max_height: u32,
) -> (u32, u32) {
    if width <= max_width && height <= max_height {
        // No resize needed
        return (width, height);
    }

    let width_ratio = max_width as f32 / width as f32;
    let height_ratio = max_height as f32 / height as f32;

    // Use the smaller ratio to ensure both dimensions fit
    let ratio = width_ratio.min(height_ratio);

    let new_width = (width as f32 * ratio) as u32;
    let new_height = (height as f32 * ratio) as u32;

    (new_width, new_height)
}

/// Create a square thumbnail by cropping to center
fn create_square_thumbnail(img: DynamicImage, size: u32) -> DynamicImage {
    let (width, height) = img.dimensions();

    if width == height {
        // Already square, just resize
        return img.resize_exact(size, size, image::imageops::FilterType::Lanczos3);
    }

    // Determine crop dimensions (center crop)
    let crop_size = width.min(height);
    let x_offset = (width.saturating_sub(crop_size)) / 2;
    let y_offset = (height.saturating_sub(crop_size)) / 2;

    // Crop to square
    let cropped = img.crop_imm(x_offset, y_offset, crop_size, crop_size);

    // Resize to target size
    cropped.resize_exact(size, size, image::imageops::FilterType::Lanczos3)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: Create a test image (100x100 red square PNG)
    fn create_test_image() -> Vec<u8> {
        let img = DynamicImage::ImageRgb8(image::RgbImage::from_fn(100, 100, |_, _| {
            image::Rgb([255, 0, 0]) // Red
        }));

        let mut output = Vec::new();
        img.write_to(&mut Cursor::new(&mut output), ImageFormat::Png)
            .unwrap();
        output
    }

    // Helper: Create a rectangular test image (200x100)
    fn create_rectangular_image() -> Vec<u8> {
        let img = DynamicImage::ImageRgb8(image::RgbImage::from_fn(200, 100, |_, _| {
            image::Rgb([0, 255, 0]) // Green
        }));

        let mut output = Vec::new();
        img.write_to(&mut Cursor::new(&mut output), ImageFormat::Png)
            .unwrap();
        output
    }

    #[test]
    fn test_compress_image_reduces_size() {
        let input = create_test_image();

        // First convert to high-quality JPEG
        let high_quality = compress_image(&input, 95).unwrap();
        let high_quality_size = high_quality.len();

        // Then compress with low quality
        let low_quality = compress_image(&high_quality, 50).unwrap();
        let low_quality_size = low_quality.len();

        // Low quality should be smaller than high quality
        assert!(
            low_quality_size < high_quality_size,
            "Low quality ({}) should be less than high quality ({})",
            low_quality_size,
            high_quality_size
        );

        // Verify it's valid JPEG
        let decoded = image::load_from_memory(&low_quality).unwrap();
        assert_eq!(decoded.dimensions(), (100, 100));
    }

    #[test]
    fn test_compress_image_quality_levels() {
        let input = create_test_image();

        let high_quality = compress_image(&input, 95).unwrap();
        let low_quality = compress_image(&input, 20).unwrap();

        // Lower quality should produce smaller file
        assert!(
            low_quality.len() < high_quality.len(),
            "Low quality ({}) should be smaller than high quality ({})",
            low_quality.len(),
            high_quality.len()
        );
    }

    #[test]
    fn test_compress_image_invalid_quality() {
        let input = create_test_image();

        // Quality 0 should error
        assert!(compress_image(&input, 0).is_err());

        // Quality > 100 should error
        assert!(compress_image(&input, 101).is_err());
    }

    #[test]
    fn test_resize_image_preserves_aspect_ratio() {
        let input = create_rectangular_image(); // 200x100

        // Resize to max 100x100
        let resized = resize_image(&input, 100, 100).unwrap();
        let decoded = image::load_from_memory(&resized).unwrap();
        let (width, height) = decoded.dimensions();

        // Should be 100x50 (aspect ratio 2:1 preserved)
        assert_eq!(width, 100, "Width should be 100");
        assert_eq!(height, 50, "Height should be 50 to preserve 2:1 ratio");
    }

    #[test]
    fn test_resize_image_no_upscale() {
        let input = create_test_image(); // 100x100

        // Request larger size
        let resized = resize_image(&input, 200, 200).unwrap();
        let decoded = image::load_from_memory(&resized).unwrap();
        let (width, height) = decoded.dimensions();

        // Should stay at original size (no upscaling)
        assert_eq!(width, 100);
        assert_eq!(height, 100);
    }

    #[test]
    fn test_resize_image_invalid_dimensions() {
        let input = create_test_image();

        // Max width 0 should error
        assert!(resize_image(&input, 0, 100).is_err());

        // Max height 0 should error
        assert!(resize_image(&input, 100, 0).is_err());
    }

    #[test]
    fn test_generate_thumbnail_square() {
        let input = create_test_image(); // 100x100

        let thumbnail = generate_thumbnail(&input, 64).unwrap();
        let decoded = image::load_from_memory(&thumbnail).unwrap();
        let (width, height) = decoded.dimensions();

        // Should be exactly 64x64
        assert_eq!(width, 64);
        assert_eq!(height, 64);
    }

    #[test]
    fn test_generate_thumbnail_from_rectangle() {
        let input = create_rectangular_image(); // 200x100

        let thumbnail = generate_thumbnail(&input, 48).unwrap();
        let decoded = image::load_from_memory(&thumbnail).unwrap();
        let (width, height) = decoded.dimensions();

        // Should be exactly 48x48 (center cropped)
        assert_eq!(width, 48);
        assert_eq!(height, 48);
    }

    #[test]
    fn test_generate_thumbnail_invalid_size() {
        let input = create_test_image();

        // Size 0 should error
        assert!(generate_thumbnail(&input, 0).is_err());
    }

    #[test]
    fn test_calculate_resize_dimensions() {
        // Landscape image 200x100 → max 100x100
        let (w, h) = calculate_resize_dimensions(200, 100, 100, 100);
        assert_eq!(w, 100);
        assert_eq!(h, 50); // Preserves 2:1 ratio

        // Portrait image 100x200 → max 100x100
        let (w, h) = calculate_resize_dimensions(100, 200, 100, 100);
        assert_eq!(w, 50); // Preserves 1:2 ratio
        assert_eq!(h, 100);

        // Already small image 50x50 → max 100x100
        let (w, h) = calculate_resize_dimensions(50, 50, 100, 100);
        assert_eq!(w, 50); // No resize needed
        assert_eq!(h, 50);

        // Exact fit 100x100 → max 100x100
        let (w, h) = calculate_resize_dimensions(100, 100, 100, 100);
        assert_eq!(w, 100);
        assert_eq!(h, 100);
    }

    #[test]
    fn test_decode_various_formats() {
        // Test PNG
        let png_input = create_test_image();
        let compressed = compress_image(&png_input, 80).unwrap();
        assert!(compressed.len() > 0);

        // Test JPEG (compress already compressed JPEG)
        let jpeg_input = compress_image(&png_input, 90).unwrap();
        let recompressed = compress_image(&jpeg_input, 70).unwrap();
        assert!(recompressed.len() > 0);
        assert!(recompressed.len() < jpeg_input.len()); // Further compressed
    }
}
