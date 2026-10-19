//! Learns the EXP font with no typing: glyphs that look alike are grouped,
//! the fixed layout `N,NNN[PP.PP%]` (read from the right) says which group is
//! which punctuation mark, and the digits are the one assignment for which
//! every sample's EXP ÷ percent gives the same level max.

use crate::capture::Frame;
use crate::parse::{GlyphSet, GlyphTemplate, Gray, exp_line, similarity};

/// Two glyphs at least this similar are the same character. A direct window
/// capture draws a character identically every time (≈1.0), while different
/// digits of the bitmap font differ by a few pixels.
const SAME_GLYPH: f32 = 0.97;
/// Distinct samples needed before trusting a solution.
const MIN_SAMPLES: usize = 4;
/// Most recent distinct samples kept (a level-up breaks the ratio; old
/// samples age out).
const MAX_SAMPLES: usize = 24;
/// Search steps before giving up (keeps a hopeless solve to well under a second).
const SEARCH_BUDGET: u32 = 2_000_000;
/// Templates kept per group.
const MAX_MEMBERS: usize = 4;

/// What the layout says each glyph of a sample is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Slot {
    Digit,
    Comma,
    Dot,
    Open,
    Close,
    Percent,
}

/// One EXP line as group ids, with the layout worked out.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Sample {
    groups: Vec<usize>,
    slots: Vec<Slot>,
    /// Index of `[` (number before, percent after).
    open: usize,
}

#[derive(Clone, Debug, Default)]
pub struct ExpFontLearner {
    /// Member images per group; the first is the representative.
    groups: Vec<Vec<Gray>>,
    samples: Vec<Vec<usize>>,
}

impl ExpFontLearner {
    /// Adds one EXP crop. Returns false if no text line was found.
    pub fn add(&mut self, crop: &Frame) -> bool {
        let Some(line) = exp_line(crop) else { return false };
        let word = last_word(&line.glyphs);
        let ids: Vec<usize> = word.map(|i| self.group_of(line.glyph(i))).collect();
        if ids.len() < 8 {
            return false;
        }
        if !self.samples.contains(&ids) {
            self.samples.push(ids);
            if self.samples.len() > MAX_SAMPLES {
                self.samples.remove(0);
            }
        }
        true
    }

    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    fn group_of(&mut self, glyph: Gray) -> usize {
        let best = self
            .groups
            .iter()
            .enumerate()
            .map(|(i, m)| (i, similarity(&glyph, &m[0])))
            .max_by(|a, b| a.1.total_cmp(&b.1));
        match best {
            Some((i, s)) if s >= SAME_GLYPH => {
                if self.groups[i].len() < MAX_MEMBERS && !self.groups[i].contains(&glyph) {
                    self.groups[i].push(glyph);
                }
                i
            }
            _ => {
                self.groups.push(vec![glyph]);
                self.groups.len() - 1
            }
        }
    }

    /// The learned font, once exactly one digit assignment fits the most
    /// recent samples. Digits that never appeared stay untaught.
    pub fn solve(&self) -> Option<GlyphSet> {
        // Newest samples first; drop older ones if they don't fit (a level-up
        // during learning changes the ratio).
        for skip in 0..self.samples.len() {
            let recent = &self.samples[skip..];
            if recent.len() < MIN_SAMPLES {
                return None;
            }
            if let Some(chars) = solve_samples(recent, |g, close| self.looks_open(g, close)) {
                return Some(self.font(&chars));
            }
        }
        None
    }

    fn looks_open(&self, g: usize, close: usize) -> f32 {
        similarity(&self.groups[g][0], &mirrored(&self.groups[close][0]))
    }

    fn font(&self, chars: &[(usize, char)]) -> GlyphSet {
        let mut set = GlyphSet::default();
        for &(group, ch) in chars {
            for image in &self.groups[group] {
                set.add(GlyphTemplate { ch, image: image.clone() });
            }
        }
        set
    }
}

