use anyhow::{Context, Result};
use image::{DynamicImage, GenericImageView, RgbImage};
use ort::session::Session;
use ort::value::Tensor;

use crate::inference::preprocessing::{
    image_to_chw_normalized, resize_for_ocr_rec, resize_max_side,
};
use crate::types::{BoundingBox, OcrResult};

const OCR_DET_MAX_SIDE: u32 = 960;
const OCR_REC_HEIGHT: u32 = 48;
const OCR_REC_MAX_WIDTH: u32 = 320;
const DB_THRESHOLD: f32 = 0.3;
const DB_BOX_THRESHOLD: f32 = 0.5;
const MIN_TEXT_AREA: f64 = 0.0001;

pub struct OcrEngine {
    det_session: Session,
    rec_session: Session,
    dictionary: Vec<char>,
}

impl OcrEngine {
    pub fn new(det_model_path: &str, rec_model_path: &str, dict_path: &str) -> Result<Self> {
        let det_session = Session::builder()
            .context("Failed to create ONNX session builder")?
            .commit_from_file(det_model_path)
            .context(format!(
                "Failed to load OCR det model from {}",
                det_model_path
            ))?;

        let rec_session = Session::builder()
            .context("Failed to create ONNX session builder")?
            .commit_from_file(rec_model_path)
            .context(format!(
                "Failed to load OCR rec model from {}",
                rec_model_path
            ))?;

        let dict_content =
            std::fs::read_to_string(dict_path).context("Failed to read OCR dictionary")?;
        let mut dictionary: Vec<char> = dict_content
            .lines()
            .filter_map(|l| l.chars().next())
            .collect();
        dictionary.insert(0, ' ');

        Ok(Self {
            det_session,
            rec_session,
            dictionary,
        })
    }

    pub fn detect_and_recognize(&mut self, img: &DynamicImage) -> Result<Vec<OcrResult>> {
        let text_boxes = self.detect_text(img)?;
        let mut results = Vec::new();

        for bbox in &text_boxes {
            let crop = crop_text_region(img, bbox);
            if let Ok((text, confidence)) = self.recognize_text(&crop) {
                if !text.trim().is_empty() && confidence > 0.5 {
                    results.push(OcrResult {
                        bbox: bbox.clone(),
                        text: text.trim().to_string(),
                        confidence,
                    });
                }
            }
        }

        Ok(results)
    }

    fn detect_text(&mut self, img: &DynamicImage) -> Result<Vec<BoundingBox>> {
        let (resized, _scale) = resize_max_side(img, OCR_DET_MAX_SIDE);
        let input = image_to_chw_normalized(&resized)?;

        let shape: Vec<i64> = input.shape().iter().map(|&s| s as i64).collect();
        let data: Vec<f32> = input.into_raw_vec_and_offset().0;
        let tensor =
            Tensor::from_array((shape, data)).context("Failed to create OCR det input tensor")?;

        let outputs = self
            .det_session
            .run(ort::inputs!["x" => tensor])
            .context("OCR detection inference failed")?;

        let output_val = &outputs[0];
        let (out_shape, prob_data) = output_val
            .try_extract_tensor::<f32>()
            .context("Failed to extract OCR det output tensor")?;

        let dims: Vec<usize> = out_shape.iter().map(|&d| d as usize).collect();
        let det_w = resized.width();
        let det_h = resized.height();
        let (orig_w, orig_h) = img.dimensions();

        let boxes = db_postprocess(prob_data, &dims, det_w, det_h, orig_w, orig_h)?;

        Ok(boxes)
    }

    fn recognize_text(&mut self, crop: &RgbImage) -> Result<(String, f64)> {
        let resized = resize_for_ocr_rec(crop, OCR_REC_HEIGHT, OCR_REC_MAX_WIDTH);
        let input = image_to_chw_normalized(&resized)?;

        let shape: Vec<i64> = input.shape().iter().map(|&s| s as i64).collect();
        let data: Vec<f32> = input.into_raw_vec_and_offset().0;
        let tensor =
            Tensor::from_array((shape, data)).context("Failed to create OCR rec input tensor")?;

        let outputs = self
            .rec_session
            .run(ort::inputs!["x" => tensor])
            .context("OCR recognition inference failed")?;

        let output_val = &outputs[0];
        let (out_shape, logits_data) = output_val
            .try_extract_tensor::<f32>()
            .context("Failed to extract OCR rec output tensor")?;

        let dims: Vec<usize> = out_shape.iter().map(|&d| d as usize).collect();
        let result = ctc_decode(logits_data, &dims, &self.dictionary)?;
        Ok(result)
    }
}

