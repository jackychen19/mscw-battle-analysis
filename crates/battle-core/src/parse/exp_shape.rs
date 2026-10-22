//! Reads the EXP text by shape, with nothing taught: the game draws it in a
//! 5×7 pixel font with 1-pixel strokes. Each digit is shrunk onto the font's
//! 5×7 grid, area-averaged so the soft edges of a smoothed or stretched
//! screen average out, and compared with the font's digits by correlation
//! (where the ink is, not how strong it is). The marks are told apart by
//! size: `,` and `.` are short, `1` `[` `]` narrow, `%` wide.

use std::ops::Range;

use super::exp::parse_exp_text;
use super::line::{TextLine, words};

/// The font's digits at 1×, read off windowed captures; `5` off a fullscreen
/// one (fullscreen-02, the first 5 seen).
const DIGITS: [(char, [&str; 7]); 9] = [
    ('0', [".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."]),
    ('2', [".###.", "#...#", "....#", "...#.", "..#..", ".#...", "#####"]),
    ('3', [".###.", "#...#", "....#", "..##.", "....#", "#...#", ".###."]),
    ('4', ["...#.", "..##.", ".#.#.", "#..#.", "#####", "...#.", "...#."]),
    ('5', ["#####", "#....", "#....", "####.", "....#", "#...#", ".###."]),
    ('6', [".###.", "#...#", "#....", "####.", "#...#", "#...#", ".###."]),
    ('7', ["#####", "....#", "....#", "....#", "...#.", "..#..", "..#.."]),
    ('8', [".###.", "#...#", "#...#", ".###.", "#...#", "#...#", ".###."]),
    ('9', [".###.", "#...#", "#...#", ".####", "....#", "#...#", ".###."]),
];
/// `1` and the brackets, the 2-pixel-wide marks. Which way a bracket faces
/// isn't read: `]` ends the text and `[` is the only other one.
const ONE: [&str; 7] = [".#", "##", ".#", ".#", ".#", ".#", ".#"];
const BRACKETS: [[&str; 7]; 2] = [
    ["##", "#.", "#.", "#.", "#.", "#.", "##"],
    ["##", ".#", ".#", ".#", ".#", ".#", "##"],
];
/// The best shape must correlate at least this much better than the
/// runner-up. On smoothed fullscreen captures the closest calls (`8` over
/// `9`) win by about 0.07.
const MIN_MARGIN: f32 = 0.03;
/// Ink at least this share of a glyph's brightest marks its extent (a share,
/// so the dimmer yellow-green brackets are measured in full too).
const STRONG: f32 = 0.5;
/// A glyph fainter than this (0–1 between background and text) isn't text.
const FAINT: f32 = 0.15;

/// `(exp, pct_hundredths)` from `line`: the rightmost word that reads as
/// `N,NNN[PP.PP%]`, so a label or a stray mark beside it is skipped.
pub fn read_exp_by_shape(line: &TextLine) -> Option<(u64, u32)> {
    let g = &line.gray;
    let otsu = g.otsu_threshold();
    let (bg, fg) = (f32::from(g.mean_at_most(otsu)), f32::from(g.max()));
    if fg <= bg {
        return None;
    }
    let ink = |x: u32, y: u32| ((f32::from(g.get(x, y)) - bg) / (fg - bg)).clamp(0.0, 1.0);
    let extents: Vec<Option<Extent>> = line.glyphs.iter().map(|run| extent(g.height(), run.clone(), &ink)).collect();
    words(&line.glyphs).into_iter().rev().find_map(|word| {
        let glyphs: Vec<Extent> = extents[word].iter().copied().collect::<Option<_>>()?;
        parse_exp_text(&read_word(&split_merged(glyphs, g.height(), &ink), &ink)?)
    })
}

/// Where a glyph's ink is, in pixels of the line.
#[derive(Clone, Copy, Debug)]
struct Extent {
    x0: f32,
    x1: f32,
    y0: f32,
    y1: f32,
}

impl Extent {
    fn w(&self) -> f32 {
        self.x1 - self.x0
    }

    fn h(&self) -> f32 {
        self.y1 - self.y0
    }
}

/// The strong ink of the columns `cols`, if there's any ink at all.
fn extent(height: u32, cols: Range<u32>, ink: &dyn Fn(u32, u32) -> f32) -> Option<Extent> {
    let peak = cols.clone().flat_map(|x| (0..height).map(move |y| ink(x, y))).fold(0.0, f32::max);
    if peak < FAINT {
        return None;
    }
    let strong = |x: u32, y: u32| ink(x, y) >= STRONG * peak;
    let xs: Vec<u32> = cols.clone().filter(|&x| (0..height).any(|y| strong(x, y))).collect();
    let ys: Vec<u32> = (0..height).filter(|&y| cols.clone().any(|x| strong(x, y))).collect();
    Some(Extent { x0: *xs.first()? as f32, x1: (*xs.last()? + 1) as f32, y0: *ys.first()? as f32, y1: (*ys.last()? + 1) as f32 })
}

