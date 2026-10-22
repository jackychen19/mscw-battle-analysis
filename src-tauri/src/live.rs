//! Live capture: finds the game window, keeps the calibration, and turns
//! region captures into stats observations.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use battle_core::calibration::{
    Calibration, EXP_CHARS, ExpFontLearner, Missing, NEEDED_SAMPLES, Regions, Sample, SampleError, WindowMatch,
    find_exp_text, find_game_window, find_level_near, find_regions, missing_chars, teach_sample,
};
use battle_core::capture::{CaptureError, Frame, FrameSource, RectPt, WindowInfo};
use battle_core::parse::{ExpReader, exp_line_with};
use battle_core::stats::Observation;
use serde::Serialize;

use crate::capture::{self, Permission};

/// How often the window list is re-checked (window lost / reopened).
const SCAN_EVERY: Duration = Duration::from_secs(2);
const CALIBRATION_FILE: &str = "calibration.json";
/// How often to look for the EXP text and level again while they're not found.
const LOCATE_EVERY: Duration = Duration::from_secs(3);
/// Shares of the window height searched for the status bar, bottom up.
const LOCATE_BANDS: [f64; 4] = [0.125, 0.25, 0.5, 1.0];
/// Distinct samples without a solution before the boxes are looked for again.
const RELOCATE_AFTER: usize = 24;
/// At most one log line per this long about EXP boxes the learner can't use.
const REJECT_LOG_EVERY: Duration = Duration::from_secs(5);

/// What the panel needs to know about capture.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureStatus {
    pub permission: Permission,
    /// Title of the game window, when it's open.
    pub game: Option<String>,
    /// Calibrated for this window at its current scale.
    pub calibrated: bool,
    /// Automatic calibration in progress (game found, not calibrated yet).
    pub learning: Option<Learning>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Learning {
    /// The EXP text and level were found on screen.
    pub found: bool,
    /// Distinct EXP readings seen so far, and how many a solution needs.
    pub samples: usize,
    pub needed: usize,
}

/// Calibrating by itself: find the boxes, then learn the EXP digits.
#[derive(Default)]
struct Auto {
    regions: Option<Regions>,
    learner: ExpFontLearner,
    last_locate: Option<Instant>,
    last_reject_log: Option<Instant>,
}

/// Result of a capture tick while recording.
pub enum Tick {
    Observed(Observation),
    WindowLost,
}

pub struct Capturer {
    source: Box<dyn FrameSource + Send>,
    dir: PathBuf,
    calibration: Option<Calibration>,
    /// The window to look for: picked by the user, else the calibrated one.
    chosen: Option<WindowMatch>,
    game: Option<WindowInfo>,
    permission: Permission,
    last_scan: Option<Instant>,
    auto: Auto,
}

impl Capturer {
    /// `dir` holds the calibration file.
    pub fn new(dir: PathBuf) -> Self {
        let calibration = load(&dir);
        Self {
            source: capture::frame_source(),
            chosen: calibration.as_ref().map(|c| c.window.clone()),
            calibration,
            dir,
            game: None,
            permission: capture::permission(),
            last_scan: None,
            auto: Auto::default(),
        }
    }

    pub fn status(&self) -> CaptureStatus {
        CaptureStatus {
            permission: self.permission,
            game: self.game.as_ref().map(|g| g.title.clone()),
            calibrated: self.ready().is_some(),
            learning: (self.game.is_some() && self.ready().is_none()).then(|| Learning {
                found: self.auto.regions.is_some(),
                samples: self.auto.learner.sample_count(),
                needed: NEEDED_SAMPLES,
            }),
        }
    }

    pub fn game(&self) -> Option<&WindowInfo> {
        self.game.as_ref()
    }

    pub fn calibration(&self) -> Option<&Calibration> {
        self.calibration.as_ref()
    }

    /// Calibration usable for the current game window.
    fn ready(&self) -> Option<&Calibration> {
        let game = self.game.as_ref()?;
        self.calibration.as_ref().filter(|c| c.fits(game))
    }

