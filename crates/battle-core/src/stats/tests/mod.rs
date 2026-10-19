//! Scenario tests: synthetic reading sequences driven through [`Session`].

mod exp;
mod meso;
mod session;

use super::*;

pub const SEC: u64 = 1_000;
pub const MIN: u64 = 60 * SEC;

/// Drives a session with a simulated clock. Reads arrive as they would from
/// capture: EXP at ~2 fps, notifications at ~5 fps, each value seen on (at
/// least) two consecutive frames so the filters accept it.
pub struct Sim {
    pub session: Session,
    pub t: u64,
}

impl Sim {
    pub fn new() -> Self {
        Self::with_config(SessionConfig::default())
    }

    pub fn with_table(to_next: Vec<u64>) -> Self {
        Self::with_config(SessionConfig { level_table: Some(LevelTable::new(to_next)), ..Default::default() })
    }

    pub fn with_config(config: SessionConfig) -> Self {
        Self { session: Session::new(config), t: 0 }
    }

    pub fn cmd(&mut self, c: Command) {
        self.session.apply(self.t, c).unwrap();
    }

    pub fn advance(&mut self, ms: u64) {
        self.t += ms;
    }

    /// One raw frame of the EXP line.
    pub fn exp_frame(&mut self, level: u32, exp: u64, pct: u32) {
        self.session.observe(self.t, Observation::Exp(ExpReading::new(level, exp, pct)));
        self.advance(500);
    }

    /// A stable EXP value (two frames).
    pub fn exp(&mut self, level: u32, exp: u64, pct: u32) {
        self.exp_frame(level, exp, pct);
        self.exp_frame(level, exp, pct);
    }

    pub fn unreadable(&mut self) {
        self.session.observe(self.t, Observation::Unreadable);
        self.advance(500);
    }

    pub fn stack_frame(&mut self, lines: &[Line]) {
        self.session.observe(self.t, Observation::Stack(lines.to_vec()));
        self.advance(200);
    }

    /// A stable stack (two frames, the second slightly more faded).
    pub fn stack(&mut self, lines: &[Line]) {
        self.stack_frame(lines);
        let faded: Vec<Line> =
            lines.iter().map(|l| Line { freshness: l.freshness.saturating_sub(2), ..*l }).collect();
        self.stack_frame(&faded);
    }

    pub fn snap(&self) -> Snapshot {
        self.session.snapshot(self.t)
    }
}

/// Percent (hundredths, truncated) of `exp` toward `max`.
pub fn pct(exp: u64, max: u64) -> u32 {
    (exp * 10_000 / max) as u32
}

pub fn meso(amount: u64, freshness: u8) -> Line {
    Line::new(LineKind::Meso, amount, freshness)
}

/// Level table where only `level` has a max.
pub fn table_with(level: u32, max: u64) -> Vec<u64> {
    let mut t = vec![0; level as usize];
    t[level as usize - 1] = max;
    t
}
