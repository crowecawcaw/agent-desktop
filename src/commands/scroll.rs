use anyhow::Result;

use crate::platform;
use crate::state::AppState;

const DEFAULT_SCROLL_AMOUNT: u32 = 3;

pub fn run_scroll(
    element_id: Option<u32>,
    direction: &str,
    amount: Option<u32>,
) -> Result<()> {
    match direction {
        "up" | "down" | "left" | "right" => {}
        _ => anyhow::bail!(
            "Invalid direction '{}'. Must be one of: up, down, left, right",
            direction
        ),
    }

    // Resolve element center to use as the scroll target point.
    let at = if let Some(eid) = element_id {
        let state = AppState::load()?;
        let elem = state.get_element(eid)?;
        elem.bounds.as_ref().map(|b| b.center())
    } else {
        None
    };

    let scroll_amount = amount.unwrap_or(DEFAULT_SCROLL_AMOUNT);
    platform::scroll(direction, scroll_amount, at)?;

    if let Some(eid) = element_id {
        println!("Scrolled {} {} clicks in element {}", direction, scroll_amount, eid);
    } else {
        println!("Scrolled {} {} clicks", direction, scroll_amount);
    }

    Ok(())
}
