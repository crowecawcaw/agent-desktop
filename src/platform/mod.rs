pub mod accessibility;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

use anyhow::Result;
use xa11y::input::{Key, ScrollDelta};

// ── Input simulation (xa11y — X11, macOS; error on Wayland) ─────────────────

fn input_sim() -> Result<xa11y::input::InputSim> {
    xa11y::input_sim().map_err(|e| {
        anyhow::anyhow!(
            "Input simulation unavailable: {}.\n\
             On Wayland, use --action for element clicks or --query with set-value for typing.",
            e
        )
    })
}

pub fn click_at(x: i32, y: i32) -> Result<()> {
    input_sim()?
        .mouse()
        .click((x, y))
        .map_err(|e| anyhow::anyhow!("Click failed: {}", e))
}

pub fn type_text(text: &str) -> Result<()> {
    input_sim()?
        .keyboard()
        .type_text(text)
        .map_err(|e| anyhow::anyhow!("Type failed: {}", e))
}

pub fn scroll(direction: &str, amount: u32, at: Option<(i32, i32)>) -> Result<()> {
    let delta = match direction {
        "up" => ScrollDelta::vertical(-(amount as i32)),
        "down" => ScrollDelta::vertical(amount as i32),
        "left" => ScrollDelta { dx: -(amount as i32), dy: 0 },
        "right" => ScrollDelta { dx: amount as i32, dy: 0 },
        _ => anyhow::bail!("Invalid scroll direction: {}", direction),
    };
    let point = at.unwrap_or_else(|| {
        let (w, h) = accessibility::get_screen_size();
        ((w / 2) as i32, (h / 2) as i32)
    });
    input_sim()?
        .mouse()
        .scroll(point, delta)
        .map_err(|e| anyhow::anyhow!("Scroll failed: {}", e))
}

pub fn key_press(name: &str, modifiers: &[&str]) -> Result<()> {
    let key = parse_key_name(name)?;
    let held: Vec<Key> = modifiers.iter().map(|m| parse_modifier(m)).collect();
    let sim = input_sim()?;
    let kb = sim.keyboard();
    if held.is_empty() {
        kb.press(key)
    } else {
        kb.chord(key, &held)
    }
    .map_err(|e| anyhow::anyhow!("Key press failed: {}", e))
}

// ── Platform-specific operations (no xa11y equivalent) ───────────────────────

pub fn focus_app(app: Option<&str>, pid: Option<u32>) -> Result<()> {
    #[cfg(target_os = "linux")]
    { return linux::focus_app(app, pid); }
    #[cfg(target_os = "macos")]
    { return macos::focus_app(app, pid); }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    { let _ = (app, pid); anyhow::bail!("Focus not supported on this platform") }
}

pub fn read_clipboard() -> Result<String> {
    #[cfg(target_os = "linux")]
    { return linux::read_clipboard(); }
    #[cfg(target_os = "macos")]
    { return macos::read_clipboard(); }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    { anyhow::bail!("Clipboard not supported on this platform") }
}

// ── Key name parsing ─────────────────────────────────────────────────────────

fn parse_key_name(name: &str) -> Result<Key> {
    Ok(match name.to_lowercase().as_str() {
        "return" | "enter" => Key::Enter,
        "tab" => Key::Tab,
        "escape" | "esc" => Key::Escape,
        "space" => Key::Space,
        "delete" | "backspace" => Key::Backspace,
        "forward_delete" | "forwarddelete" => Key::Delete,
        "up" => Key::ArrowUp,
        "down" => Key::ArrowDown,
        "left" => Key::ArrowLeft,
        "right" => Key::ArrowRight,
        "home" => Key::Home,
        "end" => Key::End,
        "page_up" | "pageup" => Key::PageUp,
        "page_down" | "pagedown" => Key::PageDown,
        "f1" => Key::F(1),
        "f2" => Key::F(2),
        "f3" => Key::F(3),
        "f4" => Key::F(4),
        "f5" => Key::F(5),
        "f6" => Key::F(6),
        "f7" => Key::F(7),
        "f8" => Key::F(8),
        "f9" => Key::F(9),
        "f10" => Key::F(10),
        "f11" => Key::F(11),
        "f12" => Key::F(12),
        other if other.len() == 1 => Key::Char(other.chars().next().unwrap()),
        _ => anyhow::bail!(
            "Unknown key '{}'. Use a single character or one of: return, tab, escape, space, \
             delete, forward_delete, up, down, left, right, home, end, page_up, page_down, f1-f12",
            name
        ),
    })
}

fn parse_modifier(m: &str) -> Key {
    match m {
        "cmd" | "command" => Key::Meta,
        "shift" => Key::Shift,
        "alt" | "option" => Key::Alt,
        "ctrl" | "control" => Key::Ctrl,
        _ => Key::Meta,
    }
}
