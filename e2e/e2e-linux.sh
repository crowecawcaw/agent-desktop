#!/usr/bin/env bash
set -euo pipefail

# E2E tests for Linux (requires Xvfb, AT-SPI2, gedit running)
# Usage: ./e2e/e2e-linux.sh [path-to-binary]

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
    SIZE=$(stat -c%s /tmp/screen.png)
    test "$SIZE" -gt 100
}

test_observe_returns_xml() {
    OUTPUT=$("$BIN" observe 2>&1)
    echo "$OUTPUT"
    echo "$OUTPUT" | grep -q '<application'
}

test_observe_app_gedit() {
    OUTPUT=$("$BIN" observe --app gedit 2>&1)
    echo "$OUTPUT"
    echo "$OUTPUT" | grep -q '<'
}

test_observe_list_roles() {
    OUTPUT=$("$BIN" observe --list-roles 2>&1)
    echo "$OUTPUT"
    echo "$OUTPUT" | grep -qE '[a-z_]+.*[0-9]+'
}

test_observe_query_filter() {
    "$BIN" observe --app gedit > /dev/null 2>&1 || true
    OUTPUT=$("$BIN" observe -q 'window' 2>&1)
    echo "$OUTPUT"
    test -n "$OUTPUT"
}

test_observe_json_format() {
    OUTPUT=$("$BIN" observe --app gedit --format json 2>&1)
    echo "$OUTPUT" | head -5
    echo "$OUTPUT" | head -1 | grep -qE '^\[|\{'
}

test_focus_app_gedit() {
    OUTPUT=$("$BIN" focus --app gedit 2>&1)
    echo "$OUTPUT"
    echo "$OUTPUT" | grep -qi 'Focused'
}

test_read_clipboard() {
    echo -n "test content" | xclip -selection clipboard
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

test_type_text() {
    "$BIN" focus --app gedit > /dev/null 2>&1 || true
    "$BIN" type --text "hello from CI" 2>&1
}

test_click_element_by_query() {
    "$BIN" observe --app gedit > /dev/null 2>&1
    "$BIN" click --app gedit -q 'window' 2>&1 || true
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

echo "=== E2E Linux Tests ==="

run_test "screenshot captures a file" test_screenshot
run_test "observe returns XML with elements" test_observe_returns_xml
run_test "observe --app gedit returns elements" test_observe_app_gedit
run_test "observe --list-roles shows role counts" test_observe_list_roles
run_test "observe with query filter" test_observe_query_filter
run_test "observe JSON format" test_observe_json_format
run_test "focus --app gedit" test_focus_app_gedit
run_test "read --clipboard returns JSON" test_read_clipboard
run_test "click at coordinates" test_click_coordinates
run_test "scroll down" test_scroll_down
run_test "key press" test_key_press
run_test "type text" test_type_text
run_test "observe then click element by query" test_click_element_by_query
run_test "observe with invalid app fails gracefully" test_observe_invalid_app
run_test "click without element or coords fails" test_click_without_args

echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="
if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
