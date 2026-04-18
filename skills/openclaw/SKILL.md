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

agent-desktop is a Rust CLI that exposes accessibility-tree-based desktop control for AI agents, built on Linux AT-SPI2 via the `xa11y` crate.

## Use AT-SPI action paths first

**This is the most important section in this skill.** agent-desktop has two distinct paths for acting on UI:

1. **AT-SPI accessibility actions** — the app exposes named actions (press, set-value, toggle, etc.) on its tree nodes. agent-desktop invokes them directly via the accessibility bus. **No virtual keyboard, no mouse simulation, no window focus required.**
2. **Input simulation** — synthesizes keyboard/mouse input via `xdotool`/`wtype`/`ydotool`. Requires X11, sway, or a running `ydotoold`. Fails on plain GNOME/Wayland.

**Always try the AT-SPI path first.** It is more reliable, faster, doesn't depend on which display server is running, and doesn't depend on which window has focus.

```bash
# Snapshot the tree (json is recommended; xml is current default)
agent-desktop observe --app gedit --format json

# Press a button via AT-SPI (no virtual keyboard needed)
agent-desktop interact --action press --element 17

# Set a text field value via AT-SPI (no typing needed)
agent-desktop interact --action set-value --element 23 --value "hello"

# `type --element` tries set-value first, falls back to click+type
agent-desktop type --element 23 --text "hello"

# Read a value back from an element
agent-desktop read --element 42

# Accessibility focus (no window manager / no sway required)
agent-desktop focus --element 17
```

If the AT-SPI action you need exists on the element, prefer it. Use `observe` to check what actions are advertised.

## When AT-SPI actions don't work — fallback to input simulation

