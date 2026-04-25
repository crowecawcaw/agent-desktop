use anyhow::Result;
use std::sync::Mutex;
use xa11y::{App, AppExt, Element, Role};

use crate::types::*;

/// Cached xa11y element handles keyed by the snapshot ID we assigned.
static ELEMENT_CACHE: Mutex<Option<Vec<CachedElement>>> = Mutex::new(None);

struct CachedElement {
    id: u32,
    element: Element,
}

// ── Public API ──────────────────────────────────────────────────────────────

/// Get a shallow overview of all running applications
pub fn get_all_apps_overview(opts: &QueryOptions) -> Result<AccessibilitySnapshot> {
    let apps = App::list().map_err(map_xa11y_error)?;
    let (screen_w, screen_h) = get_screen_size();

    let mut elements = Vec::new();
    let mut cache_entries = Vec::new();
    let mut id_counter = 0u32;

    for app in &apps {
        if elements.len() >= opts.max_elements as usize {
            break;
        }
        let app_name = if app.name.is_empty() {
            continue;
        } else {
            &app.name
        };

        let root = Element::new(app.data.clone(), app.provider().clone());
        traverse_element(
            &root,
            opts,
            &mut elements,
            &mut cache_entries,
            &mut id_counter,
            0,
            None,
            screen_w,
            screen_h,
            app_name,
        );
    }

    let element_count = elements.len();

    // Update global cache
    *ELEMENT_CACHE.lock().unwrap() = Some(cache_entries);

    Ok(AccessibilitySnapshot {
        app_name: "all".to_string(),
        pid: 0,
        screen_width: screen_w,
        screen_height: screen_h,
        element_count,
        elements,
        query_max_depth: opts.max_depth,
        query_max_elements: opts.max_elements,
        query_visible_only: opts.visible_only,
        query_roles: opts
            .roles
            .as_ref()
            .map(|r| {
                r.iter()
                    .map(|role| role.display_name().to_string())
                    .collect()
            })
            .unwrap_or_default(),
    })
}

/// Get the accessibility tree, dispatching to the right platform
pub fn get_tree(target: &AppTarget, opts: &QueryOptions) -> Result<AccessibilitySnapshot> {
    check_permissions()?;

    let app = match target {
        AppTarget::ByName(name) => App::by_name(name).map_err(map_xa11y_error)?,
        AppTarget::ByPid(pid) => App::by_pid(*pid).map_err(map_xa11y_error)?,
    };

    let pid = app.pid.unwrap_or(0);
    let app_name = app.name.clone();
    let (screen_w, screen_h) = get_screen_size();

    let mut elements = Vec::new();
    let mut cache_entries = Vec::new();
    let mut id_counter = 0u32;

    let root = Element::new(app.data.clone(), app.provider().clone());
    traverse_element(
        &root,
        opts,
        &mut elements,
        &mut cache_entries,
        &mut id_counter,
        0,
        None,
        screen_w,
        screen_h,
        &app_name,
    );

    let element_count = elements.len();

    // Update global cache
    *ELEMENT_CACHE.lock().unwrap() = Some(cache_entries);

    Ok(AccessibilitySnapshot {
        app_name,
        pid,
        screen_width: screen_w,
        screen_height: screen_h,
        element_count,
        elements,
        query_max_depth: opts.max_depth,
        query_max_elements: opts.max_elements,
        query_visible_only: opts.visible_only,
        query_roles: opts
            .roles
            .as_ref()
            .map(|r| {
                r.iter()
                    .map(|role| role.display_name().to_string())
                    .collect()
            })
            .unwrap_or_default(),
    })
}

/// Perform an accessibility action on an element.
pub fn perform_action(element_id: u32, action: &str, value: Option<&str>) -> Result<()> {
    // First try using the in-memory cache from the last traversal.
    let cached_element = {
        let guard = ELEMENT_CACHE.lock().unwrap();
        guard
            .as_ref()
            .and_then(|entries| entries.iter().find(|e| e.id == element_id))
            .map(|e| e.element.clone())
    };

    if let Some(element) = cached_element {
        return do_action(&element, action, value, element_id);
    }

    // Fall back to re-traversing the saved snapshot's app.
    let state = crate::state::AppState::load()?;
    let snapshot = state.accessibility.as_ref().ok_or_else(|| {
        anyhow::anyhow!("No accessibility data. Run `agent-desktop observe` first.")
    })?;

    if snapshot.pid == 0 {
        anyhow::bail!(
            "Current state is an all-apps overview. Run `agent-desktop observe --app <name>` to target a specific app first."
        );
    }

    let opts = QueryOptions {
        max_depth: snapshot.query_max_depth,
        max_elements: snapshot.query_max_elements,
        visible_only: snapshot.query_visible_only,
        roles: if snapshot.query_roles.is_empty() {
            None
        } else {
            Some(ElementRole::parse_filter(&snapshot.query_roles.join(",")))
        },
        include_raw: false,
    };

    let target = AppTarget::ByPid(snapshot.pid);
    // Re-traverse to populate the cache
    get_tree(&target, &opts)?;

    let guard = ELEMENT_CACHE.lock().unwrap();
    let element = guard
        .as_ref()
        .and_then(|entries| entries.iter().find(|e| e.id == element_id))
        .map(|e| e.element.clone())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Element {} not found. Run `agent-desktop observe` first.",
                element_id
            )
        })?;
    drop(guard);

    do_action(&element, action, value, element_id)
}

