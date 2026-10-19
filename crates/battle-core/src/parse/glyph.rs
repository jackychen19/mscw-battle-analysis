use std::fmt;
use std::ops::Range;

use serde::{Deserialize, Serialize};

use super::gray::Gray;
use super::line::TextLine;

/// Default minimum match score (normalized cross-correlation) for a glyph.
pub const DEFAULT_MIN_SCORE: f32 = 0.75;
/// Templates kept per character; extra variants help with blurry captures.
const MAX_VARIANTS: usize = 8;

/// One learned character: its columns at the full height of its line, in
/// grayscale so blur and fading don't erase the details that tell similar
/// glyphs (`6`/`8`) apart.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlyphTemplate {
    pub ch: char,
    pub image: Gray,
}

/// The templates for one font (EXP text, level digits, notifications…).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlyphSet {
    pub glyphs: Vec<GlyphTemplate>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TeachError {
    NoInk,
    /// The line has fewer glyphs than the typed text has characters.
    GlyphCount { expected: usize, found: usize },
}

impl fmt::Display for TeachError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoInk => write!(f, "no text found in the selected box"),
            Self::GlyphCount { expected, found } => {
                write!(f, "expected {expected} characters but found {found} in the selected box")
            }
        }
    }
}

impl std::error::Error for TeachError {}

/// A line read back as text.
#[derive(Clone, Debug, PartialEq)]
pub struct LineRead {
    pub text: String,
    /// Columns of each character, left to right.
    pub spans: Vec<Range<u32>>,
    /// Worst per-character match score.
    pub score: f32,
}

impl GlyphSet {
    /// Learns templates from `line` showing `text`. Spaces in `text` are
    /// ignored. If the line has more glyphs than `text` has characters, the
    /// extra ones on the left are skipped (a label like `EXP.`). Returns the
    /// column where the taught text starts.
    pub fn teach(&mut self, line: &TextLine, text: &str) -> Result<u32, TeachError> {
        let chars: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
        let found = line.glyphs.len();
        if found == 0 {
            return Err(TeachError::NoInk);
        }
        if found < chars.len() {
            return Err(TeachError::GlyphCount { expected: chars.len(), found });
        }
        let first = found - chars.len();
        for (i, &ch) in chars.iter().enumerate() {
            self.add(GlyphTemplate { ch, image: line.glyph(first + i) });
        }
        Ok(line.glyphs[first].start)
    }

