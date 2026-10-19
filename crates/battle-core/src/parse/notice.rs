//! The notification stack: right-aligned lines like
//! `You have gained mesos (+15)` over map art, newest at the bottom, older
//! ones moving up and fading.

use serde::{Deserialize, Serialize};

use super::glyph::{DEFAULT_MIN_SCORE, GlyphSet, TeachError, similarity};
use super::gray::{Channel, Gray};
use super::line::TextLine;
use crate::capture::{Frame, RectPx};
use crate::stats::{Line, LineKind};

/// White and yellow text over green and brown map art.
const NOTICE_CHANNEL: Channel = Channel::MinRG;
/// Minimum score for a line's opening words to match a learned prefix.
pub const PREFIX_MIN_SCORE: f32 = 0.6;
/// A `)` at the right edge marks a line as present.
const CLOSE_PAREN_MIN_SCORE: f32 = 0.8;
/// How far (px) a line's band may sit from where the layout expects it.
const BAND_SLACK: u32 = 4;
/// How far (px) a line's last glyph may end from the layout's right edge.
const RIGHT_SLACK: u32 = 4;

/// Where lines sit in the notification crop (crop pixels).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackLayout {
    /// Top row of the bottom line.
    pub bottom_top: f32,
    pub line_height: u32,
    /// Distance between consecutive lines. Fractional when the game is
    /// scaled by a non-integer factor.
    pub pitch: f32,
    /// Column just past the end of every line (they are right-aligned).
    /// Nothing at or beyond it is read.
    pub right: u32,
}

/// The opening words of one kind of line, e.g. `You have gained mesos`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrefixTemplate {
    pub kind: LineKind,
    /// Columns between the prefix's last ink and the `(`.
    pub gap: u32,
    pub image: Gray,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoticeReader {
    pub layout: StackLayout,
    /// `(`, `+`, digits and `)`.
    pub glyphs: GlyphSet,
    pub prefixes: Vec<PrefixTemplate>,
    pub min_score: f32,
}

/// What one line slot holds.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Slot {
    Empty,
    Unreadable(u8),
    Known(Line),
}

impl NoticeReader {
    pub fn new(layout: StackLayout) -> Self {
        Self { layout, glyphs: GlyphSet::default(), prefixes: Vec::new(), min_score: DEFAULT_MIN_SCORE }
    }

    /// Calibration: `lines` lists the stack bottom-up; `Some((kind, amount))`
    /// for a line to learn from, `None` to skip one (covered, or a kind
    /// without a `(+N)` amount).
    pub fn teach(&mut self, crop: &Frame, lines: &[Option<(LineKind, u64)>]) -> Result<(), TeachError> {
        let gray = Gray::from_frame(crop, NOTICE_CHANNEL);
        for (slot, line) in lines.iter().enumerate() {
            let Some((kind, amount)) = *line else { continue };
            let text_line = self.slot_line(&gray, slot).ok_or(TeachError::NoInk)?;
            let paren = self.glyphs.teach(&text_line, &format!("(+{amount})"))?;
            let Some((first, prefix_end)) = self.prefix_span(&text_line, paren) else { continue };
            let template = PrefixTemplate {
                kind,
                gap: paren - prefix_end,
                image: text_line.gray.crop_columns(first..prefix_end),
            };
            if self.prefixes.iter().filter(|p| p.kind == kind).count() < 3 {
                self.prefixes.push(template);
            }
        }
        Ok(())
    }

    /// The stack, top to bottom. A slot between readable lines that can't be
    /// read is reported as [`LineKind::Unknown`]; empty slots above the
    /// topmost line are omitted.
    pub fn read(&self, crop: &Frame) -> Vec<Line> {
        let gray = Gray::from_frame(crop, NOTICE_CHANNEL);
        let slots: Vec<Slot> = (0..self.slot_count()).map(|i| self.read_slot(&gray, i)).collect();
        let Some(top) = slots.iter().rposition(|s| *s != Slot::Empty) else {
            return Vec::new();
        };
        slots[..=top]
            .iter()
            .rev()
            .map(|slot| match *slot {
                Slot::Known(line) => line,
                Slot::Unreadable(freshness) => Line::new(LineKind::Unknown, 0, freshness),
                Slot::Empty => Line::new(LineKind::Unknown, 0, 0),
            })
            .collect()
    }