    /// Re-checks the window list if it's due (or `force`).
    pub fn scan(&mut self, force: bool) {
        let now = Instant::now();
        if !force && self.last_scan.is_some_and(|t| now - t < SCAN_EVERY) {
            return;
        }
        self.last_scan = Some(now);
        self.permission = capture::permission();
        if self.permission == Permission::Denied {
            self.game = None;
            return;
        }
        let windows = match self.source.list_windows() {
            Ok(w) => w,
            Err(e) => return self.on_error(&e),
        };
        let Some(found) = find_game_window(&windows, self.chosen.as_ref()).cloned() else {
            let maple: Vec<String> = windows
                .iter()
                .filter(|w| format!("{} {}", w.owner, w.title).to_lowercase().contains("maple"))
                .map(|w| format!("{:?} / {:?} {}x{}", w.owner, w.title, w.bounds.w, w.bounds.h))
                .collect();
            eprintln!("[scan] no game window among {} windows; MapleStory-like: {maple:?}", windows.len());
            self.game = None;
            return;
        };
        // Re-select when it's a different window or it moved to another display.
        if self.game.as_ref() != Some(&found) {
            if self.game.as_ref().is_none_or(|g| g.id != found.id || g.bounds.w != found.bounds.w || g.bounds.h != found.bounds.h) {
                // Another window, or resized: the boxes may have moved.
                if let Some(g) = &self.game {
                    eprintln!(
                        "[scan] game window changed: id {} {}x{} -> id {} {}x{}; calibrating again",
                        g.id, g.bounds.w, g.bounds.h, found.id, found.bounds.w, found.bounds.h
                    );
                }
                self.auto = Auto::default();
            }
            self.game = self.source.select_window(found.id).map_err(|e| self.on_error(&e)).ok();
        }
    }

    pub fn list_windows(&mut self) -> Result<Vec<WindowInfo>, CaptureError> {
        self.source.list_windows()
    }

    pub fn choose(&mut self, id: u64) -> Result<WindowInfo, CaptureError> {
        let info = self.source.select_window(id)?;
        self.chosen = Some(WindowMatch::of(&info));
        self.game = Some(info.clone());
        Ok(info)
    }

    /// Reads the EXP line once; call only while recording.
    pub fn tick(&mut self) -> Option<Tick> {
        self.scan(false);
        if self.game.is_none() {
            return Some(Tick::WindowLost);
        }
        let cal = self.ready()?.clone();
        let exp = self.source.capture_region(cal.regions.exp_text);
        let level = self.source.capture_region(cal.regions.level);
        match (exp, level) {
            (Ok(exp), Ok(level)) => Some(Tick::Observed(
                cal.exp.read(&exp, &level).map_or(Observation::Unreadable, Observation::Exp),
            )),
            (Err(e), _) | (_, Err(e)) => {
                self.on_error(&e);
                Some(if self.game.is_none() { Tick::WindowLost } else { Tick::Observed(Observation::Unreadable) })
            }
        }
    }