fn mirrored(g: &Gray) -> Gray {
    let (w, h) = (g.width(), g.height());
    Gray::new(w, h, (0..h).flat_map(|y| (0..w).rev().map(move |x| (x, y))).map(|(x, y)| g.get(x, y)).collect())
}

/// Glyph indices of the rightmost word: glyphs since the last gap clearly
/// wider than the usual letter spacing (drops an `EXP.` label).
pub(super) fn last_word(glyphs: &[std::ops::Range<u32>]) -> std::ops::Range<usize> {
    let gaps: Vec<u32> = glyphs.windows(2).map(|w| w[1].start - w[0].end).collect();
    let mut sorted = gaps.clone();
    sorted.sort_unstable();
    let Some(&median) = sorted.get(sorted.len() / 2) else { return 0..glyphs.len() };
    let start = gaps.iter().rposition(|&g| g > median * 2 && g >= median + 2).map_or(0, |i| i + 1);
    start..glyphs.len()
}

/// Group → character for punctuation and digits, if exactly one digit
/// assignment fits all samples.
fn solve_samples(samples: &[Vec<usize>], looks_open: impl Fn(usize, usize) -> f32) -> Option<Vec<(usize, char)>> {
    let layout = Layout::find(samples, looks_open)?;
    let parsed: Vec<Sample> = samples.iter().map(|s| layout.parse(s)).collect::<Option<_>>()?;

    // Digit groups in order of significance in the first samples, so the
    // search fixes the leading digits (the most constrained) first.
    let mut digits: Vec<usize> = Vec::new();
    for s in &parsed {
        for (i, &g) in s.groups.iter().enumerate() {
            if s.slots[i] == Slot::Digit && !digits.contains(&g) {
                digits.push(g);
            }
        }
    }
    if digits.len() > 10 {
        return None;
    }
    // After assigning digits[..=k], these samples are fully known.
    let mut complete_at: Vec<Vec<usize>> = vec![Vec::new(); digits.len()];
    for (si, s) in parsed.iter().enumerate() {
        let last = s
            .groups
            .iter()
            .zip(&s.slots)
            .filter(|(_, slot)| **slot == Slot::Digit)
            .map(|(g, _)| digits.iter().position(|d| d == g).unwrap())
            .max()?;
        complete_at[last].push(si);
    }

    let mut search = Search { parsed: &parsed, digits: &digits, complete_at: &complete_at, value: vec![None; digits.len()], solutions: Vec::new(), steps: 0 };
    search.run(0, (0.0, f64::INFINITY));
    if search.steps >= SEARCH_BUDGET {
        return None;
    }
    let [solution] = search.solutions.as_slice() else { return None };

    let mut chars: Vec<(usize, char)> = digits.iter().zip(solution).map(|(&g, &d)| (g, char::from(b'0' + d))).collect();
    chars.extend([(layout.close, ']'), (layout.percent, '%'), (layout.dot, '.'), (layout.open, '[')]);
    if let Some(c) = layout.comma {
        chars.push((c, ','));
    }
    Some(chars)
}

/// Which group is each punctuation mark, from where marks must sit.
#[derive(Debug)]
struct Layout {
    close: usize,
    percent: usize,
    dot: usize,
    open: usize,
    comma: Option<usize>,
}

