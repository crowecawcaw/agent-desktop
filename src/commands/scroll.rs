use anyhow::{Context, Result};

use crate::platform;
use crate::state::PerceptState;

const DEFAULT_SCROLL_AMOUNT: u32 = 3;

pub fn run_scroll(
    block_id: Option<u32>,
    direction: &str,
    amount: Option<u32>,
) -> Result<()> {
    // Validate direction
    match direction {
        "up" | "down" | "left" | "right" => {}
        _ => anyhow::bail!(
            "Invalid direction '{}'. Must be one of: up, down, left, right",
            direction
        ),
    }

    // If block specified, move mouse to it first
    if let Some(id) = block_id {
        let state = PerceptState::load()?;
        let block = state.get_block(id)?;
        let (x, y) = block.bbox.center_pixels(state.image_width, state.image_height);

        platform::move_mouse(x, y).context(format!(
            "Failed to move mouse to block {} at ({}, {})",
            id, x, y
        ))?;
    }

    let scroll_amount = amount.unwrap_or(DEFAULT_SCROLL_AMOUNT);
    platform::scroll(direction, scroll_amount)?;

    match block_id {
        Some(id) => println!(
            "Scrolled {} {} clicks in block {}",
            direction, scroll_amount, id
        ),
        None => println!("Scrolled {} {} clicks", direction, scroll_amount),
    }

    Ok(())
}
