mod commands;
mod platform;
mod query;
mod state;
mod types;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "percept")]
#[command(about = concat!("v", env!("CARGO_PKG_VERSION"), " — CLI tool for AI agents to observe and interact with desktop UIs via accessibility APIs"))]
#[command(long_about = concat!("v", env!("CARGO_PKG_VERSION"), " — CLI tool for AI agents to observe and interact with desktop UIs via accessibility APIs

  percept observe --app Safari
  percept observe --app Safari --query 'text_field[name*=\"Address\"]'
  percept click --query 'toolbar > text_field[name*=\"Address\"]'
  percept type --text \"https://example.com\"
  percept key --name return"))]
#[command(disable_version_flag = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Query the accessibility tree.
    Observe {
        /// Target application by name (shows full tree)
        #[arg(long)]
        app: Option<String>,

        /// Target application by PID (shows full tree)
        #[arg(long)]
        pid: Option<u32>,

        /// Maximum tree depth (default: 1 for all-apps overview, 10 for a specific app)
        #[arg(long)]
        max_depth: Option<u32>,

        /// Maximum number of elements to return (default: 500)
        #[arg(long, default_value = "500")]
        max_elements: u32,

        /// Filter elements by role (comma-separated, e.g. "button,text_field")
        #[arg(long)]
        role: Option<String>,

        /// CSS-like query to filter elements (e.g. 'button[name="Submit"]', 'toolbar > text_field')
        #[arg(long, short)]
        query: Option<String>,

        /// Include hidden/offscreen elements
        #[arg(long)]
        include_hidden: bool,

        /// Output format: flat (JSON, default) or tree (human-readable)
        #[arg(long, default_value = "flat")]
        format: String,

        /// Include platform-specific raw attributes in output
        #[arg(long)]
        raw: bool,
    },

    /// Perform an accessibility action on an element
    Interact {
        /// Element ID from the last observe
        #[arg(long, required_unless_present = "query")]
        element: Option<u32>,

        /// CSS-like query to select element (e.g. 'button[name="Submit"]')
        #[arg(long, short)]
        query: Option<String>,

        /// Action to perform (press, set-value, focus, toggle, expand, collapse, select, show-menu)
        #[arg(long)]
        action: String,

        /// Value for set-value action
        #[arg(long)]
        value: Option<String>,
    },

    /// Take a screenshot and save to path
    Screenshot {
        /// Output path for the screenshot
        #[arg(long)]
        output: String,

        /// Scale factor for the screenshot (default: 0.5)
        #[arg(long, default_value = "0.5")]
        scale: f64,

        /// Capture only the frontmost window of this app (by name)
        #[arg(long)]
        app: Option<String>,

        /// Capture only the frontmost window of this app (by PID)
        #[arg(long)]
        pid: Option<u32>,
    },

    /// Click an accessibility element
    Click {
        /// Element ID to click (from accessibility tree)
        #[arg(long, required_unless_present = "query")]
        element: Option<u32>,

        /// CSS-like query to select element (e.g. 'button[name="Submit"]')
        #[arg(long, short)]
        query: Option<String>,

        /// Pixel offset relative to center (format: x,y)
        #[arg(long)]
        offset: Option<String>,

        /// Use native accessibility press action instead of mouse simulation
        #[arg(long)]
        action: bool,
    },

    /// Type text at the current cursor position or in a specific element
    Type {
        /// Text to type
        #[arg(long)]
        text: String,

        /// Element ID to target (tries set-value first, falls back to click+type)
        #[arg(long)]
        element: Option<u32>,

        /// CSS-like query to select target element (e.g. 'text_field[name="Email"]')
        #[arg(long, short)]
        query: Option<String>,
    },

    /// Scroll the screen or within a specific element
    Scroll {
        /// Scroll direction (up, down, left, right)
        #[arg(long)]
        direction: String,

        /// Element ID to scroll within
        #[arg(long)]
        element: Option<u32>,

        /// CSS-like query to select element to scroll within
        #[arg(long, short)]
        query: Option<String>,

        /// Scroll amount in clicks (default: 3)
        #[arg(long)]
        amount: Option<u32>,
    },

    /// Press a key or key combination
    Key {
        /// Key name (e.g. return, tab, escape, space, delete, up, down, left, right, f1-f12)
        #[arg(long)]
        name: String,

        /// Modifier keys (comma-separated: cmd, shift, alt, ctrl)
        #[arg(long)]
        modifiers: Option<String>,
    },
}

