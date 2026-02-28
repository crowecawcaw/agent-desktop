pub mod nms;
pub mod preprocessing;
pub mod yolo;

use anyhow::{Context, Result};
use image::DynamicImage;
use std::path::PathBuf;

use crate::types::Block;

pub struct InferenceEngine {
    yolo: yolo::YoloDetector,
}

impl InferenceEngine {
    pub fn new(models_dir: &PathBuf) -> Result<Self> {
        let yolo = yolo::YoloDetector::new(
            models_dir.join("icon_detect.onnx").to_str().unwrap(),
        )
        .context("Failed to load YOLO model. Run `percept setup` to download models.")?;

        Ok(Self { yolo })
    }

    /// Run YOLO detection and return sorted, numbered blocks
    pub fn parse(
        &mut self,
        img: &DynamicImage,
        box_threshold: f32,
        iou_threshold: f64,
        max_blocks: Option<u32>,
        debug: bool,
    ) -> Result<Vec<Block>> {
        let t0 = std::time::Instant::now();

        let mut detections = self.yolo.detect(img, box_threshold, iou_threshold)?;

        if debug { eprintln!("[timing] YOLO: {:.0}ms", t0.elapsed().as_millis()); }

        // Keep top-N by confidence before spatial sort
        if let Some(max) = max_blocks {
            detections.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
            detections.truncate(max as usize);
        }

        // Number by Z-order so spatially close boxes get adjacent IDs
        nms::sort_by_spatial_locality(&mut detections);

        let blocks = detections
            .into_iter()
            .enumerate()
            .map(|(i, det)| Block {
                id: (i + 1) as u32,
                bbox: det.bbox,
            })
            .collect();

        Ok(blocks)
    }
}

/// Get the default models directory
pub fn models_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("percept")
        .join("models")
}
