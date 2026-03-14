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

pub fn key_press(name: &str, modifiers: &[&str]) -> Result<()> {
    // Map friendly names to X key names
    let lower = name.to_lowercase();
    let x_key = match lower.as_str() {
        "return" | "enter" => "Return",
        "tab" => "Tab",
        "escape" | "esc" => "Escape",
        "space" => "space",
        "delete" | "backspace" => "BackSpace",
        "forward_delete" | "forwarddelete" => "Delete",
        "up" => "Up",
        "down" => "Down",
        "left" => "Left",
        "right" => "Right",
        "home" => "Home",
        "end" => "End",
        "page_up" | "pageup" => "Page_Up",
        "page_down" | "pagedown" => "Page_Down",
        "f1" => "F1",
        "f2" => "F2",
        "f3" => "F3",
        "f4" => "F4",
        "f5" => "F5",
        "f6" => "F6",
        "f7" => "F7",
        "f8" => "F8",
        "f9" => "F9",
        "f10" => "F10",
        "f11" => "F11",
        "f12" => "F12",
        other => {
            if other.len() == 1 {
                // handled below
                other
            } else {
                anyhow::bail!(
                    "Unknown key '{}'. Use a single character or one of: return, tab, escape, space, \
                     delete, forward_delete, up, down, left, right, home, end, page_up, page_down, f1-f12",
                    name
                );
            }
        }
    };

    // Build xdotool key combo string like "ctrl+shift+Return"
    let mut parts: Vec<&str> = Vec::new();
    for m in modifiers {
        let x_mod = match *m {
            "cmd" | "command" => "super",
            "shift" => "shift",
            "alt" | "option" => "alt",
            "ctrl" | "control" => "ctrl",
            _ => "super", // validated earlier
        };
        parts.push(x_mod);
    }
    parts.push(x_key);
    let combo = parts.join("+");

    run_command("xdotool", &["key", &combo])
        .context("Failed to press key. Is xdotool installed?")
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
