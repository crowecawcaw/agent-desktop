use anyhow::{Context, Result};
use ndarray::Array4;
use ort::session::Session;
use ort::value::Tensor;

use crate::inference::preprocessing::{image_to_chw_tensor, letterbox_resize, LetterboxInfo};
use crate::inference::nms;
use crate::types::{BoundingBox, Detection};

const YOLO_INPUT_SIZE: u32 = 640;

pub struct YoloDetector {
    session: Session,
}

impl YoloDetector {
    pub fn new(model_path: &str) -> Result<Self> {
        let session = Session::builder()
            .context("Failed to create ONNX session builder")?
            .commit_from_file(model_path)
            .context(format!("Failed to load YOLO model from {}", model_path))?;
        Ok(Self { session })
    }

    pub fn detect(
        &mut self,
        img: &image::DynamicImage,
        box_threshold: f32,
        iou_threshold: f64,
    ) -> Result<Vec<Detection>> {
        let (letterboxed, info) = letterbox_resize(img, YOLO_INPUT_SIZE);
        let input_tensor = image_to_chw_tensor(&letterboxed)?;
        let raw_output = self.run_inference(input_tensor)?;
        let detections = postprocess(&raw_output, &info, box_threshold, iou_threshold)?;
        Ok(detections)
    }

    fn run_inference(&mut self, input: Array4<f32>) -> Result<Vec<f32>> {
        let shape: Vec<i64> = input.shape().iter().map(|&s| s as i64).collect();
        let data: Vec<f32> = input.into_raw_vec_and_offset().0;
        let tensor = Tensor::from_array((shape, data))
            .context("Failed to create input tensor")?;

        let outputs = self
            .session
            .run(ort::inputs!["images" => tensor])
            .context("YOLO inference failed")?;

        let output_val = &outputs[0];
        let (shape, raw_data) = output_val
            .try_extract_tensor::<f32>()
            .context("Failed to extract YOLO output tensor")?;

        // Store shape info in the output data for postprocessing
        // Shape is [1, 5, 8400] typically
        let dims: Vec<usize> = shape.iter().map(|&d| d as usize).collect();
        let _ = dims; // We'll infer from data length

        Ok(raw_data.to_vec())
    }
}

/// Output shape: [1, 5, 8400] flattened
/// Each of 8400 predictions has: [cx, cy, w, h, confidence]
fn postprocess(
    raw_data: &[f32],
    info: &LetterboxInfo,
    box_threshold: f32,
    iou_threshold: f64,
) -> Result<Vec<Detection>> {
    // Assume output shape [1, 5, 8400]
    let num_values = 5;
    let num_predictions = raw_data.len() / num_values;
    if raw_data.len() != num_values * num_predictions {
        anyhow::bail!(
            "Unexpected YOLO output size: {} (expected multiple of 5)",
            raw_data.len()
        );
    }

    let mut detections = Vec::new();

    for i in 0..num_predictions {
        // Data is in [1, 5, 8400] layout (channel-first):
        // index for (batch=0, channel=c, pred=i) = c * num_predictions + i
        let confidence = raw_data[4 * num_predictions + i];

        if confidence < box_threshold {
            continue;
        }

        let cx = raw_data[0 * num_predictions + i] as f64;
        let cy = raw_data[1 * num_predictions + i] as f64;
        let w = raw_data[2 * num_predictions + i] as f64;
        let h = raw_data[3 * num_predictions + i] as f64;

        let x1 = cx - w / 2.0;
        let y1 = cy - h / 2.0;
        let x2 = cx + w / 2.0;
        let y2 = cy + h / 2.0;

        let (nx1, ny1, nx2, ny2) = info.to_normalized(x1, y1, x2, y2);

        if nx2 - nx1 < 0.001 || ny2 - ny1 < 0.001 {
            continue;
        }

        detections.push(Detection {
            bbox: BoundingBox::new(nx1, ny1, nx2, ny2),
            confidence: confidence as f64,
        });
    }

    nms::nms(&mut detections, iou_threshold);

    Ok(detections)
}