fn crop_text_region(img: &DynamicImage, bbox: &BoundingBox) -> RgbImage {
    let (w, h) = img.dimensions();
    let x1 = (bbox.x1 * w as f64) as u32;
    let y1 = (bbox.y1 * h as f64) as u32;
    let x2 = (bbox.x2 * w as f64) as u32;
    let y2 = (bbox.y2 * h as f64) as u32;
    let cw = (x2 - x1).max(1);
    let ch = (y2 - y1).max(1);
    img.crop_imm(
        x1.min(w - 1),
        y1.min(h - 1),
        cw.min(w - x1),
        ch.min(h - y1),
    )
    .to_rgb8()
}

fn db_postprocess(
    prob_data: &[f32],
    dims: &[usize],
    det_w: u32,
    det_h: u32,
    _orig_w: u32,
    _orig_h: u32,
) -> Result<Vec<BoundingBox>> {
    let (h, w) = match dims.len() {
        4 => (dims[2], dims[3]),
        3 => (dims[1], dims[2]),
        2 => (dims[0], dims[1]),
        _ => anyhow::bail!("Unexpected OCR det output shape"),
    };

    let offset = prob_data.len() - h * w;
    let mut boxes = Vec::new();

    let mut binary = vec![false; h * w];
    for i in 0..(h * w) {
        binary[i] = prob_data[offset + i] > DB_THRESHOLD;
    }

    let mut visited = vec![false; h * w];
    for y in 0..h {
        for x in 0..w {
            if binary[y * w + x] && !visited[y * w + x] {
                let mut min_x = x;
                let mut min_y = y;
                let mut max_x = x;
                let mut max_y = y;
                let mut sum_prob = 0.0f32;
                let mut count = 0;
                let mut queue = vec![(x, y)];
                visited[y * w + x] = true;

                while let Some((cx, cy)) = queue.pop() {
                    min_x = min_x.min(cx);
                    min_y = min_y.min(cy);
                    max_x = max_x.max(cx);
                    max_y = max_y.max(cy);
                    sum_prob += prob_data[offset + cy * w + cx];
                    count += 1;

                    for (dx, dy) in &[(0i32, 1i32), (0, -1), (1, 0), (-1, 0)] {
                        let nx = cx as i32 + dx;
                        let ny = cy as i32 + dy;
                        if nx >= 0
                            && nx < w as i32
                            && ny >= 0
                            && ny < h as i32
                            && binary[ny as usize * w + nx as usize]
                            && !visited[ny as usize * w + nx as usize]
                        {
                            visited[ny as usize * w + nx as usize] = true;
                            queue.push((nx as usize, ny as usize));
                        }
                    }
                }

                let avg_prob = sum_prob / count as f32;
                if avg_prob < DB_BOX_THRESHOLD {
                    continue;
                }

                let expand_w = ((max_x - min_x) as f64 * 0.25) as usize;
                let expand_h = ((max_y - min_y) as f64 * 0.25) as usize;
                let min_x = min_x.saturating_sub(expand_w);
                let min_y = min_y.saturating_sub(expand_h);
                let max_x = (max_x + expand_w).min(w - 1);
                let max_y = (max_y + expand_h).min(h - 1);

                let nx1 = min_x as f64 / det_w as f64;
                let ny1 = min_y as f64 / det_h as f64;
                let nx2 = max_x as f64 / det_w as f64;
                let ny2 = max_y as f64 / det_h as f64;

                let bbox = BoundingBox::new(
                    nx1.clamp(0.0, 1.0),
                    ny1.clamp(0.0, 1.0),
                    nx2.clamp(0.0, 1.0),
                    ny2.clamp(0.0, 1.0),
                );

                if bbox.area() >= MIN_TEXT_AREA {
                    boxes.push(bbox);
                }
            }
        }
    }

    Ok(boxes)
}

fn ctc_decode(logits: &[f32], dims: &[usize], dictionary: &[char]) -> Result<(String, f64)> {
    let (seq_len, vocab_size) = match dims.len() {
        3 => (dims[1], dims[2]),
        2 => (dims[0], dims[1]),
        _ => anyhow::bail!("Unexpected OCR rec output shape"),
    };

    let offset = logits.len() - seq_len * vocab_size;
    let mut text = String::new();
    let mut prev_idx = 0usize;
    let mut total_conf = 0.0f64;
    let mut char_count = 0;

    for t in 0..seq_len {
        let mut max_idx = 0;
        let mut max_val = f32::NEG_INFINITY;
        for v in 0..vocab_size {
            let val = logits[offset + t * vocab_size + v];
            if val > max_val {
                max_val = val;
                max_idx = v;
            }
        }

        if max_idx != 0 && max_idx != prev_idx {
            if max_idx < dictionary.len() {
                text.push(dictionary[max_idx]);
                total_conf += max_val as f64;
                char_count += 1;
            }
        }
        prev_idx = max_idx;
    }

    let avg_conf = if char_count > 0 {
        total_conf / char_count as f64
    } else {
        0.0
    };

    Ok((text, avg_conf))
}
