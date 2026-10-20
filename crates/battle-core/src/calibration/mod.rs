//! Calibration data: which window, where the text regions are, and the glyph
//! templates learned from the user's own screen. The app stores it as a small
//! local file; this module only defines the format and the pure helpers.

use serde::{Deserialize, Serialize};

mod learn;
mod locate;
pub use locate::{find_exp_text, find_level, find_level_near, find_regions};
pub use learn::{ExpFontLearner, NEEDED_SAMPLES};

use std::fmt;

use crate::capture::{Frame, RectPt, WindowInfo};
use crate::parse::{ExpInk, ExpReader, GlyphSet, TeachError};

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
    /// Window size (points) the regions were found at; the status bar moves
    /// when the window is resized.
    #[serde(default)]
    pub window_size: Option<[f64; 2]>,
    pub regions: Regions,
    pub exp: ExpReader,
}

impl Calibration {
    pub fn new(window: &WindowInfo, regions: Regions, exp: ExpReader) -> Self {
        Self {
            version: CALIBRATION_VERSION,
            window: WindowMatch::of(window),
            scale: window.scale,
            window_size: Some([window.bounds.w, window.bounds.h]),
            regions,
            exp,
        }
    }

    /// Made for this window at its current size and scale.
    pub fn fits(&self, window: &WindowInfo) -> bool {
        let same_size = self.window_size.is_none_or(|[w, h]| (w - window.bounds.w).abs() < 1.0 && (h - window.bounds.h).abs() < 1.0);
        self.window.matches(window) && self.fits_scale(window.scale) && same_size
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
    /// How sharp the EXP text is in this window (set by the status bar search).
    #[serde(default)]
    pub ink: ExpInk,
}

/// Smaller than this (points) is a launcher or dialog, not the game.
const MIN_GAME_SIZE: (f64, f64) = (640.0, 360.0);

/// The game window among `windows`: the saved one if it's open, otherwise a
/// window whose app or title mentions MapleStory. Launchers (the Nexon
/// launcher has several, one screen-sized and hidden) and small dialogs are
/// skipped; visible windows beat hidden ones, then larger beats smaller.
pub fn find_game_window<'a>(windows: &'a [WindowInfo], saved: Option<&WindowMatch>) -> Option<&'a WindowInfo> {
    let best = |it: &mut dyn Iterator<Item = &'a WindowInfo>| {
        it.max_by(|a, b| (a.on_screen, a.bounds.w * a.bounds.h).partial_cmp(&(b.on_screen, b.bounds.w * b.bounds.h)).unwrap())
    };
    if let Some(saved) = saved
        && let Some(w) = best(&mut windows.iter().filter(|w| saved.matches(w)))
    {
        return Some(w);
    }
    let mentions = |w: &WindowInfo, word: &str| format!("{} {}", w.owner, w.title).to_lowercase().contains(word);
    let big = |w: &WindowInfo| w.bounds.w >= MIN_GAME_SIZE.0 && w.bounds.h >= MIN_GAME_SIZE.1;
    best(&mut windows.iter().filter(|w| mentions(w, "maplestory") && !mentions(w, "launcher") && big(w)))
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

/// What the user typed for the current screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sample {
    pub level: u32,
    pub exp: u64,
    pub pct_hundredths: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SampleError<E> {
    Capture(E),
    Exp(TeachError),
    Level(TeachError),
    /// Taught, but reading the regions back didn't give the typed values.
    ReadBack,
}

impl<E: fmt::Display> fmt::Display for SampleError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Capture(e) => write!(f, "{e}"),
            Self::Exp(e) => write!(f, "EXP box: {e}"),
            Self::Level(e) => write!(f, "level box: {e}"),
            Self::ReadBack => write!(f, "the boxes don't read back as the values you typed; check the boxes and values"),
        }
    }
}

