---
name: agent-desktop
description: "Automate desktop UI on Linux with the agent-desktop CLI. Observe accessibility tree, click, type, scroll, focus, wait. Use when a user asks OpenClaw to control a desktop application, automate UI tasks, or interact with a GUI on Linux that has no API or CLI equivalent — especially native GTK/GNOME apps, system dialogs, About dialogs, native chat clients (Telegram Desktop, Element), and Electron apps with working accessibility trees."
homepage: https://github.com/crowecawcaw/agent-desktop
metadata:
  openclaw:
    emoji: "🖥️"
    os: ["linux"]
    requires:
      bins: ["agent-desktop"]
    install:
      - id: cargo
        kind: cargo
        crate: agent-desktop
        bins: ["agent-desktop"]
        label: "Install agent-desktop (cargo)"
---

# agent-desktop — Linux UI automation for OpenClaw agents

agent-desktop is a Rust CLI that exposes accessibility-tree-based desktop control for AI agents. It uses Linux AT-SPI2 (via the `xa11y` crate) for tree traversal and actions; it shells out to `xdotool`/`ydotool`/`wtype`/`grim`/`scrot`/`xclip` for input/screenshot/clipboard. JSON-out by default, agent-first design tenets.

## When to use this

- ✅ Native Linux apps (GNOME Settings, Files, gedit, terminal, system dialogs)
- ✅ GTK creative tools (Inkscape, LibreOffice — menus and toolbars)
- ✅ Native chat clients with working a11y (Telegram Desktop, Element)
- ✅ Reading what's currently visible in any window the agent has no other API for
- ❌ Browser automation — use Playwright or browser MCPs instead
- ❌ File operations — use shell tools instead
- ❌ Electron apps with broken a11y trees — fall back to screenshot + vision

## Canonical pattern: snapshot → ref → act

This is the same pattern that made Playwright MCP work for browsers. It applies on the desktop:

```bash
# 1. Snapshot the app's accessibility tree (compact, with @ref IDs)
agent-desktop observe --app gnome-control-center

# 2. Reference an element by @ref from the snapshot
agent-desktop click --app gnome-control-center --ref ref-42

# 3. Verify (re-snapshot or wait)
agent-desktop wait --app gnome-control-center --query 'window[name="Background"]'
```

## Common commands

| command | purpose |
|---------|---------|
| `observe --app <name>` | full accessibility tree |
| `observe --app <name> --query '<sel>'` | filter by CSS-like selector |
| `click --app <name> --ref <id>` | click an element by @ref |
| `click --app <name> --query '<sel>'` | click by selector |
| `type --text "<text>"` | type into focused field |
| `scroll --app <name> --ref <id> --direction down` | scroll an element |
| `key --combo cmd+s` | send a key combo |
| `focus --app <name>` | focus an app's window |
| `wait --app <name> --query '<sel>'` | block until selector matches |
| `screenshot --app <name>` | screenshot an app's window |

Run `agent-desktop --help` and `agent-desktop <cmd> --help` for full flags.

## Selectors

CSS-like, scoped to the app's accessibility tree:

- `button[name="OK"]` — push button labeled OK
- `text-field[name="Search"]` — text input named Search
- `*[role="checkbox"][checked=true]` — any checked checkbox
- `window > toolbar > button` — child traversal

If a selector returns multiple matches, use `--ref` from the snapshot for deterministic targeting.

## Display server caveats

- **GNOME on X11**: full support.
- **GNOME on Wayland**: window focus and region screenshots may not work (agent-desktop's Wayland support is sway-only as of v0.1.2). Some `observe` calls still work via AT-SPI2 even under GNOME/Wayland — try first, fall back to switching to an X11 session if window-focus is blocking.
- **KDE Plasma 6**: not yet validated by this skill author. May work; please report issues.

## Electron caveat

Electron apps vary wildly in accessibility-tree completeness. Telegram Desktop and Element expose reasonable trees. Some custom Electron apps return empty `observe` results. **Detection pattern**: if `observe --app <electron-app>` returns a tree with fewer than ~5 nodes, fall back to screenshot + vision rather than blindly clicking.

## Dry-run pattern (recommended before destructive actions)

agent-desktop supports a dry-run posture: `observe` first, verify the @ref points at the right element, then `click`/`type`. This catches stale-tree bugs and ambiguous selectors before they cause incorrect actions.

```bash
# Step 1: confirm the target
agent-desktop observe --app firefox --query 'button[name="Quit"]'

# Step 2: verify the @ref looks right (output shape, parent context, etc.)

# Step 3: act
agent-desktop click --app firefox --ref ref-117
```

## Failure modes

| symptom | likely cause | what to do |
|---------|--------------|------------|
| empty tree | app doesn't expose a11y, or app not running | check with `agent-desktop observe` (no --app); start app if needed |
| timeout on `wait` | selector never matches | re-`observe` to confirm the selector is reachable |
| action returns "ActionNotSupported" | accessibility API doesn't expose that action | use a different action or fall back to keyboard navigation |
| GTK4 button only exposes focus | known limitation (xa11y issue #100) | use keyboard activation (Tab + Enter) |
| Linux event stream blank | xa11y polling impl misses events (issue #102) | use repeated `observe` instead of `wait --on event` |

## Install

```bash
cargo install agent-desktop
agent-desktop --version
```

Requires: `xdotool` (X11) or `ydotool`/`wtype` (Wayland), `xclip` (clipboard, X11), `scrot`/`grim` (screenshots).

## Author

Stephen Crowe ([@crowecawcaw](https://github.com/crowecawcaw)). Built on the [xa11y](https://github.com/xa11y/xa11y) accessibility library.
