use anyhow::{Context, Result};
use std::path::Path;

use crate::commands::annotate;
use crate::platform;

pub fn run_screenshot(
    output_path: &str,
    scale: f64,
    no_annotations: bool,
    box_threshold: f32,
    iou_threshold: f64,
    max_blocks: Option<u32>,
    debug: bool,
) -> Result<()> {
    // Capture screenshot to a temp file first.
    let temp_path = if no_annotations {
        output_path.to_string()
    } else {
        std::env::temp_dir()
            .join(format!("percept_{}.png", std::process::id()))
            .to_string_lossy()
            .to_string()
    };

    platform::take_screenshot(&temp_path)?;

    // Apply scaling
    if (scale - 1.0).abs() > 1e-9 {
        let img = image::open(&temp_path).context("Failed to open captured screenshot")?;
        let (w, h) = (img.width(), img.height());
        let new_w = (w as f64 * scale) as u32;
        let new_h = (h as f64 * scale) as u32;
        let resized = img.resize_exact(new_w, new_h, image::imageops::FilterType::Lanczos3);
        resized.save(&temp_path).context("Failed to save scaled screenshot")?;
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
        max_blocks,
        debug,
    )?;

    println!(
        "Annotated screenshot saved to {} ({} blocks detected)",
        output_path,
        result.blocks.len()
    );

    // Clean up temp file
    if temp_path != output_path {
        let _ = std::fs::remove_file(&temp_path);
    }

    Ok(())
}
