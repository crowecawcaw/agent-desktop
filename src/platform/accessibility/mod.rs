use anyhow::Result;

use crate::types::{AccessibilityElement, AccessibilitySnapshot, AppTarget, ElementRole, QueryOptions};

/// Get a shallow overview of all running applications
pub fn get_all_apps_overview(opts: &QueryOptions) -> Result<AccessibilitySnapshot> {
    check_permissions_or_bail()?;
    let xa_opts = to_xa_query_opts(opts);
    let tree = xa11y::all_apps(&xa_opts)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(tree_to_snapshot(&tree, opts))
}

/// Get the accessibility tree, dispatching to the right platform
pub fn get_tree(target: &AppTarget, opts: &QueryOptions) -> Result<AccessibilitySnapshot> {
    check_permissions_or_bail()?;
    let xa_target = to_xa_target(target);
    let xa_opts = to_xa_query_opts(opts);
    let tree = xa11y::app(&xa_target, &xa_opts)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(tree_to_snapshot(&tree, opts))
}

/// Perform an accessibility action on an element.
///
/// Re-traverses the application's accessibility tree using the same query
/// options that were recorded during `observe`. The traversal is deterministic
/// (DFS), so element IDs match the ones the user saw in the previous snapshot
/// as long as the application UI hasn't changed.
pub fn perform_action(element_id: u32, action: &str, value: Option<&str>) -> Result<()> {
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

    check_permissions_or_bail()?;

    let xa_target = xa11y::AppTarget::ByPid(snapshot.pid);
    let xa_opts = to_xa_query_opts(&opts);
    let tree = xa11y::app(&xa_target, &xa_opts)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let node = tree.get(element_id).ok_or_else(|| {
        anyhow::anyhow!("Element {} not found in re-traversed tree", element_id)
    })?;

    let (xa_action, xa_data) = parse_action(action, value)?;
    xa11y::perform_action(&tree, node, xa_action, xa_data)
        .map_err(|e| anyhow::anyhow!("{}", e))
}

// ---------------------------------------------------------------------------
// Conversions
// ---------------------------------------------------------------------------

fn check_permissions_or_bail() -> Result<()> {
    match xa11y::check_permissions().map_err(|e| anyhow::anyhow!("{}", e))? {
        xa11y::PermissionStatus::Granted => Ok(()),
        xa11y::PermissionStatus::Denied { instructions } => {
            anyhow::bail!(
                "Accessibility permission denied.\n\n{}\n\nRe-run after granting permission.",
                instructions
            );
        }
    }
}

fn to_xa_target(target: &AppTarget) -> xa11y::AppTarget {
    match target {
        AppTarget::ByName(name) => xa11y::AppTarget::ByName(name.clone()),
        AppTarget::ByPid(pid) => xa11y::AppTarget::ByPid(*pid),
    }
}

fn to_xa_query_opts(opts: &QueryOptions) -> xa11y::QueryOptions {
    xa11y::QueryOptions {
        max_depth: Some(opts.max_depth),
        max_elements: Some(opts.max_elements),
        visible_only: opts.visible_only,
        roles: opts.roles.as_ref().map(|roles| {
            roles.iter().filter_map(|r| xa11y::Role::from_snake_case(r.display_name())).collect()
        }),
    }
}

fn parse_action(action: &str, value: Option<&str>) -> Result<(xa11y::Action, Option<xa11y::ActionData>)> {
    match action {
        "press" | "click" | "activate" => Ok((xa11y::Action::Press, None)),
        "focus" => Ok((xa11y::Action::Focus, None)),
        "set-value" | "set_value" => {
            let v = value.ok_or_else(|| anyhow::anyhow!("set-value requires a --value"))?;
            Ok((xa11y::Action::SetValue, Some(xa11y::ActionData::Value(v.to_string()))))
        }
        "toggle" => Ok((xa11y::Action::Toggle, None)),
        "expand" => Ok((xa11y::Action::Expand, None)),
        "collapse" => Ok((xa11y::Action::Collapse, None)),
        "select" => Ok((xa11y::Action::Select, None)),
        "show-menu" | "show_menu" => Ok((xa11y::Action::ShowMenu, None)),
        "increment" => Ok((xa11y::Action::Increment, None)),
        "decrement" => Ok((xa11y::Action::Decrement, None)),
        other => anyhow::bail!(
            "Unknown action '{}'. Valid actions: press, focus, set-value, toggle, expand, collapse, select, show-menu, increment, decrement",
            other
        ),
    }
}

