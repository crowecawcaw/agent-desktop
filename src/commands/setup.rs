use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

use crate::inference::models_dir;

struct ModelInfo {
    filename: &'static str,
    url: &'static str,
    sha256: &'static str,
}

const MODELS: &[ModelInfo] = &[
    ModelInfo {
        filename: "icon_detect.onnx",
        url: "https://huggingface.co/onnx-community/OmniParser-icon_detect/resolve/main/onnx/model.onnx",
        sha256: "",
    },
];

pub async fn run_setup() -> Result<()> {
    let dir = models_dir();
    std::fs::create_dir_all(&dir).context("Failed to create models directory")?;

    println!("Models directory: {:?}", dir);

    let client = reqwest::Client::new();

    for model in MODELS {
        let dest = dir.join(model.filename);
        if dest.exists() {
            println!("  {} (already exists, skipping)", model.filename);
            continue;
        }

        if model.url.is_empty() {
            println!("  {} (no URL configured, skipping)", model.filename);
            continue;
        }
        let url = model.url.to_string();
        println!("  Downloading {}...", model.filename);

        download_file(&client, &url, &dest).await?;

        // Verify checksum if provided
        if !model.sha256.is_empty() {
            verify_checksum(&dest, model.sha256)?;
        }
    }

    // Download ONNX Runtime dylib if not already present
    download_ort_runtime(&dir, &client).await?;

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

fn ort_dylib_filename() -> &'static str {
    match std::env::consts::OS {
        "macos" => "libonnxruntime.dylib",
        "windows" => "onnxruntime.dll",
        _ => "libonnxruntime.so",
    }
}

/// Check whether the ONNX Runtime dylib is accessible, and guide the user if not.
async fn download_ort_runtime(_dir: &PathBuf, _client: &reqwest::Client) -> Result<()> {
    let dylib = ort_dylib_filename();

    // Check if ORT_DYLIB_PATH is already set by the user
    if let Ok(p) = std::env::var("ORT_DYLIB_PATH") {
        if std::path::Path::new(&p).exists() {
            println!("  ONNX Runtime: found via ORT_DYLIB_PATH ({}).", p);
            return Ok(());
        }
    }

    // Check common system locations
    let system_paths: &[&str] = match std::env::consts::OS {
        "macos" => &[
            "/opt/homebrew/lib/libonnxruntime.dylib",
            "/usr/local/lib/libonnxruntime.dylib",
        ],
        "linux" => &[
            "/usr/lib/libonnxruntime.so",
            "/usr/local/lib/libonnxruntime.so",
            "/usr/lib/x86_64-linux-gnu/libonnxruntime.so",
            "/usr/lib/aarch64-linux-gnu/libonnxruntime.so",
        ],
        _ => &[],
    };

    for path in system_paths {
        if std::path::Path::new(path).exists() {
            println!("  ONNX Runtime: found at {}.", path);
            return Ok(());
        }
    }

    // Not found — guide the user
    println!("  ONNX Runtime ({}) not found on this system.", dylib);
    match std::env::consts::OS {
        "macos" => println!("  Install it with: brew install onnxruntime"),
        "linux" => println!("  Install it with: apt install libonnxruntime-dev  (or equivalent)"),
        _ => println!("  Download ONNX Runtime from https://github.com/microsoft/onnxruntime/releases"),
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
