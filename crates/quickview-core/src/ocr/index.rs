use crate::geometry::Rect;

use super::models::OcrWord;

const DEFAULT_CELL_SIZE: f64 = 256.0;

/// A simple uniform-grid spatial index for OCR word bounding boxes.
///
/// The index is built in image coordinates and can be queried with a rectangle to
/// efficiently find intersecting words.
///
/// # Contract
/// This index stores buckets of *indices* into a specific OCR word list. Callers must
/// rebuild the index whenever the underlying `words` slice changes, including:
/// - replacing the OCR result
/// - reordering words
/// - mutating any word bounding boxes
///
/// Calling `query_intersecting()` with a different `words` slice than the one used to
/// build the index may produce incorrect results.
#[derive(Debug, Clone)]
pub struct OcrWordIndex {
    cell_size: f64,
    grid_w: usize,
    grid_h: usize,
    buckets: Vec<Vec<usize>>,
    seen: Vec<u32>,
    seen_gen: u32,
}

impl OcrWordIndex {
    pub fn build(words: &[OcrWord], image_w: f64, image_h: f64) -> Self {
        Self::build_with_cell_size(words, image_w, image_h, DEFAULT_CELL_SIZE)
    }

    pub fn build_with_cell_size(
        words: &[OcrWord],
        image_w: f64,
        image_h: f64,
        cell_size: f64,
    ) -> Self {
        let cell_size = cell_size.max(1.0);

        let grid_w = ((image_w.max(1.0) / cell_size).ceil() as usize).max(1);
        let grid_h = ((image_h.max(1.0) / cell_size).ceil() as usize).max(1);
        let mut buckets = vec![Vec::<usize>::new(); grid_w.saturating_mul(grid_h).max(1)];

        for (idx, w) in words.iter().enumerate() {
            Self::insert_bbox(&mut buckets, grid_w, grid_h, cell_size, idx, w.bbox);
        }

        Self {
            cell_size,
            grid_w,
            grid_h,
            buckets,
            seen: vec![0; words.len()],
            seen_gen: 1,
        }
    }

    /// Return indices of words whose bounding boxes intersect `rect`.
    ///
    /// `words` must be the same word list used when building this index (same ordering and
    /// bounding boxes). If you swap or mutate the word list, rebuild via `OcrWordIndex::build(...)`
    /// before calling this method again.
    pub fn query_intersecting(&mut self, words: &[OcrWord], rect: &Rect) -> Vec<usize> {
        if words.is_empty() {
            return Vec::new();
        }

        if self.seen.len() != words.len() {
            // Best-effort hygiene. The index must be rebuilt when `words` changes; this is only
            // to avoid panics from the internal dedupe vector length drifting.
            self.seen = vec![0; words.len()];
            self.seen_gen = 1;
        }

        let Some((x0, y0, x1, y1)) =
            Self::cell_range(self.cell_size, self.grid_w, self.grid_h, rect)
        else {
            return Vec::new();
        };

        let gen = self.next_seen_gen();
        let mut out = Vec::new();

        for gy in y0..=y1 {
            for gx in x0..=x1 {
                let bucket_idx = gy * self.grid_w + gx;
                if let Some(bucket) = self.buckets.get(bucket_idx) {
                    for &word_idx in bucket {
                        // If the caller violates the contract and supplies a different `words`
                        // slice than the one used at build time, buckets can contain indices that
                        // are out of range. Skip rather than panic.
                        if word_idx >= words.len() || word_idx >= self.seen.len() {
                            continue;
                        }

                        if self.seen[word_idx] == gen {
                            continue;
                        }
                        self.seen[word_idx] = gen;

                        if words.get(word_idx).is_some_and(|w| w.bbox.intersects(rect)) {
                            out.push(word_idx);
                        }
                    }
                }
            }
        }

        out
    }

    fn next_seen_gen(&mut self) -> u32 {
        if self.seen_gen == u32::MAX {
            self.seen.fill(0);
            self.seen_gen = 1;
        } else {
            self.seen_gen += 1;
        }
        self.seen_gen
    }