impl Layout {
    /// `looks_open(g, close)`: how much group `g` looks like group `close` mirrored.
    fn find(samples: &[Vec<usize>], looks_open: impl Fn(usize, usize) -> f32) -> Option<Self> {
        let at = |s: &Vec<usize>, from_end: usize| s.len().checked_sub(from_end).map(|i| s[i]);
        let same = |from_end: usize| {
            let first = at(&samples[0], from_end)?;
            samples.iter().all(|s| at(s, from_end) == Some(first)).then_some(first)
        };
        // `…[P.PP%]` or `…[PP.PP%]`.
        let (close, percent, dot) = (same(1)?, same(2)?, same(5)?);
        let marks = [close, percent, dot];
        // Groups certainly holding digits: the percent's last three digits.
        let known_digits: Vec<usize> = samples.iter().flat_map(|s| [at(s, 3), at(s, 4), at(s, 6)]).flatten().collect();
        let not_digit = |g: usize| !known_digits.contains(&g) && !marks.contains(&g);
        // `[` sits 7th or 8th from the end in every sample.
        // When both could be (`36.84%` always puts a `3` 7th from the end), the
        // one shaped like a mirrored `]` wins.
        let open = [at(&samples[0], 7), at(&samples[0], 8)]
            .into_iter()
            .flatten()
            .filter(|&g| not_digit(g) && samples.iter().all(|s| at(s, 7) == Some(g) || at(s, 8) == Some(g)))
            .max_by(|&a, &b| looks_open(a, close).total_cmp(&looks_open(b, close)))?;
        // `,` sits 4th left of `[` once EXP reaches 1,000.
        let comma = samples.iter().find_map(|s| {
            let open_at = s.iter().rposition(|&g| g == open)?;
            let g = *s.get(open_at.checked_sub(4)?)?;
            (g != open && not_digit(g) && s[open_at - 3..open_at].iter().all(|&d| d != g)).then_some(g)
        });
        Some(Self { close, percent, dot, open, comma })
    }

    fn parse(&self, s: &[usize]) -> Option<Sample> {
        let n = s.len();
        let open = s.iter().rposition(|&g| g == self.open)?;
        if !(open == n - 7 || open == n - 8) {
            return None;
        }
        let mut slots = vec![Slot::Digit; n];
        slots[open] = Slot::Open;
        slots[n - 1] = Slot::Close;
        slots[n - 2] = Slot::Percent;
        slots[n - 5] = Slot::Dot;
        // Number: groups of three digits right to left, separated by commas.
        let mut i = open;
        let mut in_group = 0;
        let mut start = open;
        while i > 0 {
            let g = s[i - 1];
            if Some(g) == self.comma {
                if in_group != 3 {
                    return None;
                }
                slots[i - 1] = Slot::Comma;
                in_group = 0;
            } else if [self.open, self.close, self.percent, self.dot].contains(&g) || in_group == 3 {
                break;
            } else {
                in_group += 1;
                start = i - 1;
            }
            i -= 1;
        }
        if start == open || slots[start] != Slot::Digit {
            return None;
        }
        // Anything left of the number isn't part of it.
        Some(Sample { groups: s[start..].to_vec(), slots: slots[start..].to_vec(), open: open - start })
    }
}

struct Search<'a> {
    parsed: &'a [Sample],
    digits: &'a [usize],
    complete_at: &'a [Vec<usize>],
    value: Vec<Option<u8>>,
    /// Stops at two: only a unique answer is useful.
    solutions: Vec<Vec<u8>>,
    steps: u32,
}