/// Teaches `reader` the glyphs on screen now. `capture` grabs a region of the
/// game window (points) at `scale`. The EXP box is trimmed past any label,
/// re-captured, and both boxes must read back as `sample`; on any error
/// `reader` is left untouched. Returns the regions to use from now on.
pub fn teach_sample<E>(
    reader: &mut ExpReader,
    regions: Regions,
    scale: f64,
    sample: Sample,
    mut capture: impl FnMut(RectPt) -> Result<Frame, E>,
) -> Result<Regions, SampleError<E>> {
    let mut grab = |r: RectPt| capture(r).map_err(SampleError::Capture);
    // A scratch reader only to find where the number starts.
    let start = ExpReader::default()
        .teach_exp(&grab(regions.exp_text)?, sample.exp, sample.pct_hundredths)
        .map_err(SampleError::Exp)?;
    let regions = Regions { exp_text: trim_left(regions.exp_text, start, scale), ..regions };
    let (exp_crop, level_crop) = (grab(regions.exp_text)?, grab(regions.level)?);

    let mut taught = reader.clone();
    taught.teach_exp(&exp_crop, sample.exp, sample.pct_hundredths).map_err(SampleError::Exp)?;
    taught.teach_level(&level_crop, sample.level).map_err(SampleError::Level)?;
    let read = taught.read(&exp_crop, &level_crop);
    if read.map(|r| (r.level, r.exp, r.pct_hundredths)) != Some((sample.level, sample.exp, sample.pct_hundredths)) {
        return Err(SampleError::ReadBack);
    }
    *reader = taught;
    Ok(regions)
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
            on_screen: true,
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
        // The Nexon Launcher is titled MapleStory too, but small.
        let launcher = [window(4, "Nexon Launcher", "MapleStory", 800.0, 300.0)];
        assert_eq!(find_game_window(&launcher, None), None);
    }

    #[test]
    fn picks_the_game_over_the_launchers() {
        // The titled windows on a real Mac with the game open (2026-10-19).
        let hidden = |w: WindowInfo| WindowInfo { on_screen: false, ..w };
        let windows = [
            hidden(window(8130, "MapleStory Launcher", "Nexon Global Launcher", 1920.0, 1022.0)),
            window(8131, "MapleStory Launcher", "NexonLauncher", 800.0, 300.0),
            window(12914, "MapleStory Classic World", "MapleStory", 1366.0, 796.0),
        ];
        assert_eq!(find_game_window(&windows, None).map(|w| w.id), Some(12914));
        // Fullscreen in another Space it's hidden, but still the game.
        let away = [windows[0].clone(), windows[1].clone(), hidden(windows[2].clone())];
        assert_eq!(find_game_window(&away, None).map(|w| w.id), Some(12914));
    }

    #[test]
    fn lists_untaught_characters() {
        let mut exp = ExpReader::default();
        exp.teach_exp(&render_frame("440,834[38.43%]"), 440_834, 3843).unwrap();
        exp.teach_level(&render_frame("59"), 59).unwrap();
        let regions = Regions {
            exp_text: RectPt { x: 0.0, y: 0.0, w: 1.0, h: 1.0 },
            level: RectPt { x: 0.0, y: 0.0, w: 1.0, h: 1.0 },
            ink: ExpInk::Otsu,
        };
        let cal = Calibration::new(&window(1, "MapleStory", "MapleStory", 1.0, 1.0), regions, exp);
        assert_eq!(cal.missing(), Missing { exp: "125679".into(), level: "01234678".into() });
        assert!(cal.fits_scale(2.0));
        assert!(!cal.fits_scale(1.0));
        let game = window(1, "MapleStory", "MapleStory", 1.0, 1.0);
        assert!(cal.fits(&game));
        assert!(!cal.fits(&WindowInfo { bounds: RectPt { w: 1366.0, ..game.bounds }, ..game }));
    }

    #[test]
    fn trims_the_label_at_any_scale() {
        let r = RectPt { x: 100.0, y: 50.0, w: 80.0, h: 8.0 };
        assert_eq!(trim_left(r, 21, 2.0), RectPt { x: 110.0, y: 50.0, w: 70.0, h: 8.0 });
        assert_eq!(trim_left(r, 26, 1.25), RectPt { x: 120.0, y: 50.0, w: 60.0, h: 8.0 });
        assert_eq!(trim_left(r, 0, 2.0), r);
        assert_eq!(trim_left(r, 10_000, 1.0).w, 0.0);
    }

    /// A fake game window at `scale`: the EXP line (label included) at
    /// y = 0 and the level at y = 20, both in points.
    fn screen(exp: &str, level: &str, scale: f64) -> impl FnMut(RectPt) -> Result<Frame, String> {
        let (exp, level) = (render_frame(exp), render_frame(level));
        move |r: RectPt| {
            let (src, top) = if r.y < 20.0 { (&exp, 0.0) } else { (&level, 20.0) };
            let px = RectPt { y: r.y - top, ..r }.to_px(1.0);
            let crop = src.crop(px);
            // Upscale by repeating pixels, as a Retina capture would.
            let (w, h) = ((f64::from(crop.width) * scale) as u32, (f64::from(crop.height) * scale) as u32);
            let mut rgba = Vec::new();
            for y in 0..h {
                for x in 0..w {
                    rgba.extend_from_slice(&crop.pixel((f64::from(x) / scale) as u32, (f64::from(y) / scale) as u32));
                }
            }
            Ok(Frame { width: w, height: h, stride: w as usize * 4, rgba })
        }
    }

    const BOXES: Regions = Regions {
        exp_text: RectPt { x: 0.0, y: 0.0, w: 200.0, h: 9.0 },
        level: RectPt { x: 0.0, y: 20.0, w: 50.0, h: 9.0 },
        ink: ExpInk::Otsu,
    };

    #[test]
    fn teaching_trims_the_label_and_reads_back() {
        for scale in [1.0, 2.0] {
            let mut reader = ExpReader::default();
            let sample = Sample { level: 59, exp: 440_834, pct_hundredths: 3843 };
            let regions = teach_sample(&mut reader, BOXES, scale, sample, screen("EXP. 440,834[38.43%]", "59", scale)).unwrap();
            assert!(regions.exp_text.x > 10.0, "label trimmed at {scale}x: {:?}", regions.exp_text);
            assert_eq!(regions.level, BOXES.level);

            // Teaching again from the trimmed box keeps it where it is.
            let sample = Sample { level: 59, exp: 765_192, pct_hundredths: 4567 };
            let again = teach_sample(&mut reader, regions, scale, sample, screen("EXP. 765,192[45.67%]", "59", scale)).unwrap();
            assert!((again.exp_text.x - regions.exp_text.x).abs() <= 1.0 / scale, "{again:?} vs {regions:?}");
        }
    }

    #[test]
    fn a_wrong_value_teaches_nothing() {
        let mut reader = ExpReader::default();
        let typo = Sample { level: 59, exp: 44_083, pct_hundredths: 3843 };
        let err = teach_sample(&mut reader, BOXES, 1.0, typo, screen("440,834[38.43%]", "59", 1.0)).unwrap_err();
        assert!(matches!(err, SampleError::ReadBack | SampleError::Exp(_)), "{err:?}");
        assert_eq!(reader, ExpReader::default());

        let err = teach_sample(&mut reader, BOXES, 1.0, typo, |_| Err::<Frame, _>("window gone")).unwrap_err();
        assert_eq!(err, SampleError::Capture("window gone"));
    }
}