/// Glyphs blurred together (`75` as one blob) split in two at their faintest
/// column near the middle: a `%` is about as wide as it's tall, two digits
/// half as wide again.
fn split_merged(glyphs: Vec<Extent>, height: u32, ink: &dyn Fn(u32, u32) -> f32) -> Vec<Extent> {
    let tall = glyphs.iter().map(Extent::h).fold(0.0, f32::max);
    let mut out = Vec::with_capacity(glyphs.len() + 1);
    for e in glyphs {
        if e.w() <= tall * 1.25 {
            out.push(e);
            continue;
        }
        let (x0, x1) = (e.x0 as u32, e.x1 as u32);
        let column = |x: u32| (e.y0 as u32..e.y1 as u32).map(|y| ink(x, y)).sum::<f32>();
        let middle = x0 + (x1 - x0) * 35 / 100..x0 + (x1 - x0) * 65 / 100;
        let cut = middle.min_by(|&a, &b| column(a).total_cmp(&column(b))).unwrap_or((x0 + x1) / 2);
        match (extent(height, x0..cut, ink), extent(height, cut + 1..x1, ink)) {
            (Some(left), Some(right)) => out.extend([left, right]),
            _ => out.push(e),
        }
    }
    out
}

/// The characters of one word, if every glyph is clearly one.
fn read_word(glyphs: &[Extent], ink: &dyn Fn(u32, u32) -> f32) -> Option<String> {
    // Digits, brackets and `%` are as tall as the font.
    let tall = glyphs.iter().map(Extent::h).fold(0.0, f32::max);
    let mut text = String::new();
    for (i, e) in glyphs.iter().enumerate() {
        let c = if e.h() < tall / 2.0 {
            // A comma in the number, the decimal point inside the brackets.
            if text.contains('[') { '.' } else { ',' }
        } else if e.w() < tall * 0.5 {
            let g = grid(e, 2, ink);
            let bracket = BRACKETS.iter().map(|p| correlation(p, &g)).fold(f32::MIN, f32::max);
            if correlation(&ONE, &g) - bracket >= MIN_MARGIN {
                '1'
            } else if i + 1 == glyphs.len() {
                ']'
            } else {
                '['
            }
        } else if e.w() > tall * 0.85 {
            '%'
        } else {
            digit(&grid(e, 5, ink))?
        };
        text.push(c);
    }
    Some(text)
}

/// The best-correlating digit, if it wins clearly.
fn digit(grid: &[f32]) -> Option<char> {
    let mut ranked: Vec<(char, f32)> = DIGITS.iter().map(|(c, p)| (*c, correlation(p, grid))).collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
    (ranked[0].1 - ranked[1].1 >= MIN_MARGIN).then_some(ranked[0].0)
}

/// The glyph's ink, area-averaged onto `cols`×7 cells over its extent.
fn grid(e: &Extent, cols: u32, ink: &dyn Fn(u32, u32) -> f32) -> Vec<f32> {
    let (cw, ch) = (e.w() / cols as f32, e.h() / 7.0);
    let mut out = Vec::with_capacity(cols as usize * 7);
    for cy in 0..7 {
        for cx in 0..cols {
            let (ax, bx) = (e.x0 + cx as f32 * cw, e.x0 + (cx + 1) as f32 * cw);
            let (ay, by) = (e.y0 + cy as f32 * ch, e.y0 + (cy + 1) as f32 * ch);
            let (mut sum, mut total) = (0.0, 0.0);
            for y in ay.floor() as u32..by.ceil() as u32 {
                for x in ax.floor() as u32..bx.ceil() as u32 {
                    let wx = (bx.min(x as f32 + 1.0) - ax.max(x as f32)).max(0.0);
                    let wy = (by.min(y as f32 + 1.0) - ay.max(y as f32)).max(0.0);
                    sum += ink(x, y) * wx * wy;
                    total += wx * wy;
                }
            }
            out.push(if total > 0.0 { sum / total } else { 0.0 });
        }
    }
    out
}

