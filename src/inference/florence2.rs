use anyhow::{Context, Result};
use image::{DynamicImage, GenericImageView};
use ort::session::Session;
use ort::value::Tensor;

use crate::inference::preprocessing::image_to_chw_normalized;
use crate::types::BoundingBox;

const FLORENCE2_INPUT_SIZE: u32 = 768;

pub struct Florence2Session {
    encoder: Session,
    decoder: Session,
    tokenizer: tokenizers::Tokenizer,
}

impl Florence2Session {
    pub fn new(encoder_path: &str, decoder_path: &str, tokenizer_path: &str) -> Result<Self> {
        let encoder = Session::builder()
            .context("Failed to create ONNX session builder")?
            .commit_from_file(encoder_path)
            .context(format!(
                "Failed to load Florence-2 encoder from {}",
                encoder_path
            ))?;

        let decoder = Session::builder()
            .context("Failed to create ONNX session builder")?
            .commit_from_file(decoder_path)
            .context(format!(
                "Failed to load Florence-2 decoder from {}",
                decoder_path
            ))?;

        let tokenizer = tokenizers::Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?;

        Ok(Self {
            encoder,
            decoder,
            tokenizer,
        })
    }

    pub fn caption_icon(&mut self, img: &DynamicImage, bbox: &BoundingBox) -> Result<String> {
        let (w, h) = img.dimensions();
        let x1 = (bbox.x1 * w as f64) as u32;
        let y1 = (bbox.y1 * h as f64) as u32;
        let x2 = (bbox.x2 * w as f64) as u32;
        let y2 = (bbox.y2 * h as f64) as u32;
        let cw = (x2 - x1).max(1);
        let ch = (y2 - y1).max(1);
        let crop = img.crop_imm(
            x1.min(w - 1),
            y1.min(h - 1),
            cw.min(w - x1),
            ch.min(h - y1),
        );

        let resized = crop
            .resize_exact(
                FLORENCE2_INPUT_SIZE,
                FLORENCE2_INPUT_SIZE,
                image::imageops::FilterType::Lanczos3,
            )
            .to_rgb8();

        let input_tensor = image_to_chw_normalized(&resized)?;

        // Run encoder
        let shape: Vec<i64> = input_tensor.shape().iter().map(|&s| s as i64).collect();
        let data: Vec<f32> = input_tensor.into_raw_vec_and_offset().0;
        let pixel_values =
            Tensor::from_array((shape, data)).context("Failed to create encoder input tensor")?;

        let (enc_features, enc_dims) = {
            let encoder_outputs = self
                .encoder
                .run(ort::inputs!["pixel_values" => pixel_values])
                .context("Florence-2 encoder failed")?;

            let (enc_shape, enc_data) = encoder_outputs[0]
                .try_extract_tensor::<f32>()
                .context("Failed to extract encoder features")?;

            let dims: Vec<i64> = enc_shape.iter().map(|&d| d as i64).collect();
            let features = enc_data.to_vec();
            (features, dims)
        };

        // Autoregressive decoding
        let caption = self.autoregressive_decode(&enc_features, &enc_dims)?;
        Ok(caption)
    }

    fn autoregressive_decode(
        &mut self,
        encoder_features: &[f32],
        enc_dims: &[i64],
    ) -> Result<String> {
        let encoding = self
            .tokenizer
            .encode("<CAPTION>", false)
            .map_err(|e| anyhow::anyhow!("Tokenization failed: {}", e))?;

        let mut token_ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
        let max_length = 64;

        for _ in 0..max_length {
            let seq_len = token_ids.len();
            let input_ids = Tensor::from_array((vec![1i64, seq_len as i64], token_ids.clone()))
                .context("Failed to create decoder input tensor")?;

            let enc_tensor =
                Tensor::from_array((enc_dims.to_vec(), encoder_features.to_vec()))
                    .context("Failed to create encoder hidden states tensor")?;

            let outputs = self
                .decoder
                .run(ort::inputs![
                    "input_ids" => input_ids,
                    "encoder_hidden_states" => enc_tensor
                ])
                .context("Florence-2 decoder failed")?;

            let (logits_shape, logits_data) = outputs[0]
                .try_extract_tensor::<f32>()
                .context("Failed to extract decoder logits")?;

            let logits_dims: Vec<usize> = logits_shape.iter().map(|&d| d as usize).collect();
            let vocab_size = logits_dims[logits_dims.len() - 1];
            let last_pos = logits_dims[1] - 1;

            let offset = last_pos * vocab_size;
            let mut max_idx = 0;
            let mut max_val = f32::NEG_INFINITY;
            for v in 0..vocab_size {
                let val = logits_data[offset + v];
                if val > max_val {
                    max_val = val;
                    max_idx = v;
                }
            }

            let next_token = max_idx as i64;
            if next_token == 2 {
                break;
            }

            token_ids.push(next_token);
        }

        let decoded = self
            .tokenizer
            .decode(
                &token_ids.iter().map(|&id| id as u32).collect::<Vec<_>>(),
                true,
            )
            .map_err(|e| anyhow::anyhow!("Token decoding failed: {}", e))?;

        let caption = decoded
            .strip_prefix("<CAPTION>")
            .unwrap_or(&decoded)
            .trim()
            .to_string();

        Ok(caption)
    }
}
