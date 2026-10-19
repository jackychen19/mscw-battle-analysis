use std::fmt;

use serde::{Deserialize, Serialize};

use super::filter::ReadingFilter;
use super::level::{LevelTable, MaxEstimator};
use super::meso_track::{DEFAULT_FRESHNESS_TOLERANCE, MesoTracker};
use super::reading::{ExpReading, Line};
use super::rolling::{DEFAULT_ROLLING_WINDOW_MS, Rolling, per_min};
use super::snapshot::Snapshot;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionState {
    Idle,
    Recording,
    Paused,
    Stopped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Start,
    Pause,
    Resume,
    Stop,
    /// "Start New Session": back to Idle with everything cleared.
    Reset,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Observation {
    Exp(ExpReading),
    /// Notification stack, top → bottom.
    Stack(Vec<Line>),
    /// Menu, tooltip, loading screen… Skipped, never treated as zero.
    Unreadable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransitionError {
    pub from: SessionState,
    pub command: Command,
}

impl fmt::Display for TransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cannot {:?} while {:?}", self.command, self.from)
    }
}

impl std::error::Error for TransitionError {}

#[derive(Clone, Debug)]
pub struct SessionConfig {
    pub level_table: Option<LevelTable>,
    pub rolling_window_ms: u64,
    pub freshness_tolerance: u8,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            level_table: None,
            rolling_window_ms: DEFAULT_ROLLING_WINDOW_MS,
            freshness_tolerance: DEFAULT_FRESHNESS_TOLERANCE,
        }
    }
}

/// One recording session: `Idle → Recording ⇄ Paused → Stopped → (Reset) Idle`.
#[derive(Clone, Debug)]
pub struct Session {
    freshness_tolerance: u8,
    state: SessionState,
    /// Survives resets: level maxes learned in one session stay valid.
    estimator: MaxEstimator,
    exp_filter: ReadingFilter,
    meso_tracker: MesoTracker,

    start_reading: Option<ExpReading>,
    last_reading: Option<ExpReading>,
    /// Set on Start/Resume: the next accepted reading/stack is a baseline,
    /// so gains while paused (or lines already on screen) aren't counted.
    exp_needs_baseline: bool,

    exp_gained: u64,
    meso_gained: u64,
    exp_approximate: bool,
    meso_possible_undercount: bool,

    active_before_ms: u64,
    segment_start_ms: Option<u64>,
    paused_before_ms: u64,
    pause_start_ms: Option<u64>,
    rolling: Rolling,
}

impl Session {
    pub fn new(config: SessionConfig) -> Self {
        Self {
            freshness_tolerance: config.freshness_tolerance,
            state: SessionState::Idle,
            estimator: MaxEstimator::new(config.level_table),
            exp_filter: ReadingFilter::default(),
            meso_tracker: MesoTracker::new(config.freshness_tolerance),
            start_reading: None,
            last_reading: None,
            exp_needs_baseline: true,
            exp_gained: 0,
            meso_gained: 0,
            exp_approximate: false,
            meso_possible_undercount: false,
            active_before_ms: 0,
            segment_start_ms: None,
            paused_before_ms: 0,
            pause_start_ms: None,
            rolling: Rolling::new(config.rolling_window_ms),
        }
    }

    pub fn state(&self) -> SessionState {
        self.state
    }

    pub fn apply(&mut self, t_ms: u64, command: Command) -> Result<(), TransitionError> {
        use SessionState::*;
        match (self.state, command) {
            (Idle, Command::Start) => {
                self.segment_start_ms = Some(t_ms);
                self.rolling.record(0, 0, 0);
                self.begin_segment();
                self.state = Recording;
            }
            (Recording, Command::Pause) => {
                self.close_segment(t_ms);
                self.pause_start_ms = Some(t_ms);
                self.state = Paused;
            }
            (Paused, Command::Resume) => {
                self.close_pause(t_ms);
                self.segment_start_ms = Some(t_ms);
                self.begin_segment();
                self.state = Recording;
            }
            (Recording | Paused, Command::Stop) => {
                self.close_segment(t_ms);
                self.close_pause(t_ms);
                self.state = Stopped;
            }
            (_, Command::Reset) => {
                let estimator = std::mem::take(&mut self.estimator);
                *self = Self::new(SessionConfig {
                    level_table: None,
                    rolling_window_ms: self.rolling.window_ms(),
                    freshness_tolerance: self.freshness_tolerance,
                });
                self.estimator = estimator;
            }
            (from, command) => return Err(TransitionError { from, command }),
        }
        Ok(())
    }

