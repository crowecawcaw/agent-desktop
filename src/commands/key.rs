use anyhow::Result;

use crate::platform;

pub fn run_key(name: &str, modifiers: Option<&str>) -> Result<()> {
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

    if mods.is_empty() {
        println!("Pressed key '{}'", name);
    } else {
        println!("Pressed {}+{}", mods.join("+"), name);
    }

    Ok(())
}
