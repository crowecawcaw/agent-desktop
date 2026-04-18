---
id: observe-running-apps
target_app: any
difficulty: easy
exercises: [observe]
requires:
  binaries: [agent-desktop]
  apps: []
  display_server: any
  notes: "Pure observation; no input simulation needed. Should succeed on any system with AT-SPI registry running."
---

# Observe: list running graphical applications

## Prompt

Use `agent-desktop observe` (with no `--app` flag) to get the system-wide accessibility tree, which lists running applications. Report the names of all applications you see and the total count.

You only need to use `observe` once. Do not focus, click, or type — this is a pure read.

## Expected outcome

The agent reports:
- A count of running applications (≥ 1, almost certainly ≥ 3 on a desktop session)
- A list of application names that includes at least one of: `gnome-shell`, `mutter`, the user's terminal app, or any other obviously-running graphical app

## Verification

Two checks; the eval passes if either succeeds:

1. **Reported count check**: `wmctrl -l 2>/dev/null | wc -l` returns N ≥ 1; agent's reported count is between max(1, N-3) and N+3 (allows for AT-SPI vs window-manager differences in what counts as "an app").
2. **Reported names check**: agent's list contains at least one substring matching an output line of `wmctrl -l 2>/dev/null | awk '{$1=$2=$3=""; print substr($0,4)}' | head -5`.

If `wmctrl` is not installed, fall back to `ps -eo comm | grep -E '^(gnome-shell|mutter|nautilus|firefox|gnome-terminal|kitty|alacritty)$' | sort -u` for the names check.

## Environment

- Linux with any DE running an AT-SPI registry (true for GNOME, KDE, Xfce by default)
- `agent-desktop` installed and on `$PATH`

## Reset / cleanup

None — the scenario does not modify state.
