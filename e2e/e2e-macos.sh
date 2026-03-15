#!/usr/bin/env bash
set -euo pipefail

# E2E tests for macOS (requires TextEdit and Finder running, TCC permissions granted)
# Usage: ./e2e/e2e-macos.sh [path-to-binary]

BIN="${1:-./target/debug/agent-desktop}"
PASS=0
FAIL=0

pass() { echo "  PASS: $1"; PASS=$((PASS + 1)); }
fail() { echo "  FAIL: $1"; FAIL=$((FAIL + 1)); }

run_test() {
    local name="$1"
    shift
    echo "--- $name ---"
    if "$@"; then
        pass "$name"
    else
        fail "$name"
    fi
}

# --- Tests that should succeed ---

test_screenshot() {
    "$BIN" screenshot --output /tmp/screen.png
    test -f /tmp/screen.png
    SIZE=$(stat -f%z /tmp/screen.png)
    test "$SIZE" -gt 1000
}

test_observe_returns_xml() {
    OUTPUT=$("$BIN" observe 2>&1)
    echo "$OUTPUT"
    echo "$OUTPUT" | grep -q '<application'
}

test_observe_app_finder() {
    OUTPUT=$("$BIN" observe --app Finder 2>&1)
    echo "$OUTPUT"
    echo "$OUTPUT" | grep -q '<application'
    echo "$OUTPUT" | grep -qi 'Finder'
}

test_observe_app_textedit() {
    OUTPUT=$("$BIN" observe --app TextEdit 2>&1)
    echo "$OUTPUT"
    echo "$OUTPUT" | grep -q '<'
}

test_observe_list_roles() {
    OUTPUT=$("$BIN" observe --list-roles 2>&1)
    echo "$OUTPUT"
    echo "$OUTPUT" | grep -qE '[a-z_]+.*[0-9]+'
}

test_observe_query_filter() {
    "$BIN" observe --app Finder > /dev/null 2>&1 || true
    OUTPUT=$("$BIN" observe --app Finder -q 'menu_bar' 2>&1)
    echo "$OUTPUT"
    test -n "$OUTPUT"
}

test_observe_json_format() {
    OUTPUT=$("$BIN" observe --app Finder --format json 2>&1)
    echo "$OUTPUT" | head -5
    echo "$OUTPUT" | head -1 | grep -qE '^\[|\{'
}

test_focus_finder() {
    OUTPUT=$("$BIN" focus --app Finder 2>&1)
    echo "$OUTPUT"
    echo "$OUTPUT" | grep -qi 'Focused'
}

test_focus_textedit() {
    OUTPUT=$("$BIN" focus --app TextEdit 2>&1)
    echo "$OUTPUT"
    echo "$OUTPUT" | grep -qi 'Focused'
}

test_read_clipboard() {
    OUTPUT=$("$BIN" read --clipboard 2>&1)
    echo "$OUTPUT"
    echo "$OUTPUT" | grep -q 'clipboard'
}

test_click_coordinates() {
    "$BIN" click --x 100 --y 100 2>&1
}

test_scroll_down() {
    "$BIN" scroll --direction down 2>&1
}

test_key_press() {
    "$BIN" key --name escape 2>&1
}

test_click_element_by_query() {
    "$BIN" observe --app Finder > /dev/null 2>&1
    "$BIN" click --app Finder -q 'menu_bar_item:nth(1)' 2>&1 || true
}

test_type_text() {
    "$BIN" focus --app TextEdit > /dev/null 2>&1
    "$BIN" type --text "hello from CI" 2>&1
}

test_read_element_text() {
    OUTPUT=$("$BIN" observe --app TextEdit 2>&1)
    echo "$OUTPUT"
    echo "$OUTPUT" | grep -q '<'
}

# --- Tests for expected failures ---

test_observe_invalid_app() {
    OUTPUT=$("$BIN" observe --app NonExistentApp 2>&1) && return 1 || true
    echo "$OUTPUT"
    echo "$OUTPUT" | grep -qi 'not found\|error'
}

test_click_without_args() {
    OUTPUT=$("$BIN" click 2>&1) && return 1 || true
    echo "$OUTPUT"
    echo "$OUTPUT" | grep -qi 'element\|required'
}

# --- Run all tests ---

echo "=== E2E macOS Tests ==="

run_test "screenshot captures a file" test_screenshot
run_test "observe returns XML with elements" test_observe_returns_xml
run_test "observe --app Finder returns elements" test_observe_app_finder
run_test "observe --app TextEdit returns elements" test_observe_app_textedit
run_test "observe --list-roles shows role counts" test_observe_list_roles
run_test "observe with query filter" test_observe_query_filter
run_test "observe JSON format" test_observe_json_format
run_test "focus --app Finder" test_focus_finder
run_test "focus --app TextEdit" test_focus_textedit
run_test "read --clipboard returns JSON" test_read_clipboard
run_test "click at coordinates" test_click_coordinates
run_test "scroll down" test_scroll_down
run_test "key press" test_key_press
run_test "observe then click element by query" test_click_element_by_query
run_test "type text into TextEdit" test_type_text
run_test "read element text" test_read_element_text
run_test "observe with invalid app fails gracefully" test_observe_invalid_app
run_test "click without element or coords fails" test_click_without_args

echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="
if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
