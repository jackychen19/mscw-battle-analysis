//! Finds the EXP text and the level on the game's bottom status bar with no
//! help: the EXP line by its layout (`…[PP.PP%]`: a `[` mirroring the final
//! `]`, a tiny `.` fifth from the end), the level by its orange tiles.

use std::ops::Range;

use super::Regions;
use crate::capture::{Frame, RectPt, RectPx};
use crate::parse::{Channel, Gray, TextLine, exp_line, similarity};

/// Pixels at least this bright can be the white text.
const TEXT_LUMA: u8 = 150;
/// A `[` must look at least this much like the `]` mirrored.
const MIRROR_SCORE: f32 = 0.75;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Blob {
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
    pixels: u32,
}

impl Blob {
    fn w(&self) -> u32 {
        self.x1 - self.x0
    }

    fn h(&self) -> u32 {
        self.y1 - self.y0
    }
}

/// 4-connected blobs of pixels where `ink(x, y)`.
fn blobs(width: u32, height: u32, ink: impl Fn(u32, u32) -> bool) -> Vec<Blob> {
    let mut seen = vec![false; (width * height) as usize];
    let mut out = Vec::new();
    let mut stack = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) as usize;
            if seen[i] || !ink(x, y) {
                continue;
            }
            seen[i] = true;
            stack.push((x, y));
            let mut b = Blob { x0: x, y0: y, x1: x + 1, y1: y + 1, pixels: 0 };
            while let Some((px, py)) = stack.pop() {
                b.pixels += 1;
                b.x0 = b.x0.min(px);
                b.y0 = b.y0.min(py);
                b.x1 = b.x1.max(px + 1);
                b.y1 = b.y1.max(py + 1);
                let neighbours = [(px.wrapping_sub(1), py), (px + 1, py), (px, py.wrapping_sub(1)), (px, py + 1)];
                for (nx, ny) in neighbours {
                    if nx < width && ny < height {
                        let j = (ny * width + nx) as usize;
                        if !seen[j] && ink(nx, ny) {
                            seen[j] = true;
                            stack.push((nx, ny));
                        }
                    }
                }
            }
            out.push(b);
        }
    }
    out
}

/// The EXP text (number through `]`, label excluded) in `frame`, with room
/// on the right for the number to grow.
pub fn find_exp_text(frame: &Frame) -> Option<RectPx> {
    let gray = Gray::from_frame(frame, Channel::Luma);
    let glyphs: Vec<Blob> = blobs(gray.width(), gray.height(), |x, y| gray.get(x, y) >= TEXT_LUMA)
        .into_iter()
        .filter(|b| b.h() <= 60 && b.w() <= 60 && b.pixels >= 2)
        .collect();
    // HP and MP have brackets too but not the percent layout; if several
    // rows qualify, the lowest, rightmost one is the status bar's.
    lines(&glyphs)
        .iter()
        .filter_map(|line| exp_in_line(frame, line))
        .max_by_key(|r| (r.y, r.x))
}

/// Chains of bright blobs left to right that sit on one text row. Blur can
/// fuse or break glyphs, so these only locate rows; glyphs are found again
/// by the EXP line segmenter.
fn lines(glyphs: &[Blob]) -> Vec<Vec<Blob>> {
    let mut sorted = glyphs.to_vec();
    sorted.sort_by_key(|b| (b.x0, b.y0));
    let mut chains: Vec<Vec<Blob>> = Vec::new();
    for g in sorted {
        let fits = |chain: &Vec<Blob>| {
            let right = chain.iter().map(|b| b.x1).max().unwrap();
            let (top, bottom) = (chain.iter().map(|b| b.y0).min().unwrap(), chain.iter().map(|b| b.y1).max().unwrap());
            let height = bottom - top;
            let gap_ok = g.x0 + 1 >= right.min(g.x0 + 1) && g.x0.saturating_sub(right) <= height.max(3);
            // Full-height glyphs overlap the row; small marks sit inside it.
            let overlap = g.y1.min(bottom).saturating_sub(g.y0.max(top));
            gap_ok && overlap * 2 >= g.h().min(height) && g.y0 + 2 >= top && g.y1 <= bottom + 3
        };
        match chains.iter_mut().rev().find(|c| fits(c)) {
            Some(c) => c.push(g),
            None => chains.push(vec![g]),
        }
    }
    chains.retain(|c| c.len() >= 4);
    chains
}

