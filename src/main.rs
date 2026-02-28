mod commands;
mod inference;
mod platform;
mod state;
mod types;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "percept")]
#[command(about = "CLI tool that annotates screenshots using OmniParser and provides computer interaction via block IDs")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Take a screenshot, annotate with numbered blocks, and save to path
    Screenshot {
        /// Output path for the screenshot
        #[arg(long)]
        output: String,

        /// Scale factor for the screenshot (default: 0.5)
        #[arg(long, default_value = "0.5")]
        scale: f64,

        /// Take screenshot without annotations
        #[arg(long)]
        no_annotations: bool,

        /// Confidence threshold for box detection (default: 0.05)
        #[arg(long, default_value = "0.05")]
        box_threshold: f32,

        /// IOU threshold for non-maximum suppression (default: 0.7)
        #[arg(long, default_value = "0.7")]
        iou_threshold: f64,

        /// Keep only the top N highest-confidence boxes
        #[arg(long)]
        max_blocks: Option<u32>,

        /// Print timing information
        #[arg(long)]
        debug: bool,
    },

    /// Click the center of an annotated block
    Click {
        /// Block ID to click
        #[arg(long)]
        block: u32,

        /// Pixel offset relative to block center (format: x,y)
        #[arg(long)]
        offset: Option<String>,
    },

    /// Type text at the current cursor position or in a specific block
    Type {
        /// Text to type
        #[arg(long)]
        text: String,

        /// Block ID to click before typing
        #[arg(long)]
        block: Option<u32>,
    },

    /// Scroll the screen or within a specific block
    Scroll {
        /// Scroll direction (up, down, left, right)
        #[arg(long)]
        direction: String,

        /// Block ID to scroll within
        #[arg(long)]
        block: Option<u32>,

        /// Scroll amount in clicks (default: 3)
        #[arg(long)]
        amount: Option<u32>,
    },

    /// Download ONNX models for inference
    Setup,
}

/// If ORT_DYLIB_PATH is not already set, search common locations for the
/// ONNX Runtime dylib and set the env var so `ort` (load-dynamic) can find it.
fn auto_detect_ort_dylib() {
    if std::env::var("ORT_DYLIB_PATH").is_ok() {
        return;
    }

    #[cfg(target_os = "macos")]
    let dylib_name = "libonnxruntime.dylib";
    #[cfg(target_os = "linux")]
    let dylib_name = "libonnxruntime.so";
    #[cfg(target_os = "windows")]
    let dylib_name = "onnxruntime.dll";

    // 1. Check the percept models directory (downloaded by `percept setup`)
    if let Some(models_dir) = dirs::data_dir().map(|d| d.join("percept").join("models")) {
        let candidate = models_dir.join(dylib_name);
        if candidate.exists() {
            unsafe { std::env::set_var("ORT_DYLIB_PATH", &candidate) };
            return;
        }
    }

    // 2. Common system paths
    #[cfg(target_os = "macos")]
    let system_paths = &[
        "/opt/homebrew/lib/libonnxruntime.dylib",
        "/usr/local/lib/libonnxruntime.dylib",
    ];
    #[cfg(target_os = "linux")]
    let system_paths = &[
        "/usr/lib/libonnxruntime.so",
        "/usr/local/lib/libonnxruntime.so",
    ];
    #[cfg(target_os = "windows")]
    let system_paths: &[&str] = &[];

    for path in system_paths {
        if std::path::Path::new(path).exists() {
            unsafe { std::env::set_var("ORT_DYLIB_PATH", path) };
            return;
        }
    }
}

fn main() -> Result<()> {
    auto_detect_ort_dylib();
    let cli = Cli::parse();

    match cli.command {
        Commands::Screenshot {
            output,
            scale,
            no_annotations,
            box_threshold,
            iou_threshold,
            max_blocks,
            debug,
        } => {
            commands::screenshot::run_screenshot(
                &output,
                scale,
                no_annotations,
                box_threshold,
                iou_threshold,
                max_blocks,
                debug,
            )?;
        }
        Commands::Click { block, offset } => {
            let offset = match offset {
                Some(ref s) => Some(commands::click::parse_offset(s)?),
                None => None,
            };
            commands::click::run_click(block, offset)?;
        }
        Commands::Type { text, block } => {
            commands::type_text::run_type(block, &text)?;
        }
        Commands::Scroll {
            direction,
            block,
            amount,
        } => {
            commands::scroll::run_scroll(block, &direction, amount)?;
        }
        Commands::Setup => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(commands::setup::run_setup())?;
        }
    }

    Ok(())
}
