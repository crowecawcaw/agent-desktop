use crate::types::{BoundingBox, Detection};

/// Compute intersection-over-union between two bounding boxes
pub fn iou(a: &BoundingBox, b: &BoundingBox) -> f64 {
    let x1 = a.x1.max(b.x1);
    let y1 = a.y1.max(b.y1);
    let x2 = a.x2.min(b.x2);
    let y2 = a.y2.min(b.y2);

    let intersection = (x2 - x1).max(0.0) * (y2 - y1).max(0.0);
    let union = a.area() + b.area() - intersection;

    if union <= 0.0 {
        0.0
    } else {
        intersection / union
    }
}

/// Non-maximum suppression: remove overlapping detections, keeping highest confidence
pub fn nms(detections: &mut Vec<Detection>, iou_threshold: f64) {
    detections.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

    let mut keep = vec![true; detections.len()];

    for i in 0..detections.len() {
        if !keep[i] {
            continue;
        }
        for j in (i + 1)..detections.len() {
            if !keep[j] {
                continue;
            }
            if iou(&detections[i].bbox, &detections[j].bbox) > iou_threshold {
                keep[j] = false;
            }
        }
    }

    let mut idx = 0;
    detections.retain(|_| {
        let k = keep[idx];
        idx += 1;
        k
    });
}

/// Sort detections by Z-order (Morton) curve so spatially close boxes get adjacent IDs.
pub fn sort_by_spatial_locality(detections: &mut [Detection]) {
    detections.sort_by_key(|d| morton(d.bbox.x1, d.bbox.y1));
}

/// Compute Morton (Z-order) code by interleaving 16-bit x and y.
fn morton(x: f64, y: f64) -> u64 {
    let xi = (x.clamp(0.0, 1.0) * 65535.0) as u64;
    let yi = (y.clamp(0.0, 1.0) * 65535.0) as u64;
    let mut z = 0u64;
    for i in 0..16 {
        z |= ((xi >> i) & 1) << (2 * i + 1);
        z |= ((yi >> i) & 1) << (2 * i);
    }
    z
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iou_no_overlap() {
        let a = BoundingBox::new(0.0, 0.0, 0.5, 0.5);
        let b = BoundingBox::new(0.6, 0.6, 1.0, 1.0);
        assert!((iou(&a, &b) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_iou_perfect_overlap() {
        let a = BoundingBox::new(0.0, 0.0, 1.0, 1.0);
        let b = BoundingBox::new(0.0, 0.0, 1.0, 1.0);
        assert!((iou(&a, &b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_iou_partial_overlap() {
        let a = BoundingBox::new(0.0, 0.0, 0.5, 0.5);
        let b = BoundingBox::new(0.25, 0.25, 0.75, 0.75);
        let expected = 0.0625 / 0.4375;
        assert!((iou(&a, &b) - expected).abs() < 1e-6);
    }

    #[test]
    fn test_nms_removes_overlapping() {
        let mut dets = vec![
            Detection {
                bbox: BoundingBox::new(0.0, 0.0, 0.5, 0.5),
                confidence: 0.9,
            },
            Detection {
                bbox: BoundingBox::new(0.05, 0.05, 0.55, 0.55),
                confidence: 0.7,
            },
            Detection {
                bbox: BoundingBox::new(0.8, 0.8, 1.0, 1.0),
                confidence: 0.8,
            },
        ];
        nms(&mut dets, 0.3);
        assert_eq!(dets.len(), 2);
        assert!((dets[0].confidence - 0.9).abs() < 1e-6);
        assert!((dets[1].confidence - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_sort_by_spatial_locality() {
        let mut detections = vec![
            Detection { bbox: BoundingBox::new(0.9, 0.9, 1.0, 1.0), confidence: 0.9 }, // bottom-right
            Detection { bbox: BoundingBox::new(0.0, 0.0, 0.1, 0.1), confidence: 0.9 }, // top-left
            Detection { bbox: BoundingBox::new(0.5, 0.5, 0.6, 0.6), confidence: 0.9 }, // center
        ];
        sort_by_spatial_locality(&mut detections);
        // top-left should be first (lowest Morton code), bottom-right last
        assert!((detections[0].bbox.x1 - 0.0).abs() < 1e-6);
        assert!((detections[2].bbox.x1 - 0.9).abs() < 1e-6);
    }

    #[test]
    fn test_morton_nearby_boxes_have_close_codes() {
        // Two adjacent boxes should have closer Morton codes than two distant boxes
        let close_a = morton(0.5, 0.5);
        let close_b = morton(0.51, 0.5);
        let far = morton(0.9, 0.9);
        let close_diff = close_a.abs_diff(close_b);
        let far_diff = close_a.abs_diff(far);
        assert!(close_diff < far_diff);
    }
}