    fn slot_count(&self) -> usize {
        (self.layout.bottom_top / self.layout.pitch) as usize + 1
    }

    /// Slot `i` (0 = bottom) as a text line, cropped to the right edge.
    fn slot_line(&self, gray: &Gray, i: usize) -> Option<TextLine> {
        let top = self.layout.bottom_top - i as f32 * self.layout.pitch;
        if top < 0.0 {
            return None;
        }
        let y = (top.round() as u32).saturating_sub(BAND_SLACK);
        let rect = RectPx { x: 0, y, w: self.layout.right, h: self.layout.line_height + 2 * BAND_SLACK };
        let line = TextLine::from_gray(&gray.crop(rect))?;
        // A band much thinner than a line is art, not text.
        (line.bitmap.height() * 2 >= self.layout.line_height).then_some(line)
    }

    /// Only meso lines need their amount. Other lines are identified by
    /// their opening words alone (their amounts are never counted).
    fn read_slot(&self, gray: &Gray, i: usize) -> Slot {
        let Some(line) = self.slot_line(gray, i) else { return Slot::Empty };
        if !self.ends_with_close_paren(&line) {
            return Slot::Empty;
        }
        let Some(read) = self.glyphs.read_from_right(&line, self.min_score, |t| t.starts_with("(+")) else {
            return Slot::Unreadable(freshness(&line, line.glyphs.last().unwrap().clone()));
        };
        let fresh = freshness(&line, read.spans[0].start..read.spans.last().unwrap().end);
        match self.match_prefix(&line, read.spans[0].start) {
            Some(LineKind::Meso) => match parse_amount(&read.text) {
                Some(amount) => Slot::Known(Line::new(LineKind::Meso, amount, fresh)),
                None => Slot::Unreadable(fresh),
            },
            Some(kind) => Slot::Known(Line::new(kind, 0, fresh)),
            None => Slot::Unreadable(fresh),
        }
    }

    /// Columns of the words before the `(` at `paren`: glyph runs walking
    /// left from it, stopping at a gap wider than a word space (map art
    /// beside the line is usually further away).
    fn prefix_span(&self, line: &TextLine, paren: u32) -> Option<(u32, u32)> {
        let max_gap = self.layout.line_height / 2;
        let mut runs = line.glyphs.iter().rev().filter(|r| r.end <= paren);
        let last = runs.next()?;
        let mut start = last.start;
        for run in runs {
            if start - run.end > max_gap {
                break;
            }
            start = run.start;
        }
        Some((start, last.end))
    }

    fn ends_with_close_paren(&self, line: &TextLine) -> bool {
        let Some(last) = line.glyphs.last() else { return false };
        if last.end.abs_diff(self.layout.right) > RIGHT_SLACK {
            return false;
        }
        let piece = line.gray.crop_columns(last.clone());
        self.glyphs
            .glyphs
            .iter()
            .filter(|g| g.ch == ')')
            .any(|g| similarity(&piece, &g.image) >= CLOSE_PAREN_MIN_SCORE)
    }

    /// Kind of the line whose `(` starts at column `paren`.
    fn match_prefix(&self, line: &TextLine, paren: u32) -> Option<LineKind> {
        self.prefixes
            .iter()
            .filter_map(|p| {
                let end = paren.checked_sub(p.gap)?;
                let start = end.checked_sub(p.image.width())?;
                Some((p.kind, similarity(&line.gray.crop_columns(start..end), &p.image)))
            })
            .filter(|&(_, score)| score >= PREFIX_MIN_SCORE)
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(kind, _)| kind)
    }
}

/// `(+48)` → 48.
fn parse_amount(text: &str) -> Option<u64> {
    let digits = text.strip_prefix("(+")?.strip_suffix(')')?;
    let ok = !digits.is_empty() && digits.len() <= 9 && digits.bytes().all(|b| b.is_ascii_digit());
    ok.then(|| digits.parse().ok()).flatten()
}

/// Mean brightness of the ink in `cols`: how faded the line is.
fn freshness(line: &TextLine, cols: std::ops::Range<u32>) -> u8 {
    line.gray.crop_columns(cols.clone()).mean_under(&line.bitmap.crop_columns(cols))
}
