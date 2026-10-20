use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use battle_core::calibration::{Missing, Regions, Sample};
use battle_core::capture::{Frame, RectPt, WindowInfo};
use battle_core::stats::{Command, Session, SessionConfig, SessionState, Snapshot};
use serde::Serialize;
use tauri::ipc::Response;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::capture::{self, Permission};
use crate::history::{self, SavedSession};
use crate::live::{CaptureStatus, Capturer, Tick};
use crate::mock_feed::MockFeed;

pub struct AppState {
    origin: Instant,
    /// Unix time (ms) of `origin`, to date saved sessions.
    origin_unix_ms: u64,
    /// Where finished sessions are saved; set once the app's data dir is known.
    history_dir: Option<PathBuf>,
    session: Session,
    last_session: Option<Snapshot>,
    /// Synthetic feed instead of the game (`BATTLE_MOCK=1`).
    feed: Option<MockFeed>,
    capture: CaptureStatus,
    /// Paused by us because the game window went away.
    auto_paused: bool,
}

/// What the panel renders.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    snapshot: Snapshot,
    /// Results of the previous session, shown while idle.
    last_session: Option<Snapshot>,
    /// True while the numbers come from the synthetic feed.
    mock: bool,
    capture: CaptureStatus,
    /// Start is possible: game found and calibrated (always, with the mock feed).
    can_start: bool,
    auto_paused: bool,
}

impl AppState {
    pub fn new(mock: bool) -> Self {
        Self {
            origin: Instant::now(),
            origin_unix_ms: SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64),
            history_dir: None,
            session: Session::new(SessionConfig::default()),
            last_session: None,
            feed: mock.then(MockFeed::new),
            capture: CaptureStatus { permission: capture::permission(), game: None, calibrated: false, learning: None },
            auto_paused: false,
        }
    }

    pub fn set_history_dir(&mut self, dir: PathBuf) {
        self.history_dir = Some(dir);
    }

    /// Saves the stopped session to history. Mock-feed sessions aren't saved.
    fn save_session(&self) {
        let (Some(dir), Some(record), None) = (&self.history_dir, self.session.record(), &self.feed) else { return };
        let saved = SavedSession::new(record, |t| self.origin_unix_ms + t);
        match history::save(dir, &saved) {
            Ok(path) => eprintln!("[history] saved {}", path.display()),
            Err(e) => eprintln!("[history] couldn't save the session: {e}"),
        }
    }

    fn now_ms(&self) -> u64 {
        self.origin.elapsed().as_millis() as u64
    }

    fn view(&self) -> View {
        let mock = self.feed.is_some();
        View {
            snapshot: self.session.snapshot(self.now_ms()),
            last_session: self.last_session.clone(),
            mock,
            capture: self.capture.clone(),
            can_start: mock || (self.capture.game.is_some() && self.capture.calibrated),
            auto_paused: self.auto_paused,
        }
    }

    pub fn recording(&self) -> bool {
        self.session.state() == SessionState::Recording
    }

    pub fn tick_mock(&mut self) {
        let t = self.now_ms();
        if let Some(feed) = &mut self.feed {
            feed.tick(t, &mut self.session);
        }
    }

    /// Applies one live capture result.
    pub fn on_capture(&mut self, status: CaptureStatus, tick: Option<Tick>) {
        self.capture = status;
        let t = self.now_ms();
        match tick {
            Some(Tick::Observed(o)) if self.recording() => self.session.observe(t, o),
            Some(Tick::WindowLost) if self.recording() => {
                self.auto_paused = self.session.apply(t, Command::Pause).is_ok();
            }
            _ => {}
        }
    }

    fn apply(&mut self, command: Command) -> Result<(), String> {
        let t = self.now_ms();
        if command == Command::Reset && self.session.state() == SessionState::Stopped {
            self.last_session = Some(self.session.snapshot(t));
        }
        self.session.apply(t, command).map_err(|e| e.to_string())?;
        self.auto_paused = false;
        if command == Command::Stop {
            self.save_session();
        }
        Ok(())
    }
}

#[tauri::command]
pub fn get_view(state: State<'_, Mutex<AppState>>) -> View {
    state.lock().unwrap().view()
}

