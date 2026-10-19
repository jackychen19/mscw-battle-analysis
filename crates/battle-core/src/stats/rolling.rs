use std::collections::VecDeque;

/// The "recent" rate window, in active (non-paused) time.
pub const DEFAULT_ROLLING_WINDOW_MS: u64 = 5 * 60 * 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Sample {
    active_ms: u64,
    exp: u64,
    meso: u64,
}

/// Cumulative totals sampled over active time, for last-N-minutes rates.
#[derive(Clone, Debug)]
pub struct Rolling {
    window_ms: u64,
    samples: VecDeque<Sample>,
}

impl Rolling {
    pub fn new(window_ms: u64) -> Self {
        Self { window_ms, samples: VecDeque::new() }
    }

    pub fn window_ms(&self) -> u64 {
        self.window_ms
    }

    pub fn clear(&mut self) {
        self.samples.clear();
    }

    pub fn record(&mut self, active_ms: u64, exp: u64, meso: u64) {
        let sample = Sample { active_ms, exp, meso };
        match self.samples.back_mut() {
            Some(last) if last.active_ms >= active_ms => *last = Sample { active_ms: last.active_ms, ..sample },
            _ => self.samples.push_back(sample),
        }
        // Keep exactly one sample at or before the window start.
        let cutoff = active_ms.saturating_sub(self.window_ms);
        while self.samples.len() >= 2 && self.samples[1].active_ms <= cutoff {
            self.samples.pop_front();
        }
    }

    /// (exp/min, meso/min) over the last window ending at `active_ms`, given
    /// the current totals. Uses everything since the first sample if the
    /// session is shorter than the window.
    pub fn rates_per_min(&self, active_ms: u64, exp: u64, meso: u64) -> (f64, f64) {
        let cutoff = active_ms.saturating_sub(self.window_ms);
        let base = self.samples.iter().rev().find(|s| s.active_ms <= cutoff).or(self.samples.front());
        let Some(base) = base else { return (0.0, 0.0) };
        let elapsed = active_ms.saturating_sub(base.active_ms);
        (
            per_min(exp.saturating_sub(base.exp), elapsed),
            per_min(meso.saturating_sub(base.meso), elapsed),
        )
    }
}

pub(crate) fn per_min(amount: u64, elapsed_ms: u64) -> f64 {
    if elapsed_ms == 0 { 0.0 } else { amount as f64 * 60_000.0 / elapsed_ms as f64 }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: u64 = 60_000;

    #[test]
    fn short_session_uses_everything() {
        let mut r = Rolling::new(5 * MIN);
        r.record(0, 0, 0);
        r.record(2 * MIN, 200, 50);
        assert_eq!(r.rates_per_min(2 * MIN, 200, 50), (100.0, 25.0));
    }

    #[test]
    fn long_session_only_counts_the_window() {
        let mut r = Rolling::new(5 * MIN);
        for m in 0..=10 {
            // 100/min for the first 5 minutes, 300/min after.
            let exp = if m <= 5 { m * 100 } else { 500 + (m - 5) * 300 };
            r.record(m * MIN, exp, 0);
        }
        assert_eq!(r.rates_per_min(10 * MIN, 2000, 0), (300.0, 0.0));
        assert!(r.samples.len() <= 7);
    }

    #[test]
    fn empty_is_zero() {
        assert_eq!(Rolling::new(MIN).rates_per_min(10, 5, 5), (0.0, 0.0));
    }
}
