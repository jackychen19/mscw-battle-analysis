use std::ops::Range;

use serde::{Deserialize, Serialize};

/// A binarized image; `true` pixels are text ("ink").
///
/// Serialized as rows of `#` (ink) and `.` so calibration files stay readable.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "BitmapRows", try_from = "BitmapRows")]
pub struct Bitmap {
    width: u32,
    height: u32,
    bits: Vec<bool>,
}

impl Bitmap {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height, bits: vec![false; width as usize * height as usize] }
    }

    /// Builds a bitmap from rows like `".##."`; any char other than `.` or
    /// space is ink. Rows shorter than the longest are padded.
    pub fn from_ascii(rows: &[&str]) -> Self {
        let width = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0) as u32;
        let mut b = Self::new(width, rows.len() as u32);
        for (y, row) in rows.iter().enumerate() {
            for (x, c) in row.chars().enumerate() {
                b.set(x as u32, y as u32, c != '.' && c != ' ');
            }
        }
        b
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn get(&self, x: u32, y: u32) -> bool {
        x < self.width && y < self.height && self.bits[(y * self.width + x) as usize]
    }

    pub fn set(&mut self, x: u32, y: u32, ink: bool) {
        let i = (y * self.width + x) as usize;
        self.bits[i] = ink;
    }

    pub fn ink_count(&self) -> usize {
        self.bits.iter().filter(|&&b| b).count()
    }

    pub fn column_has_ink(&self, x: u32) -> bool {
        (0..self.height).any(|y| self.get(x, y))
    }

    pub fn row_has_ink(&self, y: u32) -> bool {
        (0..self.width).any(|x| self.get(x, y))
    }

    /// Rows from the first to the last one containing ink.
    pub fn ink_rows(&self) -> Option<Range<u32>> {
        let first = (0..self.height).find(|&y| self.row_has_ink(y))?;
        let last = (0..self.height).rev().find(|&y| self.row_has_ink(y))?;
        Some(first..last + 1)
    }

    /// Sub-bitmap; parts outside the source are empty.
    pub fn crop(&self, x: u32, y: u32, width: u32, height: u32) -> Bitmap {
        let mut out = Bitmap::new(width, height);
        for dy in 0..height {
            for dx in 0..width {
                if self.get(x + dx, y + dy) {
                    out.set(dx, dy, true);
                }
            }
        }
        out
    }

    pub fn crop_columns(&self, cols: Range<u32>) -> Bitmap {
        self.crop(cols.start, 0, cols.end - cols.start, self.height)
    }

    pub fn to_ascii(&self) -> Vec<String> {
        (0..self.height)
            .map(|y| (0..self.width).map(|x| if self.get(x, y) { '#' } else { '.' }).collect())
            .collect()
    }
}

#[derive(Serialize, Deserialize)]
struct BitmapRows {
    width: u32,
    height: u32,
    rows: Vec<String>,
}

impl From<Bitmap> for BitmapRows {
    fn from(b: Bitmap) -> Self {
        Self { width: b.width, height: b.height, rows: b.to_ascii() }
    }
}

impl TryFrom<BitmapRows> for Bitmap {
    type Error = String;

    fn try_from(r: BitmapRows) -> Result<Self, Self::Error> {
        let fits = r.rows.len() == r.height as usize && r.rows.iter().all(|row| row.chars().count() == r.width as usize);
        if !fits {
            return Err(format!("bitmap rows do not match {}x{}", r.width, r.height));
        }
        let rows: Vec<&str> = r.rows.iter().map(String::as_str).collect();
        let mut b = Bitmap::from_ascii(&rows);
        b.width = r.width;
        b.height = r.height;
        b.bits.resize(r.width as usize * r.height as usize, false);
        Ok(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_round_trip_and_queries() {
        let b = Bitmap::from_ascii(&["....", ".##.", "..#.", "...."]);
        assert_eq!((b.width(), b.height(), b.ink_count()), (4, 4, 3));
        assert!(b.get(1, 1) && !b.get(0, 0) && !b.get(9, 9));
        assert_eq!(b.ink_rows(), Some(1..3));
        assert!(b.column_has_ink(2) && !b.column_has_ink(3));
        assert_eq!(b.crop_columns(1..3).to_ascii(), vec!["..", "##", ".#", ".."]);
        assert_eq!(Bitmap::new(3, 2).ink_rows(), None);
    }

    #[test]
    fn serializes_as_rows() {
        let b = Bitmap::from_ascii(&["#.", ".#"]);
        let json = serde_json::to_string(&b).unwrap();
        assert_eq!(json, r##"{"width":2,"height":2,"rows":["#.",".#"]}"##);
        assert_eq!(serde_json::from_str::<Bitmap>(&json).unwrap(), b);
        assert!(serde_json::from_str::<Bitmap>(r##"{"width":3,"height":1,"rows":["#."]}"##).is_err());
    }
}
