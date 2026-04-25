use anyhow::{Context, Result};
use std::process::Command;

pub fn focus_app(app: Option<&str>, pid: Option<u32>) -> Result<()> {
    let script = if let Some(name) = app {
        format!(r#"tell application "{}" to activate"#, name)
    } else if let Some(p) = pid {
        format!(
            r#"tell application "System Events"
    set frontmost of (first process whose unix id is {}) to true
end tell"#,
            p
        )
    } else {
        unreachable!()
    };
    let output = Command::new("osascript")
        .args(["-e", &script])
        .output()
        .context("Failed to focus app")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to focus app: {}", stderr);
    }
    std::thread::sleep(std::time::Duration::from_millis(100));
    Ok(())
}

pub fn read_clipboard() -> Result<String> {
    let output = Command::new("pbpaste")
        .output()
        .context("Failed to read clipboard via pbpaste")?;
    if !output.status.success() {
        anyhow::bail!("pbpaste failed");
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
