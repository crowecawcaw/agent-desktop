use anyhow::Result;

use crate::platform::accessibility;

pub fn run_interact(
    element_id: u32,
    action: &str,
    value: Option<&str>,
) -> Result<String> {
    accessibility::perform_action(element_id, action, value)?;

    let msg = match value {
        Some(v) => format!(
            "Performed '{}' on element {} with value '{}'",
            action, element_id, v
        ),
        None => format!("Performed '{}' on element {}", action, element_id),
    };

    Ok(msg)
}
