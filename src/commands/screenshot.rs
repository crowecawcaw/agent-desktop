use anyhow::{Context, Result};
use xa11y::{App, AppExt, Role};

pub fn run_screenshot(output_path: &str, scale: f64, app: Option<&str>, pid: Option<u32>) -> Result<()> {
    let shot = if app.is_some() || pid.is_some() {
        let xa_app = match (pid, app) {
            (Some(p), _) => App::by_pid(p).map_err(|e| anyhow::anyhow!("{}", e))?,
            (None, Some(name)) => App::by_name(name).map_err(|e| anyhow::anyhow!("{}", e))?,
            _ => unreachable!(),
        };
        let window = xa_app
            .children()
            .map_err(|e| anyhow::anyhow!("{}", e))?
            .into_iter()
            .find(|e| matches!(e.data().role, Role::Window))
            .ok_or_else(|| anyhow::anyhow!("No window found for the specified app"))?;
        xa11y::screenshot_element(&window)
            .map_err(|e| anyhow::anyhow!("Screenshot failed: {}", e))?
    } else {
        xa11y::screenshot().map_err(|e| anyhow::anyhow!("Screenshot failed: {}", e))?
    };

    if (scale - 1.0).abs() > 1e-9 {
        let rgba = image::RgbaImage::from_raw(shot.width, shot.height, shot.pixels)
            .ok_or_else(|| anyhow::anyhow!("Invalid screenshot pixel data"))?;
        let new_w = ((shot.width as f64) * scale) as u32;
        let new_h = ((shot.height as f64) * scale) as u32;
        image::DynamicImage::ImageRgba8(rgba)
            .resize_exact(new_w, new_h, image::imageops::FilterType::Lanczos3)
            .save(output_path)
            .context("Failed to save scaled screenshot")?;
    } else {
        shot.save_png(output_path)
            .map_err(|e| anyhow::anyhow!("Failed to save screenshot: {}", e))?;
    }

    println!("Screenshot saved to {}", output_path);
    Ok(())
}
