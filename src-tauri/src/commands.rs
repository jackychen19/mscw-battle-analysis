use std::sync::Mutex;
use std::time::Instant;

use battle_core::stats::{Command, Session, SessionConfig, SessionState, Snapshot};
use serde::Serialize;
use tauri::State;

use crate::mock_feed::MockFeed;

pub struct AppState {
    origin: Instant,
    session: Session,
    last_session: Option<Snapshot>,
    feed: MockFeed,
}

/// What the panel renders.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    snapshot: Snapshot,
    /// Results of the previous session, shown while idle.
    last_session: Option<Snapshot>,
    /// True while the numbers come from the synthetic feed (Milestone 1).
    mock: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
            session: Session::new(SessionConfig::default()),
            last_session: None,
            feed: MockFeed::new(),
        }
    }

    fn now_ms(&self) -> u64 {
        self.origin.elapsed().as_millis() as u64
    }

    fn view(&self) -> View {
        View {
            snapshot: self.session.snapshot(self.now_ms()),
            last_session: self.last_session.clone(),
            mock: true,
        }
    }

    pub fn tick_mock(&mut self) {
        let t = self.now_ms();
        self.feed.tick(t, &mut self.session);
    }

    fn apply(&mut self, command: Command) -> Result<(), String> {
        let t = self.now_ms();
        if command == Command::Reset && self.session.state() == SessionState::Stopped {
            self.last_session = Some(self.session.snapshot(t));
        }
        self.session.apply(t, command).map_err(|e| e.to_string())
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
