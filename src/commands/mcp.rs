//! MCP (Model Context Protocol) server — JSON-RPC 2.0 over stdio.
//!
//! Exposes every agent-desktop CLI command as an MCP tool.
//! Messages are newline-delimited JSON objects.

use std::io::{BufRead, Write};

use anyhow::Result;
use base64::Engine as _;
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn write_response(out: &mut impl Write, msg: &Value) {
    let s = serde_json::to_string(msg).unwrap_or_default();
    let _ = out.write_all(s.as_bytes());
    let _ = out.write_all(b"\n");
    let _ = out.flush();
}

fn ok_response(id: &Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

fn err_response(id: &Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
}

fn tool_result(text: &str, is_error: bool) -> Value {
    json!({
        "content": [{ "type": "text", "text": text }],
        "isError": is_error
    })
}

fn tool_ok(text: &str) -> Value {
    tool_result(text, false)
}

fn tool_err(text: &str) -> Value {
    tool_result(text, true)
}

// ---------------------------------------------------------------------------
// Tool definitions
// ---------------------------------------------------------------------------

fn tools_list() -> Value {
    json!([
        {
            "name": "observe",
            "description": "Query the accessibility tree of desktop applications.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "app": { "type": "string", "description": "Target application by name" },
                    "pid": { "type": "integer", "description": "Target application by PID" },
                    "query": { "type": "string", "description": "CSS-like query to filter elements" },
                    "element": { "type": "integer", "description": "Show a specific element by ID" },
                    "max_depth": { "type": "integer", "description": "Maximum tree depth" },
                    "max_elements": { "type": "integer", "description": "Maximum number of elements to return" },
                    "format": { "type": "string", "description": "Output format: json (default) or xml" },
                    "include_hidden": { "type": "boolean", "description": "Include hidden/offscreen elements" }
                }
            }
        },
        {
            "name": "screenshot",
            "description": "Take a screenshot. Returns base64-encoded PNG.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "app": { "type": "string", "description": "Capture only this app's window (by name)" },
                    "pid": { "type": "integer", "description": "Capture only this app's window (by PID)" },
                    "scale": { "type": "number", "description": "Scale factor (default: 0.5)" }
                }
            }
        },
        {
            "name": "click",
            "description": "Click an accessibility element or screen coordinate.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "app": { "type": "string" },
                    "pid": { "type": "integer" },
                    "query": { "type": "string", "description": "CSS-like query to select element" },
                    "element": { "type": "integer", "description": "Element ID to click" },
                    "x": { "type": "integer", "description": "Absolute X coordinate" },
                    "y": { "type": "integer", "description": "Absolute Y coordinate" }
                }
            }
        },
        {
            "name": "type",
            "description": "Type text at the current cursor position or in a specific element.",
            "inputSchema": {
                "type": "object",
                "required": ["text"],
                "properties": {
                    "text": { "type": "string", "description": "Text to type" },
                    "app": { "type": "string" },
                    "pid": { "type": "integer" },
                    "query": { "type": "string" },
                    "element": { "type": "integer" }
                }
            }
        },
        {
            "name": "scroll",
            "description": "Scroll the screen or within a specific element.",
            "inputSchema": {
                "type": "object",
                "required": ["direction"],
                "properties": {
                    "direction": { "type": "string", "description": "up, down, left, or right" },
                    "app": { "type": "string" },
                    "pid": { "type": "integer" },
                    "query": { "type": "string" },
                    "element": { "type": "integer" },
                    "amount": { "type": "integer", "description": "Scroll amount in clicks (default: 3)" }
                }
            }
        },
        {
            "name": "key",
            "description": "Press a key or key combination.",
            "inputSchema": {
                "type": "object",
                "required": ["name"],
                "properties": {
                    "name": { "type": "string", "description": "Key name, optionally with modifiers (e.g. cmd+n)" },
                    "app": { "type": "string" },
                    "pid": { "type": "integer" }
                }
            }
        },
        {
            "name": "focus",
            "description": "Focus an application or element without clicking.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "app": { "type": "string" },
                    "pid": { "type": "integer" },
                    "query": { "type": "string" },
                    "element": { "type": "integer" }
                }
            }
        },
        {
            "name": "interact",
            "description": "Perform an accessibility action on an element.",
            "inputSchema": {
                "type": "object",
                "required": ["action"],
                "properties": {
                    "action": { "type": "string", "description": "press, set-value, focus, toggle, expand, collapse, select, show-menu" },
                    "app": { "type": "string" },
                    "pid": { "type": "integer" },
                    "query": { "type": "string" },
                    "element": { "type": "integer" },
                    "value": { "type": "string", "description": "Value for set-value action" }
                }
            }
        },
        {
            "name": "read",
            "description": "Read text content from an element or the clipboard.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "element": { "type": "integer" },
                    "clipboard": { "type": "boolean", "description": "Read from clipboard instead of an element" }
                }
            }
        },
        {
            "name": "wait",
            "description": "Wait for an element matching a query to appear.",
            "inputSchema": {
                "type": "object",
                "required": ["query"],
                "properties": {
                    "query": { "type": "string", "description": "CSS-like query to wait for" },
                    "app": { "type": "string" },
                    "pid": { "type": "integer" },
                    "timeout": { "type": "integer", "description": "Timeout in seconds (default: 10)" },
                    "interval": { "type": "integer", "description": "Poll interval in milliseconds (default: 500)" }
                }
            }
        }
    ])
}

