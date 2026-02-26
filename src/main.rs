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

        /// Scale factor for the screenshot
        #[arg(long)]
        scale: Option<f64>,

        /// Take screenshot without annotations
        #[arg(long)]
        no_annotations: bool,

        /// Confidence threshold for box detection (default: 0.05)
        #[arg(long, default_value = "0.05")]
        box_threshold: f32,

        /// IOU threshold for non-maximum suppression (default: 0.7)
        #[arg(long, default_value = "0.7")]
        iou_threshold: f64,

        /// Enable Florence-2 icon captioning
        #[arg(long)]
        captions: bool,
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
    Setup {
        /// Also download Florence-2 models for icon captioning (~400MB)
        #[arg(long)]
        with_captions: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Screenshot {
            output,
            scale,
            no_annotations,
            box_threshold,
            iou_threshold,
            captions,
        } => {
            commands::screenshot::run_screenshot(
                &output,
                scale,
                no_annotations,
                box_threshold,
                iou_threshold,
                captions,
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
        Commands::Setup { with_captions } => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(commands::setup::run_setup(with_captions))?;
        }
    }

    Ok(())
}
