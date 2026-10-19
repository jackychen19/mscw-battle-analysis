use std::fmt;
use std::ops::Range;

use serde::{Deserialize, Serialize};

use super::bitmap::Bitmap;
use super::segment::column_runs;

/// Default minimum match score (intersection over union) for a glyph.
pub const DEFAULT_MIN_SCORE: f32 = 0.7;
/// Templates kept per character; extra variants help with blurry captures.
const MAX_VARIANTS: usize = 4;

/// One learned character: its columns, at the full height of its line.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlyphTemplate {
    pub ch: char,
    pub bitmap: Bitmap,
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

/// Crops a bitmap to the rows that contain ink: the band that teaching and
/// reading both work in.
pub fn line_band(b: &Bitmap) -> Option<Bitmap> {
    let rows = b.ink_rows()?;
    Some(b.crop(0, rows.start, b.width(), rows.end - rows.start))
}

impl GlyphSet {
    /// Learns templates from `line` (a single text line) showing `text`.
    /// Spaces in `text` are ignored. If the line has more glyphs than `text`
    /// has characters, the extra ones on the left are skipped (a label like
    /// `EXP.`). Returns the column where the taught text starts.
    pub fn teach(&mut self, line: &Bitmap, text: &str) -> Result<u32, TeachError> {
        let chars: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
        let runs = column_runs(line);
        if runs.is_empty() {
            return Err(TeachError::NoInk);
        }
        if runs.len() < chars.len() {
            return Err(TeachError::GlyphCount { expected: chars.len(), found: runs.len() });
        }
        let runs = &runs[runs.len() - chars.len()..];
        for (&ch, run) in chars.iter().zip(runs) {
            self.add(GlyphTemplate { ch, bitmap: line.crop_columns(run.clone()) });
        }
        Ok(runs[0].start)
    }

    pub fn add(&mut self, t: GlyphTemplate) {
        let same_char = self.glyphs.iter().filter(|g| g.ch == t.ch);
        if same_char.clone().count() < MAX_VARIANTS && !same_char.into_iter().any(|g| g.bitmap == t.bitmap) {
            self.glyphs.push(t);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
    }

    /// Reads `line` left to right. `None` if any glyph matches no template
    /// with at least `min_score`.
    pub fn read(&self, line: &Bitmap, min_score: f32) -> Option<LineRead> {
        let max_width = self.glyphs.iter().map(|g| g.bitmap.width()).max()?;
        let mut out = LineRead { text: String::new(), spans: Vec::new(), score: 1.0 };
        for run in column_runs(line) {
            let piece = line.crop_columns(run.clone());
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

    fn best_match(&self, piece: &Bitmap) -> Option<(char, f32)> {
        self.glyphs
            .iter()
            .map(|g| (g.ch, similarity(piece, &g.bitmap)))
            .max_by(|a, b| a.1.total_cmp(&b.1))
    }

    /// Best partition of a merged run into templates, maximising total
    /// width-weighted score.
    fn split(&self, piece: &Bitmap, min_score: f32) -> Option<Vec<(char, Range<u32>, f32)>> {
        let w = piece.width() as usize;
        // best[i] = (total score, previous cut, char, score) for columns 0..i.
        let mut best: Vec<Option<(f32, usize, char, f32)>> = vec![None; w + 1];
        best[0] = Some((0.0, 0, ' ', 1.0));
        for start in 0..w {
            let Some((total, ..)) = best[start] else { continue };
            for g in &self.glyphs {
                let tw = g.bitmap.width() as usize;
                for len in tw.saturating_sub(1).max(1)..=tw + 1 {
                    let end = start + len;
                    if end > w {
                        break;
                    }
                    let score = similarity(&piece.crop_columns(start as u32..end as u32), &g.bitmap);
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

/// Intersection over union of two glyph bitmaps, aligned on their left edge,
/// trying small shifts to absorb a pixel of jitter.
pub fn similarity(a: &Bitmap, b: &Bitmap) -> f32 {
    let w = a.width().max(b.width()) as i32;
    let h = a.height().max(b.height()) as i32;
    let mut best = 0.0f32;
    for dy in -2..=2 {
        for dx in -1..=1 {
            let (mut inter, mut union) = (0u32, 0u32);
            for y in -2..h + 2 {
                for x in -1..w + 1 {
                    let pa = in_bounds(a, x, y);
                    let pb = in_bounds(b, x - dx, y - dy);
                    inter += u32::from(pa && pb);
                    union += u32::from(pa || pb);
                }
            }
            if union > 0 {
                best = best.max(inter as f32 / union as f32);
            }
        }
    }
    best
}

fn in_bounds(b: &Bitmap, x: i32, y: i32) -> bool {
    x >= 0 && y >= 0 && b.get(x as u32, y as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::test_font::render;

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
        assert_eq!(render(SAMPLE, 1).width(), set.read(&render(SAMPLE, 1), DEFAULT_MIN_SCORE).unwrap().spans.last().unwrap().end);
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
        assert_eq!(column_runs(&line).len(), 1);
        assert_eq!(set.read(&line, DEFAULT_MIN_SCORE).unwrap().text, "38.43");
    }

    #[test]
    fn tolerates_a_flipped_pixel() {
        let set = taught();
        let mut line = render("905", 1);
        line.set(1, 1, !line.get(1, 1));
        assert_eq!(set.read(&line, DEFAULT_MIN_SCORE).unwrap().text, "905");
    }

    #[test]
    fn unknown_glyph_fails_the_read() {
        let set = taught();
        assert!(set.read(&render("12X", 1), DEFAULT_MIN_SCORE).is_none());
    }

    #[test]
    fn teach_skips_a_leading_label() {
        let mut set = GlyphSet::default();
        let line = render("EXP.12", 1);
        let start = set.teach(&line, "1 2").unwrap();
        assert_eq!(start, render("EXP.", 1).width() + 1);
        assert_eq!(set.glyphs.iter().map(|g| g.ch).collect::<String>(), "12");
    }

    #[test]
    fn teach_errors() {
        let mut set = GlyphSet::default();
        assert_eq!(set.teach(&Bitmap::new(5, 5), "1"), Err(TeachError::NoInk));
        assert_eq!(set.teach(&render("12", 1), "123"), Err(TeachError::GlyphCount { expected: 3, found: 2 }));
    }

    #[test]
    fn duplicate_templates_are_not_stored() {
        let mut set = GlyphSet::default();
        set.teach(&render("1111", 1), "1111").unwrap();
        assert_eq!(set.glyphs.len(), 1);
    }
}
