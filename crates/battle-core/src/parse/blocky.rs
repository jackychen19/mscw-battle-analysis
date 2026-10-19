//! Reads the level's blocky digits by shape, with no templates: each digit is
//! drawn from seven-segment-like strokes, so which segments hold ink says
//! which digit it is. Works for digits never seen before (a level-up).

use std::ops::Range;

use super::line::TextLine;

/// The best digit must fit at least this much better than the runner-up
/// (fit is summed over seven segments, each 0–1). Misreads that slip
/// through still have to pass the stats engine's two-identical-reads check.
const MIN_MARGIN: f32 = 0.2;
/// Segment ink at or below this is "off" (neighbouring strokes spill in);
/// from `OFF_INK + 0.3` up it's fully "on".
const OFF_INK: f32 = 0.3;
/// A glyph narrower than this fraction of its height is a `1`.
const ONE_ASPECT: f32 = 0.45;

/// Segments: top, top-right, bottom-right, bottom, bottom-left, top-left, middle.
const DIGITS: [(char, [bool; 7]); 9] = [
    ('0', [true, true, true, true, true, true, false]),
    ('2', [true, true, false, true, true, false, true]),
    ('3', [true, true, true, true, false, false, true]),
    ('4', [false, true, true, false, false, true, true]),
    ('5', [true, false, true, true, false, true, true]),
    ('6', [true, false, true, true, true, true, true]),
    ('7', [true, true, true, false, false, false, false]),
    ('8', [true, true, true, true, true, true, true]),
    ('9', [true, true, true, true, false, true, true]),
];

/// The digits of `line`, or `None` if a glyph isn't clearly one digit.
pub fn read_blocky_digits(line: &TextLine) -> Option<String> {
    line.glyphs.iter().map(|run| read_digit(line, run)).collect()
}

fn read_digit(line: &TextLine, run: &Range<u32>) -> Option<char> {
    let glyph = line.bitmap.crop_columns(run.clone());
    let rows = glyph.ink_rows()?;
    let (w, h) = (glyph.width(), rows.end - rows.start);
    if h < 5 {
        return None;
    }
    if (w as f32) < h as f32 * ONE_ASPECT {
        return Some('1');
    }
    // Fraction of ink in a box given as fractions of the glyph.
    let ink = |x0: f32, x1: f32, y0: f32, y1: f32| {
        let (xa, xb) = ((x0 * w as f32) as u32, ((x1 * w as f32).ceil() as u32).min(w));
        let (ya, yb) = (rows.start + (y0 * h as f32) as u32, (rows.start + (y1 * h as f32).ceil() as u32).min(rows.end));
        let total = (xb.saturating_sub(xa) * yb.saturating_sub(ya)).max(1);
        let on = (ya..yb).flat_map(|y| (xa..xb).map(move |x| (x, y))).filter(|&(x, y)| glyph.get(x, y)).count();
        on as f32 / total as f32
    };
    let (left, mid, right) = ((0.0, 0.34), (0.34, 0.66), (0.66, 1.0));
    let seg = [
        ink(mid.0, mid.1, 0.0, 0.16),
        ink(right.0, right.1, 0.18, 0.4),
        ink(right.0, right.1, 0.6, 0.82),
        // The game's bottom bars are short and off-centre: look wider.
        ink(0.2, 0.8, 0.84, 1.0),
        // Above the hook the game's 5 and 9 have at the bottom left.
        ink(left.0, left.1, 0.56, 0.74),
        ink(left.0, left.1, 0.18, 0.4),
        ink(mid.0, mid.1, 0.42, 0.58),
    ];
    // How well the ink fits each digit: drawn segments should be full, the
    // others empty. Blur leaves partial values, so take the best clear fit.
    let fit = |pattern: &[bool; 7]| -> f32 {
        pattern
            .iter()
            .zip(seg)
            .map(|(&on, ink)| {
                let drawn = ((ink - OFF_INK) / 0.3).clamp(0.0, 1.0);
                if on { drawn } else { 1.0 - drawn }
            })
            .sum()
    };
    let mut ranked: Vec<(char, f32)> = DIGITS.iter().map(|(c, p)| (*c, fit(p))).collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
    (ranked[0].1 - ranked[1].1 >= MIN_MARGIN).then_some(ranked[0].0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::bitmap::Bitmap;

    /// A digit drawn at 4× from a 3×5 seven-segment pattern, as the game's
    /// tile font is.
    fn big(text: &str) -> TextLine {
        let pattern = |c: char| -> [&str; 5] {
            match c {
                '0' => ["###", "#.#", "#.#", "#.#", "###"],
                '1' => [".#", ".#", ".#", ".#", ".#"],
                '2' => ["###", "..#", "###", "#..", "###"],
                '3' => ["###", "..#", "###", "..#", "###"],
                '4' => ["#.#", "#.#", "###", "..#", "..#"],
                '5' => ["###", "#..", "###", "..#", "###"],
                '6' => ["###", "#..", "###", "#.#", "###"],
                '7' => ["###", "..#", "..#", "..#", "..#"],
                '8' => ["###", "#.#", "###", "#.#", "###"],
                '9' => ["###", "#.#", "###", "..#", "###"],
                _ => unreachable!(),
            }
        };
        let mut rows = vec![String::new(); 20];
        for c in text.chars() {
            for (y, row) in rows.iter_mut().enumerate() {
                let line = pattern(c)[y / 4];
                row.extend(line.chars().flat_map(|p| std::iter::repeat_n(p, 4)));
                row.push_str("...");
            }
        }
        let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
        TextLine::from_bitmap(Bitmap::from_ascii(&rows)).unwrap()
    }

    #[test]
    fn reads_every_digit_by_shape() {
        assert_eq!(read_blocky_digits(&big("0123456789")).as_deref(), Some("0123456789"));
        assert_eq!(read_blocky_digits(&big("59")).as_deref(), Some("59"));
    }
}
