use std::ops::Range;

use super::bitmap::Bitmap;
use super::gray::Gray;
use crate::capture::RectPx;
use super::segment::{column_runs, row_runs};


/// One line of text with its glyphs located: the binarized band (used to
/// find glyphs) and the same band in grayscale (used to match them).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextLine {
    /// The line's rows only (the band holding most of the ink).
    pub bitmap: Bitmap,
    pub gray: Gray,
    /// Column span of each glyph, left to right.
    pub glyphs: Vec<Range<u32>>,
}

impl TextLine {
    /// Segments an already-binarized line: one glyph per run of ink columns.
    pub fn from_bitmap(bitmap: Bitmap) -> Option<Self> {
        let rows = bitmap.ink_rows()?;
        let bitmap = bitmap.crop(0, rows.start, bitmap.width(), rows.end - rows.start);
        let glyphs = column_runs(&bitmap);
        let gray = Gray::from_bitmap(&bitmap, 255, 0);
        Some(Self { bitmap, gray, glyphs })
    }

    /// Binarizes a single-line crop and finds its glyphs.
    ///
    /// Blurry captures (scaled or video-compressed) glow, so at the Otsu
    /// threshold neighbouring glyphs touch. A second, brighter pass keeps only
    /// stroke cores (a third of the way from the Otsu threshold to the
    /// brightest pixel); where one blob holds several separate cores, it is
    /// split between them. Dim marks (brackets, periods) are still found by
    /// the first pass. Stray bright pixels outside the main band of rows are
    /// dropped.
    pub fn from_gray(gray: &Gray) -> Option<Self> {
        Self::from_gray_at(gray, gray.otsu_threshold())
    }

    /// [`TextLine::from_gray`] with an explicit ink threshold `t`.
    pub fn from_gray_at(gray: &Gray, t: u8) -> Option<Self> {
        let core = t + (gray.max() - t) / 3;
        let low = gray.binarize(t);
        let rows = main_band(&low)?;
        let crop = |b: &Bitmap| b.crop(0, rows.start, b.width(), rows.end - rows.start);
        let low = crop(&low);
        let high = crop(&gray.binarize(core));
        let glyphs = split_on_cores(&low, &high);
        let gray = gray.crop(RectPx { x: 0, y: rows.start, w: gray.width(), h: rows.end - rows.start });
        Some(Self { bitmap: low, gray, glyphs })
    }

    /// Grayscale image of glyph `i`.
    pub fn glyph(&self, i: usize) -> Gray {
        self.gray.crop_columns(self.glyphs[i].clone())
    }
}

/// The run of rows (bridging 1-row gaps) holding the most ink.
fn main_band(b: &Bitmap) -> Option<Range<u32>> {
    row_runs(b, 2).into_iter().max_by_key(|rows| {
        let band = b.crop(0, rows.start, b.width(), rows.end - rows.start);
        band.ink_count()
    })
}

fn split_on_cores(low: &Bitmap, high: &Bitmap) -> Vec<Range<u32>> {
    let mut out = Vec::new();
    for run in column_runs(low) {
        let cores: Vec<Range<u32>> = column_runs(&high.crop_columns(run.clone()))
            .into_iter()
            .map(|c| run.start + c.start..run.start + c.end)
            .collect();
        if cores.len() < 2 {
            out.push(run);
            continue;
        }
        let mut start = run.start;
        for pair in cores.windows(2) {
            let cut = (pair[0].end + pair[1].start).div_ceil(2);
            out.push(start..cut);
            start = cut;
        }
        out.push(start..run.end);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gray(rows: &[&str]) -> Gray {
        // '#' = 250 (core), '+' = 120 (glow), '.' = 20 (background).
        let w = rows[0].len() as u32;
        let data = rows
            .iter()
            .flat_map(|r| r.bytes().map(|c| match c {
                b'#' => 250,
                b'+' => 120,
                _ => 20,
            }))
            .collect();
        Gray::new(w, rows.len() as u32, data)
    }

    #[test]
    fn splits_glowing_glyphs_between_cores() {
        let g = gray(&["..........", ".#+#..++..", ".#+#..++..", ".#+#..++.."]);
        let line = TextLine::from_gray(&g).unwrap();
        // Two cores in the first blob (the glow column between them goes
        // left), then a dim mark.
        assert_eq!(line.glyphs, vec![1..3, 3..4, 6..8]);
        assert_eq!(line.bitmap.height(), 3);
    }

    #[test]
    fn ignores_stray_pixels_outside_the_main_band() {
        let g = gray(&["........#", ".........", ".........", ".##......", ".##......", ".##......"]);
        let line = TextLine::from_gray(&g).unwrap();
        assert_eq!(line.glyphs, vec![1..3]);
        assert_eq!(line.bitmap.height(), 3);
    }
}
