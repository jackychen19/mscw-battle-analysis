use serde::{Deserialize, Serialize};

use super::engine::SessionState;

/// Everything the UI shows, as of one instant. Serialized camelCase for the
/// frontend (`src/types.ts` mirrors this).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub state: SessionState,
    /// Level at the session's baseline reading.
    pub start_level: Option<u32>,
    pub level: Option<u32>,
    pub exp: Option<u64>,
    pub pct_hundredths: Option<u32>,
    /// Active time, excluding pauses.
    pub duration_ms: u64,
    pub paused_ms: u64,
    pub exp_gained: u64,
    pub exp_per_min: f64,
    pub exp_per_min_recent: f64,
    pub meso_gained: u64,
    pub meso_per_min: f64,
    pub meso_per_min_recent: f64,
    /// Estimated time to next level at the recent rate.
    pub next_level_ms: Option<u64>,
    /// EXP summed from `You received EXP` / party bonus lines (cross-check).
    pub notification_exp: u64,
    /// Some level-up gain could not be computed (unknown level max).
    pub exp_approximate: bool,
    /// A burst may have pushed meso lines off screen between frames.
    pub meso_possible_undercount: bool,
}
