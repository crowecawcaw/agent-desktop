use anyhow::{Context, Result};
use std::process::Command;

pub fn take_screenshot(output_path: &str) -> Result<()> {
    let output = Command::new("screencapture")
        .args(["-x", output_path])
        .output()
        .context("Failed to run screencapture")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("screencapture failed: {}", stderr);
    }
    Ok(())
}

pub fn click_at(x: i32, y: i32) -> Result<()> {
    let script = format!(
        r#"tell application "System Events" to click at {{{}, {}}}"#,
        x, y
    );
    let output = Command::new("osascript")
        .args(["-e", &script])
        .output()
        .context("Failed to run osascript for click")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("osascript click failed: {}", stderr);
    }
    Ok(())
}

pub fn type_text(text: &str) -> Result<()> {
    let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
    let script = format!(
        r#"tell application "System Events" to keystroke "{}""#,
        escaped
    );
    let output = Command::new("osascript")
        .args(["-e", &script])
        .output()
        .context("Failed to run osascript for typing")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("osascript type failed: {}", stderr);
    }
    Ok(())
}

pub fn move_mouse(x: i32, y: i32) -> Result<()> {
    let script = format!(
        r#"tell application "System Events"
    set mousePosition to {{{}, {}}}
end tell"#,
        x, y
    );
    let output = Command::new("osascript")
        .args(["-e", &script])
        .output()
        .context("Failed to run osascript for mouse move")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("osascript mouse move failed: {}", stderr);
    }
    Ok(())
}

pub fn scroll(direction: &str, amount: u32) -> Result<()> {
    let (dx, dy) = match direction {
        "up" => (0, amount as i32),
        "down" => (0, -(amount as i32)),
        "left" => (amount as i32, 0),
        "right" => (-(amount as i32), 0),
        _ => anyhow::bail!("Invalid scroll direction: {}", direction),
    };
    let script = format!(
        r#"tell application "System Events"
    scroll {{0, 0}} by {{{}, {}}}
end tell"#,
        dx, dy
    );
    let output = Command::new("osascript")
        .args(["-e", &script])
        .output()
        .context("Failed to run osascript for scroll")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("osascript scroll failed: {}", stderr);
    }
    Ok(())
}
