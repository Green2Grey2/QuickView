use crate::{
    geometry::Rect,
    ocr::models::{OcrResult, OcrWord},
};

use anyhow::{anyhow, Context, Result};

/// Parse Tesseract TSV output and return word-level bounding boxes.
///
/// Tesseract TSV format is documented in tessdoc; word-level entries have `level = 5`.
pub fn parse_tesseract_tsv(tsv: &str) -> Result<OcrResult> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .flexible(true)
        .from_reader(tsv.as_bytes());

    let headers = rdr.headers()?.clone();

    // We try to find indices by header name to be resilient.
    let idx = |name: &str| -> Result<usize> {
        headers
            .iter()
            .position(|h| h == name)
            .ok_or_else(|| anyhow!("TSV missing column: {name}"))
    };

    let i_level = idx("level")?;
    let i_left = idx("left")?;
    let i_top = idx("top")?;
    let i_width = idx("width")?;
    let i_height = idx("height")?;
    let i_conf = idx("conf")?;
    let i_text = idx("text")?;

    let mut words = Vec::new();

    for (order, rec) in rdr.records().enumerate() {
        let rec = rec.context("read TSV record")?;
        let level: i32 = rec.get(i_level).unwrap_or_default().parse().unwrap_or(0);

        if level != 5 {
            continue;
        }

        let text = rec.get(i_text).unwrap_or("").trim();
        if text.is_empty() {
            continue;
        }

        let left: f64 = rec.get(i_left).unwrap_or("0").parse().unwrap_or(0.0);
        let top: f64 = rec.get(i_top).unwrap_or("0").parse().unwrap_or(0.0);
        let width: f64 = rec.get(i_width).unwrap_or("0").parse().unwrap_or(0.0);
        let height: f64 = rec.get(i_height).unwrap_or("0").parse().unwrap_or(0.0);
        let conf: f32 = rec.get(i_conf).unwrap_or("-1").parse().unwrap_or(-1.0);

        words.push(OcrWord {
            text: text.to_string(),
            confidence: conf,
            bbox: Rect {
                x: left,
                y: top,
                w: width,
                h: height,
            },
            order,
        });
    }

    Ok(OcrResult { words })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_word_level_entries() {
        let sample = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n\
1\t1\t0\t0\t0\t0\t0\t0\t640\t480\t-1\t\n\
5\t1\t1\t1\t1\t1\t10\t20\t30\t40\t95.0\tHello\n\
5\t1\t1\t1\t1\t2\t50\t20\t30\t40\t90.0\tworld\n";

        let r = parse_tesseract_tsv(sample).unwrap();
        assert_eq!(r.words.len(), 2);
        assert_eq!(r.words[0].text, "Hello");
        assert_eq!(r.words[1].text, "world");
        assert_eq!(r.words[0].bbox.x, 10.0);
    }
}
