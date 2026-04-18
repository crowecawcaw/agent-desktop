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

`gnome-calculator` is a GTK4 app and is **known to expose a degenerate accessibility tree** — typically ~25 nodes, all `group`/`unknown`, all marked disabled, no button roles, no actions, no readable display text. As of 2026-04 this scenario is expected to be `blocked` on Linux until the GTK4 a11y issue is resolved upstream.

**Acceptable outcomes for this scenario**:

- `success` — the tree exposed actionable buttons and a readable display, the agent computed and reported `56088`.
- `blocked-with-tree-investigation` — the agent ran `observe --app gnome-calculator --list-roles`, confirmed the tree is degenerate (no button/text-field roles), and reported `blocked` with the tree summary as the blocker.

A bare `failed` here likely indicates the agent did not investigate the tree; treat as a SKILL.md regression rather than a real attempt.