    /// Calibrates by itself, one step per call: finds the EXP text and level,
    /// then learns the EXP digits from what the screen shows as EXP changes.
    /// Keeps learning after calibrating until every character is known.
    pub fn learn(&mut self) {
        let Some(game) = self.game.clone() else { return };
        let ready = self.ready().cloned();
        if ready.as_ref().is_some_and(|c| c.missing().exp.is_empty()) {
            return;
        }
        let regions = match ready.as_ref().map(|c| c.regions).or(self.auto.regions) {
            Some(r) => r,
            None => match self.locate(&game) {
                Some(r) => r,
                None => return,
            },
        };
        if self.auto.learner.ink() != regions.ink {
            // Samples split at another threshold don't group with these.
            self.auto.learner = ExpFontLearner::with_ink(regions.ink);
        }
        let Ok(crop) = self.capture(regions.exp_text) else { return };
        // The EXP font is built in: if the text reads by shape, that's the
        // whole calibration. Learning it from kills is only the fallback.
        let by_shape = ExpReader { ink: regions.ink, ..ExpReader::default() };
        if ready.is_none() && by_shape.read_exp(&crop).is_some() {
            let cal = Calibration::new(&game, regions, by_shape);
            if save(&self.dir, &cal).is_ok() {
                eprintln!("[learn] EXP reads by shape: calibrated");
                self.calibration = Some(cal);
                self.chosen = Some(WindowMatch::of(&game));
            }
            return;
        }
        let before = self.auto.learner.sample_count();
        if !self.auto.learner.add(&crop) {
            if self.auto.last_reject_log.is_none_or(|t| t.elapsed() >= REJECT_LOG_EVERY) {
                self.auto.last_reject_log = Some(Instant::now());
                let glyphs = exp_line_with(&crop, regions.ink).map(|l| l.glyphs);
                eprintln!("[learn] EXP box {}x{} px not usable (ink {:?}): glyphs {glyphs:?}", crop.width, crop.height, regions.ink);
            }
            return;
        }
        if self.auto.learner.sample_count() == before {
            return;
        }
        eprintln!("[learn] {} samples", self.auto.learner.sample_count());
        let Some(font) = self.auto.learner.solve() else {
            if self.auto.learner.sample_count() >= NEEDED_SAMPLES {
                let why = self.auto.learner.describe();
                eprintln!("[learn] {}", why.lines().last().unwrap_or_default());
            }
            if ready.is_none() && self.auto.learner.sample_count() >= RELOCATE_AFTER {
                self.auto = Auto::default();
            }
            return;
        };
        let known = |set: &battle_core::parse::GlyphSet| EXP_CHARS.len() - missing_chars(set, EXP_CHARS).len();
        if ready.as_ref().is_some_and(|c| known(&font) <= known(&c.exp.exp_glyphs)) {
            return;
        }
        // No level templates: the level's blocky digits are read by shape.
        let reader = ExpReader { exp_glyphs: font, ink: regions.ink, ..ExpReader::default() };
        let cal = Calibration::new(&game, regions, reader);
        if save(&self.dir, &cal).is_ok() {
            self.calibration = Some(cal);
            self.chosen = Some(WindowMatch::of(&game));
        }
    }

