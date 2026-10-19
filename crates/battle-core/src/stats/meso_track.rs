//! Following notification lines from frame to frame so each is counted once.
//!
//! New lines appear at the bottom and push older ones up, several at a time
//! (one kill adds EXP, party bonus and meso lines). Reads are noisy: lines
//! over busy map art come and go, the top line is often lost, a digit can be
//! misread for a frame. So the stack is anchored at the bottom and each
//! frame's shift (how many lines are new) is the one best supported by
//! evidence: lines that read the same, minus the cost of every line it
//! claims is new or appeared from nowhere. A line can only fade, never
//! brighten, which tells `+15, +15` (nothing new) apart from a fresh `+15`
//! replacing an old one.
//!
//! A meso line is counted once two consecutive reads agree on its amount.

use super::reading::{Line, LineKind};

/// How much brighter a line may read between frames and still be the same
/// line (absorbs binarization noise).
pub const DEFAULT_FRESHNESS_TOLERANCE: u8 = 24;

/// Evidence for a shift: a line read the same as before…
const MATCH: i32 = 2;
/// …or differently (cheaper than a new line, so a one-frame misread doesn't
/// rearrange the stack).
const MISMATCH: i32 = -3;
/// Each line the shift claims is new.
const NEW_LINE: i32 = -4;
/// Each line above everything tracked: lines don't appear at the top.
const APPEARED_ABOVE: i32 = -5;
/// Frames a line may go unread above the stack before it's forgotten.
const MAX_MISSING: u8 = 5;
const MAX_TRACKS: usize = 16;

/// What one read says a line is: kind plus amount (or fingerprint).
type Text = (LineKind, u64);

#[derive(Clone, Debug)]
struct Track {
    /// Text agreed on by two consecutive reads.
    confirmed: Option<Text>,
    /// The latest read text and how many consecutive reads agreed on it.
    last: Option<Text>,
    streak: u8,
    /// Last measured freshness; 0 = never measured.
    freshness: u8,
    /// Already counted, or not countable (on screen at start, or appeared
    /// above the stack).
    counted: bool,
    /// Consecutive frames this line wasn't in the read stack.
    missing: u8,
}

impl Track {
    fn new(line: &Line, counted: bool) -> Self {
        let mut t = Self { confirmed: None, last: None, streak: 0, freshness: 0, counted, missing: 0 };
        t.observe(line);
        t
    }

    fn text(&self) -> Option<Text> {
        self.confirmed.or(self.last)
    }

    /// Records a read. Returns mesos to add: a meso line once its amount is
    /// confirmed, and again if a different amount is then confirmed at this
    /// position (the line was replaced, e.g. a burst pushed it off).
    fn observe(&mut self, line: &Line) -> u64 {
        self.missing = 0;
        if line.freshness > 0 {
            self.freshness = line.freshness;
        }
        if line.is_unknown() {
            return 0;
        }
        let text = (line.kind, line.amount);
        self.streak = if self.last == Some(text) { self.streak.saturating_add(1) } else { 1 };
        self.last = Some(text);
        if self.streak < 2 || self.confirmed == Some(text) {
            return 0;
        }
        if self.confirmed.is_some() {
            self.counted = false;
        }
        self.confirmed = Some(text);
        if self.counted {
            return 0;
        }
        self.counted = true;
        if text.0 == LineKind::Meso { text.1 } else { 0 }
    }
}

/// What one frame added.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Update {
    pub meso: u64,
    /// New lines arrived and nothing on screen matched what was there
    /// before: a burst may have pushed lines off unseen.
    pub possible_overflow: bool,
}

#[derive(Clone, Debug)]
pub struct MesoTracker {
    tolerance: u8,
    /// Bottom line first. `None` until the first frame, which is a baseline.
    tracks: Option<Vec<Track>>,
}

impl MesoTracker {
    pub fn new(freshness_tolerance: u8) -> Self {
        Self { tolerance: freshness_tolerance, tracks: None }
    }

    /// The next frame is a baseline again: lines on it are not counted.
    pub fn reset(&mut self) {
        self.tracks = None;
    }

    /// Feeds one frame's stack, top to bottom.
    pub fn feed(&mut self, stack: &[Line]) -> Update {
        let cur: Vec<Line> = stack.iter().rev().copied().collect();
        let Some(tracks) = self.tracks.take() else {
            self.tracks = Some(cur.iter().map(|l| Track::new(l, true)).collect());
            return Update::default();
        };
        let (shift, matches) = (0..=cur.len())
            .filter_map(|s| Some((s, self.score(&tracks, &cur, s)?)))
            // Highest score; on a tie, the fewest new lines.
            .max_by(|a, b| a.1.0.cmp(&b.1.0).then(b.0.cmp(&a.0)))
            .map(|(s, (_, matches))| (s, matches))
            .expect("shift == cur.len() always scores");
        let visible_before = tracks.iter().any(|t| t.missing == 0);
        let mut update =
            Update { meso: 0, possible_overflow: shift > 0 && matches == 0 && visible_before };

        let mut next: Vec<Track> = cur[..shift].iter().map(|l| Track::new(l, false)).collect();
        let above = tracks.len() + shift;
        for (j, mut t) in tracks.into_iter().enumerate() {
            match cur.get(j + shift) {
                Some(line) => update.meso += t.observe(line),
                None if t.missing >= MAX_MISSING => break,
                None => t.missing += 1,
            }
            next.push(t);
        }
        next.extend(cur.iter().skip(above).map(|l| Track::new(l, true)));
        next.truncate(MAX_TRACKS);
        self.tracks = Some(next);
        update
    }

