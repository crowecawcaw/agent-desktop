use anyhow::{Context, Result};
use std::process::Command;

fn is_wayland() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
        && std::env::var("XDG_SESSION_TYPE")
            .map(|v| v == "wayland")
            .unwrap_or(true)
}

fn run_command(cmd: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .context(format!("Failed to run `{}`. Is it installed?", cmd))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("`{} {}` failed: {}", cmd, args.join(" "), stderr);
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn run_command_ok(cmd: &str, args: &[&str]) -> Result<()> {
    run_command(cmd, args)?;
    Ok(())
}

fn has_command(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn focus_app(app: Option<&str>, pid: Option<u32>) -> Result<()> {
    if is_wayland() {
        if has_command("swaymsg") {
            if let Some(name) = app {
                let criteria = format!("[app_id=\"{}\"] focus", name);
                if run_command_ok("swaymsg", &[&criteria]).is_err() {
                    let criteria = format!("[title=\"{}\"] focus", name);
                    run_command_ok("swaymsg", &[&criteria])
                        .context(format!("Failed to focus app '{}' via swaymsg", name))?;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
                return Ok(());
            }
            if let Some(p) = pid {
                let criteria = format!("[pid={}] focus", p);
                run_command_ok("swaymsg", &[&criteria])
                    .context(format!("Failed to focus PID {} via swaymsg", p))?;
                std::thread::sleep(std::time::Duration::from_millis(100));
                return Ok(());
            }
            return Ok(());
        }
        anyhow::bail!(
            "Window focus on Wayland currently requires sway (swaymsg). \
             Other compositors are not yet supported."
        );
    }

    // X11
    if let Some(name) = app {
        run_command_ok("xdotool", &["search", "--name", name, "windowactivate"])
            .context("Failed to focus app. Is xdotool installed?")?;
    } else if let Some(p) = pid {
        let pid_str = p.to_string();
        run_command_ok("xdotool", &["search", "--pid", &pid_str, "windowactivate"])
            .context("Failed to focus app. Is xdotool installed?")?;
    }
    std::thread::sleep(std::time::Duration::from_millis(100));
    Ok(())
}

pub fn read_clipboard() -> Result<String> {
    if is_wayland() {
        if has_command("wl-paste") {
            return run_command("wl-paste", &["--no-newline"])
                .context("Failed to read clipboard with wl-paste");
        }
        anyhow::bail!(
            "No Wayland clipboard tool found. Install `wl-clipboard`:\n  \
             sudo apt install wl-clipboard    # Debian/Ubuntu\n  \
             sudo dnf install wl-clipboard    # Fedora"
        );
    }
    if has_command("xclip") {
        return run_command("xclip", &["-selection", "clipboard", "-o"])
            .context("Failed to read clipboard with xclip");
    }
    if has_command("xsel") {
        return run_command("xsel", &["--clipboard", "--output"])
            .context("Failed to read clipboard with xsel");
    }
    anyhow::bail!(
        "No clipboard tool found. Install `xclip` or `xsel`:\n  \
         sudo apt install xclip    # Debian/Ubuntu\n  \
         sudo dnf install xclip    # Fedora"
    )
}
