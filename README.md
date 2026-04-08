# agent-desktop

Desktop automation CLI for AI agents. Observe and interact with any UI via accessibility APIs. Works on macOS, Linux, and Windows.

## Install

```
cargo install agent-desktop
```

## Features

- **Agent-first** — JSON output by default, minimal tokens, structured for LLM consumption
- **Ref-based** — Elements have stable IDs and support CSS-like selectors for deterministic targeting
- **Cross-platform** — Single binary works on macOS, Linux, and Windows via native accessibility APIs
- **Complete** — Observe, click, type, scroll, key press, screenshot, clipboard, focus, wait
- **Fast** — Native Rust CLI, instant command parsing

## Works with

Claude Code, Cursor, GitHub Copilot, OpenAI Codex, Google Gemini, and any agent that can run shell commands.

## Example

```
# List all running apps
agent-desktop observe

# Get accessibility tree for an app
agent-desktop observe --app Safari

# Filter with CSS-like selectors
agent-desktop observe --app Safari --query 'button[name="OK"]'

# Interact
agent-desktop click --app Safari --query 'button[name="OK"]'
agent-desktop type --text "hello world"
agent-desktop screenshot --output /tmp/screen.png
```

## How it works

1. `agent-desktop observe` — query the accessibility tree, get structured element data
2. Use element IDs or CSS-like queries to click, type, scroll, or interact
3. Repeat — re-observe after each action to get updated state

## Commands

### Observe

```
agent-desktop observe                                          # List all running apps
agent-desktop observe --app Safari                             # Accessibility tree for an app
agent-desktop observe --app Safari --query 'button[name="OK"]' # Filter with CSS-like selectors
agent-desktop observe --app Safari --list-roles                # Show role distribution
```

### Interact

```
agent-desktop click --app Safari --query 'button[name="OK"]'  # Click an element
agent-desktop click --x 400 --y 300                            # Click absolute coordinates
agent-desktop type --text "hello world"                        # Type at cursor
agent-desktop type --app Notes --query 'text_area' --text "hi" # Type into a specific element
agent-desktop scroll --direction down                           # Scroll the screen
agent-desktop key --name cmd+n                                 # Press a key combination
```

### Read & Wait

```
agent-desktop focus --app Safari                               # Focus an app
agent-desktop read --element 5                                 # Read element text/value
agent-desktop read --clipboard                                 # Read clipboard
agent-desktop wait --app Safari --query 'button[name="Done"]'  # Wait for element to appear
```

### Advanced

```
agent-desktop interact --element 3 --action press              # Native accessibility action
agent-desktop screenshot --output /tmp/screen.png              # Take a screenshot
```

## Platforms

Native Rust binary for macOS (ARM64, x64), Linux (x64), and Windows (x64). Uses platform-native accessibility APIs:

- **macOS** — AXUIElement
- **Linux** — AT-SPI2 via D-Bus
- **Windows** — UI Automation

## License

MIT
