//! Calibration data: which window, where the text regions are, and the glyph
//! templates learned from the user's own screen. The app stores it as a small
//! local file; this module only defines the format and the pure helpers.

use serde::{Deserialize, Serialize};

use crate::capture::{RectPt, WindowInfo};
use crate::parse::{ExpReader, GlyphSet};

pub const CALIBRATION_VERSION: u32 = 2;

/// Every character the EXP line can show.
pub const EXP_CHARS: &str = "0123456789,.[]%";
/// Every character the level can show.
pub const LEVEL_CHARS: &str = "0123456789";

/// Two scale factors closer than this are the same display setting.
const SCALE_TOLERANCE: f64 = 0.01;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Calibration {
    pub version: u32,
    pub window: WindowMatch,
    /// Scale factor the templates were captured at; they're only valid there.
    pub scale: f64,
    pub regions: Regions,
    pub exp: ExpReader,
}

impl Calibration {
    pub fn new(window: &WindowInfo, regions: Regions, exp: ExpReader) -> Self {
        Self { version: CALIBRATION_VERSION, window: WindowMatch::of(window), scale: window.scale, regions, exp }
    }

    /// Templates match only at the scale they were learned at.
    pub fn fits_scale(&self, scale: f64) -> bool {
        (self.scale - scale).abs() < SCALE_TOLERANCE
    }

    /// Characters the EXP and level readers haven't been taught yet.
    pub fn missing(&self) -> Missing {
        Missing { exp: missing_chars(&self.exp.exp_glyphs, EXP_CHARS), level: missing_chars(&self.exp.level_glyphs, LEVEL_CHARS) }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Missing {
    pub exp: String,
    pub level: String,
}

/// How to find the game window again on next launch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowMatch {
    pub title: String,
    pub owner: String,
}

impl WindowMatch {
    pub fn of(w: &WindowInfo) -> Self {
        Self { title: w.title.clone(), owner: w.owner.clone() }
    }

    pub fn matches(&self, w: &WindowInfo) -> bool {
        self.owner == w.owner && self.title == w.title
    }
}

/// Text regions, in points relative to the top-left of the game window as
/// the platform captures it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Regions {
    pub exp_text: RectPt,
    pub level: RectPt,
}

/// The game window among `windows`: the saved one if it's open, otherwise the
/// largest window whose app or title mentions MapleStory.
pub fn find_game_window<'a>(windows: &'a [WindowInfo], saved: Option<&WindowMatch>) -> Option<&'a WindowInfo> {
    let largest = |it: &mut dyn Iterator<Item = &'a WindowInfo>| {
        it.max_by(|a, b| (a.bounds.w * a.bounds.h).total_cmp(&(b.bounds.w * b.bounds.h)))
    };
    if let Some(saved) = saved
        && let Some(w) = largest(&mut windows.iter().filter(|w| saved.matches(w)))
    {
        return Some(w);
    }
    let maple = |s: &str| s.to_lowercase().contains("maplestory");
    largest(&mut windows.iter().filter(|w| maple(&w.owner) || maple(&w.title)))
}

/// The characters of `chars` that `set` has no template for.
pub fn missing_chars(set: &GlyphSet, chars: &str) -> String {
    chars.chars().filter(|&c| !set.glyphs.iter().any(|g| g.ch == c)).collect()
}

/// Moves the region's left edge to `start_px`, a column of a crop of it taken
/// at `scale`, so a label left of the number (`EXP.`) is no longer captured.
/// Keeps one pixel of margin.
pub fn trim_left(region: RectPt, start_px: u32, scale: f64) -> RectPt {
    let dx = (f64::from(start_px.saturating_sub(1)) / scale).min(region.w);
    RectPt { x: region.x + dx, w: region.w - dx, ..region }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::test_font::render_frame;

    fn window(id: u64, owner: &str, title: &str, w: f64, h: f64) -> WindowInfo {
        WindowInfo {
            id,
            title: title.into(),
            owner: owner.into(),
            bounds: RectPt { x: 0.0, y: 0.0, w, h },
            scale: 2.0,
        }
    }

    #[test]
    fn finds_the_saved_window_first() {
        let windows = [
            window(1, "MapleStory", "MapleStory", 1366.0, 768.0),
            window(2, "QuickTime Player", "grinding.mov", 800.0, 600.0),
        ];
        let saved = WindowMatch { title: "grinding.mov".into(), owner: "QuickTime Player".into() };
        assert_eq!(find_game_window(&windows, Some(&saved)).map(|w| w.id), Some(2));
    }

    #[test]
    fn falls_back_to_the_largest_maplestory_window() {
        let windows = [
            window(1, "Finder", "MapleStory notes", 300.0, 200.0),
            window(2, "MapleStory", "MapleStory", 1366.0, 768.0),
            window(3, "Safari", "Wiki", 1600.0, 900.0),
        ];
        let gone = WindowMatch { title: "x".into(), owner: "y".into() };
        assert_eq!(find_game_window(&windows, Some(&gone)).map(|w| w.id), Some(2));
        assert_eq!(find_game_window(&windows, None).map(|w| w.id), Some(2));
        assert_eq!(find_game_window(&windows[2..], None), None);
    }

    #[test]
    fn lists_untaught_characters() {
        let mut exp = ExpReader::default();
        exp.teach_exp(&render_frame("440,834[38.43%]"), 440_834, 3843).unwrap();
        exp.teach_level(&render_frame("59"), 59).unwrap();
        let regions = Regions { exp_text: RectPt { x: 0.0, y: 0.0, w: 1.0, h: 1.0 }, level: RectPt { x: 0.0, y: 0.0, w: 1.0, h: 1.0 } };
        let cal = Calibration::new(&window(1, "MapleStory", "MapleStory", 1.0, 1.0), regions, exp);
        assert_eq!(cal.missing(), Missing { exp: "125679".into(), level: "01234678".into() });
        assert!(cal.fits_scale(2.0));
        assert!(!cal.fits_scale(1.0));
    }

    #[test]
    fn trims_the_label_at_any_scale() {
        let r = RectPt { x: 100.0, y: 50.0, w: 80.0, h: 8.0 };
        assert_eq!(trim_left(r, 21, 2.0), RectPt { x: 110.0, y: 50.0, w: 70.0, h: 8.0 });
        assert_eq!(trim_left(r, 26, 1.25), RectPt { x: 120.0, y: 50.0, w: 60.0, h: 8.0 });
        assert_eq!(trim_left(r, 0, 2.0), r);
        assert_eq!(trim_left(r, 10_000, 1.0).w, 0.0);
    }
}
