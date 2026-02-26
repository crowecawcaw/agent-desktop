use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

use crate::inference::models_dir;

const MODEL_BASE_URL: &str =
    "https://github.com/nicholaschenai/omniparser-onnx/releases/download/v0.1.0";

struct ModelInfo {
    filename: &'static str,
    sha256: &'static str,
    required: bool, // true = core, false = captions only
}

const MODELS: &[ModelInfo] = &[
    ModelInfo {
        filename: "icon_detect.onnx",
        sha256: "",
        required: true,
    },
    ModelInfo {
        filename: "text_det.onnx",
        sha256: "",
        required: true,
    },
    ModelInfo {
        filename: "text_rec.onnx",
        sha256: "",
        required: true,
    },
    ModelInfo {
        filename: "rec_dictionary.txt",
        sha256: "",
        required: true,
    },
    ModelInfo {
        filename: "florence2_encoder.onnx",
        sha256: "",
        required: false,
    },
    ModelInfo {
        filename: "florence2_decoder.onnx",
        sha256: "",
        required: false,
    },
    ModelInfo {
        filename: "tokenizer.json",
        sha256: "",
        required: false,
    },
];

pub async fn run_setup(with_captions: bool) -> Result<()> {
    let dir = models_dir();
    std::fs::create_dir_all(&dir).context("Failed to create models directory")?;

    println!("Models directory: {:?}", dir);

    let client = reqwest::Client::new();

    for model in MODELS {
        if !model.required && !with_captions {
            continue;
        }

        let dest = dir.join(model.filename);
        if dest.exists() {
            println!("  {} (already exists, skipping)", model.filename);
            continue;
        }

        let url = format!("{}/{}", MODEL_BASE_URL, model.filename);
        println!("  Downloading {}...", model.filename);

        download_file(&client, &url, &dest).await?;

        // Verify checksum if provided
        if !model.sha256.is_empty() {
            verify_checksum(&dest, model.sha256)?;
        }
    }

    println!("\nSetup complete! Models are ready.");

    // Check GPU availability
    check_gpu_availability();

    Ok(())
}

async fn download_file(client: &reqwest::Client, url: &str, dest: &PathBuf) -> Result<()> {
    use futures_util::StreamExt;

    let response = client
        .get(url)
        .send()
        .await
        .context(format!("Failed to download {}", url))?
        .error_for_status()
        .context(format!("HTTP error downloading {}", url))?;

    let total_size = response.content_length().unwrap_or(0);

    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );

    let mut file = std::fs::File::create(dest)
        .context(format!("Failed to create file {:?}", dest))?;
    let mut stream = response.bytes_stream();

    use std::io::Write;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("Error downloading chunk")?;
        file.write_all(&chunk)
            .context("Failed to write to file")?;
        pb.inc(chunk.len() as u64);
    }

    pb.finish_with_message("done");
    Ok(())
}

fn verify_checksum(path: &PathBuf, expected: &str) -> Result<()> {
    let data = std::fs::read(path).context("Failed to read file for checksum")?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let result = format!("{:x}", hasher.finalize());

    if result != expected {
        std::fs::remove_file(path).ok();
        anyhow::bail!(
            "Checksum mismatch for {:?}. Expected {}, got {}. File removed.",
            path,
            expected,
            result
        );
    }

    Ok(())
}

fn check_gpu_availability() {
    // Check if CUDA is available by looking for libcuda
    let has_cuda = std::path::Path::new("/usr/lib/x86_64-linux-gnu/libcuda.so").exists()
        || std::path::Path::new("/usr/lib/libcuda.so").exists()
        || std::env::var("CUDA_HOME").is_ok();

    if has_cuda {
        println!("GPU: CUDA detected. ONNX Runtime will use GPU acceleration.");
    } else {
        println!("GPU: No CUDA detected. Using CPU inference (this is fine, just slower).");
    }
}
