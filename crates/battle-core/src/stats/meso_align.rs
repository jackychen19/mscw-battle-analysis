//! Aligning consecutive notification stacks so each line is counted once.
//!
//! New lines appear at the bottom and older ones scroll off the top, so the
//! current stack is `prev[shift..] ++ new`. We pick the smallest `shift` for
//! which the kept lines match by text *and* none of them got brighter (lines
//! only fade). Brightness is what tells `+15, +15` (nothing new) apart from
//! `+15` scrolling off while a fresh `+15` arrives.

use super::reading::Line;

/// How much brighter a line may read between frames and still be the same
/// line (absorbs binarization noise).
pub const DEFAULT_FRESHNESS_TOLERANCE: u8 = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Alignment {
    /// Lines of `prev` that scrolled off the top.
    pub shift: usize,
    /// Lines shared by both stacks; `cur[overlap..]` are new.
    pub overlap: usize,
}

impl Alignment {
    pub fn new_lines<'a>(&self, cur: &'a [Line]) -> &'a [Line] {
        &cur[self.overlap..]
    }
}

pub fn align(prev: &[Line], cur: &[Line], freshness_tolerance: u8) -> Alignment {
    (0..=prev.len())
        .find_map(|shift| {
            let kept = &prev[shift..];
            let matches = kept.len() <= cur.len()
                && kept.iter().zip(cur).all(|(p, c)| {
                    let faded = p.is_unknown()
                        || c.is_unknown()
                        || c.freshness <= p.freshness.saturating_add(freshness_tolerance);
                    p.same_text(c) && faded
                });
            matches.then_some(Alignment { shift, overlap: kept.len() })
        })
        .expect("shift == prev.len() always matches")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::LineKind::{Exp, Meso, Other, Unknown};

    fn m(amount: u64, freshness: u8) -> Line {
        Line::new(Meso, amount, freshness)
    }

    fn new_amounts(prev: &[Line], cur: &[Line]) -> Vec<u64> {
        align(prev, cur, DEFAULT_FRESHNESS_TOLERANCE).new_lines(cur).iter().map(|l| l.amount).collect()
    }

    #[test]
    fn unchanged_stack_has_nothing_new() {
        let s = [m(15, 200), m(20, 255)];
        assert_eq!(new_amounts(&s, &[m(15, 190), m(20, 250)]), Vec::<u64>::new());
    }

    #[test]
    fn appended_lines_are_new() {
        assert_eq!(new_amounts(&[m(15, 255)], &[m(15, 230), m(20, 255)]), vec![20]);
        assert_eq!(new_amounts(&[], &[m(7, 255)]), vec![7]);
    }

    #[test]
    fn duplicate_amount_appended_is_new() {
        assert_eq!(new_amounts(&[m(15, 255)], &[m(15, 230), m(15, 255)]), vec![15]);
    }

    #[test]
    fn scroll_off_with_identical_replacement_detected_by_brightness() {
        // The faded +15 left the top; a fresh +15 arrived at the bottom.
        assert_eq!(new_amounts(&[m(15, 100)], &[m(15, 255)]), vec![15]);
        assert_eq!(new_amounts(&[m(15, 120), m(15, 200)], &[m(15, 190), m(15, 255)]), vec![15]);
    }

    #[test]
    fn lines_expiring_from_top_are_not_new() {
        assert_eq!(new_amounts(&[m(1, 50), m(2, 100), m(3, 200)], &[m(2, 90), m(3, 190)]), Vec::<u64>::new());
        assert_eq!(new_amounts(&[m(1, 50)], &[]), Vec::<u64>::new());
    }

    #[test]
    fn burst_between_frames() {
        let prev = [m(5, 120), m(6, 200)];
        let cur = [m(6, 150), m(10, 255), m(20, 255), m(30, 255), m(40, 255)];
        let a = align(&prev, &cur, DEFAULT_FRESHNESS_TOLERANCE);
        assert_eq!(a, Alignment { shift: 1, overlap: 1 });
        assert_eq!(new_amounts(&prev, &cur), vec![10, 20, 30, 40]);
    }

    #[test]
    fn other_line_kinds_anchor_the_alignment() {
        let prev = [Line::new(Exp, 15, 200), m(15, 220)];
        let cur = [m(15, 210), Line::new(Other, 99, 255)];
        assert_eq!(align(&prev, &cur, DEFAULT_FRESHNESS_TOLERANCE), Alignment { shift: 1, overlap: 1 });
    }

    #[test]
    fn unknown_lines_match_anything() {
        let u = Line::new(Unknown, 0, 0);
        // A covered line in the middle doesn't break the alignment.
        assert_eq!(new_amounts(&[m(1, 200), u, m(3, 240)], &[m(1, 190), m(2, 230), m(3, 230), m(4, 255)]), vec![4]);
        // Ambiguous with a wildcard: the no-new-lines reading wins, so a
        // duplicate arriving while its neighbour is covered can be missed.
        assert_eq!(new_amounts(&[m(15, 120), u], &[u, m(15, 255)]), Vec::<u64>::new());
    }

    #[test]
    fn no_overlap_means_everything_is_new() {
        let a = align(&[m(1, 255)], &[m(2, 255), m(3, 255)], DEFAULT_FRESHNESS_TOLERANCE);
        assert_eq!(a, Alignment { shift: 1, overlap: 0 });
    }
}
