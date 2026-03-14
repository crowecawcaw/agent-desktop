use anyhow::{Context, Result};

use crate::platform;

pub fn run_screenshot(output_path: &str, scale: f64, app: Option<&str>, pid: Option<u32>) -> Result<()> {
    if app.is_some() || pid.is_some() {
        platform::take_screenshot_window(output_path, app, pid)?;
    } else {
        platform::take_screenshot(output_path)?;
    }

    if (scale - 1.0).abs() > 1e-9 {
        let img = image::open(output_path).context("Failed to open captured screenshot")?;
        let (w, h) = (img.width(), img.height());
        let new_w = (w as f64 * scale) as u32;
        let new_h = (h as f64 * scale) as u32;
        let resized = img.resize_exact(new_w, new_h, image::imageops::FilterType::Lanczos3);
        resized
            .save(output_path)
            .context("Failed to save scaled screenshot")?;
    }

    println!("Screenshot saved to {}", output_path);
    Ok(())
}
