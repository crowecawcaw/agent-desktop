use anyhow::{Context, Result};

use crate::platform;
use crate::state::PerceptState;

pub fn run_type(block_id: Option<u32>, text: &str) -> Result<()> {
    // If block specified, click it first
    if let Some(id) = block_id {
        let state = PerceptState::load()?;
        let block = state.get_block(id)?;
        let (x, y) = block.bbox.center_pixels(state.image_width, state.image_height);

        platform::click_at(x, y).context(format!(
            "Failed to click block {} at ({}, {})",
            id, x, y
        ))?;

        // Small delay to let the click register
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    platform::type_text(text).context("Failed to type text")?;

    match block_id {
        Some(id) => println!("Typed '{}' in block {}", text, id),
        None => println!("Typed '{}'", text),
    }

    Ok(())
}
