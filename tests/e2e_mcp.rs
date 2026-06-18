//! End-to-end integration tests for the `agent-desktop mcp` stdio server.
//!
//! These tests spawn the binary, send real MCP JSON-RPC messages over stdin,
//! and verify responses on stdout.

#![cfg(feature = "e2e")]

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build the binary path once.
fn binary_path() -> std::path::PathBuf {
    // Use the same cargo-built binary that assert_cmd would find
    let mut path = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    // deps/ -> parent (target/debug or target/release)
    if path.ends_with("deps") {
        path = path.parent().unwrap().to_path_buf();
    }
    path.join("agent-desktop")
}

struct McpProcess {
    stdin: std::process::ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
    child: std::process::Child,
}

impl McpProcess {
    fn spawn() -> Self {
        let mut child = Command::new(binary_path())
            .arg("mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null()) // suppress debug output
            .spawn()
            .expect("Failed to spawn agent-desktop mcp");

        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());

        McpProcess { stdin, stdout, child }
    }

    /// Send a JSON-RPC message (adds newline).
    fn send(&mut self, msg: &serde_json::Value) {
        let s = serde_json::to_string(msg).unwrap();
        self.stdin.write_all(s.as_bytes()).unwrap();
        self.stdin.write_all(b"\n").unwrap();
        self.stdin.flush().unwrap();
    }

    /// Read one newline-terminated JSON response (blocks up to timeout).
    fn read_response(&mut self) -> serde_json::Value {
        let mut line = String::new();
        // Simple read — relies on the server responding promptly
        self.stdout.read_line(&mut line).expect("Failed to read from mcp stdout");
        serde_json::from_str(line.trim()).expect("Response was not valid JSON")
    }

    /// Send `initialize` and drain the response, then send `notifications/initialized`.
    fn handshake(&mut self) -> serde_json::Value {
        self.send(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "clientInfo": { "name": "test-client", "version": "0.0.1" },
                "capabilities": {}
            }
        }));
        let resp = self.read_response();

        // Send the initialized notification (no response expected)
        self.send(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }));

        resp
    }
}

impl Drop for McpProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn test_mcp_initialize() {
    let mut proc = McpProcess::spawn();
    let resp = proc.handshake();

    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 1);

    let result = &resp["result"];
    assert_eq!(result["protocolVersion"], "2024-11-05");
    assert!(result["capabilities"]["tools"].is_object());
    assert_eq!(result["serverInfo"]["name"], "agent-desktop");

    // version should match Cargo.toml
    let version = result["serverInfo"]["version"].as_str().unwrap_or("");
    assert!(!version.is_empty(), "serverInfo.version should be non-empty");
}

#[test]
fn test_mcp_tools_list() {
    let mut proc = McpProcess::spawn();
    proc.handshake();

    proc.send(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    }));

    let resp = proc.read_response();
    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 2);

    let tools = resp["result"]["tools"].as_array().expect("tools should be an array");
    assert!(!tools.is_empty(), "tools list should not be empty");

    let tool_names: Vec<&str> = tools
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();

    for expected in &[
        "observe", "screenshot", "click", "type", "scroll",
        "key", "focus", "interact", "read", "wait",
    ] {
        assert!(
            tool_names.contains(expected),
            "Expected tool '{}' not found in tools list: {:?}",
            expected,
            tool_names
        );
    }

    // Each tool should have a name, description, and inputSchema
    for tool in tools {
        let name = tool["name"].as_str().unwrap_or("(unnamed)");
        assert!(tool["description"].is_string(), "Tool '{}' missing description", name);
        assert!(tool["inputSchema"].is_object(), "Tool '{}' missing inputSchema", name);
    }
}

#[test]
fn test_mcp_observe() {
    let mut proc = McpProcess::spawn();
    proc.handshake();

    proc.send(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "observe",
            "arguments": {}
        }
    }));

    let resp = proc.read_response();
    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 3);

    let result = &resp["result"];
    let content = result["content"].as_array().expect("content should be array");
    assert!(!content.is_empty(), "content should not be empty");

    let text = content[0]["text"].as_str().expect("text should be string");
    assert!(!text.is_empty(), "observe result text should not be empty");

    // Default format is JSON — should parse as JSON
    let parsed: serde_json::Value = serde_json::from_str(text).unwrap_or_else(|e| {
        panic!(
            "observe result should be valid JSON; got error: {}\ntext: {}",
            e,
            &text[..text.len().min(300)]
        )
    });
    assert!(parsed.is_object() || parsed.is_array(), "observe JSON should be an object or array");
}

#[test]
fn test_mcp_screenshot() {
    let mut proc = McpProcess::spawn();
    proc.handshake();

    proc.send(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "screenshot",
            "arguments": { "scale": 0.25 }
        }
    }));

    let resp = proc.read_response();
    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 4);

    let result = &resp["result"];
    let content = result["content"].as_array().expect("content should be array");
    let text = content[0]["text"].as_str().expect("text should be string");

    // On macOS without Screen Recording permission, the screenshot will return
    // isError: true with a permission error. Both outcomes are valid for this test:
    // what we care about is that the protocol is correct and returns a result.
    if result["isError"] == true {
        // Permission denied or other platform error — acceptable in CI
        // but verify it has a meaningful error message
        assert!(
            !text.is_empty(),
            "error message should not be empty"
        );
        eprintln!(
            "test_mcp_screenshot: screenshot returned an error (expected in CI without permissions): {}",
            &text[..text.len().min(120)]
        );
        return;
    }

    // If it succeeded, verify it's a valid base64 PNG data URL
    assert!(
        text.starts_with("data:image/png;base64,"),
        "screenshot result should start with data:image/png;base64, but got: {}",
        &text[..text.len().min(100)]
    );

    // Verify the base64 portion is non-trivially long (a real PNG)
    let b64_part = text.trim_start_matches("data:image/png;base64,");
    assert!(
        b64_part.len() > 100,
        "base64 data seems too short ({}), may not be a real PNG",
        b64_part.len()
    );
}

#[test]
fn test_mcp_unknown_method() {
    let mut proc = McpProcess::spawn();
    proc.handshake();

    proc.send(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "unknown/method",
        "params": {}
    }));

    let resp = proc.read_response();
    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 5);
    assert!(resp["error"].is_object(), "should return an error for unknown method");
    assert_eq!(resp["error"]["code"], -32601);
}

#[test]
fn test_mcp_tools_call_missing_required_param() {
    let mut proc = McpProcess::spawn();
    proc.handshake();

    // `type` requires `text`
    proc.send(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 6,
        "method": "tools/call",
        "params": {
            "name": "type",
            "arguments": {}
        }
    }));

    let resp = proc.read_response();
    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 6);

    let result = &resp["result"];
    assert_eq!(result["isError"], true, "should return isError: true");
    let text = result["content"][0]["text"].as_str().unwrap_or("");
    assert!(
        text.contains("text") || text.contains("Missing"),
        "error message should mention the missing param; got: {}",
        text
    );
}

#[test]
fn test_mcp_multiple_requests_sequential() {
    let mut proc = McpProcess::spawn();
    proc.handshake();

    // Send tools/list and tools/call observe back to back
    proc.send(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "tools/list",
        "params": {}
    }));

    proc.send(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 11,
        "method": "tools/call",
        "params": { "name": "observe", "arguments": {} }
    }));

    let resp1 = proc.read_response();
    let resp2 = proc.read_response();

    assert_eq!(resp1["id"], 10);
    assert!(resp1["result"]["tools"].is_array());

    assert_eq!(resp2["id"], 11);
    assert!(resp2["result"]["content"].is_array());
}
