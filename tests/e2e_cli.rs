//! End-to-end tests for the agent-desktop CLI.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[allow(deprecated)]
fn agent_desktop_cmd() -> Command {
    Command::cargo_bin("agent-desktop").unwrap()
}

// =============================================================================
// CLI Help & Version
// =============================================================================

#[test]
fn test_help_output() {
    agent_desktop_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("accessibility APIs"))
        .stdout(predicate::str::contains("screenshot"))
        .stdout(predicate::str::contains("click"))
        .stdout(predicate::str::contains("type"))
        .stdout(predicate::str::contains("scroll"));
}

#[test]
fn test_version_in_help() {
    agent_desktop_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("v0."))
        .stdout(predicate::str::contains("agent-desktop"));
}

#[test]
fn test_no_subcommand_shows_help() {
    agent_desktop_cmd()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}

// =============================================================================
// Screenshot command
// =============================================================================

#[test]
fn test_screenshot_help() {
    agent_desktop_cmd()
        .args(["screenshot", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--output"))
        .stdout(predicate::str::contains("--scale"));
}

#[test]
fn test_screenshot_requires_output() {
    agent_desktop_cmd()
        .arg("screenshot")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--output"));
}

#[test]
#[cfg(not(target_os = "macos"))]
fn test_screenshot_fails_without_screenshot_tool() {
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("screen.png");

    agent_desktop_cmd()
        .args(["screenshot", "--output", output.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("scrot")
                .or(predicate::str::contains("grim"))
                .or(predicate::str::contains("screenshot")),
        );
}

// =============================================================================
// Click command
// =============================================================================

#[test]
fn test_click_help() {
    agent_desktop_cmd()
        .args(["click", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--element"))
        .stdout(predicate::str::contains("--offset"));
}

#[test]
fn test_click_requires_element() {
    agent_desktop_cmd()
        .arg("click")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--element"));
}

#[test]
fn test_click_without_state_fails() {
    let tmp = TempDir::new().unwrap();
    agent_desktop_cmd()
        .args(["click", "--element", "1"])
        .env("HOME", tmp.path())
        .env("XDG_DATA_HOME", tmp.path().join("data"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("agent-desktop observe"));
}

#[test]
fn test_click_invalid_offset_format() {
    let tmp = TempDir::new().unwrap();
    let data_dir = tmp.path().join("agent-desktop");
    fs::create_dir_all(&data_dir).unwrap();

    let state = serde_json::json!({ "accessibility": null });
    fs::write(data_dir.join("state.json"), state.to_string()).unwrap();

    agent_desktop_cmd()
        .args(["click", "--element", "1", "--offset", "invalid"])
        .env("HOME", tmp.path())
        .env("XDG_DATA_HOME", tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Offset must be in format"));
}

// =============================================================================
// Type command
// =============================================================================

#[test]
fn test_type_help() {
    agent_desktop_cmd()
        .args(["type", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--text"))
        .stdout(predicate::str::contains("--element"));
}

#[test]
fn test_type_requires_text() {
    agent_desktop_cmd()
        .arg("type")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--text"));
}

// =============================================================================
// Scroll command
// =============================================================================

#[test]
fn test_scroll_help() {
    agent_desktop_cmd()
        .args(["scroll", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--direction"))
        .stdout(predicate::str::contains("--element"))
        .stdout(predicate::str::contains("--amount"));
}

#[test]
fn test_scroll_requires_direction() {
    agent_desktop_cmd()
        .arg("scroll")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--direction"));
}

#[test]
fn test_scroll_invalid_direction() {
    agent_desktop_cmd()
        .args(["scroll", "--direction", "diagonal"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid direction"));
}

#[test]
fn test_scroll_without_element_no_state_needed() {
    // Scroll without --element should not require state.
    // On platforms with native input simulation (macOS/Windows/X11) it succeeds;
    // on Wayland it fails with an unsupported-platform error rather than a
    // missing-state error.
    let assert = agent_desktop_cmd()
        .args(["scroll", "--direction", "up"])
        .assert();

    assert
        .stderr(predicate::str::contains("state").not())
        .stderr(predicate::str::contains("observe").not());
}

// =============================================================================
// Observe --format default (issue #21)
// =============================================================================

#[test]
fn observe_help_documents_json_as_default() {
    agent_desktop_cmd()
        .args(["observe", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--format").and(predicate::str::contains("json")));
}

#[test]
fn observe_default_format_is_json() {
    // The default output of `observe` should be valid JSON, not XML.
    // A weak "doesn't start with <" check is not enough — empty output
    // or an error banner would pass that. We assert the command succeeds
    // AND that stdout parses as a JSON value.
    let output = agent_desktop_cmd()
        .args(["observe"])
        .output()
        .expect("agent-desktop should run");

    assert!(
        output.status.success(),
        "observe should exit 0; got status={:?}, stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    assert!(!trimmed.is_empty(), "expected non-empty stdout");

    let parsed: serde_json::Value = serde_json::from_str(trimmed).unwrap_or_else(|e| {
        panic!(
            "expected JSON default but parse failed: {}\nfirst 200 chars of stdout: {}",
            e,
            &trimmed[..trimmed.len().min(200)]
        )
    });

    // Sanity: the parsed value should be either an object with fields or a
    // non-empty array. We don't assume a specific shape — that's another test.
    let non_empty = match &parsed {
        serde_json::Value::Object(m) => !m.is_empty(),
        serde_json::Value::Array(a) => !a.is_empty(),
        _ => true, // bare scalar is unexpected but technically valid JSON
    };
    assert!(non_empty, "parsed JSON should not be an empty object/array");
}

#[test]
fn observe_with_explicit_xml_still_works() {
    // Backward-compat: --format xml must still work after the default flip.
    let output = agent_desktop_cmd()
        .args(["observe", "--format", "xml"])
        .output()
        .expect("agent-desktop should run with explicit format");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim_start();
    assert!(
        trimmed.starts_with("<"),
        "expected XML output with --format xml but got: {}",
        &trimmed[..trimmed.len().min(80)]
    );
}
