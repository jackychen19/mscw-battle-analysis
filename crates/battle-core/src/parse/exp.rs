use serde::{Deserialize, Serialize};

use super::glyph::{DEFAULT_MIN_SCORE, GlyphSet, TeachError, line_band};
use super::gray::Gray;
use crate::stats::ExpReading;

/// Reads the EXP line (`419,165[36.54%]`, label excluded) and the level.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpReader {
    pub exp_glyphs: GlyphSet,
    pub level_glyphs: GlyphSet,
    pub min_score: f32,
}

impl Default for ExpReader {
    fn default() -> Self {
        Self { exp_glyphs: GlyphSet::default(), level_glyphs: GlyphSet::default(), min_score: DEFAULT_MIN_SCORE }
    }
}

impl ExpReader {
    /// Calibration: the EXP crop currently shows `exp` and `pct_hundredths`.
    /// Returns the column where the number starts (anything left of it, like
    /// the `EXP.` label, should be cropped off from then on).
    pub fn teach_exp(&mut self, crop: &Gray, exp: u64, pct_hundredths: u32) -> Result<u32, TeachError> {
        let band = line_band(&binarize(crop)).ok_or(TeachError::NoInk)?;
        self.exp_glyphs.teach(&band, &format_exp_text(exp, pct_hundredths))
    }

    /// Calibration: the level crop currently shows `level`.
    pub fn teach_level(&mut self, crop: &Gray, level: u32) -> Result<u32, TeachError> {
        let band = line_band(&binarize(crop)).ok_or(TeachError::NoInk)?;
        self.level_glyphs.teach(&band, &level.to_string())
    }

    /// `None` when the crop can't be read confidently (menus, tooltips,
    /// loading screens, or a malformed result).
    pub fn read(&self, exp_crop: &Gray, level_crop: &Gray) -> Option<ExpReading> {
        let (exp, pct_hundredths) = self.read_exp(exp_crop)?;
        let level = self.read_level(level_crop)?;
        Some(ExpReading { level, exp, pct_hundredths })
    }

    pub fn read_exp(&self, crop: &Gray) -> Option<(u64, u32)> {
        let band = line_band(&binarize(crop))?;
        parse_exp_text(&self.exp_glyphs.read(&band, self.min_score)?.text)
    }

    pub fn read_level(&self, crop: &Gray) -> Option<u32> {
        let band = line_band(&binarize(crop))?;
        parse_level_text(&self.level_glyphs.read(&band, self.min_score)?.text)
    }
}

fn binarize(crop: &Gray) -> super::Bitmap {
    crop.binarize(crop.otsu_threshold())
}

/// `440834, 3843` → `440,834[38.43%]`, as the game draws it (without spaces).
pub fn format_exp_text(exp: u64, pct_hundredths: u32) -> String {
    let digits = exp.to_string();
    let mut grouped = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(c);
    }
    format!("{grouped}[{}.{:02}%]", pct_hundredths / 100, pct_hundredths % 100)
}

/// Strict parse of `419,165[36.54%]`: correct comma grouping, two decimals,
/// percent below 100. Anything else is treated as a misread.
pub fn parse_exp_text(s: &str) -> Option<(u64, u32)> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    let (number, rest) = s.split_once('[')?;
    let pct = rest.strip_suffix("%]")?;

    let groups: Vec<&str> = number.split(',').collect();
    let well_grouped = groups.iter().enumerate().all(|(i, g)| {
        let len_ok = if i == 0 { (1..=3).contains(&g.len()) } else { g.len() == 3 };
        len_ok && g.bytes().all(|b| b.is_ascii_digit())
    });
    if !well_grouped || (groups.len() > 1 && groups[0] == "0") {
        return None;
    }
    let exp: u64 = groups.concat().parse().ok()?;

    let (whole, frac) = pct.split_once('.')?;
    let pct_ok = (1..=2).contains(&whole.len()) && frac.len() == 2;
    if !pct_ok || !whole.bytes().chain(frac.bytes()).all(|b| b.is_ascii_digit()) {
        return None;
    }
    let pct_hundredths = whole.parse::<u32>().ok()? * 100 + frac.parse::<u32>().ok()?;
    (pct_hundredths < 10_000).then_some((exp, pct_hundredths))
}

pub fn parse_level_text(s: &str) -> Option<u32> {
    if s.is_empty() || s.len() > 3 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok().filter(|l| (1..=250).contains(l))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::test_font::render_gray;

    #[test]
    fn formats_like_the_game() {
        assert_eq!(format_exp_text(440_834, 3843), "440,834[38.43%]");
        assert_eq!(format_exp_text(1_234_567, 5), "1,234,567[0.05%]");
        assert_eq!(format_exp_text(12, 1000), "12[10.00%]");
    }

    #[test]
    fn parses_valid_text() {
        assert_eq!(parse_exp_text("419,165[36.54%]"), Some((419_165, 3654)));
        assert_eq!(parse_exp_text("0[0.00%]"), Some((0, 0)));
        assert_eq!(parse_exp_text("1,234,567 [99.99%]"), Some((1_234_567, 9999)));
    }

    #[test]
    fn rejects_misreads() {
        for bad in [
            "419,16[36.54%]",   // bad grouping
            "41,9165[36.54%]",  // bad grouping
            "419165[36.54%]",   // missing comma
            "0,165[36.54%]",    // leading zero group
            "419,165[36.5%]",   // one decimal
            "419,165[100.00%]", // ≥ 100%
            "419,165[36.54%",   // truncated
            "419,165(36.54%]",
            "",
        ] {
            assert_eq!(parse_exp_text(bad), None, "{bad}");
        }
        assert_eq!(parse_level_text("59"), Some(59));
        assert_eq!(parse_level_text("0"), None);
        assert_eq!(parse_level_text("5,9"), None);
        assert_eq!(parse_level_text("1000"), None);
    }

    #[test]
    fn teach_then_read_a_reading() {
        let mut reader = ExpReader::default();
        let start = reader.teach_exp(&render_gray("EXP. 440,834[38.43%]"), 440_834, 3843).unwrap();
        assert!(start > 0, "label skipped");
        reader.teach_exp(&render_gray("765,192[45.67%]"), 765_192, 4567).unwrap();
        reader.teach_level(&render_gray("59"), 59).unwrap();
        reader.teach_level(&render_gray("60"), 60).unwrap();

        let r = reader.read(&render_gray("441,912[38.52%]"), &render_gray("65"));
        assert_eq!(r, Some(ExpReading::new(65, 441_912, 3852)));
    }

    #[test]
    fn unreadable_crops_give_none() {
        let mut reader = ExpReader::default();
        reader.teach_exp(&render_gray("440,834[38.43%]"), 440_834, 3843).unwrap();
        reader.teach_level(&render_gray("59"), 59).unwrap();
        let blank = Gray::new(20, 9, vec![40; 180]);
        assert_eq!(reader.read_exp(&blank), None);
        // Readable glyphs, malformed number.
        assert_eq!(reader.read_exp(&render_gray("44,0834[38.43%]")), None);
        assert_eq!(reader.read(&render_gray("440,834[38.43%]"), &blank), None);
    }
}