/// Resolve --element vs --query, returning the element ID.
/// If --query is given, searches the last observe state and errors on 0 or >1 matches.
fn resolve_element(element: Option<u32>, query: Option<&str>) -> Result<u32> {
    match (element, query) {
        (Some(id), None) => Ok(id),
        (None, Some(q)) => {
            let selector = crate::query::parse_selector(q)
                .map_err(|e| anyhow::anyhow!("Invalid query: {}", e))?;
            let state = crate::state::PerceptState::load()?;
            let snapshot = state.accessibility.as_ref().ok_or_else(|| {
                anyhow::anyhow!("No accessibility data. Run `percept observe` first.")
            })?;
            let ids = crate::query::query_elements(&snapshot.elements, &selector);
            match ids.len() {
                0 => anyhow::bail!("Query '{}' matched no elements", q),
                1 => Ok(ids[0]),
                n => anyhow::bail!(
                    "Query '{}' matched {} elements (IDs: {:?}). Use :nth(N) to select one.",
                    q, n, ids
                ),
            }
        }
        (Some(_), Some(_)) => anyhow::bail!("Cannot specify both --element and --query"),
        (None, None) => anyhow::bail!("Must specify either --element or --query"),
    }
}

/// Resolve --element vs --query for optional element targeting.
fn resolve_element_optional(element: Option<u32>, query: Option<&str>) -> Result<Option<u32>> {
    match (element, query) {
        (None, None) => Ok(None),
        (Some(id), None) => Ok(Some(id)),
        (None, Some(q)) => Ok(Some(resolve_element(None, Some(q))?)),
        (Some(_), Some(_)) => anyhow::bail!("Cannot specify both --element and --query"),
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Observe {
            app,
            pid,
            max_depth,
            max_elements,
            role,
            query,
            include_hidden,
            format,
            raw,
        } => {
            commands::observe::run_observe(
                app.as_deref(),
                pid,
                max_depth,
                max_elements,
                role.as_deref(),
                query.as_deref(),
                !include_hidden,
                &format,
                raw,
            )?;
        }
        Commands::Interact {
            element,
            query,
            action,
            value,
        } => {
            let eid = resolve_element(element, query.as_deref())?;
            commands::interact::run_interact(eid, &action, value.as_deref())?;
        }
        Commands::Screenshot { output, scale, app, pid } => {
            commands::screenshot::run_screenshot(&output, scale, app.as_deref(), pid)?;
        }
        Commands::Click {
            element,
            query,
            offset,
            action,
        } => {
            let eid = resolve_element(element, query.as_deref())?;
            let parsed_offset = match offset {
                Some(ref s) => Some(commands::click::parse_offset(s)?),
                None => None,
            };
            commands::click::run_click_element(eid, action, parsed_offset)?;
        }
        Commands::Type { text, element, query } => {
            let eid = resolve_element_optional(element, query.as_deref())?;
            commands::type_text::run_type(eid, &text)?;
        }
        Commands::Scroll {
            direction,
            element,
            query,
            amount,
        } => {
            let eid = resolve_element_optional(element, query.as_deref())?;
            commands::scroll::run_scroll(eid, &direction, amount)?;
        }
        Commands::Key { name, modifiers } => {
            commands::key::run_key(&name, modifiers.as_deref())?;
        }
    }

    Ok(())
}
