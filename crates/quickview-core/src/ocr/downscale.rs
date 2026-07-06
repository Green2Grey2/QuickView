//! Downscale planning for the OCR max-dimension guardrail.
//!
//! Tesseract re-decodes the image file itself, so oversized images make OCR
//! slow and memory-hungry out of proportion to recognition quality. When an
//! image exceeds the configured maximum dimension, the UI feeds tesseract a
//! downscaled temporary copy instead and maps the resulting word boxes back
//! into original image space with [`upscale_bboxes`] — so everything
//! downstream (cache entries, the spatial index, selection) only ever sees
//! original-space coordinates.
//!
//! This module is pure math; producing the actual downscaled pixels is the
//! UI layer's job (it owns the decoded texture).

use crate::ocr::models::OcrResult;

/// Default for the `[ocr] max_dimension` config setting. `0` disables the
/// guardrail entirely.
pub const DEFAULT_MAX_OCR_DIMENSION: u32 = 4000;

/// A decision to OCR a downscaled copy instead of the original.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DownscalePlan {
    pub target_w: u32,
    pub target_h: u32,
    /// `original / target` (> 1.0). Multiply target-space lengths by this to
    /// get back to original image space.
    pub factor: f64,
}

/// Decide whether an image of `width` x `height` needs downscaling to fit
/// `max_dimension`. Returns `None` when it already fits (or the guardrail is
/// disabled with `max_dimension == 0`); aspect ratio is preserved and no
/// target dimension falls below 1.
pub fn plan_downscale(width: u32, height: u32, max_dimension: u32) -> Option<DownscalePlan> {
    if max_dimension == 0 || width == 0 || height == 0 {
        return None;
    }
    let largest = width.max(height);
    if largest <= max_dimension {
        return None;
    }

    let factor = f64::from(largest) / f64::from(max_dimension);
    let scale = |dim: u32| ((f64::from(dim) / factor).round() as u32).max(1);
    Some(DownscalePlan {
        target_w: scale(width),
        target_h: scale(height),
        factor,
    })
}

/// Map word bounding boxes recognized in downscaled space back to original
/// image space. Run this *before* the result is cached or indexed, so a
/// cache hit is indistinguishable from a full-resolution parse.
///
/// Factors are per-axis (`original / actual target`, computed from the
/// dimensions of the image actually fed to tesseract): target sizes are
/// rounded independently per axis, so one uniform factor would drift boxes
/// on the minor axis.
pub fn upscale_bboxes(result: &mut OcrResult, factor_x: f64, factor_y: f64) {
    for word in &mut result.words {
        word.bbox.x *= factor_x;
        word.bbox.y *= factor_y;
        word.bbox.w *= factor_x;
        word.bbox.h *= factor_y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;
    use crate::ocr::models::OcrWord;

    #[test]
    fn below_and_at_threshold_are_untouched() {
        assert_eq!(plan_downscale(1000, 500, 4000), None);
        assert_eq!(plan_downscale(4000, 3000, 4000), None);
        assert_eq!(plan_downscale(3000, 4000, 4000), None);
    }

    #[test]
    fn zero_disables_and_degenerate_dims_are_ignored() {
        assert_eq!(plan_downscale(100_000, 50, 0), None);
        assert_eq!(plan_downscale(0, 5000, 4000), None);
        assert_eq!(plan_downscale(5000, 0, 4000), None);
    }

    #[test]
    fn above_threshold_scales_and_preserves_aspect() {
        let plan = plan_downscale(8000, 4000, 4000).unwrap();
        assert_eq!((plan.target_w, plan.target_h), (4000, 2000));
        assert_eq!(plan.factor, 2.0);

        // Portrait: the larger dimension drives the factor.
        let plan = plan_downscale(3000, 6000, 4000).unwrap();
        assert_eq!((plan.target_w, plan.target_h), (2000, 4000));
        assert_eq!(plan.factor, 1.5);
    }

    #[test]
    fn extreme_aspect_never_reaches_zero() {
        let plan = plan_downscale(100_000, 50, 4000).unwrap();
        assert_eq!(plan.target_w, 4000);
        assert!(plan.target_h >= 1);
        assert_eq!(plan.target_h, 2); // 50 / 25 = 2

        let plan = plan_downscale(1_000_000, 1, 4000).unwrap();
        assert_eq!(plan.target_h, 1);
    }

    #[test]
    fn upscale_round_trips_within_epsilon() {
        let factor = 8000.0 / 4000.0;
        let original = Rect {
            x: 123.0,
            y: 456.0,
            w: 78.0,
            h: 90.0,
        };
        // What tesseract would report in downscaled space.
        let mut result = OcrResult {
            words: vec![OcrWord {
                text: "hi".into(),
                confidence: 90.0,
                bbox: Rect {
                    x: original.x / factor,
                    y: original.y / factor,
                    w: original.w / factor,
                    h: original.h / factor,
                },
                order: 0,
            }],
        };
        upscale_bboxes(&mut result, factor, factor);
        let got = &result.words[0].bbox;
        for (a, b) in [
            (got.x, original.x),
            (got.y, original.y),
            (got.w, original.w),
            (got.h, original.h),
        ] {
            assert!((a - b).abs() < 1e-9, "{a} vs {b}");
        }
    }
}