    /// Feeds one parsed frame. Ignored unless recording.
    pub fn observe(&mut self, t_ms: u64, observation: Observation) {
        if self.state != SessionState::Recording {
            return;
        }
        match observation {
            Observation::Exp(raw) => {
                if let Some(reading) = self.exp_filter.feed(raw, &self.estimator) {
                    self.accept_reading(reading);
                }
            }
            Observation::Stack(lines) => {
                let update = self.meso_tracker.feed(&lines);
                self.meso_gained += update.meso;
                self.meso_possible_undercount |= update.possible_overflow;
            }
            Observation::Unreadable => {}
        }
        self.rolling.record(self.active_ms(t_ms), self.exp_gained, self.meso_gained);
    }

    pub fn snapshot(&self, t_ms: u64) -> Snapshot {
        let duration_ms = self.active_ms(t_ms);
        let (exp_recent, meso_recent) =
            self.rolling.rates_per_min(duration_ms, self.exp_gained, self.meso_gained);
        let exp_per_min = per_min(self.exp_gained, duration_ms);
        let current = self.last_reading;
        let next_level_ms = current.and_then(|r| {
            let max = self.estimator.max_for(r.level)?;
            let rate = if duration_ms >= self.rolling.window_ms() { exp_recent } else { exp_per_min };
            (rate > 0.0).then(|| (max.value.saturating_sub(r.exp) as f64 / rate * 60_000.0) as u64)
        });
        Snapshot {
            state: self.state,
            start_level: self.start_reading.map(|r| r.level),
            level: current.map(|r| r.level),
            exp: current.map(|r| r.exp),
            pct_hundredths: current.map(|r| r.pct_hundredths),
            duration_ms,
            paused_ms: self.paused_before_ms
                + self.pause_start_ms.map_or(0, |p| t_ms.saturating_sub(p)),
            exp_gained: self.exp_gained,
            exp_per_min,
            exp_per_min_recent: exp_recent,
            meso_gained: self.meso_gained,
            meso_per_min: per_min(self.meso_gained, duration_ms),
            meso_per_min_recent: meso_recent,
            next_level_ms,
            exp_approximate: self.exp_approximate,
            meso_possible_undercount: self.meso_possible_undercount,
        }
    }

    fn active_ms(&self, t_ms: u64) -> u64 {
        self.active_before_ms + self.segment_start_ms.map_or(0, |s| t_ms.saturating_sub(s))
    }

    fn begin_segment(&mut self) {
        self.exp_filter.reset();
        self.meso_tracker.reset();
        self.exp_needs_baseline = true;
    }

    fn close_segment(&mut self, t_ms: u64) {
        if let Some(start) = self.segment_start_ms.take() {
            self.active_before_ms += t_ms.saturating_sub(start);
        }
    }

    fn close_pause(&mut self, t_ms: u64) {
        if let Some(start) = self.pause_start_ms.take() {
            self.paused_before_ms += t_ms.saturating_sub(start);
        }
    }

    fn accept_reading(&mut self, r: ExpReading) {
        match self.last_reading {
            Some(prev) if !self.exp_needs_baseline => {
                if r.level < prev.level {
                    return; // Levels never go down: a misread.
                }
                if r.level == prev.level {
                    // EXP down at the same level is a death: add nothing, re-baseline.
                    self.exp_gained += r.exp.saturating_sub(prev.exp);
                } else {
                    self.exp_gained += self.level_up_gain(prev, r);
                }
            }
            _ => {
                self.start_reading.get_or_insert(r);
                self.exp_needs_baseline = false;
            }
        }
        self.estimator.observe(r);
        self.last_reading = Some(r);
    }

    /// Rest of the old level + any skipped whole levels + EXP into the new one.
    fn level_up_gain(&mut self, prev: ExpReading, r: ExpReading) -> u64 {
        let mut gain = r.exp;
        match self.estimator.max_for(prev.level) {
            Some(max) => gain += max.value.saturating_sub(prev.exp),
            None => self.exp_approximate = true,
        }
        for level in prev.level + 1..r.level {
            match self.estimator.max_for(level) {
                Some(max) => gain += max.value,
                None => self.exp_approximate = true,
            }
        }
        gain
    }
}
