# percept — Implementation Plan

## Overview

`percept` is a Rust CLI that annotates screenshots using OmniParser V2 (Microsoft's screen parsing tool) and provides computer interaction commands using block IDs instead of pixel coordinates. It's designed for AI agents that struggle with precise coordinate targeting.

---

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                   percept (Rust CLI)                 │
│                                                     │
│  ┌──────────┐  ┌──────────┐  ┌───────────────────┐ │
│  │ Commands │  │  State   │  │   Platform Layer  │ │
│  │ annotate │  │ (blocks  │  │ screenshot capture│ │
│  │ click    │  │  store)  │  │ mouse/keyboard    │ │
│  │ type     │  │          │  │ scrolling         │ │
│  │ scroll   │  │          │  │                   │ │
│  │ screenshot│ │          │  │                   │ │
│  └────┬─────┘  └────┬─────┘  └───────────────────┘ │
│       │              │                               │
│  ┌────▼──────────────▼──────┐                       │
│  │   OmniParser Backend     │                       │
│  │  ┌────────┐ ┌──────────┐ │                       │
│  │  │Replicate│ │  Local   │ │                       │
│  │  │  API   │ │  Server  │ │                       │
│  │  └────────┘ └──────────┘ │                       │
│  └──────────────────────────┘                       │
└─────────────────────────────────────────────────────┘
                       │ (local backend only)
                       ▼
        ┌──────────────────────────┐
        │  OmniParser Python       │
        │  Sidecar Server (FastAPI)│
        │  ┌────────┐ ┌─────────┐ │
        │  │YOLOv8  │ │Florence2│ │
        │  │(detect)│ │(caption)│ │
        │  └────────┘ └─────────┘ │
        └──────────────────────────┘
```

---

## OmniParser Integration Strategy

OmniParser V2 is a Python-based tool with two models:
- **Icon Detection** — YOLOv8 fine-tuned model that outputs bounding boxes for UI elements
- **Icon Captioning** — Florence2 model that generates semantic descriptions of detected elements

### Two supported backends:

**1. Replicate API (cloud, default)**
- No local GPU required
- ~$0.00077 per call, ~4s latency
- Input: screenshot image, box_threshold, iou_threshold
- Output: labeled image + structured element list with bounding boxes and captions
- Requires `REPLICATE_API_TOKEN` env var

**2. Local sidecar server (offline)**
- Bundled Python FastAPI server that wraps OmniParser
- Started/managed by the Rust CLI on first use
- Requires: Python 3.12, CUDA GPU, ~2GB model weights
- Setup via `percept setup` command that installs deps + downloads weights

---

## Implementation Steps

### Phase 1: Project Scaffolding

1. **Initialize Rust project**
   - `Cargo.toml` with dependencies: `clap` (CLI), `serde`/`serde_json` (serialization), `reqwest` (HTTP), `base64`, `image`, `tokio` (async runtime), `tempfile`, `dirs`
   - Module structure:
     ```
     src/
       main.rs           — entry point, CLI definition
       commands/
         mod.rs
         annotate.rs     — annotate command
         click.rs        — click command
         type_text.rs    — type command
         scroll.rs       — scroll command
         screenshot.rs   — screenshot command
       backend/
         mod.rs          — backend trait definition
         replicate.rs    — Replicate API backend
         local.rs        — local sidecar server backend
       platform/
         mod.rs          — platform detection + dispatch
         linux.rs        — xdotool-based interactions
         macos.rs        — osascript/cliclick interactions
       state.rs          — block state management
       types.rs          — shared types (Block, BoundingBox, etc.)
     ```

2. **Define core types** (`types.rs`)
   ```rust
   struct BoundingBox { x1: f64, y1: f64, x2: f64, y2: f64 }
   struct Block { id: u32, bbox: BoundingBox, label: String }
   struct AnnotationResult { blocks: Vec<Block>, annotated_image_path: String }
   ```

3. **Define CLI with clap** (`main.rs`)
   - All commands from the README
   - Global flags: `--backend <replicate|local>`, `--server-url <url>`

### Phase 2: OmniParser Backend

4. **Backend trait** (`backend/mod.rs`)
   ```rust
   #[async_trait]
   trait OmniParserBackend {
       async fn annotate(&self, screenshot_path: &Path) -> Result<AnnotationResult>;
   }
   ```

5. **Replicate backend** (`backend/replicate.rs`)
   - Upload image, call `microsoft/omniparser-v2` via Replicate HTTP API
   - Parse response: extract bounding boxes, labels, annotated image
   - Parameters: `box_threshold` (default 0.05), `iou_threshold` (default 0.1)

6. **Local backend** (`backend/local.rs`)
   - POST screenshot to local FastAPI server at `http://localhost:8901/annotate`
   - Same request/response format as Replicate but self-hosted

7. **Python sidecar server** (`server/`)
   ```
   server/
     requirements.txt    — omniparser deps (torch, ultralytics, transformers, fastapi, etc.)
     server.py           — FastAPI app wrapping OmniParser
     setup.sh            — install deps + download model weights
   ```
   - Single `/annotate` endpoint: accepts image (base64 or multipart), returns JSON with blocks + annotated image
   - Managed by Rust CLI: start on first annotate, health-check, auto-restart

### Phase 3: Screenshot Capture

8. **Screenshot command** (`commands/screenshot.rs`, `platform/`)
   - Linux: use `scrot` or `grim` (Wayland) via subprocess, or `xcb` crate
   - macOS: use `screencapture` via subprocess
   - Save to specified `--output` path or temp file

### Phase 4: Annotation Pipeline

9. **Annotate command** (`commands/annotate.rs`)
   - Read screenshot from `--screenshot <path>`
   - Send to selected OmniParser backend
   - Receive blocks with bounding boxes and labels
   - Assign sequential block IDs (1, 2, 3, ...)
   - Save state to `~/.percept/state.json` (block ID → bounding box mapping)
   - Output: numbered block list to stdout + annotated image path
   - Example output:
     ```
     Annotated image saved to: /tmp/percept_annotated_1234.png

     Blocks detected:
       [1] "Search bar" (120,45)-(580,75)
       [2] "Submit button" (590,45)-(660,75)
       [3] "Navigation menu" (10,10)-(100,30)
       ...
     ```

### Phase 5: Interaction Commands

10. **Click command** (`commands/click.rs`)
    - Load state from `~/.percept/state.json`
    - Look up block by ID → get bounding box center
    - Apply optional `--offset <x>,<y>` relative to center
    - Execute platform-specific click (xdotool on Linux, osascript on macOS)

11. **Type command** (`commands/type_text.rs`)
    - If `--block <id>` provided: click block first, then type
    - Use `xdotool type` on Linux, osascript on macOS

12. **Scroll command** (`commands/scroll.rs`)
    - If `--block <id>`: move mouse to block center first
    - Execute scroll in `--direction` with optional `--amount`
    - Use `xdotool` scroll on Linux, osascript on macOS

### Phase 6: Polish & Distribution

13. **Error handling & user experience**
    - Helpful error messages when backend is unavailable
    - `percept setup` command to install local OmniParser + download weights
    - Config file at `~/.percept/config.toml` for default backend, API keys, thresholds

14. **Testing**
    - Unit tests for types, state management, CLI parsing
    - Integration tests with mock OmniParser server
    - Test fixtures with sample screenshots and expected annotations

15. **Documentation & packaging**
    - Update README with full usage guide
    - `cargo publish` preparation
    - Include sidecar server in release assets

---

## Key Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Language | Rust | Fast CLI, single binary, specified in README |
| Default backend | Replicate API | Zero setup, works immediately |
| Local backend | Python sidecar | OmniParser is Python; re-implementing in Rust is impractical |
| State storage | JSON file | Simple, human-readable, no database needed |
| Platform interaction | Subprocess (xdotool, etc.) | Reliable, well-tested tools |
| Async runtime | tokio | Standard for Rust async HTTP |

## Dependencies

### Rust (Cargo.toml)
- `clap` — CLI argument parsing (derive macros)
- `serde` + `serde_json` — serialization
- `reqwest` — HTTP client (for Replicate API + local server)
- `tokio` — async runtime
- `base64` — image encoding
- `image` — image manipulation (optional, for rendering annotations)
- `tempfile` — temp file management
- `dirs` — platform config/data directories
- `toml` — config file parsing

### Python sidecar (requirements.txt)
- `torch` + `torchvision` — PyTorch
- `ultralytics` — YOLOv8
- `transformers` — Florence2
- `fastapi` + `uvicorn` — HTTP server
- `Pillow` — image processing
- `easyocr` — OCR component

## OmniParser Output Format

OmniParser V2 returns:
- **Bounding boxes**: `[x1, y1, x2, y2]` normalized coordinates (0.0-1.0) for each detected UI element
- **Labels/captions**: semantic descriptions like "search bar", "submit button", "navigation menu"
- **Confidence scores**: detection confidence per element
- **Annotated image**: original screenshot with numbered boxes drawn on it

These map directly to percept's `Block` type with sequential IDs assigned by the CLI.

---

## Execution Order

```
Phase 1 (scaffolding)  →  Phase 2 (backend)  →  Phase 3 (screenshot)
                                                        ↓
Phase 6 (polish)  ←  Phase 5 (interactions)  ←  Phase 4 (annotate)
```

Phases 2 and 3 can be developed in parallel. Phase 4 depends on both. Phase 5 depends on Phase 4 for state. Phase 6 is final polish.
