use anyhow::Result;
use image::{DynamicImage, GenericImageView, RgbImage};
use ndarray::Array4;

/// Letterbox resize: fit image into target size maintaining aspect ratio, padding with gray
pub fn letterbox_resize(img: &DynamicImage, target_size: u32) -> (RgbImage, LetterboxInfo) {
    let (orig_w, orig_h) = img.dimensions();
    let scale = (target_size as f64 / orig_w as f64).min(target_size as f64 / orig_h as f64);
    let new_w = (orig_w as f64 * scale) as u32;
    let new_h = (orig_h as f64 * scale) as u32;

    let resized = img.resize_exact(new_w, new_h, image::imageops::FilterType::Lanczos3);

    let mut padded = RgbImage::from_pixel(target_size, target_size, image::Rgb([114, 114, 114]));
    let pad_x = (target_size - new_w) / 2;
    let pad_y = (target_size - new_h) / 2;

    image::imageops::overlay(
        &mut padded,
        &resized.to_rgb8(),
        pad_x as i64,
        pad_y as i64,
    );

    let info = LetterboxInfo {
        scale,
        pad_x,
        pad_y,
        orig_w,
        orig_h,
        target_size,
    };

    (padded, info)
}

/// Information about letterbox padding for coordinate conversion
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LetterboxInfo {
    pub scale: f64,
    pub pad_x: u32,
    pub pad_y: u32,
    pub orig_w: u32,
    pub orig_h: u32,
    pub target_size: u32,
}

impl LetterboxInfo {
    /// Convert coordinates from model space (target_size x target_size) to normalized [0,1]
    pub fn to_normalized(&self, x1: f64, y1: f64, x2: f64, y2: f64) -> (f64, f64, f64, f64) {
        let x1 = ((x1 - self.pad_x as f64) / self.scale) / self.orig_w as f64;
        let y1 = ((y1 - self.pad_y as f64) / self.scale) / self.orig_h as f64;
        let x2 = ((x2 - self.pad_x as f64) / self.scale) / self.orig_w as f64;
        let y2 = ((y2 - self.pad_y as f64) / self.scale) / self.orig_h as f64;
        (x1.clamp(0.0, 1.0), y1.clamp(0.0, 1.0), x2.clamp(0.0, 1.0), y2.clamp(0.0, 1.0))
    }
}

/// Convert RGB image to CHW float32 tensor normalized to [0, 1]
pub fn image_to_chw_tensor(img: &RgbImage) -> Result<Array4<f32>> {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let mut tensor = Array4::<f32>::zeros((1, 3, h, w));

    for y in 0..h {
        for x in 0..w {
            let pixel = img.get_pixel(x as u32, y as u32);
            tensor[[0, 0, y, x]] = pixel[0] as f32 / 255.0;
            tensor[[0, 1, y, x]] = pixel[1] as f32 / 255.0;
            tensor[[0, 2, y, x]] = pixel[2] as f32 / 255.0;
        }
    }

    Ok(tensor)
}

/// Convert RGB image to CHW float32 tensor with ImageNet normalization
pub fn image_to_chw_normalized(img: &RgbImage) -> Result<Array4<f32>> {
    let mean = [0.485f32, 0.456, 0.406];
    let std = [0.229f32, 0.224, 0.225];

    let (w, h) = (img.width() as usize, img.height() as usize);
    let mut tensor = Array4::<f32>::zeros((1, 3, h, w));

    for y in 0..h {
        for x in 0..w {
            let pixel = img.get_pixel(x as u32, y as u32);
            tensor[[0, 0, y, x]] = (pixel[0] as f32 / 255.0 - mean[0]) / std[0];
            tensor[[0, 1, y, x]] = (pixel[1] as f32 / 255.0 - mean[1]) / std[1];
            tensor[[0, 2, y, x]] = (pixel[2] as f32 / 255.0 - mean[2]) / std[2];
        }
    }

    Ok(tensor)
}

