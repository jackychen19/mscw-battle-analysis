use std::collections::BTreeMap;

use super::reading::ExpReading;

/// EXP needed to go from each level to the next. `to_next[0]` is level 1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LevelTable {
    to_next: Vec<u64>,
}

impl LevelTable {
    pub fn new(to_next: Vec<u64>) -> Self {
        Self { to_next }
    }

    pub fn max_for(&self, level: u32) -> Option<u64> {
        let i = usize::try_from(level).ok()?.checked_sub(1)?;
        self.to_next.get(i).copied().filter(|&m| m > 0)
    }
}

/// The EXP needed to finish a level, and how much to trust it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaxExp {
    pub value: u64,
    /// True when it came from the static table.
    pub exact: bool,
    /// Percent (hundredths) of the reading it was derived from; 10000 if exact.
    /// The relative error of a derived max is about `0.5 / basis_pct`, so
    /// readings taken at a high percent give much better estimates.
    pub basis_pct: u32,
}

/// Knows each level's max EXP, from the table when available, otherwise
/// derived as `exp ÷ percent` from the highest-percent reading seen so far.
#[derive(Clone, Debug, Default)]
pub struct MaxEstimator {
    table: Option<LevelTable>,
    best: BTreeMap<u32, ExpReading>,
}

impl MaxEstimator {
    pub fn new(table: Option<LevelTable>) -> Self {
        Self { table, best: BTreeMap::new() }
    }

    /// Records an accepted reading as a candidate for deriving its level's max.
    pub fn observe(&mut self, r: ExpReading) {
        if r.pct_hundredths == 0 || self.table_max(r.level).is_some() {
            return;
        }
        let entry = self.best.entry(r.level).or_insert(r);
        if r.pct_hundredths > entry.pct_hundredths {
            *entry = r;
        }
    }

    pub fn max_for(&self, level: u32) -> Option<MaxExp> {
        if let Some(value) = self.table_max(level) {
            return Some(MaxExp { value, exact: true, basis_pct: 10_000 });
        }
        let r = self.best.get(&level)?;
        let pct = u128::from(r.pct_hundredths);
        // Rounded exp * 10000 / pct.
        let value = (u128::from(r.exp) * 10_000 + pct / 2) / pct;
        Some(MaxExp { value: value as u64, exact: false, basis_pct: r.pct_hundredths })
    }

    /// Whether `exp` and `percent` agree with each other given what we know of
    /// the level's max. Readings for levels with no known max pass if the
    /// percent is in range.
    pub fn is_consistent(&self, r: &ExpReading) -> bool {
        if r.pct_hundredths >= 10_000 {
            return false;
        }
        let Some(max) = self.max_for(r.level) else {
            return true;
        };
        if max.exact && r.exp >= max.value {
            return false;
        }
        let expected = (u128::from(r.exp) * 10_000 / u128::from(max.value)) as u64;
        let tolerance = 1 + u64::from(r.pct_hundredths.div_ceil(max.basis_pct));
        expected.abs_diff(u64::from(r.pct_hundredths)) <= tolerance
    }

    fn table_max(&self, level: u32) -> Option<u64> {
        self.table.as_ref()?.max_for(level)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_max_from_highest_percent_reading() {
        let mut est = MaxEstimator::new(None);
        est.observe(ExpReading::new(10, 20, 100)); // 1.00% → 2000
        est.observe(ExpReading::new(10, 1900, 9500)); // 95.00% → 2000
        est.observe(ExpReading::new(10, 400, 2000)); // lower pct, ignored
        let max = est.max_for(10).unwrap();
        assert_eq!(max.value, 2000);
        assert!(!max.exact);
        assert_eq!(max.basis_pct, 9500);
    }

    #[test]
    fn derived_max_rounding_error_matches_documentation() {
        // True max 1,147,000. A 38.43% reading (truncated from 38.4337%).
        let est = {
            let mut e = MaxEstimator::new(None);
            e.observe(ExpReading::new(59, 440_834, 3843));
            e
        };
        let derived = est.max_for(59).unwrap().value as f64;
        let rel = (derived - 1_147_000.0).abs() / 1_147_000.0;
        assert!(rel < 0.01 / 38.43, "relative error {rel}");
    }

    #[test]
    fn table_wins_and_is_exact() {
        let mut est = MaxEstimator::new(Some(LevelTable::new(vec![15, 34, 57])));
        est.observe(ExpReading::new(2, 10, 2941));
        assert_eq!(est.max_for(2), Some(MaxExp { value: 34, exact: true, basis_pct: 10_000 }));
        assert_eq!(est.max_for(4), None);
        assert_eq!(LevelTable::new(vec![15]).max_for(0), None);
    }

    #[test]
    fn consistency_checks() {
        let mut est = MaxEstimator::new(Some(LevelTable::new(vec![0; 9].into_iter().chain([2000]).collect())));
        assert!(est.is_consistent(&ExpReading::new(10, 1000, 5000)));
        assert!(est.is_consistent(&ExpReading::new(10, 1001, 5005)));
        assert!(!est.is_consistent(&ExpReading::new(10, 1000, 6000)));
        assert!(!est.is_consistent(&ExpReading::new(10, 2000, 9999)));
        assert!(!est.is_consistent(&ExpReading::new(10, 0, 10_000)));
        // Unknown level: only the range check applies.
        assert!(est.is_consistent(&ExpReading::new(11, 5, 9000)));
        // Derived max: tolerance grows when the basis reading was low.
        est.observe(ExpReading::new(11, 50, 100)); // ~5000, ±0.5%
        assert!(est.is_consistent(&ExpReading::new(11, 4000, 8000)));
        assert!(!est.is_consistent(&ExpReading::new(11, 4000, 7000)));
    }
}
