use super::*;

const MAX59: u64 = 1_147_000;

#[test]
fn steady_kills_accumulate_deltas() {
    let mut sim = Sim::new();
    sim.cmd(Command::Start);
    let mut exp = 440_834;
    sim.exp(59, exp, pct(exp, MAX59));
    for _ in 0..20 {
        exp += 78;
        sim.exp(59, exp, pct(exp, MAX59));
        sim.advance(2 * SEC);
    }
    let s = sim.snap();
    assert_eq!(s.exp_gained, 20 * 78);
    assert_eq!(s.start_level, Some(59));
    assert_eq!(s.level, Some(59));
    assert_eq!(s.exp, Some(exp));
    // 20 kills in 61 s of active time.
    assert_eq!(s.duration_ms, 61 * SEC);
    assert!((s.exp_per_min - 1560.0 * 60.0 / 61.0).abs() < 1e-9);
    assert!(!s.exp_approximate);
}

#[test]
fn level_up_with_derived_max() {
    let mut sim = Sim::new();
    sim.cmd(Command::Start);
    sim.exp(10, 1_900, 9_500); // derives max ≈ 2000
    sim.exp(11, 50, 100);
    let s = sim.snap();
    assert_eq!(s.exp_gained, 100 + 50);
    assert_eq!((s.start_level, s.level), (Some(10), Some(11)));
}

#[test]
fn level_up_with_table_is_exact() {
    let mut sim = Sim::with_table(table_with(59, MAX59));
    sim.cmd(Command::Start);
    sim.exp(59, 1_146_900, pct(1_146_900, MAX59));
    sim.exp(60, 23, 0);
    assert_eq!(sim.snap().exp_gained, 100 + 23);
}

#[test]
fn level_up_derived_uses_highest_percent_estimate() {
    let mut sim = Sim::new();
    sim.cmd(Command::Start);
    // Low-percent reading gives a poor estimate (≈ 1,149,925) …
    sim.exp(59, 11_499, 100);
    // … superseded by a 99.99% reading (≈ 1,147,000).
    sim.exp(59, 1_146_900, 9_999);
    sim.exp(60, 0, 0);
    let s = sim.snap();
    let expected = 1_146_900 - 11_499 + (1_147_015 - 1_146_900);
    assert_eq!(s.exp_gained, expected);
}

#[test]
fn multi_level_jump_with_table() {
    let mut sim = Sim::with_table(vec![15, 34, 57, 92]);
    sim.cmd(Command::Start);
    sim.exp(1, 10, pct(10, 15));
    // Missed frames: straight to level 4.
    sim.exp(4, 5, pct(5, 92));
    let s = sim.snap();
    assert_eq!(s.exp_gained, 5 + 34 + 57 + 5);
    assert!(!s.exp_approximate);
}

#[test]
fn multi_level_jump_without_table_is_flagged() {
    let mut sim = Sim::new();
    sim.cmd(Command::Start);
    sim.exp(20, 9_000, 9_000); // max 10,000
    sim.exp(22, 100, 50);
    let s = sim.snap();
    assert_eq!(s.exp_gained, 1_000 + 100);
    assert!(s.exp_approximate);
}

#[test]
fn death_adds_nothing_and_rebaselines() {
    let max = 100_000;
    let mut sim = Sim::new();
    sim.cmd(Command::Start);
    sim.exp(30, 50_000, pct(50_000, max));
    sim.exp(30, 51_000, pct(51_000, max));
    sim.exp(30, 46_000, pct(46_000, max)); // died, lost EXP
    sim.exp(30, 46_500, pct(46_500, max));
    assert_eq!(sim.snap().exp_gained, 1_000 + 500);
}

#[test]
fn unreadable_frames_are_skipped_not_zero() {
    let max = 100_000;
    let mut sim = Sim::new();
    sim.cmd(Command::Start);
    sim.exp(30, 50_000, pct(50_000, max));
    for _ in 0..10 {
        sim.unreadable();
    }
    let s = sim.snap();
    assert_eq!(s.exp, Some(50_000));
    assert_eq!(s.exp_gained, 0);
    // A value seen on either side of unreadable frames still confirms.
    sim.exp_frame(30, 50_100, pct(50_100, max));
    sim.unreadable();
    sim.exp_frame(30, 50_100, pct(50_100, max));
    assert_eq!(sim.snap().exp_gained, 100);
}

#[test]
fn single_frame_misread_is_ignored() {
    let max = 100_000;
    let mut sim = Sim::new();
    sim.cmd(Command::Start);
    sim.exp(30, 10_000, pct(10_000, max));
    sim.exp_frame(30, 18_000, pct(18_000, max)); // flicker, self-consistent but alone
    sim.exp(30, 10_078, pct(10_078, max));
    assert_eq!(sim.snap().exp_gained, 78);
}

#[test]
fn self_inconsistent_reading_is_rejected_even_if_repeated() {
    let max = 100_000;
    let mut sim = Sim::new();
    sim.cmd(Command::Start);
    sim.exp(30, 50_000, pct(50_000, max));
    sim.exp(30, 80_000, pct(50_100, max)); // digits misread, percent fine
    sim.exp(30, 50_100, pct(50_100, max));
    assert_eq!(sim.snap().exp_gained, 100);
}

#[test]
fn level_down_is_ignored() {
    let mut sim = Sim::new();
    sim.cmd(Command::Start);
    sim.exp(30, 50_000, 5_000);
    sim.exp(3, 50, 500);
    sim.exp(30, 50_200, 5_020);
    let s = sim.snap();
    assert_eq!(s.exp_gained, 200);
    assert_eq!(s.level, Some(30));
}

#[test]
fn next_level_eta_from_rate() {
    let max = 100_000;
    let mut sim = Sim::new();
    sim.cmd(Command::Start);
    sim.exp(30, 50_000, pct(50_000, max)); // t = 1 s at baseline
    sim.advance(59 * SEC);
    sim.exp(30, 51_000, pct(51_000, max)); // t = 61 s after
    let s = sim.snap();
    assert_eq!(s.duration_ms, 61 * SEC);
    // 1000 EXP per 61 s; 49,000 remaining.
    let expected = (49_000.0 / (1_000.0 * 60.0 / 61.0) * 60_000.0) as u64;
    assert_eq!(s.next_level_ms, Some(expected));
}

#[test]
fn next_level_eta_absent_without_progress() {
    let mut sim = Sim::new();
    sim.cmd(Command::Start);
    sim.exp(30, 50_000, 5_000);
    assert_eq!(sim.snap().next_level_ms, None);
}

#[test]
fn rolling_rate_tracks_last_five_minutes() {
    let max = 10_000_000;
    let mut sim = Sim::with_table(table_with(70, max));
    sim.cmd(Command::Start);
    let mut exp = 1_000_000;
    sim.exp(70, exp, pct(exp, max));
    // 10 minutes: 100 EXP/min, then 300 EXP/min. One stable read per 10 s.
    for i in 0..60 {
        let per_tick = if i < 30 { 100 / 6 } else { 300 / 6 };
        exp += per_tick;
        sim.advance(10 * SEC - 1_000);
        sim.exp(70, exp, pct(exp, max));
    }
    let s = sim.snap();
    assert!((s.exp_per_min_recent - 300.0).abs() < 300.0 * 0.05, "{}", s.exp_per_min_recent);
    assert!(s.exp_per_min < 200.0);
    // Past the window, the ETA uses the recent rate.
    let remaining = (max - exp) as f64;
    let eta = s.next_level_ms.unwrap() as f64;
    assert!((eta - remaining / s.exp_per_min_recent * 60_000.0).abs() < 1.0);
}
