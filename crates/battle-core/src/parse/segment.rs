use std::ops::Range;

use super::bitmap::Bitmap;

/// Runs of consecutive columns that contain ink — one per glyph when the
/// font leaves at least one empty column between characters.
pub fn column_runs(b: &Bitmap) -> Vec<Range<u32>> {
    let mut runs = Vec::new();
    let mut start = None;
    for x in 0..b.width() {
        match (b.column_has_ink(x), start) {
            (true, None) => start = Some(x),
            (false, Some(s)) => {
                runs.push(s..x);
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        runs.push(s..b.width());
    }
    runs
}

/// Runs of consecutive rows that contain ink: the text lines of a stack.
/// Gaps shorter than `min_gap` rows are bridged (e.g. the dot over an `i`).
pub fn row_runs(b: &Bitmap, min_gap: u32) -> Vec<Range<u32>> {
    let mut runs: Vec<Range<u32>> = Vec::new();
    for y in 0..b.height() {
        if !b.row_has_ink(y) {
            continue;
        }
        match runs.last_mut() {
            Some(last) if y - last.end < min_gap => last.end = y + 1,
            _ => runs.push(y..y + 1),
        }
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_columns_on_gaps() {
        let b = Bitmap::from_ascii(&["#.##..#", "#..#..#"]);
        assert_eq!(column_runs(&b), vec![0..1, 2..4, 6..7]);
        assert_eq!(column_runs(&Bitmap::new(3, 1)), Vec::<Range<u32>>::new());
    }

    #[test]
    fn splits_rows_bridging_small_gaps() {
        let b = Bitmap::from_ascii(&["#", ".", "#", "#", ".", ".", ".", "#"]);
        assert_eq!(row_runs(&b, 2), vec![0..4, 7..8]);
        assert_eq!(row_runs(&b, 1), vec![0..1, 2..4, 7..8]);
    }
}
