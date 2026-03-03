use serde::{Deserialize, Serialize};

use crate::geometry::Rect;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrWord {
    /// Recognized word text.
    pub text: String,

    /// Confidence score as reported by the engine.
    pub confidence: f32,

    /// Bounding box in **image pixel coordinates**.
    pub bbox: Rect,

    /// Position in original OCR output order.
    pub order: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    pub words: Vec<OcrWord>,
}
