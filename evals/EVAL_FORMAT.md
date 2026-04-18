# Eval scenario format (v1)

This document specifies the markdown format used for `agent-desktop` evaluation scenarios. Scenarios are the maintainer's contract with anyone running the agent: a precise prompt, a precise success definition, and a precise verification step. Treat each scenario as a self-contained reproducible test of the CLI surface against a real Linux desktop.

## File location

```
evals/scenarios/<id>.md
```

The `<id>` matches the scenario's `id:` frontmatter field and is used by tooling to address the scenario.

## Frontmatter schema

YAML between `---` markers at the top of the file:

```yaml
---
id: <kebab-case>                                    # required, unique
target_app: <app name or comma list>                # required, e.g. "gedit" or "gedit,gnome-text-editor"
difficulty: easy | medium | hard                    # required
exercises: [<command names>]                        # required, e.g. [observe, interact, read]
requires:
  binaries: [<bin names>]                           # binaries that must be on $PATH
  apps: [<app names>]                               # GUI apps that must be installed
  display_server: any | x11 | wayland | sway | x11-or-sway   # what the scenario can run under
  notes: <free text>                                # caveats, e.g. "AT-SPI tree must expose buttons"
---
```

`requires.display_server` semantics:

- `any` — works under X11, sway, or GNOME/Wayland (the scenario only uses AT-SPI action paths).
- `x11` — needs xdotool-style input simulation; X11 only.
- `wayland` — needs Wayland-specific behavior; works on any Wayland compositor.
- `sway` — needs `wtype`/`grim`/`swaymsg`; sway only.
- `x11-or-sway` — needs input simulation OR window focus, but either an X11 session or sway is acceptable.

## Body sections

The body is markdown with these headers, in order:

### `## Prompt`

The verbatim text given to the agent. Should be self-contained: the agent does not see the frontmatter or other sections. If the agent should know about the agent-desktop command surface, say so explicitly:

> Use `agent-desktop --help` to see commands. Prefer AT-SPI action paths
> (`interact --action press --element <id>`) over keyboard simulation when
> possible.

### `## Expected outcome`

Human-readable description of what success looks like. One paragraph, no ambiguity.

### `## Verification`

A concrete check, ideally a shell command with expected exit code or stdout. Examples:

```bash
test -f /tmp/eval-hello.txt && diff <(echo "Hello, evals!") /tmp/eval-hello.txt
```

If verification can only be done by inspecting live UI state, give an exact `agent-desktop observe` or `agent-desktop read` invocation and the expected substring.

### `## Environment`

Anything not captured by frontmatter: assumptions about display configuration, network, prior state, version-pinned packages, etc.

### `## Reset / cleanup`

Shell commands that restore the system to baseline. Run after every scenario, pass or fail.

```bash
rm -f /tmp/eval-hello.txt
pkill -f gedit || true
```

## Agent reporting contract

When the agent finishes (or aborts) a scenario, it MUST report in this shape:

```
Outcome: success | partial | failed | blocked
Reported value: <whatever the prompt asks for, or "n/a">
Blocker (if blocked): <e.g. "ydotool daemon not running" — environmental, not agent capability>
Trace: brief chronological log of commands run
Friction: anything that wasted turns (degenerate trees, missing flags, surprising errors)
```

### Outcome definitions

- **`success`** — verification passes. Agent did the task.
- **`partial`** — agent made meaningful progress but verification doesn't fully pass (e.g. answer reported but file not saved).
- **`failed`** — agent could have done the task but didn't (wrong commands, gave up early, hallucinated state).
- **`blocked`** — the **environment** prevented completion. The agent's reasoning was sound but the tool/OS/app couldn't deliver. Examples:
  - `ydotool daemon not running` and the scenario needs input simulation on Wayland.
  - GTK4 app exposes degenerate accessibility tree (no actionable elements).
  - `agent-desktop` binary missing.
  - Required app not installed.

The distinction between `failed` and `blocked` is **critical for grading**. `failed` means iterate on the agent (better skill, better prompt). `blocked` means iterate on the environment or the scenario (different app, daemon setup, upstream bug fix).

## Pre-flight check responsibility

Before handing the prompt to the agent, the runner (or the agent itself, as its first step) MUST run a pre-flight that checks `requires:`:

- Each binary in `requires.binaries` resolves on `$PATH` (verify it exists and is callable, e.g. `command -v agent-desktop` or `agent-desktop --help >/dev/null 2>&1` — note that `agent-desktop` does not implement a `--version` flag).
- Each app in `requires.apps` is installed (`which` or package check; do not rely on `--version` probes since not every binary implements it).
- The current display server matches `requires.display_server`.

If pre-flight fails, the run is reported as `blocked` with the missing requirement named — **do not proceed to the prompt**. This keeps environmental misses out of the agent's failure rate.