    fn insert_bbox(
        buckets: &mut [Vec<usize>],
        grid_w: usize,
        grid_h: usize,
        cell_size: f64,
        word_idx: usize,
        bbox: Rect,
    ) {
        if grid_w == 0 || grid_h == 0 || cell_size <= 0.0 {
            return;
        }

        if bbox.w <= 0.0 || bbox.h <= 0.0 {
            return;
        }

        let x0 = (bbox.x / cell_size).floor() as isize;
        let y0 = (bbox.y / cell_size).floor() as isize;
        let x1 = ((bbox.x + bbox.w) / cell_size).floor() as isize;
        let y1 = ((bbox.y + bbox.h) / cell_size).floor() as isize;

        let x0 = x0.clamp(0, (grid_w - 1) as isize) as usize;
        let y0 = y0.clamp(0, (grid_h - 1) as isize) as usize;
        let x1 = x1.clamp(0, (grid_w - 1) as isize) as usize;
        let y1 = y1.clamp(0, (grid_h - 1) as isize) as usize;

        for gy in y0..=y1 {
            for gx in x0..=x1 {
                let bucket_idx = gy * grid_w + gx;
                if let Some(bucket) = buckets.get_mut(bucket_idx) {
                    bucket.push(word_idx);
                }
            }
        }
    }

    fn cell_range(
        cell_size: f64,
        grid_w: usize,
        grid_h: usize,
        rect: &Rect,
    ) -> Option<(usize, usize, usize, usize)> {
        if cell_size <= 0.0 || grid_w == 0 || grid_h == 0 {
            return None;
        }

        // Keep semantics aligned with `Rect::intersects()`: degenerate rectangles (w==0 or h==0)
        // can still "hit" boxes like a line/point selection.
        if rect.w < 0.0 || rect.h < 0.0 {
            return None;
        }

        let x0 = (rect.x / cell_size).floor() as isize;
        let y0 = (rect.y / cell_size).floor() as isize;
        let x1 = ((rect.x + rect.w) / cell_size).floor() as isize;
        let y1 = ((rect.y + rect.h) / cell_size).floor() as isize;

        let x0 = x0.clamp(0, (grid_w - 1) as isize) as usize;
        let y0 = y0.clamp(0, (grid_h - 1) as isize) as usize;
        let x1 = x1.clamp(0, (grid_w - 1) as isize) as usize;
        let y1 = y1.clamp(0, (grid_h - 1) as isize) as usize;

        Some((x0, y0, x1, y1))
    }
}

#[cfg(test)]
mod tests {
    use super::OcrWordIndex;
    use crate::geometry::Rect;

    use super::super::models::OcrWord;

    fn w(text: &str, bbox: Rect, order: usize) -> OcrWord {
        OcrWord {
            text: text.to_string(),
            confidence: 99.0,
            bbox,
            order,
        }
    }

    #[test]
    fn query_returns_intersecting_words_only() {
        let words = vec![
            w(
                "a",
                Rect {
                    x: 10.0,
                    y: 10.0,
                    w: 10.0,
                    h: 10.0,
                },
                0,
            ),
            w(
                "b",
                Rect {
                    x: 300.0,
                    y: 10.0,
                    w: 10.0,
                    h: 10.0,
                },
                1,
            ),
            w(
                "c",
                Rect {
                    x: 10.0,
                    y: 300.0,
                    w: 10.0,
                    h: 10.0,
                },
                2,
            ),
        ];

        let mut idx = OcrWordIndex::build_with_cell_size(&words, 1000.0, 1000.0, 64.0);
        let r = Rect {
            x: 290.0,
            y: 0.0,
            w: 50.0,
            h: 50.0,
        };
        let mut out = idx.query_intersecting(&words, &r);
        out.sort_unstable();

        assert_eq!(out, vec![1]);
    }

    #[test]
    fn query_deduplicates_words_that_span_multiple_cells() {
        let words = vec![w(
            "x",
            Rect {
                x: 60.0,
                y: 60.0,
                w: 10.0,
                h: 10.0,
            },
            0,
        )];

        // With cell_size=64, this bbox overlaps both cell (0,0) and (1,1).
        let mut idx = OcrWordIndex::build_with_cell_size(&words, 256.0, 256.0, 64.0);
        let r = Rect {
            x: 0.0,
            y: 0.0,
            w: 200.0,
            h: 200.0,
        };
        let out = idx.query_intersecting(&words, &r);

        assert_eq!(out, vec![0]);
    }

    #[test]
    fn degenerate_rects_still_hit_via_intersects_semantics() {
        let words = vec![w(
            "a",
            Rect {
                x: 10.0,
                y: 10.0,
                w: 10.0,
                h: 10.0,
            },
            0,
        )];
        let mut idx = OcrWordIndex::build_with_cell_size(&words, 100.0, 100.0, 32.0);

        // Point hit inside the word bbox.
        let p = Rect {
            x: 15.0,
            y: 15.0,
            w: 0.0,
            h: 0.0,
        };
        assert_eq!(idx.query_intersecting(&words, &p), vec![0]);
    }
}
