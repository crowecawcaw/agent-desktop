use anyhow::Result;

use crate::platform;

pub fn run_key(name: &str, modifiers: Option<&str>) -> Result<String> {
    let mods: Vec<&str> = match modifiers {
        Some(s) => s.split(',').map(|m| m.trim()).collect(),
        None => vec![],
    };

    // Validate modifier names
    for m in &mods {
        match *m {
            "cmd" | "command" | "shift" | "alt" | "option" | "ctrl" | "control" => {}
            other => anyhow::bail!(
                "Unknown modifier '{}'. Valid modifiers: cmd, shift, alt, ctrl",
                other
            ),
        }
    }

    platform::key_press(name, &mods)?;

    let msg = if mods.is_empty() {
        format!("Pressed key '{}'", name)
    } else {
        format!("Pressed {}+{}", mods.join("+"), name)
    };

    Ok(msg)
}