    pub fn add(&mut self, t: GlyphTemplate) {
        let same_char = self.glyphs.iter().filter(|g| g.ch == t.ch);
        if same_char.clone().count() < MAX_VARIANTS && !same_char.into_iter().any(|g| g.image == t.image) {
            self.glyphs.push(t);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
    }

    /// Reads `line` left to right. `None` if any glyph matches no template
    /// with at least `min_score`.
    pub fn read(&self, line: &TextLine, min_score: f32) -> Option<LineRead> {
        let max_width = self.glyphs.iter().map(|g| g.image.width()).max()?;
        let mut out = LineRead { text: String::new(), spans: Vec::new(), score: 1.0 };
        for run in line.glyphs.iter().cloned() {
            let piece = line.gray.crop_columns(run.clone());
            let single = self.best_match(&piece).filter(|&(_, s)| s >= min_score);
            match single {
                Some((ch, score)) if piece.width() <= max_width + 1 => {
                    out.text.push(ch);
                    out.spans.push(run);
                    out.score = out.score.min(score);
                }
                // Blur can merge neighbouring glyphs into one run: split it.
                _ => {
                    for (ch, span, score) in self.split(&piece, min_score)? {
                        out.text.push(ch);
                        out.spans.push(run.start + span.start..run.start + span.end);
                        out.score = out.score.min(score);
                    }
                }
            }
        }
        Some(out)
    }

    fn best_match(&self, piece: &Gray) -> Option<(char, f32)> {
        self.glyphs
            .iter()
            .map(|g| (g.ch, similarity(piece, &g.image)))
            .max_by(|a, b| a.1.total_cmp(&b.1))
    }

    /// Best partition of a merged run into templates, maximising total
    /// width-weighted score.
    fn split(&self, piece: &Gray, min_score: f32) -> Option<Vec<(char, Range<u32>, f32)>> {
        let w = piece.width() as usize;
        // best[i] = (total score, previous cut, char, score) for columns 0..i.
        let mut best: Vec<Option<(f32, usize, char, f32)>> = vec![None; w + 1];
        best[0] = Some((0.0, 0, ' ', 1.0));
        for start in 0..w {
            let Some((total, ..)) = best[start] else { continue };
            for g in &self.glyphs {
                let tw = g.image.width() as usize;
                for len in tw.saturating_sub(1).max(1)..=tw + 1 {
                    let end = start + len;
                    if end > w {
                        break;
                    }
                    let score = similarity(&piece.crop_columns(start as u32..end as u32), &g.image);
                    if score < min_score {
                        continue;
                    }
                    let candidate = total + score * len as f32;
                    if best[end].is_none_or(|(t, ..)| candidate > t) {
                        best[end] = Some((candidate, start, g.ch, score));
                    }
                }
            }
        }
        let mut parts = Vec::new();
        let mut end = w;
        while end > 0 {
            let (_, start, ch, score) = best[end]?;
            parts.push((ch, start as u32..end as u32, score));
            end = start;
        }
        parts.reverse();
        Some(parts)
    }
}

/// Normalized cross-correlation of two glyph images, aligned on their left
/// edge. Searches vertical shifts of up to 2 px and horizontal shifts of up
/// to 1.5 px in half-pixel steps: when the game is scaled by a non-integer
/// factor, the same glyph renders differently depending on where it falls
/// between pixels. Insensitive to overall brightness and contrast (fading
/// lines). Areas outside an image count as that image's background (its
/// darkest pixel). 1.0 is identical.
pub fn similarity(a: &Gray, b: &Gray) -> f32 {
    let w = a.width().max(b.width()) as usize;
    let h = a.height().max(b.height()) as usize;
    // Both images on one padded canvas (outside = their background), so the
    // shifted comparisons below are plain array reads.
    let (pa, pb) = (Padded::new(a, w, h), Padded::new(b, w, h));
    let score = |dy: i32, half_dx: i32| ncc_shifted(&pa, &pb, w, h, dy, half_dx);
    // Coarse to fine: vertical first, then horizontal, then vertical again.
    let best_of = |candidates: &mut dyn Iterator<Item = (i32, i32)>| {
        candidates.map(|(dy, dx)| (dy, dx, score(dy, dx))).max_by(|p, q| p.2.total_cmp(&q.2)).unwrap()
    };
    let (dy, _, _) = best_of(&mut (-2..=2).map(|dy| (dy, 0)));
    let (_, dx, _) = best_of(&mut (-3..=3).map(|dx| (dy, dx)));
    let (_, _, best) = best_of(&mut (-2..=2).map(|dy| (dy, dx)));
    best.max(0.0) as f32
}

/// Margin around the compared area: 2 px for the window, 2 more for shifts.
const PAD: usize = 4;

struct Padded {
    stride: usize,
    data: Vec<f32>,
}

impl Padded {
    fn new(g: &Gray, w: usize, h: usize) -> Self {
        let stride = w + 2 * PAD;
        let bg = f32::from(g.min());
        let mut data = vec![bg; stride * (h + 2 * PAD)];
        for y in 0..g.height() as usize {
            for x in 0..g.width() as usize {
                data[(y + PAD) * stride + x + PAD] = f32::from(g.get(x as u32, y as u32));
            }
        }
        Self { stride, data }
    }

