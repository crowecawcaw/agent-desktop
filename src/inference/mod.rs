pub mod florence2;
pub mod nms;
pub mod ocr;
pub mod preprocessing;
pub mod yolo;

use anyhow::{Context, Result};
use image::DynamicImage;
use std::path::PathBuf;

use crate::types::Block;

pub struct InferenceEngine {
    yolo: yolo::YoloDetector,
    ocr: ocr::OcrEngine,
    florence2: Option<florence2::Florence2Session>,
}

impl InferenceEngine {
    pub fn new(models_dir: &PathBuf, enable_captions: bool) -> Result<Self> {
        let yolo = yolo::YoloDetector::new(
            models_dir.join("icon_detect.onnx").to_str().unwrap(),
        )
        .context("Failed to load YOLO model. Run `percept setup` to download models.")?;

        let ocr = ocr::OcrEngine::new(
            models_dir.join("text_det.onnx").to_str().unwrap(),
            models_dir.join("text_rec.onnx").to_str().unwrap(),
            models_dir.join("rec_dictionary.txt").to_str().unwrap(),
        )
        .context("Failed to load OCR models. Run `percept setup` to download models.")?;

        let florence2 = if enable_captions {
            Some(
                florence2::Florence2Session::new(
                    models_dir.join("florence2_encoder.onnx").to_str().unwrap(),
                    models_dir.join("florence2_decoder.onnx").to_str().unwrap(),
                    models_dir.join("tokenizer.json").to_str().unwrap(),
                )
                .context(
                    "Failed to load Florence-2 models. Run `percept setup --with-captions` to download.",
                )?,
            )
        } else {
            None
        };

        Ok(Self {
            yolo,
            ocr,
            florence2,
        })
    }

    /// Run the full inference pipeline on an image
    pub fn parse(
        &mut self,
        img: &DynamicImage,
        box_threshold: f32,
        iou_threshold: f64,
    ) -> Result<Vec<Block>> {
        // Step 1: YOLO detection for interactive elements
        let yolo_detections = self.yolo.detect(img, box_threshold, iou_threshold)?;

        // Step 2: OCR for text regions
        let ocr_results = self.ocr.detect_and_recognize(img)?;

        // Step 3: Merge YOLO + OCR results
        let mut merged = nms::merge_detections(&yolo_detections, &ocr_results, 0.1);

        // Step 4: Optional Florence-2 captioning for unlabeled icons
        if let Some(ref mut florence) = self.florence2 {
            for element in &mut merged {
                if element.interactable && element.label.is_empty() {
                    match florence.caption_icon(img, &element.bbox) {
                        Ok(caption) => element.label = caption,
                        Err(_) => {}
                    }
                }
            }
        }

        // Step 5: Sort by position (top-to-bottom, left-to-right)
        nms::sort_by_position(&mut merged);

        // Step 6: Assign sequential IDs
        let blocks = merged
            .into_iter()
            .enumerate()
            .map(|(i, elem)| Block {
                id: (i + 1) as u32,
                bbox: elem.bbox,
                label: elem.label,
                interactable: elem.interactable,
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
