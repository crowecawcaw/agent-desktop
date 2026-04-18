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

## Reset / cleanup

```bash
rm -f /tmp/eval-hello.txt
pkill -f '^gedit$|^gnome-text-editor$' || true
```
