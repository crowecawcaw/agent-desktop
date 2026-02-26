use anyhow::{Context, Result};
use std::path::Path;

use crate::commands::annotate;
use crate::platform;

pub fn run_screenshot(
    output_path: &str,
    scale: Option<f64>,
    no_annotations: bool,
    box_threshold: f32,
    iou_threshold: f64,
    captions: bool,
) -> Result<()> {
    // Capture screenshot to a temp file first
    let temp_path = if no_annotations {
        output_path.to_string()
    } else {
        let temp = tempfile::NamedTempFile::new().context("Failed to create temp file")?;
        temp.path().to_string_lossy().to_string()
    };

    platform::take_screenshot(&temp_path)?;

    // Apply scaling if requested
    if let Some(factor) = scale {
        let img = image::open(&temp_path).context("Failed to open captured screenshot")?;
        let (w, h) = (img.width(), img.height());
        let new_w = (w as f64 * factor) as u32;
        let new_h = (h as f64 * factor) as u32;
        let resized = img.resize_exact(
            new_w,
            new_h,
            image::imageops::FilterType::Lanczos3,
        );
        resized
            .save(&temp_path)
            .context("Failed to save scaled screenshot")?;
    }

    if no_annotations {
        println!("Screenshot saved to {}", output_path);
        return Ok(());
    }

    // Run annotation pipeline
    let result = annotate::run_annotate(
        Path::new(&temp_path),
        Path::new(output_path),
        box_threshold,
        iou_threshold,
        captions,
    )?;

    println!(
        "Annotated screenshot saved to {} ({} blocks detected)",
        output_path,
        result.blocks.len()
    );

    // Clean up temp file if different from output
    if temp_path != output_path {
        let _ = std::fs::remove_file(&temp_path);
    }

    Ok(())
}