fn xa_role_to_element_role(role: xa11y::Role) -> ElementRole {
    match role {
        xa11y::Role::Window => ElementRole::Window,
        xa11y::Role::Application => ElementRole::Application,
        xa11y::Role::Button => ElementRole::Button,
        xa11y::Role::CheckBox => ElementRole::CheckBox,
        xa11y::Role::RadioButton => ElementRole::RadioButton,
        xa11y::Role::TextField => ElementRole::TextField,
        xa11y::Role::StaticText | xa11y::Role::TextArea => ElementRole::StaticText,
        xa11y::Role::ComboBox => ElementRole::ComboBox,
        xa11y::Role::List => ElementRole::List,
        xa11y::Role::ListItem => ElementRole::ListItem,
        xa11y::Role::Menu => ElementRole::Menu,
        xa11y::Role::MenuItem => ElementRole::MenuItem,
        xa11y::Role::MenuBar => ElementRole::MenuBar,
        xa11y::Role::Tab => ElementRole::Tab,
        xa11y::Role::TabGroup => ElementRole::TabGroup,
        xa11y::Role::Table => ElementRole::Table,
        xa11y::Role::TableRow => ElementRole::TableRow,
        xa11y::Role::TableCell => ElementRole::TableCell,
        xa11y::Role::Toolbar => ElementRole::Toolbar,
        xa11y::Role::ScrollBar => ElementRole::ScrollBar,
        xa11y::Role::Slider => ElementRole::Slider,
        xa11y::Role::Image => ElementRole::Image,
        xa11y::Role::Link => ElementRole::Link,
        xa11y::Role::Group => ElementRole::Group,
        xa11y::Role::Dialog => ElementRole::Dialog,
        xa11y::Role::Alert => ElementRole::Alert,
        xa11y::Role::ProgressBar => ElementRole::ProgressBar,
        xa11y::Role::TreeItem => ElementRole::TreeItem,
        xa11y::Role::WebArea => ElementRole::WebArea,
        xa11y::Role::Heading => ElementRole::Heading,
        xa11y::Role::Separator => ElementRole::Separator,
        xa11y::Role::SplitGroup => ElementRole::SplitGroup,
        _ => ElementRole::Unknown,
    }
}

fn tree_to_snapshot(tree: &xa11y::Tree, opts: &QueryOptions) -> AccessibilitySnapshot {
    let (screen_w, screen_h) = tree.screen_size;

    let mut elements: Vec<AccessibilityElement> = Vec::new();

    for node in tree.iter() {
        let role = xa_role_to_element_role(node.role);
        let role_name = role.display_name().to_string();

        let bounds = node.bounds.map(|r| crate::types::ElementBounds {
            x: r.x,
            y: r.y,
            width: r.width as i32,
            height: r.height as i32,
        });

        let bbox = bounds.as_ref().map(|b| {
            crate::types::BoundingBox::from_pixel_bounds(b, screen_w, screen_h)
        });

        let actions: Vec<String> = node.actions.iter().map(|a| format!("{}", a).to_lowercase()).collect();

        let checked = match node.states.checked {
            Some(xa11y::Toggled::On) => Some(true),
            Some(xa11y::Toggled::Off) | Some(xa11y::Toggled::Mixed) => Some(false),
            None => None,
        };

        let states = crate::types::ElementStates {
            enabled: node.states.enabled,
            visible: node.states.visible,
            focused: node.states.focused,
            checked,
            selected: node.states.selected,
            expanded: node.states.expanded,
            editable: node.states.editable,
        };

        // Compute depth from parent chain
        let mut depth = 0u32;
        let mut current = node.parent_index;
        while let Some(pidx) = current {
            depth += 1;
            if let Some(pnode) = tree.get(pidx) {
                current = pnode.parent_index;
            } else {
                break;
            }
        }

        let raw = if opts.include_raw {
            serde_json::to_value(&node.raw).ok()
        } else {
            None
        };

        elements.push(AccessibilityElement {
            id: node.index,
            role,
            role_name,
            name: node.name.clone(),
            value: node.value.clone(),
            description: node.description.clone(),
            bounds,
            bbox,
            actions,
            states,
            children: node.children_indices.clone(),
            parent: node.parent_index,
            depth,
            app: None,
            raw,
        });
    }

    let element_count = elements.len();
    let role_strs: Vec<String> = opts.roles.as_ref().map(|roles| {
        roles.iter().map(|r| r.display_name().to_string()).collect()
    }).unwrap_or_default();

    AccessibilitySnapshot {
        app_name: tree.app_name.clone(),
        pid: tree.pid.unwrap_or(0),
        screen_width: screen_w,
        screen_height: screen_h,
        element_count,
        elements,
        query_max_depth: opts.max_depth,
        query_max_elements: opts.max_elements,
        query_visible_only: opts.visible_only,
        query_roles: role_strs,
    }
}
