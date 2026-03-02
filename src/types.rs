use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Bounding box with coordinates normalized to [0.0, 1.0] range
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BoundingBox {
    pub x1: f64, // top-left x
    pub y1: f64, // top-left y
    pub x2: f64, // bottom-right x
    pub y2: f64, // bottom-right y
}

impl BoundingBox {
    pub fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        Self { x1, y1, x2, y2 }
    }

    pub fn width(&self) -> f64 {
        self.x2 - self.x1
    }

    pub fn height(&self) -> f64 {
        self.y2 - self.y1
    }

    pub fn area(&self) -> f64 {
        self.width() * self.height()
    }

    pub fn center(&self) -> (f64, f64) {
        ((self.x1 + self.x2) / 2.0, (self.y1 + self.y2) / 2.0)
    }

    /// Compute center pixel coordinates given image dimensions
    pub fn center_pixels(&self, img_width: u32, img_height: u32) -> (i32, i32) {
        let (cx, cy) = self.center();
        ((cx * img_width as f64) as i32, (cy * img_height as f64) as i32)
    }
}

/// A detected UI element with an assigned block ID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub id: u32,
    pub bbox: BoundingBox,
}

/// Result of running the annotation pipeline
#[allow(dead_code)]
pub struct AnnotationResult {
    pub blocks: Vec<Block>,
    pub annotated_image_path: PathBuf,
}

/// Raw detection from YOLO before NMS
#[derive(Debug, Clone)]
pub struct Detection {
    pub bbox: BoundingBox,
    pub confidence: f64,
}


