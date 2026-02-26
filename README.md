# percept

A CLI tool that annotates screenshots using [OmniParser](https://github.com/microsoft/OmniParser) and provides computer interaction commands that reference annotated blocks instead of pixel coordinates. Built for general-purpose agents that struggle with precise coordinate targeting.

## How it works

1. Take a screenshot and run `percept annotate` to detect UI elements as numbered blocks
2. Use block IDs from the annotation to click, scroll, or interact with elements
3. Repeat — re-annotate after each action to get updated block IDs

## Commands

```
percept annotate --screenshot <path>                      # Annotate a screenshot, output numbered blocks with bounding boxes
percept click --block <id>                                # Click the center of an annotated block
percept click --block <id> --offset <x>,<y>               # Click with pixel offset relative to block center
percept type --text <string>                              # Type text at the current cursor position
percept type --block <id> --text <string>                 # Click a block then type text
percept scroll --direction <up|down|left|right>           # Scroll the screen in a direction
percept scroll --block <id> --direction <up|down|left|right>  # Scroll within a specific block
percept scroll --block <id> --amount <pixels>             # Scroll a specific pixel amount within a block
percept screenshot --output <path>                        # Take a screenshot and save to path
```

## Install

```
cargo install percept
```