    /// Pixel at canvas coordinates relative to the image origin.
    fn at(&self, x: i32, y: i32) -> f32 {
        self.data[(y + PAD as i32) as usize * self.stride + (x + PAD as i32) as usize]
    }
}

/// NCC over the image area plus a 2 px margin, with `b` shifted down by `dy`
/// and right by `half_dx / 2` pixels (odd steps blend two columns).
fn ncc_shifted(a: &Padded, b: &Padded, w: usize, h: usize, dy: i32, half_dx: i32) -> f64 {
    let (whole, frac) = (half_dx.div_euclid(2), if half_dx % 2 == 0 { 0.0 } else { 0.5 });
    let (mut sa, mut sb, mut saa, mut sbb, mut sab) = (0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64);
    let mut n = 0.0f64;
    for y in -2..h as i32 + 2 {
        for x in -2..w as i32 + 2 {
            let va = f64::from(a.at(x, y));
            let bx = x - whole;
            let vb = f64::from((1.0 - frac) * b.at(bx, y - dy) + frac * b.at(bx - 1, y - dy));
            sa += va;
            sb += vb;
            saa += va * va;
            sbb += vb * vb;
            sab += va * vb;
            n += 1.0;
        }
    }
    let var_a = saa - sa * sa / n;
    let var_b = sbb - sb * sb / n;
    if var_a > 0.0 && var_b > 0.0 { (sab - sa * sb / n) / (var_a * var_b).sqrt() } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::bitmap::Bitmap;
    use crate::parse::segment::column_runs;
    use crate::parse::test_font;

    fn render(text: &str, gap: u32) -> TextLine {
        TextLine::from_bitmap(test_font::render(text, gap)).unwrap()
    }

    const SAMPLE: &str = "0123456789,.[]%";

    fn taught() -> GlyphSet {
        let mut set = GlyphSet::default();
        set.teach(&render("0123456789", 1), "0123456789").unwrap();
        set.teach(&render("1,0.0[5]%", 1), "1,0.0[5]%").unwrap();
        set
    }

    #[test]
    fn teach_then_read_round_trips() {
        let set = taught();
        let line = render("440,834[38.43%]", 1);
        let read = set.read(&line, DEFAULT_MIN_SCORE).unwrap();
        assert_eq!(read.text, "440,834[38.43%]");
        assert_eq!(read.score, 1.0);
        assert_eq!(read.spans.len(), 15);
        let sample = render(SAMPLE, 1);
        assert_eq!(sample.bitmap.width(), set.read(&sample, DEFAULT_MIN_SCORE).unwrap().spans.last().unwrap().end);
    }

    #[test]
    fn comma_and_period_differ_by_height() {
        let set = taught();
        assert_eq!(set.read(&render("1,1.1", 1), DEFAULT_MIN_SCORE).unwrap().text, "1,1.1");
    }

    #[test]
    fn merged_glyphs_are_split() {
        let set = taught();
        let line = render("38.43", 0); // no gaps: one run
        assert_eq!(column_runs(&line.bitmap).len(), 1);
        assert_eq!(set.read(&line, DEFAULT_MIN_SCORE).unwrap().text, "38.43");
    }

    #[test]
    fn tolerates_a_flipped_pixel() {
        let set = taught();
        let mut bitmap = test_font::render("905", 1);
        bitmap.set(1, 1, !bitmap.get(1, 1));
        let line = TextLine::from_bitmap(bitmap).unwrap();
        assert_eq!(set.read(&line, DEFAULT_MIN_SCORE).unwrap().text, "905");
    }

    #[test]
    fn a_glyph_below_min_score_fails_the_read() {
        let set = taught();
        // Known glyphs match exactly; the untaught X only resembles them.
        assert_eq!(set.read(&render("12", 1), 0.95).unwrap().text, "12");
        assert!(set.read(&render("12X", 1), 0.95).is_none());
    }

    #[test]
    fn teach_skips_a_leading_label() {
        let mut set = GlyphSet::default();
        let line = render("EXP.12", 1);
        let start = set.teach(&line, "1 2").unwrap();
        assert_eq!(start, test_font::render("EXP.", 1).width() + 1);
        assert_eq!(set.glyphs.iter().map(|g| g.ch).collect::<String>(), "12");
    }

    #[test]
    fn teach_errors() {
        let mut set = GlyphSet::default();
        let empty = TextLine { bitmap: Bitmap::new(5, 5), gray: Gray::new(5, 5, vec![0; 25]), glyphs: vec![] };
        assert_eq!(set.teach(&empty, "1"), Err(TeachError::NoInk));
        assert_eq!(set.teach(&render("12", 1), "123"), Err(TeachError::GlyphCount { expected: 3, found: 2 }));
    }

    #[test]
    fn duplicate_templates_are_not_stored() {
        let mut set = GlyphSet::default();
        set.teach(&render("1111", 1), "1111").unwrap();
        assert_eq!(set.glyphs.len(), 1);
    }
}