/// Pearson correlation of a pattern's cells (ink 1, background 0) with `grid`.
fn correlation(pattern: &[&str; 7], grid: &[f32]) -> f32 {
    let t: Vec<f32> = pattern.iter().flat_map(|row| row.bytes().map(|b| f32::from(u8::from(b == b'#')))).collect();
    let n = t.len() as f32;
    let (mt, mg) = (t.iter().sum::<f32>() / n, grid.iter().sum::<f32>() / n);
    let cov: f32 = t.iter().zip(grid).map(|(a, b)| (a - mt) * (b - mg)).sum();
    let vt: f32 = t.iter().map(|a| (a - mt).powi(2)).sum();
    let vg: f32 = grid.iter().map(|b| (b - mg).powi(2)).sum();
    cov / (vt * vg).sqrt().max(1e-6)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::Frame;
    use crate::parse::{ExpInk, exp_line_with};

    /// The rest of the font, for drawing test lines.
    fn pattern(c: char) -> Vec<&'static str> {
        match c {
            '1' => ONE.to_vec(),
            '[' => BRACKETS[0].to_vec(),
            ']' => BRACKETS[1].to_vec(),
            ',' => vec!["..", "..", "..", "..", "..", ".#", "#."],
            '.' => vec![".", ".", ".", ".", ".", ".", "#"],
            '%' => vec![".#....#", "#.#..#.", ".#..#..", "...#...", "..#..#.", ".#..#.#", "#....#."],
            ' ' => vec!["...."; 7],
            d => DIGITS.iter().find(|(c, _)| *c == d).unwrap().1.to_vec(),
        }
    }

    /// `text` in the EXP font, white on a dark bar, each font pixel drawn as
    /// `scale`×`scale` screen pixels and area-averaged (smoothed) where
    /// `scale` isn't whole, as a stretched fullscreen game is.
    fn render(text: &str, scale: f32) -> Frame {
        // The font at 1×: one blank column between glyphs, a margin around.
        let mut rows = vec![String::from(".."); 11];
        for c in text.chars() {
            let p = pattern(c);
            for (y, row) in rows.iter_mut().enumerate() {
                row.push_str(if (2..9).contains(&y) { p[y - 2] } else { &"......."[..p[0].len()] });
                row.push('.');
            }
        }
        rows.iter_mut().for_each(|r| r.push_str(".."));
        let (w, h) = (rows[0].len(), rows.len());
        let on = |x: usize, y: usize| rows[y].as_bytes()[x] == b'#';
        let (ow, oh) = ((w as f32 * scale) as u32, (h as f32 * scale) as u32);
        let mut rgba = Vec::with_capacity((ow * oh * 4) as usize);
        for oy in 0..oh {
            for ox in 0..ow {
                // Share of this screen pixel covered by ink at 1×.
                let (ax, bx) = (ox as f32 / scale, (ox + 1) as f32 / scale);
                let (ay, by) = (oy as f32 / scale, (oy + 1) as f32 / scale);
                let mut cover = 0.0;
                for y in ay.floor() as usize..(by.ceil() as usize).min(h) {
                    for x in ax.floor() as usize..(bx.ceil() as usize).min(w) {
                        let wx = (bx.min(x as f32 + 1.0) - ax.max(x as f32)).max(0.0);
                        let wy = (by.min(y as f32 + 1.0) - ay.max(y as f32)).max(0.0);
                        cover += if on(x, y) { wx * wy } else { 0.0 };
                    }
                }
                let v = (60.0 + 190.0 * cover * scale * scale) as u8;
                rgba.extend([v, v, v, 255]);
            }
        }
        Frame { width: ow, height: oh, stride: ow as usize * 4, rgba }
    }

    fn read(frame: &Frame, ink: ExpInk) -> Option<(u64, u32)> {
        read_exp_by_shape(&exp_line_with(frame, ink)?)
    }

    const TEXTS: [(&str, (u64, u32)); 3] =
        [("117,832[84.98%]", (117_832, 8498)), ("27,066[17.40%]", (27_066, 1740)), ("5,690[3.71%]", (5_690, 371))];

    #[test]
    fn reads_every_character_drawn_sharp() {
        for scale in [1.0, 2.0, 3.0] {
            for (text, want) in TEXTS {
                assert_eq!(read(&render(text, scale), ExpInk::Otsu), Some(want), "{text} at {scale}x");
            }
        }
    }

    #[test]
    fn reads_text_stretched_and_smoothed() {
        // A fullscreen game stretched to the display by a non-whole factor.
        for scale in [1.998, 2.5, 2.8] {
            for (text, want) in TEXTS {
                assert_eq!(read(&render(text, scale), ExpInk::Soft), Some(want), "{text} at {scale}x");
            }
        }
    }

    #[test]
    fn skips_a_stray_mark_beside_the_text() {
        // The EXP gauge's edge, past where the number may grow.
        let frame = render("27,066[17.40%]     1", 2.0);
        assert_eq!(read(&frame, ExpInk::Otsu), Some((27_066, 1740)));
    }
}
