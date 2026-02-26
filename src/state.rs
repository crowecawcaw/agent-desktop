use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::types::Block;

#[derive(Debug, Serialize, Deserialize)]
pub struct PerceptState {
    pub blocks: Vec<Block>,
    pub image_width: u32,
    pub image_height: u32,
    pub screenshot_path: Option<String>,
}

impl PerceptState {
    pub fn new(blocks: Vec<Block>, image_width: u32, image_height: u32) -> Self {
        Self {
            blocks,
            image_width,
            image_height,
            screenshot_path: None,
        }
    }

    fn state_path() -> Result<PathBuf> {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("percept");
        std::fs::create_dir_all(&data_dir)
            .context("Failed to create percept data directory")?;
        Ok(data_dir.join("state.json"))
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::state_path()?;
        let json = serde_json::to_string_pretty(self)
            .context("Failed to serialize state")?;
        std::fs::write(&path, json).context("Failed to write state file")?;
        Ok(())
    }

    pub fn load() -> Result<Self> {
        let path = Self::state_path()?;
        if !path.exists() {
            anyhow::bail!(
                "No annotation state found. Run `percept screenshot` first to annotate the screen."
            );
        }
        let json = std::fs::read_to_string(&path).context("Failed to read state file")?;
        let state: PerceptState =
            serde_json::from_str(&json).context("Failed to parse state file")?;
        Ok(state)
    }

    pub fn get_block(&self, id: u32) -> Result<&Block> {
        self.blocks
            .iter()
            .find(|b| b.id == id)
            .ok_or_else(|| anyhow::anyhow!("Block {} not found. Available blocks: 1-{}", id, self.blocks.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::BoundingBox;

    #[test]
    fn test_state_roundtrip() {
        let blocks = vec![
            Block {
                id: 1,
                bbox: BoundingBox::new(0.1, 0.2, 0.3, 0.4),
                label: "Button".to_string(),
                interactable: true,
            },
            Block {
                id: 2,
                bbox: BoundingBox::new(0.5, 0.6, 0.7, 0.8),
                label: "Text field".to_string(),
                interactable: false,
            },
        ];
        let state = PerceptState::new(blocks, 1920, 1080);
        let json = serde_json::to_string(&state).unwrap();
        let loaded: PerceptState = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.blocks.len(), 2);
        assert_eq!(loaded.image_width, 1920);
        assert_eq!(loaded.blocks[0].label, "Button");
    }

    #[test]
    fn test_get_block() {
        let blocks = vec![
            Block {
                id: 1,
                bbox: BoundingBox::new(0.1, 0.2, 0.3, 0.4),
                label: "OK".to_string(),
                interactable: true,
            },
        ];
        let state = PerceptState::new(blocks, 800, 600);
        assert!(state.get_block(1).is_ok());
        assert!(state.get_block(99).is_err());
    }
}