/// Ink rows of the columns `run` of `line`.
fn ink_rows(line: &TextLine, run: &Range<u32>) -> Range<u32> {
    let b = line.bitmap.crop_columns(run.clone());
    b.ink_rows().unwrap_or(0..0)
}

fn exp_in_line(frame: &Frame, chain: &[Blob]) -> Option<RectPx> {
    let top = chain.iter().map(|b| b.y0).min()?;
    let bottom = chain.iter().map(|b| b.y1).max()?;
    let height = bottom - top;
    let margin = (height / 3).max(2);
    // A dim `]` may not have made it into the chain: look a little further right.
    let left = chain[0].x0.saturating_sub(margin);
    let right = (chain.iter().map(|b| b.x1).max()? + height * 2).min(frame.width);
    let y0 = top.saturating_sub(margin);
    // The EXP gauge starts right under the text: no margin below.
    let area = RectPx { x: left, y: y0, w: right - left, h: (bottom + 1).min(frame.height) - y0 };
    let line = exp_line(&frame.crop(area))?;
    words(&line.glyphs).into_iter().find_map(|word| exp_in_word(frame, area, &line, word))
}

/// Glyph index ranges split at gaps clearly wider than the letter spacing.
fn words(runs: &[Range<u32>]) -> Vec<Range<usize>> {
    let gaps: Vec<u32> = runs.windows(2).map(|w| w[1].start - w[0].end).collect();
    let mut sorted = gaps.clone();
    sorted.sort_unstable();
    let median = sorted.get(sorted.len() / 2).copied().unwrap_or(1);
    let mut out = Vec::new();
    let mut start = 0;
    for (i, &g) in gaps.iter().enumerate() {
        if g > median * 2 && g >= median + 2 {
            out.push(start..i + 1);
            start = i + 1;
        }
    }
    out.push(start..runs.len());
    out
}

/// `word` of `line` (cropped from `area` of the frame) if it's the EXP number
/// and percent; the label, if any, is a separate word to its left.
fn exp_in_word(frame: &Frame, area: RectPx, line: &TextLine, word: Range<usize>) -> Option<RectPx> {
    let runs = &line.glyphs[word.clone()];
    let n = runs.len();
    if n < 8 {
        return None;
    }
    let rows = |i: usize| ink_rows(line, &runs[i]);
    let size = |i: usize| {
        let r = rows(i);
        (runs[i].end - runs[i].start, r.end - r.start, r.end)
    };
    let (close_w, close_h, _) = size(n - 1);
    let (_, dot_h, dot_bottom) = size(n - 5);
    // Measured against a digit (the label, if any, is in a bigger font).
    let (_, digit_h, digit_bottom) = size(n - 3);
    // `]` is a tall, narrow mark; `.` is small and sits on the baseline.
    if close_h * 10 < digit_h * 8 || close_w * 2 > close_h || dot_h * 2 > digit_h || dot_bottom + 2 < digit_bottom {
        return None;
    }
    let glyph = |i: usize| line.glyph(word.start + i);
    let mirrored = mirror(&glyph(n - 1));
    let score = |i: usize| similarity(&glyph(i), &mirrored);
    let open = [n - 7, n - 8].into_iter().filter(|&i| score(i) >= MIRROR_SCORE).max_by(|&a, &b| score(a).total_cmp(&score(b)))?;
    // The number runs left from `[` while glyphs are digit-sized (or a small
    // comma on the baseline); the `EXP.` label is taller.
    let digit = rows(n - 3);
    let tol = (digit_h / 6).max(2);
    let in_number = |i: usize| {
        let r = rows(i);
        let digit_like = r.start.abs_diff(digit.start) <= tol && r.end.abs_diff(digit.end) <= tol;
        let comma = r.end - r.start <= digit_h / 2 && r.end + tol >= digit.end;
        digit_like || comma
    };
    let start = (0..open).rev().take_while(|&i| in_number(i)).last()?;
    let glyph_w = runs[open - 1].end - runs[open - 1].start;
    let x0 = area.x + runs[start].start.saturating_sub(2);
    // Room to grow on the right, up to the next ink on the row (the gauge's end).
    let next = line.glyphs.get(word.end).map_or(u32::MAX, |r| area.x + r.start.saturating_sub(2));
    let x1 = (area.x + runs[n - 1].end + glyph_w * 4).min(next).min(frame.width);
    Some(RectPx { x: x0, y: area.y, w: x1 - x0, h: area.h })
}