    /// Evidence for `shift` new lines, and how many lines read the same.
    /// `None` if a line would have got brighter.
    fn score(&self, tracks: &[Track], cur: &[Line], shift: usize) -> Option<(i32, usize)> {
        let mut score = NEW_LINE * shift as i32;
        let mut matches = 0;
        for (t, c) in tracks.iter().zip(&cur[shift..]) {
            if c.freshness > 0 && t.freshness > 0 && c.freshness > t.freshness.saturating_add(self.tolerance) {
                return None;
            }
            match t.text() {
                Some(text) if !c.is_unknown() && text == (c.kind, c.amount) => {
                    score += MATCH;
                    matches += 1;
                }
                Some(_) if !c.is_unknown() => score += MISMATCH,
                _ => {}
            }
        }
        let appeared = cur.len().saturating_sub(tracks.len() + shift);
        Some((score + APPEARED_ABOVE * appeared as i32, matches))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::LineKind::{Exp, Meso, PartyBonus, Unknown};

    fn m(amount: u64, freshness: u8) -> Line {
        Line::new(Meso, amount, freshness)
    }

    /// Feeds each frame (top → bottom) and returns the total mesos counted.
    fn run(frames: &[&[Line]]) -> u64 {
        let mut t = MesoTracker::new(DEFAULT_FRESHNESS_TOLERANCE);
        frames.iter().map(|f| t.feed(f).meso).sum()
    }

    #[test]
    fn first_frame_is_a_baseline() {
        assert_eq!(run(&[&[m(15, 200)], &[m(15, 190)], &[m(15, 180)]]), 0);
    }

    #[test]
    fn a_line_needs_two_agreeing_reads() {
        assert_eq!(run(&[&[], &[m(15, 255)]]), 0);
        assert_eq!(run(&[&[], &[m(15, 255)], &[m(15, 250)]]), 15);
        // Covered on the second frame: still waiting.
        let u = Line::new(Unknown, 0, 0);
        assert_eq!(run(&[&[], &[m(15, 255)], &[u]]), 0);
        assert_eq!(run(&[&[], &[m(15, 255)], &[u], &[m(15, 240)], &[m(15, 235)]]), 15);
    }

    #[test]
    fn duplicate_amount_appended_counts_again() {
        let frames: &[&[Line]] = &[&[], &[m(15, 255)], &[m(15, 250)], &[m(15, 240), m(15, 255)], &[m(15, 235), m(15, 250)]];
        assert_eq!(run(frames), 30);
    }

    #[test]
    fn identical_line_replacing_a_faded_one_is_new() {
        let frames: &[&[Line]] = &[&[], &[m(15, 255)], &[m(15, 250)], &[m(15, 90)], &[m(15, 255)], &[m(15, 250)]];
        assert_eq!(run(frames), 30);
    }

    #[test]
    fn burst_of_four_with_the_top_line_lost() {
        // One kill adds EXP, party bonus and two meso lines; the old top
        // line is unreadable over map art in the second frame.
        let exp = Line::new(Exp, 0, 255);
        let pb = Line::new(PartyBonus, 0, 255);
        let frames: &[&[Line]] = &[
            &[m(5, 200), m(15, 255)],
            &[m(5, 195), m(15, 250)],
            &[m(15, 240), exp, pb, m(20, 255), m(30, 255)],
            &[exp, pb, m(20, 250), m(30, 250)],
        ];
        assert_eq!(run(frames), 50);
    }

    #[test]
    fn one_frame_misread_is_not_counted() {
        let frames: &[&[Line]] = &[&[m(15, 255)], &[m(15, 250)], &[m(16, 245)], &[m(15, 240)], &[m(15, 235)]];
        assert_eq!(run(frames), 0);
    }

    #[test]
    fn line_lost_for_a_few_frames_is_not_counted_again() {
        let frames: &[&[Line]] = &[
            &[],
            &[m(30, 255), m(7, 255)],
            &[m(30, 250), m(7, 250)],
            &[m(7, 240)],
            &[m(7, 235)],
            &[m(30, 220), m(7, 230)],
            &[m(30, 215), m(7, 225)],
        ];
        assert_eq!(run(frames), 37);
    }

    #[test]
    fn unknown_line_does_not_absorb_a_brighter_new_one() {
        // Grinding-01 frames 122–127: a faded line goes unreadable, then a
        // fresh +25 arrives as the stack expires.
        let pb = Line::new(PartyBonus, 0, 90);
        let frames: &[&[Line]] = &[
            &[m(15, 100), pb],
            &[m(15, 95), pb],
            &[Line::new(Unknown, 0, 42), Line::new(Unknown, 0, 86)],
            &[Line::new(Unknown, 0, 40), Line::new(Unknown, 0, 0), m(25, 154)],
            &[m(25, 142)],
        ];
        assert_eq!(run(frames), 25);
    }

    #[test]
    fn replaced_line_is_counted_when_reads_confirm_it() {
        // A burst overflowed the stack: the old +5 is gone and +11 sits where
        // it was, read the same on two frames.
        let mut t = MesoTracker::new(DEFAULT_FRESHNESS_TOLERANCE);
        t.feed(&[]);
        t.feed(&[m(5, 255)]);
        assert_eq!(t.feed(&[m(5, 250)]).meso, 5);
        let burst = [m(11, 255), m(12, 255), m(13, 255), m(14, 255)];
        let first = t.feed(&burst);
        assert!(first.possible_overflow);
        assert_eq!(first.meso + t.feed(&burst).meso, 11 + 12 + 13 + 14);
    }

    #[test]
    fn steady_stack_counts_nothing() {
        let s = [m(1, 200), m(2, 220), m(3, 240)];
        let mut t = MesoTracker::new(DEFAULT_FRESHNESS_TOLERANCE);
        t.feed(&[]);
        let total: u64 = (0..10).map(|_| t.feed(&s).meso).sum();
        assert_eq!(total, 6);
    }
}
