use anyhow::{Context, Result};
use std::process::Command;

fn run_command(cmd: &str, args: &[&str]) -> Result<()> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .context(format!("Failed to run `{}`. Is it installed?", cmd))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("`{} {}` failed: {}", cmd, args.join(" "), stderr);
    }
    Ok(())
}

pub fn take_screenshot(output_path: &str) -> Result<()> {
    // Try scrot first, then grim (Wayland)
    if Command::new("scrot").arg("--version").output().is_ok() {
        run_command("scrot", &[output_path])
            .context("Failed to take screenshot with scrot")
    } else if Command::new("grim").arg("--help").output().is_ok() {
        run_command("grim", &[output_path])
            .context("Failed to take screenshot with grim")
    } else {
        anyhow::bail!(
            "No screenshot tool found. Install `scrot` (X11) or `grim` (Wayland):\n  \
             sudo apt install scrot    # Debian/Ubuntu X11\n  \
             sudo apt install grim     # Debian/Ubuntu Wayland"
        )
    }
}

pub fn click_at(x: i32, y: i32) -> Result<()> {
    run_command("xdotool", &["mousemove", &x.to_string(), &y.to_string()])
        .context("Failed to move mouse. Is xdotool installed?")?;
    run_command("xdotool", &["click", "1"])
        .context("Failed to click. Is xdotool installed?")?;
    Ok(())
}

pub fn type_text(text: &str) -> Result<()> {
    run_command("xdotool", &["type", "--delay", "12", text])
        .context("Failed to type text. Is xdotool installed?")
}

pub fn move_mouse(x: i32, y: i32) -> Result<()> {
    run_command("xdotool", &["mousemove", &x.to_string(), &y.to_string()])
        .context("Failed to move mouse. Is xdotool installed?")
}

pub fn scroll(direction: &str, amount: u32) -> Result<()> {
    let button = match direction {
        "up" => "4",
        "down" => "5",
        "left" => "6",
        "right" => "7",
        _ => anyhow::bail!("Invalid scroll direction: {}", direction),
    };

    for _ in 0..amount {
        run_command("xdotool", &["click", button])?;
    }
    Ok(())
}
