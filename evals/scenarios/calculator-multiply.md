---
id: calculator-multiply
target_app: gnome-calculator
difficulty: hard
exercises: [observe, interact, click, read]
requires:
  binaries: [agent-desktop, gnome-calculator]
  apps: [gnome-calculator]
  display_server: any
  notes: "AT-SPI tree must expose buttons (GTK4 may not — see NOTE below)"
---

# Calculator: multiply 123 × 456

## Prompt

Use the `agent-desktop` CLI to open gnome-calculator on this machine and calculate `123 × 456`. Report the result you see in the calculator display.

Use `agent-desktop --help` to see commands. Prefer AT-SPI action paths (`interact --action press --element <id>`) over keyboard simulation when possible — they bypass the virtual-keyboard requirement on Wayland.

## Expected outcome

Agent reports `56088` as the answer. Equivalent formatting acceptable: `56088`, `56,088`, `= 56088`, `Result: 56088` — the numeric value is what matters.

## Verification

Either of these passes the eval:

1. **Reported answer**: the agent's final response contains the substring `56088`.
2. **Live UI state**: extract the display value with

   ```bash
   agent-desktop read --element <calc-display-id>
   ```

   after running `agent-desktop observe --app gnome-calculator --format json` to find the display element ID. The returned text must contain `56088`.

If both checks fail but the agent reported a near-correct value, record it as a partial-credit observation in the run notes (`partial` outcome, not `success`).

## Environment

- Linux with GNOME or any DE that runs `gnome-calculator`
- `agent-desktop` installed and on `$PATH`
- No prior gnome-calculator process required (the agent should launch it if needed)

## Reset / cleanup

```bash
pkill -f '^gnome-calculator$' || true
```

## NOTE

`gnome-calculator` is a GTK4 app and is **known to expose a degenerate accessibility tree** — typically ~25 nodes, all `group`/`unknown`, all marked disabled, no button roles, no actions, no readable display text. As of 2026-04 the tree path is expected to be a dead end on stock GNOME — but the calculator buttons have a known visible grid layout and the app responds to standard digit/operator key input, so non-tree paths are theoretically viable.

**Before declaring blocked, the agent MUST attempt non-tree paths**: keyboard navigation (`key --app gnome-calculator --name 1`, etc.), coordinate clicks based on a screenshot of the visible button grid (`screenshot --output /tmp/calc.png` then `click --x --y`), and clipboard verification (`Ctrl+C` then `read --clipboard`). See EVAL_FORMAT.md "Pre-flight responsibility" for the required path order.

**Acceptable outcomes for this scenario** (per EVAL_FORMAT.md taxonomy):

- `success` — buttons exposed via tree, agent computed and reported `56088` via `interact --action press`.
- `success-via-fallback` — agent used a non-tree path (e.g., `key --app gnome-calculator --name 1` for digits, coordinate clicks against a screenshot of the button grid, `read --clipboard` after `Ctrl+C` on the display). Document the path in the `Path used` field.
- `blocked-all-paths-exhausted` — agent attempted tree path, keyboard navigation, coordinate clicks (with screenshot reference), and clipboard read, and got nothing actionable. All four attempts documented in the trace.
- `partial` — agent computed something but mis-reported the result (e.g., reported `56,088.0` and harness rejects it, or reported intermediate state).
- `failed` — agent reported a wrong number for reasons other than environment issues.

A bare `blocked-tree-inaccessible` is **NOT acceptable** for this scenario — gnome-calculator has known visible button positions and standard keyboard input, so coordinate clicks and keyboard input are theoretically viable. The agent must at least attempt them before declaring blocked.