impl Search<'_> {
    /// `m` is the range of level max EXP consistent with the samples so far.
    fn run(&mut self, k: usize, m: (f64, f64)) {
        self.steps += 1;
        if self.solutions.len() > 1 || self.steps >= SEARCH_BUDGET {
            return;
        }
        if k == self.digits.len() {
            self.solutions.push(self.value.iter().map(|v| v.unwrap()).collect());
            return;
        }
        for d in 0..10u8 {
            if self.value.contains(&Some(d)) {
                continue;
            }
            self.value[k] = Some(d);
            let mut range = m;
            let ok = self.complete_at[k].iter().all(|&si| {
                let Some((exp, pct)) = self.decode(&self.parsed[si]) else { return false };
                let (lo, hi) = max_range(exp, pct);
                range = (range.0.max(lo), range.1.min(hi));
                range.0 <= range.1
            });
            if ok {
                self.run(k + 1, range);
            }
        }
        self.value[k] = None;
    }

    fn decode(&self, s: &Sample) -> Option<(u64, u32)> {
        let digit = |g: usize| self.value[self.digits.iter().position(|&d| d == g).unwrap()].map(u64::from);
        let number = |range: &mut dyn Iterator<Item = usize>| -> Option<(u64, usize, Option<u64>)> {
            let mut v = 0u64;
            let mut count = 0;
            let mut first = None;
            for i in range {
                if s.slots[i] == Slot::Digit {
                    let d = digit(s.groups[i])?;
                    first.get_or_insert(d);
                    v = v * 10 + d;
                    count += 1;
                }
            }
            Some((v, count, first))
        };
        let (exp, exp_len, exp_first) = number(&mut (0..s.open))?;
        let n = s.groups.len();
        let (whole, whole_len, whole_first) = number(&mut (s.open + 1..n - 5))?;
        let (frac, ..) = number(&mut (n - 4..n - 2))?;
        // No leading zeros.
        if (exp_len > 1 && exp_first == Some(0)) || (whole_len > 1 && whole_first == Some(0)) {
            return None;
        }
        Some((exp, (whole * 100 + frac) as u32))
    }
}

/// Level max EXP consistent with `exp` at `pct` hundredths, whether the game
/// rounds or truncates the percent.
fn max_range(exp: u64, pct: u32) -> (f64, f64) {
    let e = exp as f64 * 10_000.0;
    let pct = f64::from(pct);
    let lo = e / (pct + 1.0);
    let hi = if pct > 0.5 { e / (pct - 0.5) } else { f64::INFINITY };
    // The max must exceed the current EXP.
    (lo.max(exp as f64), hi)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::test_font::render_frame;

    const MAX: u64 = 1_147_000;

    fn line(exp: u64) -> String {
        let pct = (exp * 10_000 / MAX) as u32;
        crate::parse::format_exp_text(exp, pct)
    }

    #[test]
    fn finds_the_layout_and_drops_the_label() {
        let mut learner = ExpFontLearner::default();
        for exp in [417_750u64, 418_083, 418_876] {
            assert!(learner.add(&render_frame(&format!("EXP. {}", line(exp)))));
        }
        let layout = Layout::find(&learner.samples, |g, close| learner.looks_open(g, close)).unwrap();
        let parsed = layout.parse(&learner.samples[0]).unwrap();
        let shape: String = parsed
            .slots
            .iter()
            .map(|s| match s {
                Slot::Digit => 'd',
                Slot::Comma => ',',
                Slot::Dot => '.',
                Slot::Open => '[',
                Slot::Close => ']',
                Slot::Percent => '%',
            })
            .collect();
        assert_eq!(shape, "ddd,ddd[dd.dd%]");
    }

    #[test]
    fn learns_digits_from_consistent_samples() {
        let mut learner = ExpFontLearner::default();
        // A few kills apart; together they show every digit.
        for exp in [417_750u64, 418_083, 418_876, 419_165, 420_790, 421_770, 422_518, 424_066] {
            assert!(learner.add(&render_frame(&format!("EXP. {}", line(exp)))));
        }
        let font = learner.solve().expect("unique solution");
        for exp in [425_123u64, 430_987, 1_000_456] {
            let read = font.read(&crate::parse::exp_line(&render_frame(&line(exp))).unwrap(), crate::parse::DEFAULT_MIN_SCORE);
            assert_eq!(read.and_then(|r| crate::parse::parse_exp_text(&r.text)).map(|r| r.0), Some(exp), "{exp}");
        }
    }

    #[test]
    fn too_few_samples_give_nothing() {
        let mut learner = ExpFontLearner::default();
        for exp in [417_750u64, 418_083] {
            learner.add(&render_frame(&line(exp)));
        }
        assert!(learner.solve().is_none());
    }
}
