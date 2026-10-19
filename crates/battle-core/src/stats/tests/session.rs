use super::*;

#[test]
fn state_machine_transitions() {
    let mut s = Session::new(SessionConfig::default());
    assert_eq!(s.state(), SessionState::Idle);
    assert!(s.apply(0, Command::Pause).is_err());
    assert!(s.apply(0, Command::Stop).is_err());
    s.apply(0, Command::Start).unwrap();
    assert_eq!(s.apply(0, Command::Start), Err(TransitionError { from: SessionState::Recording, command: Command::Start }));
    assert!(s.apply(0, Command::Resume).is_err());
    s.apply(0, Command::Pause).unwrap();
    assert_eq!(s.state(), SessionState::Paused);
    assert!(s.apply(0, Command::Pause).is_err());
    s.apply(0, Command::Resume).unwrap();
    s.apply(0, Command::Stop).unwrap();
    assert_eq!(s.state(), SessionState::Stopped);
    assert!(s.apply(0, Command::Start).is_err());
    assert!(s.apply(0, Command::Resume).is_err());
    s.apply(0, Command::Reset).unwrap();
    assert_eq!(s.state(), SessionState::Idle);
}

#[test]
fn stop_while_paused() {
    let mut sim = Sim::new();
    sim.cmd(Command::Start);
    sim.advance(MIN);
    sim.cmd(Command::Pause);
    sim.advance(MIN);
    sim.cmd(Command::Stop);
    let s = sim.snap();
    assert_eq!((s.duration_ms, s.paused_ms), (MIN, MIN));
}

#[test]
fn pause_excludes_time_and_gains() {
    let max = 100_000;
    let mut sim = Sim::new();
    sim.cmd(Command::Start); // t = 0
    sim.exp(30, 10_000, pct(10_000, max));
    sim.advance(59 * SEC);
    sim.exp(30, 11_000, pct(11_000, max)); // t = 61 s
    sim.cmd(Command::Pause);
    // Readings while paused are ignored entirely.
    sim.exp(30, 20_000, pct(20_000, max));
    sim.advance(2 * MIN - SEC);
    let paused = sim.snap();
    assert_eq!(paused.state, SessionState::Paused);
    assert_eq!(paused.duration_ms, 61 * SEC);
    assert_eq!(paused.paused_ms, 2 * MIN);
    sim.cmd(Command::Resume);
    // Gains made while paused show up as the new baseline, not as EXP.
    sim.exp(30, 30_000, pct(30_000, max));
    sim.exp(30, 30_500, pct(30_500, max));
    sim.advance(57 * SEC);
    sim.cmd(Command::Stop);
    let s = sim.snap();
    assert_eq!(s.exp_gained, 1_000 + 500);
    assert_eq!(s.duration_ms, 2 * MIN);
    assert_eq!(s.paused_ms, 2 * MIN);
    assert_eq!(s.start_level, Some(30));
}

#[test]
fn stopped_results_are_frozen() {
    let max = 100_000;
    let mut sim = Sim::new();
    sim.cmd(Command::Start);
    sim.exp(30, 10_000, pct(10_000, max));
    sim.exp(30, 10_500, pct(10_500, max));
    sim.cmd(Command::Stop);
    let before = sim.snap();
    sim.advance(10 * MIN);
    sim.exp(30, 99_000, pct(99_000, max));
    sim.stack(&[meso(1_000, 255)]);
    assert_eq!(sim.snap(), before);
}

#[test]
fn reset_clears_stats_but_keeps_learned_level_max() {
    let mut sim = Sim::new();
    sim.cmd(Command::Start);
    sim.exp(10, 1_900, 9_500); // learns level 10 max ≈ 2000
    sim.cmd(Command::Stop);
    sim.cmd(Command::Reset);
    let s = sim.snap();
    assert_eq!(s.state, SessionState::Idle);
    assert_eq!((s.exp_gained, s.duration_ms, s.level), (0, 0, None));
    sim.cmd(Command::Start);
    sim.exp(10, 1_950, 9_750);
    sim.exp(11, 10, 50);
    assert_eq!(sim.snap().exp_gained, 50 + 10);
}

#[test]
fn snapshot_serializes_camel_case() {
    let json = serde_json::to_value(Session::new(SessionConfig::default()).snapshot(0)).unwrap();
    assert_eq!(json["state"], "idle");
    assert_eq!(json["expPerMinRecent"], 0.0);
    assert!(json.get("mesoPossibleUndercount").is_some());
}
