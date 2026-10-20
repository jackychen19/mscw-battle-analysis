use super::*;

#[test]
fn no_record_until_stopped_with_a_reading() {
    let mut sim = Sim::new();
    assert_eq!(sim.session.record(), None);
    sim.cmd(Command::Start);
    sim.exp(10, 1_000, 5_000);
    assert_eq!(sim.session.record(), None, "still recording");
    sim.cmd(Command::Stop);
    assert!(sim.session.record().is_some());

    // Nothing ever read (game behind a menu the whole time): nothing to save.
    let mut sim = Sim::new();
    sim.cmd(Command::Start);
    sim.unreadable();
    sim.cmd(Command::Stop);
    assert_eq!(sim.session.record(), None);
}

#[test]
fn record_has_times_level_ups_and_a_minute_timeline() {
    let mut sim = Sim::new();
    sim.advance(5 * SEC); // the clock doesn't start at the session
    sim.cmd(Command::Start);
    sim.exp(10, 1_000, 5_000); // max 2,000; t = 6 s, active 1 s
    // 100 EXP every 30 s of active time.
    let mut exp = 1_000;
    for _ in 0..4 {
        sim.advance(29 * SEC);
        exp += 100;
        sim.exp(10, exp, pct(exp, 2_000));
    }
    // Active ~2 min 1 s; pause a minute: no time, no gains.
    sim.cmd(Command::Pause);
    sim.advance(MIN);
    sim.cmd(Command::Resume);
    sim.exp(10, exp, pct(exp, 2_000)); // re-baseline
    sim.advance(20 * SEC);
    sim.exp(11, 50, 25); // level up: 600 to finish 10, 50 into 11
    let level_up_at = sim.t - 500; // accepted on the second frame
    sim.advance(10 * SEC);
    sim.cmd(Command::Stop);

    let r = sim.session.record().unwrap();
    assert_eq!(r.version, RECORD_VERSION);
    assert_eq!((r.started_ms, r.stopped_ms), (5 * SEC, sim.t));
    assert_eq!(r.paused_ms, MIN);
    assert_eq!(r.active_ms, sim.t - 5 * SEC - MIN);
    assert_eq!(r.exp_gained, 400 + 650);
    assert_eq!(r.start, ExpReading::new(10, 1_000, 5_000));
    assert_eq!(r.end, ExpReading::new(11, 50, 25));
    assert_eq!(r.level_ups.len(), 1);
    let up = r.level_ups[0];
    assert_eq!((up.at_ms, up.to_level), (level_up_at, 11));
    assert_eq!(up.active_ms, level_up_at - 5 * SEC - MIN);
    // Kills are accepted at 30.5, 60.5, 90.5 and 120.5 s of active time, so
    // each minute mark falls just before the 2nd and 4th.
    assert_eq!(r.exp_by_minute, vec![100, 300]);
    assert_eq!(r.meso_gained, Some(0));
}

#[test]
fn quiet_minutes_repeat_the_total() {
    let mut sim = Sim::new();
    sim.cmd(Command::Start);
    sim.exp(10, 1_000, 5_000);
    sim.exp(10, 1_100, 5_500);
    // Three minutes of nothing readable, then stop.
    for _ in 0..360 {
        sim.unreadable();
    }
    sim.cmd(Command::Stop);
    assert_eq!(sim.session.record().unwrap().exp_by_minute, vec![100, 100, 100]);
}

#[test]
fn reset_clears_the_record() {
    let mut sim = Sim::new();
    sim.cmd(Command::Start);
    sim.exp(10, 1_000, 5_000);
    sim.cmd(Command::Stop);
    sim.cmd(Command::Reset);
    assert_eq!(sim.session.record(), None);
}