// ---------------------------------------------------------------------------
// Tool dispatch
// ---------------------------------------------------------------------------

fn dispatch_tool(name: &str, params: &Value) -> Value {
    let get_str = |key: &str| -> Option<String> {
        params.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
    };
    let get_u32 = |key: &str| -> Option<u32> {
        params.get(key).and_then(|v| v.as_u64()).map(|n| n as u32)
    };
    let get_u64 = |key: &str| -> Option<u64> {
        params.get(key).and_then(|v| v.as_u64())
    };
    let get_i32 = |key: &str| -> Option<i32> {
        params.get(key).and_then(|v| v.as_i64()).map(|n| n as i32)
    };
    let get_f64 = |key: &str| -> Option<f64> {
        params.get(key).and_then(|v| v.as_f64())
    };
    let get_bool = |key: &str| -> bool {
        params.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
    };

    match name {
        "observe" => {
            let app = get_str("app");
            let pid = get_u32("pid");
            let query = get_str("query");
            let element = get_u32("element");
            let max_depth = get_u32("max_depth");
            let max_elements = get_u32("max_elements").unwrap_or(100);
            let format = get_str("format").unwrap_or_else(|| "json".to_string());
            let include_hidden = get_bool("include_hidden");

            let result = if let Some(eid) = element {
                crate::commands::observe::run_observe_element(eid, &format)
            } else {
                crate::commands::observe::run_observe(
                    app.as_deref(),
                    pid,
                    max_depth,
                    max_elements,
                    None,
                    query.as_deref(),
                    !include_hidden,
                    &format,
                    false,
                    false,
                )
            };

            match result {
                Ok(s) => tool_ok(&s),
                Err(e) => tool_err(&e.to_string()),
            }
        }

        "screenshot" => {
            let app = get_str("app");
            let pid = get_u32("pid");
            let scale = get_f64("scale").unwrap_or(0.5);

            match crate::commands::screenshot::take_screenshot_bytes(scale, app.as_deref(), pid) {
                Ok(bytes) => {
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                    let data_url = format!("data:image/png;base64,{}", b64);
                    tool_ok(&data_url)
                }
                Err(e) => tool_err(&e.to_string()),
            }
        }

        "click" => {
            let app = get_str("app");
            let pid = get_u32("pid");
            let query = get_str("query");
            let element = get_u32("element");
            let x = get_i32("x");
            let y = get_i32("y");

            // Absolute coordinate click
            if let (Some(cx), Some(cy)) = (x, y) {
                if let Err(e) = (|| -> Result<()> {
                    if app.is_some() || pid.is_some() {
                        crate::platform::focus_app(app.as_deref(), pid)?;
                    }
                    crate::platform::click_at(cx, cy)?;
                    Ok(())
                })() {
                    return tool_err(&e.to_string());
                }
                return tool_ok(&format!("Clicked at ({}, {})", cx, cy));
            }

            let result = (|| -> Result<String> {
                ensure_app_observed_mcp(app.as_deref(), pid)?;
                let eid = resolve_element_mcp(element, query.as_deref())?;
                crate::commands::click::run_click_element(eid, false, None)
            })();

            match result {
                Ok(s) => tool_ok(&s),
                Err(e) => tool_err(&e.to_string()),
            }
        }

        "type" => {
            let text = match get_str("text") {
                Some(t) => t,
                None => return tool_err("Missing required parameter: text"),
            };
            let app = get_str("app");
            let pid = get_u32("pid");
            let query = get_str("query");
            let element = get_u32("element");

            let result = (|| -> Result<String> {
                ensure_app_observed_mcp(app.as_deref(), pid)?;
                let eid = resolve_element_optional_mcp(element, query.as_deref())?;
                crate::commands::type_text::run_type(eid, &text)
            })();

            match result {
                Ok(s) => tool_ok(&s),
                Err(e) => tool_err(&e.to_string()),
            }
        }

        "scroll" => {
            let direction = match get_str("direction") {
                Some(d) => d,
                None => return tool_err("Missing required parameter: direction"),
            };
            let app = get_str("app");
            let pid = get_u32("pid");
            let query = get_str("query");
            let element = get_u32("element");
            let amount = get_u32("amount");

            let result = (|| -> Result<String> {
                ensure_app_observed_mcp(app.as_deref(), pid)?;
                let eid = resolve_element_optional_mcp(element, query.as_deref())?;
                crate::commands::scroll::run_scroll(eid, &direction, amount)
            })();

            match result {
                Ok(s) => tool_ok(&s),
                Err(e) => tool_err(&e.to_string()),
            }
        }

        "key" => {
            let name = match get_str("name") {
                Some(n) => n,
                None => return tool_err("Missing required parameter: name"),
            };
            let app = get_str("app");
            let pid = get_u32("pid");

            let result = (|| -> Result<String> {
                if app.is_some() || pid.is_some() {
                    crate::platform::focus_app(app.as_deref(), pid)?;
                }
                let (key, mods) = crate::parse_key_shorthand_pub(&name, None);
                crate::commands::key::run_key(&key, mods.as_deref())
            })();

            match result {
                Ok(s) => tool_ok(&s),
                Err(e) => tool_err(&e.to_string()),
            }
        }

        "focus" => {
            let app = get_str("app");
            let pid = get_u32("pid");
            let query = get_str("query");
            let element = get_u32("element");

            let result = (|| -> Result<String> {
                if app.is_some() || pid.is_some() {
                    crate::platform::focus_app(app.as_deref(), pid)?;
                    if element.is_some() || query.is_some() {
                        crate::commands::observe::run_observe_silent(app.as_deref(), pid)?;
                    } else {
                        return Ok(format!(
                            "Focused {}",
                            app.clone().unwrap_or_else(|| pid.unwrap().to_string())
                        ));
                    }
                }
                if element.is_some() || query.is_some() {
                    let eid = resolve_element_mcp(element, query.as_deref())?;
                    crate::commands::interact::run_interact(eid, "focus", None)
                } else {
                    Ok("No target specified".to_string())
                }
            })();

            match result {
                Ok(s) => tool_ok(&s),
                Err(e) => tool_err(&e.to_string()),
            }
        }

        "interact" => {
            let action = match get_str("action") {
                Some(a) => a,
                None => return tool_err("Missing required parameter: action"),
            };
            let app = get_str("app");
            let pid = get_u32("pid");
            let query = get_str("query");
            let element = get_u32("element");
            let value = get_str("value");

            let result = (|| -> Result<String> {
                ensure_app_observed_mcp(app.as_deref(), pid)?;
                let eid = resolve_element_mcp(element, query.as_deref())?;
                crate::commands::interact::run_interact(eid, &action, value.as_deref())
            })();

            match result {
                Ok(s) => tool_ok(&s),
                Err(e) => tool_err(&e.to_string()),
            }
        }

        "read" => {
            let query = get_str("query");
            let element = get_u32("element");
            let clipboard = get_bool("clipboard");

            let result = if clipboard {
                crate::commands::read::run_read_clipboard()
            } else {
                let eid = match resolve_element_mcp(element, query.as_deref()) {
                    Ok(id) => id,
                    Err(e) => return tool_err(&e.to_string()),
                };
                crate::commands::read::run_read_element(eid)
            };

            match result {
                Ok(s) => tool_ok(&s),
                Err(e) => tool_err(&e.to_string()),
            }
        }

        "wait" => {
            let query = match get_str("query") {
                Some(q) => q,
                None => return tool_err("Missing required parameter: query"),
            };
            let app = get_str("app");
            let pid = get_u32("pid");
            let timeout = get_u64("timeout").unwrap_or(10);
            let interval = get_u64("interval").unwrap_or(500);

            match crate::commands::wait::run_wait(&query, app.as_deref(), pid, timeout, interval) {
                Ok(s) => tool_ok(&s),
                Err(e) => tool_err(&e.to_string()),
            }
        }

        other => tool_err(&format!("Unknown tool: {}", other)),
    }
}

