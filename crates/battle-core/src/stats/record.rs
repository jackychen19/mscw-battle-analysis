use serde::{Deserialize, Serialize};

use super::reading::ExpReading;

pub const RECORD_VERSION: u32 = 1;

/// A finished session, as saved to history. Times are milliseconds on the
/// caller's clock (the same clock passed to [`super::Session::apply`]); the
/// app adds wall-clock dates. Rates aren't stored: they follow from the
/// totals, and the timeline gives any window's rate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionRecord {
    pub version: u32,
    pub started_ms: u64,
    pub stopped_ms: u64,
    /// Recording time, excluding pauses.
    pub active_ms: u64,
    pub paused_ms: u64,
    pub exp_gained: u64,
    /// First and last accepted readings.
    pub start: ExpReading,
    pub end: ExpReading,
    pub level_ups: Vec<LevelUp>,
    /// EXP gained so far at the end of each whole minute of active time.
    pub exp_by_minute: Vec<u64>,
    /// Some level-up gain was estimated (a level's max EXP was unknown).
    pub exp_approximate: bool,
    /// `None` while meso tracking is on hold.
    pub meso_gained: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelUp {
    /// When it was seen, on the caller's clock and in active time.
    pub at_ms: u64,
    pub active_ms: u64,
    pub to_level: u32,
}
