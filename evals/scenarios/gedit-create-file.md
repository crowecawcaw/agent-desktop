---
id: gedit-create-file
target_app: gedit
difficulty: easy
exercises: [observe, type, key]
requires:
  binaries: [agent-desktop]
  apps: [gedit]
  display_server: any
  notes: "NOTE: as of 2026-04, gnome-text-editor (GTK4) on Mutter exposes a degenerate AT-SPI tree (no text-field roles, no actions). gedit (older GTK3) is the better target if installed; this scenario will likely be 'blocked' on stock GNOME 46+ until upstream a11y improves."
---

# gedit: create a file with specific content

## Prompt

Use `agent-desktop` to open the Text Editor app on this machine (gedit OR gnome-text-editor — whichever is installed; check with `which`). Type the text `Hello, evals!` into the document body. Save the file as `/tmp/eval-hello.txt`. Report what you did.

Use `agent-desktop --help` to see commands. Prefer AT-SPI action paths (`interact --action set-value`, `interact --action press`) over keyboard simulation when possible — they bypass the virtual-keyboard requirement on Wayland.

## Expected outcome

The file `/tmp/eval-hello.txt` exists and contains `Hello, evals!` (trailing newline acceptable).

## Verification

```bash
test -f /tmp/eval-hello.txt && diff <(echo "Hello, evals!") /tmp/eval-hello.txt
```

Exit 0 = pass. Exit 1 = file content differs. File missing = fail.

## Environment

- Linux with gedit OR gnome-text-editor installed
- Save dialog UX may differ between the two; either is acceptable

## NOTE

As of 2026-04, `gnome-text-editor` (GTK4) on Mutter exposes a degenerate AT-SPI tree (no text-field roles, no actions). `gedit` (older GTK3) is the better target if installed. **However**: gedit / gnome-text-editor both respond to standard keyboard shortcuts (`Ctrl+N` for new, typing for content, `Ctrl+S` for save, then File chooser navigation). The tree-inaccessibility does **not** preclude completing this task — keyboard-driven workflow is fully viable.

**Before declaring blocked, the agent MUST attempt non-tree paths**: keyboard typing (`key --app gnome-text-editor --name <chars>` or `type --text` if input simulation is available), `Ctrl+S` to invoke the save dialog, then either `set-value` on the path entry (if exposed) or further keyboard input. See EVAL_FORMAT.md "Pre-flight responsibility" for the required path order.

**Acceptable outcomes** (per EVAL_FORMAT.md taxonomy):

- `success` — agent used the tree path: `observe` to identify the text-entry widget, `interact --action set-value` for content, `interact --action press` for menu/save, file written.
- `success-via-fallback` — agent typed via `type --text` / `key --app <app> --name <chars>`, used `key --name ctrl+s` to drive the save flow, and the file was written without tree introspection. Document path used.
- `blocked-all-paths-exhausted` — agent attempted tree, keyboard, and (where appropriate) coordinate paths without success. All attempts documented.
- `partial` — agent typed the content but couldn't complete the save (e.g., file dialog couldn't be driven), or saved to the wrong path.
- `failed` — agent had a viable path but the file content or path was wrong.

A bare `blocked-tree-inaccessible` is **NOT acceptable** — gedit/gnome-text-editor responds to standard keyboard shortcuts (`Ctrl+N`, `Ctrl+S`, typing) regardless of tree exposure. The agent must at least attempt the keyboard-driven workflow before declaring blocked.

## Reset / cleanup

```bash
rm -f /tmp/eval-hello.txt
pkill -f '^gedit$|^gnome-text-editor$' || true
```