// ---------------------------------------------------------------------------
// MCP helpers (mirrors main.rs helpers but usable from mcp context)
// ---------------------------------------------------------------------------

fn ensure_app_observed_mcp(app: Option<&str>, pid: Option<u32>) -> Result<()> {
    if app.is_none() && pid.is_none() {
        return Ok(());
    }
    crate::platform::focus_app(app, pid)?;
    crate::commands::observe::run_observe_silent(app, pid)?;
    Ok(())
}

fn resolve_element_mcp(element: Option<u32>, query: Option<&str>) -> Result<u32> {
    match (element, query) {
        (Some(id), None) => Ok(id),
        (None, Some(q)) => {
            let selector = crate::query::parse_selector(q)
                .map_err(|e| anyhow::anyhow!("Invalid query: {}", e))?;
            let state = crate::state::AppState::load()?;
            let snapshot = state.accessibility.as_ref().ok_or_else(|| {
                anyhow::anyhow!("No accessibility data. Run observe first.")
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
        (Some(_), Some(_)) => anyhow::bail!("Cannot specify both element and query"),
        (None, None) => anyhow::bail!("Must specify either element or query"),
    }
}

fn resolve_element_optional_mcp(element: Option<u32>, query: Option<&str>) -> Result<Option<u32>> {
    match (element, query) {
        (None, None) => Ok(None),
        (Some(id), None) => Ok(Some(id)),
        (None, Some(q)) => Ok(Some(resolve_element_mcp(None, Some(q))?)),
        (Some(_), Some(_)) => anyhow::bail!("Cannot specify both element and query"),
    }
}

// ---------------------------------------------------------------------------
// Main server loop
// ---------------------------------------------------------------------------

pub fn run_mcp() -> Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let msg: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[mcp] JSON parse error: {}", e);
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": { "code": -32700, "message": "Parse error" }
                });
                write_response(&mut out, &resp);
                continue;
            }
        };

        let method = msg.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let id = msg.get("id").cloned().unwrap_or(Value::Null);
        let is_notification = msg.get("id").is_none();

        match method {
            "initialize" => {
                let resp = ok_response(
                    &id,
                    json!({
                        "protocolVersion": "2024-11-05",
                        "capabilities": { "tools": {} },
                        "serverInfo": {
                            "name": "agent-desktop",
                            "version": env!("CARGO_PKG_VERSION")
                        }
                    }),
                );
                write_response(&mut out, &resp);
            }

            "notifications/initialized" => {
                // Notification — no response needed
            }

            "tools/list" => {
                let resp = ok_response(
                    &id,
                    json!({ "tools": tools_list() }),
                );
                write_response(&mut out, &resp);
            }

            "tools/call" => {
                let params = msg.get("params").cloned().unwrap_or(json!({}));
                let tool_name = params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let tool_args = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or(json!({}));

                let result = dispatch_tool(tool_name, &tool_args);
                let resp = ok_response(&id, result);
                write_response(&mut out, &resp);
            }

            other => {
                eprintln!("[mcp] Unknown method: {}", other);
                if !is_notification {
                    let resp = err_response(&id, -32601, &format!("Method not found: {}", other));
                    write_response(&mut out, &resp);
                }
            }
        }
    }

    Ok(())
}