/// Resize image maintaining aspect ratio with max side constraint
pub fn resize_max_side(img: &DynamicImage, max_side: u32) -> (RgbImage, f64) {
    let (w, h) = img.dimensions();
    let scale = if w.max(h) > max_side {
        max_side as f64 / w.max(h) as f64
    } else {
        1.0
    };
    let new_w = (w as f64 * scale) as u32;
    let new_h = (h as f64 * scale) as u32;
    // Round to multiples of 32 for PaddleOCR
    let new_w = ((new_w + 31) / 32) * 32;
    let new_h = ((new_h + 31) / 32) * 32;

    let resized = img
        .resize_exact(new_w, new_h, image::imageops::FilterType::Lanczos3)
        .to_rgb8();
    (resized, scale)
}

/// Resize and pad for OCR recognition (fixed height, variable width)
pub fn resize_for_ocr_rec(crop: &RgbImage, target_height: u32, max_width: u32) -> RgbImage {
    let (w, h) = (crop.width(), crop.height());
    let scale = target_height as f64 / h as f64;
    let new_w = ((w as f64 * scale) as u32).min(max_width);

    let resized = image::imageops::resize(
        crop,
        new_w,
        target_height,
        image::imageops::FilterType::Lanczos3,
    );

    if new_w < max_width {
        let mut padded =
            RgbImage::from_pixel(max_width, target_height, image::Rgb([0, 0, 0]));
        image::imageops::overlay(&mut padded, &resized, 0, 0);
        padded
    } else {
        resized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_letterbox_resize_square() {
        let img = DynamicImage::new_rgb8(640, 640);
        let (result, info) = letterbox_resize(&img, 640);
        assert_eq!(result.width(), 640);
        assert_eq!(result.height(), 640);
        assert!((info.scale - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_letterbox_resize_landscape() {
        let img = DynamicImage::new_rgb8(1280, 640);
        let (result, info) = letterbox_resize(&img, 640);
        assert_eq!(result.width(), 640);
        assert_eq!(result.height(), 640);
        assert!((info.scale - 0.5).abs() < 1e-6);
        assert!(info.pad_y > 0);
    }

    #[test]
    fn test_letterbox_coordinate_conversion() {
        let info = LetterboxInfo {
            scale: 0.5,
            pad_x: 0,
            pad_y: 160,
            orig_w: 1280,
            orig_h: 640,
            target_size: 640,
        };
        // A box at the center of model space
        let (nx1, ny1, nx2, ny2) = info.to_normalized(200.0, 260.0, 440.0, 380.0);
        // x1: (200 - 0) / 0.5 / 1280 = 400 / 1280 ≈ 0.3125
        assert!((nx1 - 0.3125).abs() < 1e-4);
        // y1: (260 - 160) / 0.5 / 640 = 200 / 640 ≈ 0.3125
        assert!((ny1 - 0.3125).abs() < 1e-4);
    }

    #[test]
    fn test_image_to_chw_tensor() {
        let img = RgbImage::from_pixel(2, 2, image::Rgb([255, 128, 0]));
        let tensor = image_to_chw_tensor(&img).unwrap();
        assert_eq!(tensor.shape(), &[1, 3, 2, 2]);
        assert!((tensor[[0, 0, 0, 0]] - 1.0).abs() < 1e-6); // R=255/255
        assert!((tensor[[0, 1, 0, 0]] - 128.0 / 255.0).abs() < 1e-3); // G
        assert!((tensor[[0, 2, 0, 0]] - 0.0).abs() < 1e-6); // B=0
    }

    #[test]
    fn test_image_to_chw_normalized() {
        let img = RgbImage::from_pixel(2, 2, image::Rgb([128, 128, 128]));
        let tensor = image_to_chw_normalized(&img).unwrap();
        assert_eq!(tensor.shape(), &[1, 3, 2, 2]);
        // (128/255 - 0.485) / 0.229
        let expected_r = (128.0 / 255.0 - 0.485) / 0.229;
        assert!((tensor[[0, 0, 0, 0]] - expected_r).abs() < 1e-3);
    }
}
