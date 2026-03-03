use crate::geometry::Rect;

use super::models::OcrWord;

/// Return all words whose bounding boxes intersect `rect`.
///
/// `rect` is in image pixel coordinates.
pub fn select_words(words: &[OcrWord], rect: Rect) -> Vec<&OcrWord> {
    words.iter().filter(|w| w.bbox.intersects(&rect)).collect()
}

/// Join selected words into a single string.
///
/// For now we preserve the OCR output order (`order`).
pub fn selected_text(mut words: Vec<&OcrWord>) -> String {
    words.sort_by_key(|w| w.order);
    words
        .into_iter()
        .map(|w| w.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}
