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
Outcome: <one of below>
Reported value: <whatever the prompt asks for, or "n/a">
Path used: <tree | key | click-coord | clipboard | composite>
Blocker (if blocked): <specific reason, e.g. "observe returned 22 group nodes, no actionable roles; key, click-coord, clipboard all attempted (see trace)">
Trace: brief chronological log of commands run
Friction: anything that wasted turns (degenerate trees, missing flags, surprising errors)
```

### Outcome values

- **`success`** — task completed via the **canonical (tree-based) path**: `observe` → `interact --action` (or `type --element`, `focus --element`, `read --element`) → verification passes.
- **`success-via-fallback`** — task completed, but via a **non-tree path**: keyboard simulation (`key --app` / `key --name`), coordinate click (`click --x --y`), or clipboard read (`read --clipboard`). Document which path in `Path used`. This is still a success, but the maintainer should know the tree path didn't carry the run — useful signal for upstream a11y fixes.
- **`partial`** — agent made meaningful progress but verification doesn't fully pass (e.g. answer reported but file not saved). Agent reports what it could.
- **`blocked-tree-inaccessible`** — `observe` returned a degenerate tree (the canary case: ~25 `group`/`unknown` nodes, no actionable roles). The agent did **not** exhaust non-tree paths. Use this **only** if the scenario's prompt explicitly limits to the tree path, or the scenario explicitly accepts this outcome.
- **`blocked-all-paths-exhausted`** — agent attempted all four canonical paths (tree, keyboard, coordinate click, clipboard) and none worked. Each attempt MUST be documented in `Trace`. This is the strongest "the environment defeats us" signal.
- **`failed`** — agent had a viable path but the result didn't match expected outcome (wrong answer reported, wrong file saved, hallucinated state). Distinct from `blocked-*` because the agent **could have** completed but didn't.

The distinction between `failed` and `blocked-*` is **critical for grading**. `failed` means iterate on the agent (better skill, better prompt). `blocked-*` means iterate on the environment or the scenario (different app, daemon setup, upstream bug fix). The split between `blocked-tree-inaccessible` and `blocked-all-paths-exhausted` tells the maintainer whether the agent is giving up too early or whether the environment really is unworkable.

### Pre-flight responsibility (agent-side)

Before declaring `blocked-tree-inaccessible` or `blocked-all-paths-exhausted`, the agent MUST attempt at minimum:

1. **Tree path** — `observe --app <name>`, then `interact --action` / `click --query` / `type --element`.
2. **Keyboard path** — `key --app <name> --name <combo>` for known shortcuts (`Ctrl+S`, `Ctrl+N`, `Tab`, `Enter`, digits for calculator, etc.). Most apps respond to standard shortcuts regardless of tree exposure.
3. **Coordinate path** — if a screenshot reveals button positions (`screenshot --output /tmp/x.png` then visually parse), use `click --x --y`.
4. **Clipboard path** — for verification or extracting values, `read --clipboard` after a known clipboard-modifying action (e.g. `Ctrl+C` after selecting display text).

Only after all four fail does `blocked-all-paths-exhausted` apply. If the scenario explicitly accepts `blocked-tree-inaccessible` (rare — only when the scenario is itself a tree-availability canary), the agent may stop after step 1.

## Pre-flight check responsibility

Before handing the prompt to the agent, the runner (or the agent itself, as its first step) MUST run a pre-flight that checks `requires:`:

- Each binary in `requires.binaries` resolves on `$PATH` (verify it exists and is callable, e.g. `command -v agent-desktop` or `agent-desktop --help >/dev/null 2>&1` — note that `agent-desktop` does not implement a `--version` flag).
- Each app in `requires.apps` is installed (`which` or package check; do not rely on `--version` probes since not every binary implements it).
- The current display server matches `requires.display_server`.

If pre-flight fails, the run is reported as `blocked` with the missing requirement named — **do not proceed to the prompt**. This keeps environmental misses out of the agent's failure rate.