// ── Internal helpers ────────────────────────────────────────────────────────

fn check_permissions() -> Result<()> {
    // xa11y checks permissions in provider construction (App::by_name, etc.)
    // and returns Error::PermissionDenied. We let those propagate naturally.
    Ok(())
}

fn do_action(element: &Element, action: &str, value: Option<&str>, element_id: u32) -> Result<()> {
    let provider = element.provider();
    let data = element.data();

    let result = match action {
        "press" | "click" | "activate" => provider.press(data),
        "focus" => provider.focus(data),
        "toggle" => provider.toggle(data),
        "expand" => provider.expand(data),
        "collapse" => provider.collapse(data),
        "select" => provider.select(data),
        "show-menu" | "show_menu" => provider.show_menu(data),
        "set-value" | "set_value" | "setvalue" => {
            let text = value
                .ok_or_else(|| anyhow::anyhow!("set-value action requires --value parameter"))?;
            if let Ok(num) = text.parse::<f64>() {
                provider.set_numeric_value(data, num)
            } else {
                provider.set_value(data, text)
            }
        }
        other => provider.perform_action(data, other),
    };

    result.map_err(|e| {
        anyhow::anyhow!(
            "Action '{}' failed on element {}: {}",
            action,
            element_id,
            e
        )
    })
}

#[allow(clippy::too_many_arguments)]
fn traverse_element(
    element: &Element,
    opts: &QueryOptions,
    elements: &mut Vec<AccessibilityElement>,
    cache: &mut Vec<CachedElement>,
    id_counter: &mut u32,
    depth: u32,
    parent_id: Option<u32>,
    screen_w: u32,
    screen_h: u32,
    app_name: &str,
) {
    if depth > opts.max_depth || elements.len() >= opts.max_elements as usize {
        return;
    }

    let data = element.data();
    let normalized_role = map_xa11y_role(data.role);

    let is_visible = data.states.visible;

    if opts.visible_only && !is_visible && depth > 0 {
        // Still traverse children — a hidden container may have visible children
        if let Ok(children) = element.children() {
            for child in &children {
                traverse_element(
                    child, opts, elements, cache, id_counter, depth + 1, parent_id,
                    screen_w, screen_h, app_name,
                );
            }
        }
        return;
    }

    // Role filter — skip node but traverse children
    if let Some(ref role_filter) = opts.roles {
        if !role_filter.contains(&normalized_role) && depth > 0 {
            if let Ok(children) = element.children() {
                for child in &children {
                    traverse_element(
                        child, opts, elements, cache, id_counter, depth + 1, parent_id,
                        screen_w, screen_h, app_name,
                    );
                }
            }
            return;
        }
    }

    let bounds = data.bounds.as_ref().map(|r| ElementBounds {
        x: r.x,
        y: r.y,
        width: r.width as i32,
        height: r.height as i32,
    });

    let bbox = bounds
        .as_ref()
        .map(|b| BoundingBox::from_pixel_bounds(b, screen_w, screen_h));

    let checked = data
        .states
        .checked
        .as_ref()
        .map(|t| matches!(t, xa11y::Toggled::On));

    let elem_states = ElementStates {
        enabled: data.states.enabled,
        visible: is_visible,
        focused: data.states.focused,
        checked,
        selected: data.states.selected,
        expanded: data.states.expanded,
        editable: data.states.editable,
    };

    let value = data.value.clone();

    let raw = if opts.include_raw && !data.raw.is_empty() {
        Some(serde_json::Value::Object(
            data.raw
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        ))
    } else {
        None
    };

    *id_counter += 1;
    let my_id = *id_counter;

    cache.push(CachedElement {
        id: my_id,
        element: element.clone(),
    });

    let elem = AccessibilityElement {
        id: my_id,
        role: normalized_role.clone(),
        role_name: normalized_role.display_name().to_string(),
        name: data.name.clone(),
        value,
        description: data.description.clone(),
        bounds,
        bbox,
        actions: data.actions.clone(),
        states: elem_states,
        children: Vec::new(),
        parent: parent_id,
        depth,
        app: Some(app_name.to_string()),
        raw,
    };
    elements.push(elem);

    // Traverse children
    let mut child_ids = Vec::new();
    if let Ok(children) = element.children() {
        for child in &children {
            if elements.len() >= opts.max_elements as usize {
                break;
            }
            let child_start = *id_counter + 1;
            traverse_element(
                child, opts, elements, cache, id_counter, depth + 1, Some(my_id),
                screen_w, screen_h, app_name,
            );
            for cid in child_start..=*id_counter {
                if let Some(child_elem) = elements.iter().find(|e| e.id == cid) {
                    if child_elem.parent == Some(my_id) {
                        child_ids.push(cid);
                    }
                }
            }
        }
    }

    if let Some(elem) = elements.iter_mut().find(|e| e.id == my_id) {
        elem.children = child_ids;
    }
}

