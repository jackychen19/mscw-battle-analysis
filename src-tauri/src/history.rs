//! Session history: one small JSON file per finished session in the app's
//! data dir (`sessions/<startedAt>.json`), for a future debrief view.

use std::path::{Path, PathBuf};

use battle_core::stats::{ExpReading, SessionRecord};
use serde::Serialize;

/// A [`SessionRecord`] with its clock times turned into wall-clock dates
/// (Unix milliseconds, UTC; shown in local time when displayed).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedSession {
    pub version: u32,
    pub started_at: u64,
    pub stopped_at: u64,
    pub active_ms: u64,
    pub paused_ms: u64,
    pub exp_gained: u64,
    pub start: ExpReading,
    pub end: ExpReading,
    pub level_ups: Vec<SavedLevelUp>,
    pub exp_by_minute: Vec<u64>,
    pub exp_approximate: bool,
    pub meso_gained: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedLevelUp {
    pub at: u64,
    pub active_ms: u64,
    pub to_level: u32,
}

impl SavedSession {
    /// `wall_ms(t)`: the Unix time of session clock time `t`.
    pub fn new(r: SessionRecord, wall_ms: impl Fn(u64) -> u64) -> Self {
        Self {
            version: r.version,
            started_at: wall_ms(r.started_ms),
            stopped_at: wall_ms(r.stopped_ms),
            active_ms: r.active_ms,
            paused_ms: r.paused_ms,
            exp_gained: r.exp_gained,
            start: r.start,
            end: r.end,
            level_ups: r
                .level_ups
                .iter()
                .map(|u| SavedLevelUp { at: wall_ms(u.at_ms), active_ms: u.active_ms, to_level: u.to_level })
                .collect(),
            exp_by_minute: r.exp_by_minute,
            exp_approximate: r.exp_approximate,
            // Meso tracking is on hold: not measured, rather than zero.
            meso_gained: None,
        }
    }
}

/// Writes `session` to `<dir>/sessions/<startedAt>.json`.
pub fn save(dir: &Path, session: &SavedSession) -> Result<PathBuf, String> {
    let sessions = dir.join("sessions");
    std::fs::create_dir_all(&sessions).map_err(|e| e.to_string())?;
    let path = sessions.join(format!("{}.json", session.started_at));
    let json = serde_json::to_vec_pretty(session).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use battle_core::stats::{LevelUp, RECORD_VERSION};

    use super::*;

    #[test]
    fn dates_the_record_and_names_fields_for_the_frontend() {
        let r = SessionRecord {
            version: RECORD_VERSION,
            started_ms: 1_000,
            stopped_ms: 61_000,
            active_ms: 60_000,
            paused_ms: 0,
            exp_gained: 650,
            start: ExpReading::new(10, 1_000, 5_000),
            end: ExpReading::new(11, 50, 25),
            level_ups: vec![LevelUp { at_ms: 31_000, active_ms: 30_000, to_level: 11 }],
            exp_by_minute: vec![650],
            exp_approximate: false,
            meso_gained: Some(0),
        };
        let saved = SavedSession::new(r, |t| 1_760_000_000_000 + t);
        let json = serde_json::to_value(&saved).unwrap();
        assert_eq!(json["startedAt"], 1_760_000_001_000u64);
        assert_eq!(json["stoppedAt"], 1_760_000_061_000u64);
        assert_eq!(json["levelUps"][0]["at"], 1_760_000_031_000u64);
        assert_eq!(json["levelUps"][0]["toLevel"], 11);
        assert_eq!(json["end"]["pctHundredths"], 25);
        assert_eq!(json["expByMinute"][0], 650);
        assert!(json["mesoGained"].is_null(), "on hold: not measured");
    }
}
