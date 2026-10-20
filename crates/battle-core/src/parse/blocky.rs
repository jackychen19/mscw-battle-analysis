//! Reads the level's blocky digits by shape, with no templates taught: the
//! game draws them in a 7×7 pixel font (2-pixel-wide uprights, 1-pixel bars),
//! so each glyph is shrunk to a 7×7 grid of ink fractions and compared with
//! the font's digits. Works for digits never seen on this screen before (a
//! level-up).

use std::ops::Range;

use super::line::TextLine;

/// The best digit must fit at least this much better than the runner-up
/// (fit is the mean over 49 cells, each 0–1). Misreads that slip through
/// still have to pass the stats engine's two-identical-reads check.
const MIN_MARGIN: f32 = 0.04;
/// A glyph narrower than this fraction of its height is a `1`.
const ONE_ASPECT: f32 = 0.45;

/// The font's digits at 1×. `3` and `4` are from native captures, `5` and `9`
/// from footage; the rest are drawn in the same style and not yet confirmed
/// on screen.
const DIGITS: [(char, [&str; 7]); 10] = [
    ('0', [".#####.", "##...##", "##...##", "##...##", "##...##", "##...##", ".#####."]),
    ('1', ["..##...", ".###...", "..##...", "..##...", "..##...", "..##...", ".####.."]),
    ('2', [".#####.", "##...##", ".....##", "....##.", "..##...", ".##....", "#######"]),
    ('3', [".#####.", "##...##", ".....##", "..####.", ".....##", "##...##", ".#####."]),
    ('4', ["...###.", "..####.", ".##.##.", "##..##.", "#######", "....##.", "....##."]),
    ('5', ["#######", "##.....", "##.....", "######.", ".....##", "##...##", ".#####."]),
    ('6', [".#####.", "##.....", "##.....", "######.", "##...##", "##...##", ".#####."]),
    ('7', ["#######", ".....##", "....##.", "...##..", "..##...", "..##...", "..##..."]),
    ('8', [".#####.", "##...##", "##...##", ".#####.", "##...##", "##...##", ".#####."]),
    ('9', [".#####.", "##...##", "##...##", ".######", ".....##", "##...##", ".#####."]),
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
    // Ink fraction of each cell of a 7×7 grid over the glyph's box.
    let cell = |cx: u32, cy: u32| {
        let (x0, x1) = (cx * w / 7, ((cx + 1) * w).div_ceil(7).max(cx * w / 7 + 1));
        let (y0, y1) = (cy * h / 7, ((cy + 1) * h).div_ceil(7).max(cy * h / 7 + 1));
        let total = (x1 - x0) * (y1 - y0);
        let on = (y0..y1).flat_map(|y| (x0..x1).map(move |x| (x, y))).filter(|&(x, y)| glyph.get(x, rows.start + y)).count();
        on as f32 / total as f32
    };
    let grid: Vec<f32> = (0..7).flat_map(|cy| (0..7).map(move |cx| (cx, cy))).map(|(cx, cy)| cell(cx, cy)).collect();
    let fit = |pattern: &[&str; 7]| -> f32 {
        let cells = pattern.iter().flat_map(|row| row.bytes().map(|b| b == b'#'));
        cells.zip(&grid).map(|(on, &ink)| if on { ink } else { 1.0 - ink }).sum::<f32>() / 49.0
    };
    let mut ranked: Vec<(char, f32)> = DIGITS.iter().map(|(c, p)| (*c, fit(p))).collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
    (ranked[0].1 - ranked[1].1 >= MIN_MARGIN).then_some(ranked[0].0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::bitmap::Bitmap;

    /// `text` in the game's level font at `scale`× (each font pixel a
    /// `scale`×`scale` block), as a native Retina capture draws it.
    fn tiles(text: &str, scale: usize) -> TextLine {
        let mut rows = vec![String::new(); 7 * scale];
        for c in text.chars() {
            let (_, pattern) = DIGITS.iter().find(|(d, _)| *d == c).unwrap();
            for (y, row) in rows.iter_mut().enumerate() {
                row.extend(pattern[y / scale].chars().flat_map(|p| std::iter::repeat_n(p, scale)));
                row.push_str(&".".repeat(4 * scale));
            }
        }
        let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
        TextLine::from_bitmap(Bitmap::from_ascii(&rows)).unwrap()
    }

    #[test]
    fn reads_every_digit_at_any_scale() {
        for scale in [1, 2, 3] {
            assert_eq!(read_blocky_digits(&tiles("0123456789", scale)).as_deref(), Some("0123456789"), "{scale}x");
        }
    }
}