/// `start` | `pause` | `resume` | `stop` | `new` (reset, then start).
#[tauri::command]
pub fn session_command(state: State<'_, Mutex<AppState>>, command: &str) -> Result<View, String> {
    let mut app = state.lock().unwrap();
    let starting = matches!(command, "start" | "resume" | "new");
    if starting && !app.view().can_start {
        return Err("MapleStory not detected".into());
    }
    match command {
        "start" => app.apply(Command::Start)?,
        "pause" => app.apply(Command::Pause)?,
        "resume" => app.apply(Command::Resume)?,
        "stop" => app.apply(Command::Stop)?,
        "new" => {
            app.apply(Command::Reset)?;
            app.apply(Command::Start)?;
        }
        other => return Err(format!("unknown command {other:?}")),
    }
    Ok(app.view())
}

// ---- Calibration window ----

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupStatus {
    permission: Permission,
    windows: Vec<WindowInfo>,
    game: Option<WindowInfo>,
    /// Saved regions for the current game window, if calibrated.
    regions: Option<Regions>,
    missing: Option<Missing>,
}

#[tauri::command]
pub fn open_calibration(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("calibrate") {
        return w.set_focus().map_err(|e| e.to_string());
    }
    WebviewWindowBuilder::new(&app, "calibrate", WebviewUrl::App("index.html".into()))
        .title("Battle Analysis: Calibration")
        .inner_size(960.0, 720.0)
        .min_inner_size(640.0, 480.0)
        .build()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn setup_status(cap: State<'_, Mutex<Capturer>>) -> SetupStatus {
    let mut cap = cap.lock().unwrap();
    cap.scan(true);
    let windows = cap.list_windows().unwrap_or_default();
    let game = cap.game().cloned();
    let cal = cap.calibration().filter(|c| game.as_ref().is_some_and(|g| c.window.matches(g)));
    SetupStatus {
        permission: cap.status().permission,
        windows,
        regions: cal.map(|c| c.regions),
        missing: cal.map(|c| c.missing()),
        game,
    }
}

#[tauri::command]
pub fn request_permission() -> Permission {
    capture::request_permission()
}

/// Opens System Settings → Privacy & Security → Screen Recording.
#[tauri::command]
pub fn open_permission_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn choose_window(cap: State<'_, Mutex<Capturer>>, id: u64) -> Result<WindowInfo, String> {
    cap.lock().unwrap().choose(id).map_err(|e| e.to_string())
}

/// The game window at one pixel per point, as `[width u32 LE][height u32 LE][RGBA…]`.
/// Kept in memory only.
#[tauri::command]
pub fn window_preview(cap: State<'_, Mutex<Capturer>>) -> Result<Response, String> {
    let frame = cap.lock().unwrap().preview().map_err(|e| e.to_string())?;
    Ok(Response::new(encode(&frame)))
}

/// A region at full resolution, same encoding as [`window_preview`].
#[tauri::command]
pub fn region_preview(cap: State<'_, Mutex<Capturer>>, region: RectPt) -> Result<Response, String> {
    let frame = cap.lock().unwrap().capture(region).map_err(|e| e.to_string())?;
    Ok(Response::new(encode(&frame)))
}

#[tauri::command]
pub fn teach_sample(
    cap: State<'_, Mutex<Capturer>>,
    state: State<'_, Mutex<AppState>>,
    regions: Regions,
    sample: Sample,
) -> Result<Missing, String> {
    let session = state.lock().unwrap().session.state();
    if !matches!(session, SessionState::Idle | SessionState::Stopped) {
        return Err("stop the session before calibrating".into());
    }
    cap.lock().unwrap().teach(regions, sample)
}

#[tauri::command]
pub fn reset_calibration(cap: State<'_, Mutex<Capturer>>) -> Result<(), String> {
    cap.lock().unwrap().reset()
}

fn encode(f: &Frame) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + (f.width * f.height * 4) as usize);
    out.extend_from_slice(&f.width.to_le_bytes());
    out.extend_from_slice(&f.height.to_le_bytes());
    for y in 0..f.height as usize {
        out.extend_from_slice(&f.rgba[y * f.stride..y * f.stride + f.width as usize * 4]);
    }
    out
}
