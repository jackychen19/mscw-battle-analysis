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

/// `render(text, 1)` as a grayscale crop: bright text on a dark background
/// with a 1-pixel margin, the way a captured region looks.
pub fn render_gray(text: &str) -> super::gray::Gray {
    let b = render(text, 1);
    let (w, h) = (b.width() + 2, b.height() + 2);
    let mut data = vec![40u8; (w * h) as usize];
    for y in 0..b.height() {
        for x in 0..b.width() {
            if b.get(x, y) {
                data[((y + 1) * w + x + 1) as usize] = 230;
            }
        }
    }
    super::gray::Gray::new(w, h, data)
}