fn mirror(g: &Gray) -> Gray {
    let (w, h) = (g.width(), g.height());
    Gray::new(w, h, (0..h).flat_map(|y| (0..w).rev().map(move |x| (x, y))).map(|(x, y)| g.get(x, y)).collect())
}

/// The level tiles on the EXP line's status bar: in the band of rows around
/// it and to its left. Elsewhere in the window, other orange things (the
/// minimap's buttons, quest icons) would match too.
pub fn find_level_near(frame: &Frame, exp: RectPx) -> Option<RectPx> {
    let y0 = exp.y.saturating_sub(exp.h * 2);
    let y1 = (exp.y + exp.h * 3).min(frame.height);
    let band = RectPx { x: 0, y: y0, w: exp.x, h: y1 - y0 };
    // Tiles are about as tall as the text; the nearest such group to the
    // left is on the same bar (map art further left can be orange too).
    let r = tile_groups(&frame.crop(band), exp.h / 2).into_iter().max_by_key(|g| g.x)?;
    Some(RectPx { x: r.x + band.x, y: r.y + band.y, ..r })
}

/// Both regions in `strip`, a capture of `area` (points, window-relative) at
/// `scale`, converted back to window points.
pub fn find_regions(strip: &Frame, area: RectPt, scale: f64) -> Option<Regions> {
    let to_pt = |r: RectPx| RectPt {
        x: area.x + f64::from(r.x) / scale,
        y: area.y + f64::from(r.y) / scale,
        w: f64::from(r.w) / scale,
        h: f64::from(r.h) / scale,
    };
    let exp = find_exp_text(strip)?;
    Some(Regions { exp_text: to_pt(exp), level: to_pt(find_level_near(strip, exp)?) })
}

fn orange(p: [u8; 4]) -> bool {
    let [r, g, b, _] = p;
    r >= 170 && (60..=180).contains(&g) && b <= 110 && r >= g + 50
}

/// The level's orange digit tiles: the leftmost row of one to three
/// same-sized, roughly square orange tiles.
pub fn find_level(frame: &Frame) -> Option<RectPx> {
    tile_groups(frame, 6).into_iter().min_by_key(|g| g.x)
}

/// Rows of one to three same-sized, roughly square orange tiles at least
/// `min_h` pixels tall, as boxes with a small margin.
fn tile_groups(frame: &Frame, min_h: u32) -> Vec<RectPx> {
    let tiles: Vec<Blob> = blobs(frame.width, frame.height, |x, y| orange(frame.pixel(x, y)))
        .into_iter()
        .filter(|b| (min_h.max(6)..=80).contains(&b.h()) && b.w() * 10 >= b.h() * 6 && b.w() * 10 <= b.h() * 15)
        .filter(|b| b.pixels * 10 >= b.w() * b.h() * 3)
        .collect();
    let mut groups: Vec<Vec<Blob>> = Vec::new();
    let mut sorted = tiles;
    sorted.sort_by_key(|b| b.x0);
    for t in sorted {
        let joins = |g: &Vec<Blob>| {
            let last = g.last().unwrap();
            let same_size = last.h().abs_diff(t.h()) * 5 <= last.h();
            let level = last.y0.abs_diff(t.y0) * 4 <= last.h();
            same_size && level && t.x0 >= last.x0 && t.x0.saturating_sub(last.x1) * 2 <= last.h()
        };
        match groups.iter_mut().find(|g| joins(g)) {
            Some(g) => g.push(t),
            None => groups.push(vec![t]),
        }
    }
    groups
        .into_iter()
        .filter(|g| g.len() <= 3)
        .map(|g| {
            let margin = (g[0].h() / 6).max(2);
            let x0 = g[0].x0.saturating_sub(margin);
            let y0 = g.iter().map(|b| b.y0).min().unwrap().saturating_sub(margin);
            let x1 = (g.last().unwrap().x1 + margin).min(frame.width);
            let y1 = (g.iter().map(|b| b.y1).max().unwrap() + margin).min(frame.height);
            RectPx { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
        })
        .collect()
}
