use super::level::MaxEstimator;
use super::reading::{ExpReading, Line};

/// Accepts an EXP reading only when it is self-consistent and the previous
/// raw reading was identical. Unreadable frames are simply not fed in.
#[derive(Clone, Debug, Default)]
pub struct ReadingFilter {
    pending: Option<ExpReading>,
}

impl ReadingFilter {
    pub fn feed(&mut self, raw: ExpReading, est: &MaxEstimator) -> Option<ExpReading> {
        if !est.is_consistent(&raw) {
            self.pending = None;
            return None;
        }
        if self.pending == Some(raw) {
            return Some(raw);
        }
        self.pending = Some(raw);
        None
    }

    pub fn reset(&mut self) {
        self.pending = None;
    }
}

/// Accepts a notification stack only when two consecutive frames show the
/// same text (amounts and kinds). The accepted stack carries the latest
/// frame's freshness values.
#[derive(Clone, Debug, Default)]
pub struct StackFilter {
    pending: Option<Vec<Line>>,
}

impl StackFilter {
    pub fn feed(&mut self, raw: Vec<Line>) -> Option<Vec<Line>> {
        let confirmed = self.pending.as_ref().is_some_and(|p| {
            p.len() == raw.len() && p.iter().zip(&raw).all(|(a, b)| a.same_text(b))
        });
        self.pending = Some(raw.clone());
        confirmed.then_some(raw)
    }

    pub fn reset(&mut self) {
        self.pending = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::LineKind::Meso;

    #[test]
    fn reading_needs_two_identical_consecutive_reads() {
        let est = MaxEstimator::new(None);
        let mut f = ReadingFilter::default();
        let a = ExpReading::new(5, 100, 1000);
        let b = ExpReading::new(5, 108, 1080);
        assert_eq!(f.feed(a, &est), None);
        assert_eq!(f.feed(b, &est), None);
        assert_eq!(f.feed(b, &est), Some(b));
        assert_eq!(f.feed(b, &est), Some(b));
    }

    #[test]
    fn inconsistent_reading_clears_pending() {
        let mut est = MaxEstimator::new(None);
        est.observe(ExpReading::new(5, 500, 5000));
        let mut f = ReadingFilter::default();
        let a = ExpReading::new(5, 100, 1000);
        assert_eq!(f.feed(a, &est), None);
        assert_eq!(f.feed(ExpReading::new(5, 100, 9000), &est), None);
        assert_eq!(f.feed(a, &est), None);
        assert_eq!(f.feed(a, &est), Some(a));
    }

    #[test]
    fn stack_confirms_on_same_text_and_keeps_latest_freshness() {
        let mut f = StackFilter::default();
        assert_eq!(f.feed(vec![Line::new(Meso, 15, 255)]), None);
        assert_eq!(f.feed(vec![Line::new(Meso, 15, 240)]), Some(vec![Line::new(Meso, 15, 240)]));
        assert_eq!(f.feed(vec![Line::new(Meso, 16, 240)]), None);
        assert_eq!(f.feed(vec![]), None);
        assert_eq!(f.feed(vec![]), Some(vec![]));
    }
}
