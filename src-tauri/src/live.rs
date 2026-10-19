//! Live capture: finds the game window, keeps the calibration, and turns
//! region captures into stats observations.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use battle_core::calibration::{
    Calibration, Missing, Regions, Sample, SampleError, WindowMatch, find_game_window, teach_sample,
};
use battle_core::capture::{CaptureError, Frame, FrameSource, RectPt, WindowInfo};
use battle_core::stats::Observation;
use serde::Serialize;

use crate::capture::{self, Permission};

/// How often the window list is re-checked (window lost / reopened).
const SCAN_EVERY: Duration = Duration::from_secs(2);
const CALIBRATION_FILE: &str = "calibration.json";

/// What the panel needs to know about capture.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureStatus {
    pub permission: Permission,
    /// Title of the game window, when it's open.
    pub game: Option<String>,
    /// Calibrated for this window at its current scale.
    pub calibrated: bool,
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
        }
    }

    pub fn status(&self) -> CaptureStatus {
        CaptureStatus {
            permission: self.permission,
            game: self.game.as_ref().map(|g| g.title.clone()),
            calibrated: self.ready().is_some(),
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
        self.calibration.as_ref().filter(|c| c.window.matches(game) && c.fits_scale(game.scale))
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
            self.game = None;
            return;
        };
        // Re-select when it's a different window or it moved to another display.
        if self.game.as_ref() != Some(&found) {
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
        match std::fs::remove_file(self.dir.join(CALIBRATION_FILE)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        }
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