fn map_xa11y_role(role: Role) -> ElementRole {
    match role {
        Role::Window => ElementRole::Window,
        Role::Application => ElementRole::Application,
        Role::Button => ElementRole::Button,
        Role::TextField | Role::TextArea | Role::SpinButton => ElementRole::TextField,
        Role::StaticText => ElementRole::StaticText,
        Role::CheckBox | Role::Switch => ElementRole::CheckBox,
        Role::RadioButton => ElementRole::RadioButton,
        Role::ComboBox => ElementRole::ComboBox,
        Role::List => ElementRole::List,
        Role::ListItem => ElementRole::ListItem,
        Role::Menu => ElementRole::Menu,
        Role::MenuItem => ElementRole::MenuItem,
        Role::MenuBar => ElementRole::MenuBar,
        Role::Tab => ElementRole::Tab,
        Role::TabGroup => ElementRole::TabGroup,
        Role::Table => ElementRole::Table,
        Role::TableRow => ElementRole::TableRow,
        Role::TableCell => ElementRole::TableCell,
        Role::Toolbar => ElementRole::Toolbar,
        Role::ScrollBar | Role::ScrollThumb => ElementRole::ScrollBar,
        Role::Slider => ElementRole::Slider,
        Role::Image => ElementRole::Image,
        Role::Link => ElementRole::Link,
        Role::Group | Role::Navigation => ElementRole::Group,
        Role::Dialog => ElementRole::Dialog,
        Role::Alert => ElementRole::Alert,
        Role::ProgressBar => ElementRole::ProgressBar,
        Role::TreeItem => ElementRole::TreeItem,
        Role::WebArea => ElementRole::WebArea,
        Role::Heading => ElementRole::Heading,
        Role::Separator => ElementRole::Separator,
        Role::SplitGroup => ElementRole::SplitGroup,
        Role::Tooltip | Role::Status => ElementRole::Unknown,
        Role::Unknown => ElementRole::Unknown,
    }
}

fn map_xa11y_error(e: xa11y::Error) -> anyhow::Error {
    match e {
        xa11y::Error::PermissionDenied { instructions } => {
            anyhow::anyhow!(
                "Accessibility permission denied.\n\n{}\n\nRe-run after granting permission.",
                instructions
            )
        }
        other => anyhow::anyhow!("{}", other),
    }
}

/// Detect screen resolution using platform-specific tools.
pub fn get_screen_size() -> (u32, u32) {
    #[cfg(target_os = "linux")]
    {
        if let Some(size) = get_screen_size_linux() {
            return size;
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(size) = get_screen_size_macos() {
            return size;
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(size) = get_screen_size_windows() {
            return size;
        }
    }
    (1920, 1080)
}

#[cfg(target_os = "linux")]
fn get_screen_size_linux() -> Option<(u32, u32)> {
    // Try X11
    if let Ok(output) = std::process::Command::new("xdpyinfo").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("dimensions:") {
                if let Some(dims) = line.split_whitespace().nth(1) {
                    if let Some((w, h)) = dims.split_once('x') {
                        if let (Ok(w), Ok(h)) = (w.parse(), h.parse()) {
                            return Some((w, h));
                        }
                    }
                }
            }
        }
    }
    // Try swaymsg (Wayland)
    if let Ok(output) = std::process::Command::new("swaymsg")
        .args(["-t", "get_outputs", "--raw"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Ok(outputs) = serde_json::from_str::<serde_json::Value>(&stdout) {
            if let Some(arr) = outputs.as_array() {
                for out in arr {
                    if out.get("active").and_then(|v| v.as_bool()) == Some(true) {
                        if let Some(rect) = out.get("rect") {
                            let w =
                                rect.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                            let h =
                                rect.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                            if w > 0 && h > 0 {
                                return Some((w, h));
                            }
                        }
                    }
                }
            }
        }
    }
    // Try wlr-randr
    if let Ok(output) = std::process::Command::new("wlr-randr").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let trimmed = line.trim();
            if trimmed.contains("current") && trimmed.contains(" x ") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 3 {
                    if let (Ok(w), Ok(h)) = (parts[0].parse(), parts[2].parse()) {
                        return Some((w, h));
                    }
                }
            }
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn get_screen_size_macos() -> Option<(u32, u32)> {
    if let Ok(output) = std::process::Command::new("osascript")
        .args([
            "-e",
            "tell application \"Finder\" to get bounds of window of desktop",
        ])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = stdout.trim().split(", ").collect();
        if parts.len() == 4 {
            if let (Ok(w), Ok(h)) = (parts[2].parse::<u32>(), parts[3].parse::<u32>()) {
                return Some((w, h));
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn get_screen_size_windows() -> Option<(u32, u32)> {
    // Use Windows API if available at runtime, otherwise fall back to default
    None
}
