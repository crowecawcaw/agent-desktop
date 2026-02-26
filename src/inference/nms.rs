use crate::types::{BoundingBox, Detection, MergedElement, OcrResult};

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

/// Merge YOLO detections with OCR results based on IOU overlap.
///
/// - YOLO box overlapping OCR box -> merge (keep YOLO box, attach OCR text, mark interactable)
/// - YOLO box without OCR overlap -> "icon" (mark interactable, label empty)
/// - OCR box without YOLO overlap -> text-only element (mark non-interactable)
pub fn merge_detections(
    yolo_detections: &[Detection],
    ocr_results: &[OcrResult],
    iou_threshold: f64,
) -> Vec<MergedElement> {
    let mut merged = Vec::new();
    let mut ocr_matched = vec![false; ocr_results.len()];

    for det in yolo_detections {
        let mut best_ocr_idx = None;
        let mut best_iou = 0.0;

        for (i, ocr) in ocr_results.iter().enumerate() {
            if ocr_matched[i] {
                continue;
            }
            let overlap = iou(&det.bbox, &ocr.bbox);
            if overlap > iou_threshold && overlap > best_iou {
                best_iou = overlap;
                best_ocr_idx = Some(i);
            }
        }

        let label = if let Some(idx) = best_ocr_idx {
            ocr_matched[idx] = true;
            ocr_results[idx].text.clone()
        } else {
            String::new() // icon without text, may be captioned by Florence-2
        };

        merged.push(MergedElement {
            bbox: det.bbox.clone(),
            label,
            interactable: true,
            confidence: det.confidence,
        });
    }

    // Add unmatched OCR results as non-interactable text elements
    for (i, ocr) in ocr_results.iter().enumerate() {
        if !ocr_matched[i] {
            merged.push(MergedElement {
                bbox: ocr.bbox.clone(),
                label: ocr.text.clone(),
                interactable: false,
                confidence: ocr.confidence,
            });
        }
    }

    merged
}

/// Sort elements by position: top-to-bottom, then left-to-right
pub fn sort_by_position(elements: &mut [MergedElement]) {
    elements.sort_by(|a, b| {
        let ay = a.bbox.y1;
        let by = b.bbox.y1;
        // Group into rows (elements within 2% vertical distance are same row)
        let row_a = (ay * 50.0) as i32;
        let row_b = (by * 50.0) as i32;
        if row_a != row_b {
            row_a.cmp(&row_b)
        } else {
            a.bbox
                .x1
                .partial_cmp(&b.bbox.x1)
                .unwrap_or(std::cmp::Ordering::Equal)
        }
    });
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
        // Intersection: [0.25, 0.25] to [0.5, 0.5] = 0.25 * 0.25 = 0.0625
        // Union: 0.25 + 0.25 - 0.0625 = 0.4375
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
    fn test_merge_detections() {
        let yolo = vec![
            Detection {
                bbox: BoundingBox::new(0.1, 0.1, 0.3, 0.3),
                confidence: 0.9,
            },
            Detection {
                bbox: BoundingBox::new(0.5, 0.5, 0.7, 0.7),
                confidence: 0.85,
            },
        ];
        let ocr = vec![
            OcrResult {
                bbox: BoundingBox::new(0.1, 0.1, 0.3, 0.3),
                text: "OK".to_string(),
                confidence: 0.95,
            },
            OcrResult {
                bbox: BoundingBox::new(0.8, 0.8, 1.0, 1.0),
                text: "Status bar".to_string(),
                confidence: 0.9,
            },
        ];
        let merged = merge_detections(&yolo, &ocr, 0.1);
        // First YOLO box matches first OCR -> merged with text "OK"
        // Second YOLO box has no OCR match -> icon
        // Second OCR box unmatched -> non-interactable text
        assert_eq!(merged.len(), 3);
        assert_eq!(merged[0].label, "OK");
        assert!(merged[0].interactable);
        assert_eq!(merged[1].label, "");
        assert!(merged[1].interactable);
        assert_eq!(merged[2].label, "Status bar");
        assert!(!merged[2].interactable);
    }

    #[test]
    fn test_sort_by_position() {
        let mut elements = vec![
            MergedElement {
                bbox: BoundingBox::new(0.5, 0.1, 0.6, 0.2),
                label: "B".to_string(),
                interactable: true,
                confidence: 0.9,
            },
            MergedElement {
                bbox: BoundingBox::new(0.1, 0.1, 0.2, 0.2),
                label: "A".to_string(),
                interactable: true,
                confidence: 0.9,
            },
            MergedElement {
                bbox: BoundingBox::new(0.1, 0.5, 0.2, 0.6),
                label: "C".to_string(),
                interactable: true,
                confidence: 0.9,
            },
        ];
        sort_by_position(&mut elements);
        assert_eq!(elements[0].label, "A");
        assert_eq!(elements[1].label, "B");
        assert_eq!(elements[2].label, "C");
    }
}