If the element has no usable action (or the app's a11y tree is degenerate — see GTK4 caveat below), fall back to input simulation. Preconditions:

- **X11 session** — `xdotool` works out of the box.
- **sway on Wayland** — agent-desktop's Wayland input path is sway-only via `wtype`.
- **ydotool daemon** — works on any Wayland compositor, but `ydotoold` must be running as a system or user service. Installing the `ydotool` binary is not enough.

If none of those hold, input simulation will fail and you must rely on AT-SPI actions or fall back to vision/screenshot.

```bash
# Click a screen coordinate (mouse simulation)
agent-desktop click --x 400 --y 300

# Type into the focused field (no element ref → uses keyboard simulation)
agent-desktop type --text "hello world"

# Send a key combo (note: --name, NOT --combo)
agent-desktop key --name "ctrl+s"
agent-desktop key --name "Return" --modifiers "ctrl,shift"
```

## When to use this skill

- ✅ Native Linux apps (GNOME Settings, Files, gedit, gnome-text-editor, terminal, system dialogs)
- ✅ GTK creative tools (Inkscape, LibreOffice — menus and toolbars)
- ✅ Native chat clients with working a11y (Telegram Desktop, Element)
- ✅ Reading what's currently visible in any window the agent has no other API for
- ❌ Browser automation — use Playwright or browser MCPs instead
- ❌ File operations — use shell tools instead
- ❌ Electron apps with broken a11y trees — fall back to screenshot + vision
- ❌ GTK4 apps that expose degenerate trees (gnome-calculator, etc.) — see GTK4 caveat

## Canonical pattern: snapshot → ref → act

This is the same pattern that made Playwright MCP work for browsers. It applies on the desktop, with **AT-SPI actions preferred**:

```bash
# 1. Snapshot the app's accessibility tree to discover element IDs
agent-desktop observe --app gedit --format json

# 2. Pick an element ID from the snapshot, then act via AT-SPI:
agent-desktop interact --action press --element 17

# 3. For text fields, use set-value (no keyboard required):
agent-desktop interact --action set-value --element 23 --value "Hello"

# 4. Verify by re-observing or reading the element:
agent-desktop read --element 23
agent-desktop wait -q 'window[name="Saved"]' --app gedit
```

Only fall through to `click --query`, `type --text`, or `key --name` if the AT-SPI action isn't available.

## Commands

Authoritative as of agent-desktop v0.1.x. Run `agent-desktop --help` and `agent-desktop <cmd> --help` for full flags.

| command | purpose | key flags |
|---------|---------|-----------|
| `observe` | snapshot accessibility tree | `--app`/`--pid`, `--max-depth`, `--max-elements 100`, `--role <comma>`, `-q --query <css>`, `--element <id>`, `--list-roles`, `--include-hidden`, `--format xml\|json`, `--raw` |
| `interact` | invoke an AT-SPI action (preferred path) | `--action <press\|set-value\|focus\|toggle\|expand\|collapse\|select\|show-menu>`, `--element`/`-q`/`--app`/`--pid`, `--value <v>` |
| `read` | extract text from an element or clipboard | `--element`/`-q`, `--clipboard` |
| `type` | enter text into a field | `--text <text>`, `--element`/`-q`/`--app`/`--pid` (with `--element`, tries set-value first, then click+type) |
| `click` | click an element or coordinate | `--element`/`-q`/`--app`/`--pid`, `--x --y`, `--offset x,y`, `--action` (use AT-SPI press, not mouse) |
| `key` | send a key or combo | `--name <name>` (e.g. "ctrl+s" or "Return"), `--modifiers <comma>`, `--app`/`--pid` |
| `scroll` | scroll an element or window | `--direction <up\|down\|left\|right>`, `--element`/`-q`/`--app`/`--pid`, `--amount 3` |
| `focus` | focus an element (a11y) or app (window) | `--element`/`-q` for a11y focus (no sway required); `--app`/`--pid` for window focus (sway-only on Wayland) |
| `wait` | block until selector matches | `-q --query <css>`, `--app`/`--pid`, `--timeout 10`, `--interval 500` |
| `screenshot` | capture window or screen | `--output <path>` (REQUIRED), `--scale 0.5`, `--app`/`--pid` |

Notes:

- `--format` defaults to `xml` today; issue #21 will flip the default to `json` once merged.
- `focus --element` is **a11y focus** (works everywhere AT-SPI does). `focus --app` is **window focus** (sway-only on Wayland).
- `key`'s flag is `--name`, not `--combo`. Modifiers can be inline ("ctrl+s") or separated ("--name s --modifiers ctrl").

## Selectors

CSS-like, scoped to the app's accessibility tree. Used with `-q`/`--query`:

- `button[name="OK"]` — push button labeled OK
- `text-field[name="Search"]` — text input named Search
- `*[role="checkbox"][checked=true]` — any checked checkbox
- `window > toolbar > button` — child traversal

If a selector returns multiple matches, use `--element <id>` from the snapshot for deterministic targeting.

## Display server reality

Be honest with yourself about what works where. Mixing this up wastes turns.

- **X11 (any DE)**: full support. AT-SPI actions, input simulation (`xdotool`), window focus, screenshots — all work.
- **sway on Wayland**: full support. Input goes through `wtype`, screenshots through `grim`, window focus through `swaymsg`.
- **GNOME on Wayland (Mutter)**:
  - AT-SPI actions: ✅ work normally (`interact --action press`, `interact --action set-value`, `read`, a11y `focus --element`).
  - Window focus (`focus --app`): ❌ sway-only. Auto-focus done by `--app` on action commands likewise fails.
  - Region screenshots scoped to `--app`: ❌ sway-only. Full-screen screenshot may still work via `grim`/portal.
  - Virtual keyboard (`wtype`): ❌ Mutter doesn't expose `zwp_virtual_keyboard_v1`, so `type --text` and `key --name` without an element fail.
  - Mouse simulation: ❌ Wayland blocks synthetic input from arbitrary processes.
  - **Workaround**: route everything through AT-SPI element refs (`interact`, `type --element`, `focus --element`). If you need true input simulation, set up `ydotoold` as a service (out of scope for this skill).
- **KDE Plasma 6 on Wayland**: not yet validated by this skill author. AT-SPI actions are likely fine; input simulation likely needs ydotool. Please report issues.

## Electron caveat

Electron apps vary wildly in accessibility-tree completeness. Telegram Desktop and Element expose reasonable trees. Some custom Electron apps return empty `observe` results. **Detection pattern**: if `observe --app <electron-app>` returns a tree with fewer than ~5 nodes, fall back to screenshot + vision rather than blindly clicking.

## GTK4 caveat

GTK4 apps frequently expose **degenerate** accessibility trees: a handful of `group`/`unknown` nodes, all marked disabled, no `button` roles, no actions, no readable display text. **gnome-calculator is the canonical example** — its ~25-node tree exposes nothing actionable.

Detection pattern:

```bash
agent-desktop observe --app gnome-calculator --list-roles
# If output shows only group/unknown and no button/text-field, the app is a GTK4 a11y dead end.
```

If you hit this, options in order of preference:

1. Substitute a different app that exposes a richer tree (gnome-text-editor / gedit are usually rich enough).
2. Fall back to screenshot + vision.
3. Report `blocked` (environmental, not an agent capability failure) — see EVAL_FORMAT.md.

Don't waste turns hunting for selectors that aren't there.

## Failure modes

| symptom | likely cause | what to do |
|---------|--------------|------------|
| empty tree | app doesn't expose a11y, or app not running | `observe` (no `--app`) to list visible apps; start the app if needed |
| timeout on `wait` | selector never matches | re-`observe` to confirm the selector is reachable |
| `ActionNotSupported` | a11y API doesn't expose that action | try a different action or fall back to keyboard navigation |
| `--app` command errors with "focus failed" on Wayland | window focus is sway-only | drop `--app`, use `--element` ref instead |
| `wtype` / `type --text` fails on GNOME/Wayland | Mutter has no virtual keyboard protocol | use `interact --action set-value --element <id>` instead |
| GTK4 button only exposes focus | known limitation (xa11y issue #100) | use keyboard activation (Tab + Enter) or substitute app |
| GTK4 app shows only `group`/`unknown` roles | GTK4 a11y degeneracy (e.g. gnome-calculator) | substitute a richer app, or fall back to vision |
| Linux event stream blank | xa11y polling impl misses events (issue #102) | use repeated `observe` instead of `wait --on event` |

## Install

```bash
cargo install agent-desktop
agent-desktop --version
```

Runtime requirements:

- AT-SPI2 stack (already present on any GNOME/KDE install).
- `xdotool` for X11 input.
- `wtype` for sway/Wayland input.
- `xclip` for clipboard read on X11.
- `scrot` (X11) or `grim` (sway) for screenshots.

**ydotool note**: if you need keyboard/mouse simulation on GNOME/Wayland, install `ydotool` AND set up `ydotoold` as a systemd service. The package install does not configure the daemon. Note: `ydotoold` may already be running but lack `/dev/uinput` permissions (run as root or add a udev rule to grant access — out of scope here). Without that perms gap closed, ydotool input is unavailable even with the daemon up. This is out of scope for `cargo install agent-desktop` but worth knowing if input fallback is required.

## Author

Stephen Crowe ([@crowecawcaw](https://github.com/crowecawcaw)). Built on the [xa11y](https://github.com/xa11y/xa11y) accessibility library.
