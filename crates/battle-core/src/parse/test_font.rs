//! A tiny synthetic bitmap font for parser tests (7 rows: 5 body + 2 below).

use super::bitmap::Bitmap;

fn glyph(c: char) -> &'static [&'static str] {
    match c {
        '0' => &["###", "#.#", "#.#", "#.#", "###", "...", "..."],
        '1' => &[".#.", "##.", ".#.", ".#.", "###", "...", "..."],
        '2' => &["###", "..#", "###", "#..", "###", "...", "..."],
        '3' => &["###", "..#", "###", "..#", "###", "...", "..."],
        '4' => &["#.#", "#.#", "###", "..#", "..#", "...", "..."],
        '5' => &["###", "#..", "###", "..#", "###", "...", "..."],
        '6' => &["###", "#..", "###", "#.#", "###", "...", "..."],
        '7' => &["###", "..#", ".#.", ".#.", ".#.", "...", "..."],
        '8' => &["###", "#.#", "###", "#.#", "###", "...", "..."],
        '9' => &["###", "#.#", "###", "..#", "###", "...", "..."],
        ',' => &[".", ".", ".", ".", "#", "#", "."],
        '.' => &[".", ".", ".", ".", "#", ".", "."],
        '[' => &["##", "#.", "#.", "#.", "#.", "#.", "##"],
        ']' => &["##", ".#", ".#", ".#", ".#", ".#", "##"],
        '%' => &["#..#", "...#", "..#.", ".#..", "#...", "#..#", "...."],
        '(' => &[".#", "#.", "#.", "#.", "#.", "#.", ".#"],
        ')' => &["#.", ".#", ".#", ".#", ".#", ".#", "#."],
        '+' => &["...", ".#.", "###", ".#.", "...", "...", "..."],
        'E' => &["###", "#..", "##.", "#..", "###", "...", "..."],
        'X' => &["#.#", "#.#", ".#.", "#.#", "#.#", "...", "..."],
        'P' => &["##.", "#.#", "##.", "#..", "#..", "...", "..."],
        other => panic!("test font has no {other:?}"),
    }
}

/// Renders `text` with `gap` empty columns between glyphs; spaces become a
/// 2-column gap.
pub fn render(text: &str, gap: u32) -> Bitmap {
    let mut rows = vec![String::new(); 7];
    for (i, c) in text.chars().enumerate() {
        if i > 0 {
            rows.iter_mut().for_each(|r| r.push_str(&".".repeat(gap as usize)));
        }
        if c == ' ' {
            rows.iter_mut().for_each(|r| r.push_str(".."));
            continue;
        }
        for (row, g) in rows.iter_mut().zip(glyph(c)) {
            row.push_str(g);
        }
    }
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    Bitmap::from_ascii(&rows)
}

/// `render(text, 1)` as a captured RGBA crop: near-white text on a dark
/// background with a 1-pixel margin.
pub fn render_frame(text: &str) -> crate::capture::Frame {
    let b = render(text, 1);
    let (w, h) = (b.width() + 2, b.height() + 2);
    let mut rgba = [40u8, 40, 40, 255].repeat((w * h) as usize);
    for y in 0..b.height() {
        for x in 0..b.width() {
            if b.get(x, y) {
                let i = (((y + 1) * w + x + 1) * 4) as usize;
                rgba[i..i + 3].copy_from_slice(&[235, 235, 225]);
            }
        }
    }
    crate::capture::Frame { width: w, height: h, stride: w as usize * 4, rgba }
}
