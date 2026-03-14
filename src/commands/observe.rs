use anyhow::Result;

use crate::platform::accessibility;
use crate::query;
use crate::state::PerceptState;
use crate::types::*;

pub fn run_observe(
    app: Option<&str>,
    pid: Option<u32>,
    max_depth: Option<u32>,
    max_elements: u32,
    role_filter: Option<&str>,
    query_filter: Option<&str>,
    visible_only: bool,
    format: &str,
    include_raw: bool,
) -> Result<()> {
    let all_apps = app.is_none() && pid.is_none();
    let effective_depth = max_depth.unwrap_or(if all_apps { 1 } else { 10 });

    let roles = role_filter.map(ElementRole::parse_filter);

    let opts = QueryOptions {
        max_depth: effective_depth,
        max_elements,
        visible_only,
        roles,
        include_raw,
    };

    let snapshot = if all_apps {
        accessibility::get_all_apps_overview(&opts)?
    } else {
        let target = if let Some(p) = pid {
            AppTarget::ByPid(p)
        } else {
            AppTarget::ByName(app.unwrap().to_string())
        };
        accessibility::get_tree(&target, &opts)?
    };

    // Save full state for subsequent interact/click commands
    let state = PerceptState::from_accessibility(snapshot.clone());
    state.save()?;

    // If --query is given, filter the output to matching elements
    if let Some(q) = query_filter {
        let selector = query::parse_selector(q)
            .map_err(|e| anyhow::anyhow!("Invalid query: {}", e))?;
        let ids = query::query_elements(&snapshot.elements, &selector);
        let filtered: Vec<&AccessibilityElement> = snapshot
            .elements
            .iter()
            .filter(|e| ids.contains(&e.id))
            .collect();

        match format {
            "tree" => {
                println!("Query '{}' matched {} elements:", q, filtered.len());
                for elem in &filtered {
                    print_element_summary(elem);
                }
            }
            _ => {
                let json = serde_json::to_string_pretty(&filtered)?;
                println!("{}", json);
            }
        }
        return Ok(());
    }

    match format {
        "tree" => print_tree(&snapshot),
        _ => {
            let json = serde_json::to_string_pretty(&snapshot)?;
            println!("{}", json);
        }
    }

    Ok(())
}

/// Run observe silently (no output) — used by action commands with --app/--pid
/// to auto-populate state before performing actions.
pub fn run_observe_silent(app: Option<&str>, pid: Option<u32>) -> Result<()> {
    let opts = QueryOptions {
        max_depth: 10,
        max_elements: 500,
        visible_only: true,
        roles: None,
        include_raw: false,
    };

    let target = if let Some(p) = pid {
        AppTarget::ByPid(p)
    } else if let Some(name) = app {
        AppTarget::ByName(name.to_string())
    } else {
        anyhow::bail!("No app target specified");
    };

    let snapshot = accessibility::get_tree(&target, &opts)?;
    let state = PerceptState::from_accessibility(snapshot);
    state.save()?;
    Ok(())
}

/// Show a specific element and its subtree from the last observe state.
pub fn run_observe_element(element_id: u32, format: &str) -> Result<()> {
    let state = PerceptState::load()?;
    let snapshot = state.accessibility.as_ref().ok_or_else(|| {
        anyhow::anyhow!("No accessibility data. Run `percept observe` first.")
    })?;

    // Collect element and all descendants
    let mut ids_to_show = vec![element_id];
    let mut i = 0;
    while i < ids_to_show.len() {
        let id = ids_to_show[i];
        if let Some(elem) = snapshot.elements.iter().find(|e| e.id == id) {
            for child_id in &elem.children {
                ids_to_show.push(*child_id);
            }
        }
        i += 1;
    }

    let subtree: Vec<&AccessibilityElement> = snapshot
        .elements
        .iter()
        .filter(|e| ids_to_show.contains(&e.id))
        .collect();

    if subtree.is_empty() {
        anyhow::bail!("Element {} not found in last observe state", element_id);
    }

    match format {
        "tree" => {
            println!("Element {} subtree ({} elements):", element_id, subtree.len());
            if let Some(root) = snapshot.elements.iter().find(|e| e.id == element_id) {
                print_tree_node(root, &snapshot.elements, "", true);
            }
        }
        _ => {
            let json = serde_json::to_string_pretty(&subtree)?;
            println!("{}", json);
        }
    }

    Ok(())
}

fn print_element_summary(elem: &AccessibilityElement) {
    let mut line = format!("[{}] {}", elem.id, elem.role_name);
    if let Some(ref name) = elem.name {
        line.push_str(&format!(" \"{}\"", name));
    }
    if let Some(ref bounds) = elem.bounds {
        line.push_str(&format!(
            " ({},{} {}x{})",
            bounds.x, bounds.y, bounds.width, bounds.height
        ));
    }
    println!("{}", line);
}

fn print_tree(snapshot: &AccessibilitySnapshot) {
    if snapshot.pid == 0 {
        println!("All applications ({} elements)", snapshot.element_count);
    } else {
        println!("{} (pid: {})", snapshot.app_name, snapshot.pid);
    }

    // Build a map of parent -> children for rendering
    let root_elements: Vec<&AccessibilityElement> = snapshot
        .elements
        .iter()
        .filter(|e| e.parent.is_none() || e.depth == 0)
        .collect();

    for (i, elem) in root_elements.iter().enumerate() {
        let is_last = i == root_elements.len() - 1;
        print_tree_node(elem, &snapshot.elements, "", is_last);
    }
}

fn print_tree_node(
    elem: &AccessibilityElement,
    all_elements: &[AccessibilityElement],
    prefix: &str,
    is_last: bool,
) {
    let connector = if is_last { "└── " } else { "├── " };

    let mut line = format!(
        "{}{}[{}] {}",
        prefix, connector, elem.id, elem.role_name
    );

    if let Some(ref name) = elem.name {
        line.push_str(&format!(" \"{}\"", name));
    }

    if let Some(ref bounds) = elem.bounds {
        line.push_str(&format!(
            " ({},{} {}x{})",
            bounds.x, bounds.y, bounds.width, bounds.height
        ));
    }

    if !elem.actions.is_empty() {
        line.push_str(&format!(" [{}]", elem.actions.join(",")));
    }

    // State annotations
    let mut state_tags = Vec::new();
    if !elem.states.enabled {
        state_tags.push("disabled");
    }
    if elem.states.focused {
        state_tags.push("focused");
    }
    if elem.states.selected {
        state_tags.push("selected");
    }
    if let Some(true) = elem.states.checked {
        state_tags.push("checked");
    }
    if let Some(true) = elem.states.expanded {
        state_tags.push("expanded");
    }
    if !state_tags.is_empty() {
        line.push_str(&format!(" {{{}}}", state_tags.join(",")));
    }

    println!("{}", line);

    // Print children
    let child_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });

    for (i, child_id) in elem.children.iter().enumerate() {
        if let Some(child) = all_elements.iter().find(|e| e.id == *child_id) {
            let child_is_last = i == elem.children.len() - 1;
            print_tree_node(child, all_elements, &child_prefix, child_is_last);
        }
    }
}