    /// Looks for the EXP text and level anywhere in the window, if due.
    /// Both must be found and the level must read.
    fn locate(&mut self, game: &WindowInfo) -> Option<Regions> {
        let now = Instant::now();
        if self.auto.last_locate.is_some_and(|t| now - t < LOCATE_EVERY) {
            return None;
        }
        self.auto.last_locate = Some(now);
        // The status bar is drawn at the bottom, so search upward from there in
        // growing bands. The window's reported size doesn't always match where
        // the game draws, so the last band is the whole window.
        let (area, strip) = LOCATE_BANDS.iter().find_map(|&share| {
            let h = game.bounds.h * share;
            let area = RectPt { x: 0.0, y: game.bounds.h - h, w: game.bounds.w, h };
            let strip = match self.capture(area) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[locate] capture of {area:?} failed: {e}");
                    return None;
                }
            };
            let exp = find_exp_text(&strip);
            let lvl = exp.and_then(|e| find_level_near(&strip, e));
            eprintln!(
                "[locate] window {}x{} pt @{}x, bottom {:.0}%: strip {}x{} px: exp {exp:?}, level {lvl:?}",
                game.bounds.w, game.bounds.h, game.scale, share * 100.0, strip.width, strip.height
            );
            if lvl.is_some() {
                return Some((area, strip));
            }
            if share == LOCATE_BANDS[0] {
                // Only the bottom of the window (where the status bar is drawn), never the whole frame.
                save_debug_crop("locate-bottom", &strip);
            }
            None
        })?;
        let regions = find_regions(&strip, area, game.scale)?;
        let level = self.capture(regions.level).ok()?;
        let read = ExpReader::default().read_level(&level);
        eprintln!("[locate] level reads {read:?}");
        read?;
        if let Ok(own) = self.capture(regions.exp_text) {
            let reads = ExpReader { ink: regions.ink, ..ExpReader::default() }.read_exp(&own);
            eprintln!("[locate] EXP box {}x{} px, ink {:?}: reads {reads:?} by shape", own.width, own.height, regions.ink);
        }
        self.auto.regions = Some(regions);
        Some(regions)
    }

    fn on_error(&mut self, e: &CaptureError) {
        match e {
            CaptureError::PermissionDenied => {
                self.permission = Permission::Denied;
                self.game = None;
            }
            CaptureError::WindowNotFound => self.game = None,
            CaptureError::NoFrame | CaptureError::Platform(_) => {}
        }
    }

    pub fn capture(&mut self, region: RectPt) -> Result<Frame, CaptureError> {
        let r = self.source.capture_region(region);
        if let Err(e) = &r {
            self.on_error(e);
        }
        r
    }

    /// The whole game window, one pixel per point.
    pub fn preview(&mut self) -> Result<Frame, CaptureError> {
        let game = self.game.clone().ok_or(CaptureError::WindowNotFound)?;
        let whole = RectPt { x: 0.0, y: 0.0, w: game.bounds.w, h: game.bounds.h };
        Ok(self.capture(whole)?.downsample(game.scale))
    }

    /// Teaches one sample and saves the calibration. Starts over when the
    /// window or scale changed since the last calibration.
    pub fn teach(&mut self, regions: Regions, sample: Sample) -> Result<Missing, String> {
        let game = self.game.clone().ok_or_else(|| CaptureError::WindowNotFound.to_string())?;
        let mut reader = self.ready().map(|c| c.exp.clone()).unwrap_or_default();
        let regions = teach_sample(&mut reader, regions, game.scale, sample, |r| self.capture(r))
            .map_err(|e: SampleError<CaptureError>| e.to_string())?;
        let cal = Calibration::new(&game, regions, reader);
        save(&self.dir, &cal)?;
        let missing = cal.missing();
        self.calibration = Some(cal);
        Ok(missing)
    }

    pub fn reset(&mut self) -> Result<(), String> {
        self.calibration = None;
        self.auto = Auto::default();
        match std::fs::remove_file(self.dir.join(CALIBRATION_FILE)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        }
    }
}

/// Off unless `BATTLE_DEBUG_CAPTURES=1`: saves a crop (never a full frame) to
/// `debug-captures/<name>-<w>x<h>.rgba` as raw RGBA, once per name and size.
fn save_debug_crop(name: &str, crop: &Frame) {
    if std::env::var_os("BATTLE_DEBUG_CAPTURES").is_none_or(|v| v != "1") {
        return;
    }
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../debug-captures");
    let path = dir.join(format!("{name}-{}x{}.rgba", crop.width, crop.height));
    if path.exists() {
        return;
    }
    let mut rgba = Vec::with_capacity((crop.width * crop.height * 4) as usize);
    for y in 0..crop.height as usize {
        rgba.extend_from_slice(&crop.rgba[y * crop.stride..y * crop.stride + crop.width as usize * 4]);
    }
    if std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(&path, rgba)).is_ok() {
        eprintln!("[debug] saved {}", path.display());
    }
}

fn load(dir: &std::path::Path) -> Option<Calibration> {
    let bytes = std::fs::read(dir.join(CALIBRATION_FILE)).ok()?;
    serde_json::from_slice::<Calibration>(&bytes)
        .ok()
        .filter(|c| c.version == battle_core::calibration::CALIBRATION_VERSION)
}

fn save(dir: &std::path::Path, cal: &Calibration) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_vec(cal).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(CALIBRATION_FILE), json).map_err(|e| e.to_string())
}
