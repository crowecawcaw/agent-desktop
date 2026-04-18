---
id: observe-app-window-name
target_app: gnome-shell
difficulty: easy
exercises: [observe]
requires:
  binaries: [agent-desktop, gnome-shell]
  apps: []
  display_server: any
  notes: "Pure observation of an always-running app. Tests the --app filtered observe path."
---

# Observe: read the window/application name of gnome-shell

## Prompt

Use `agent-desktop observe --app gnome-shell` to inspect the gnome-shell process's accessibility tree. Report:
- The application name (the top-level node's name)
- The number of top-level windows the application exposes (the count of `window`-role nodes at depth 1)

You only need to use `observe`. Do not interact, focus, or type.

## Expected outcome

The agent reports an application name (typically `"gnome-shell"` exactly) and a window count ≥ 0 (gnome-shell may expose 0 user-facing windows but the application node itself should be present).

## Verification

Three checks; the eval passes if at least the first succeeds:

1. **Application present**: `agent-desktop observe --app gnome-shell --list-roles` exits 0 and the output contains `application` with count ≥ 1.
2. **Reported app name** matches `gnome-shell` (exact, case-insensitive).
3. **Reported window count** is a non-negative integer.

## Environment

- Linux with GNOME (gnome-shell process running)
- `agent-desktop` installed and on `$PATH`

## Reset / cleanup

None — the scenario does not modify state.
