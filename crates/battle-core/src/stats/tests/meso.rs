use super::*;

#[test]
fn lines_on_screen_at_start_are_not_counted() {
    let mut sim = Sim::new();
    sim.cmd(Command::Start);
    sim.stack(&[meso(50, 180), meso(60, 255)]);
    assert_eq!(sim.snap().meso_gained, 0);
    sim.stack(&[meso(50, 170), meso(60, 240), meso(7, 255)]);
    assert_eq!(sim.snap().meso_gained, 7);
}

#[test]
fn duplicate_amounts_in_a_row_count_twice() {
    let mut sim = Sim::new();
    sim.cmd(Command::Start);
    sim.stack(&[]);
    sim.stack(&[meso(15, 255)]);
    sim.stack(&[meso(15, 235), meso(15, 255)]);
    sim.stack(&[meso(15, 215), meso(15, 235)]); // just fading
    assert_eq!(sim.snap().meso_gained, 30);
}

#[test]
fn identical_line_replacing_one_that_scrolled_off() {
    let mut sim = Sim::new();
    sim.cmd(Command::Start);
    sim.stack(&[]);
    sim.stack(&[meso(15, 255)]);
    sim.stack(&[meso(15, 90)]); // fading
    sim.stack(&[meso(15, 255)]); // old one gone, fresh one in
    assert_eq!(sim.snap().meso_gained, 30);
}

#[test]
fn burst_of_four_between_frames() {
    let mut sim = Sim::new();
    sim.cmd(Command::Start);
    sim.stack(&[]);
    sim.stack(&[meso(5, 255)]);
    sim.stack(&[meso(5, 200), meso(10, 255), meso(20, 255), meso(30, 255), meso(40, 255)]);
    let s = sim.snap();
    assert_eq!(s.meso_gained, 5 + 10 + 20 + 30 + 40);
    assert!(!s.meso_possible_undercount);
}

#[test]
fn burst_that_overflows_the_stack_is_flagged() {
    let mut sim = Sim::new();
    sim.cmd(Command::Start);
    sim.stack(&[]);
    sim.stack(&[meso(5, 255)]);
    sim.stack(&[meso(11, 255), meso(12, 255), meso(13, 255), meso(14, 255)]);
    let s = sim.snap();
    assert_eq!(s.meso_gained, 5 + 11 + 12 + 13 + 14);
    assert!(s.meso_possible_undercount);
}

#[test]
fn single_frame_stack_misread_is_ignored() {
    let mut sim = Sim::new();
    sim.cmd(Command::Start);
    sim.stack(&[meso(15, 255)]);
    sim.stack_frame(&[meso(16, 250)]); // misread for one frame
    sim.stack(&[meso(15, 245)]);
    assert_eq!(sim.snap().meso_gained, 0);
}

#[test]
fn only_meso_lines_are_counted() {
    let mut sim = Sim::new();
    sim.cmd(Command::Start);
    sim.stack(&[]);
    sim.stack(&[Line::new(LineKind::Exp, 0, 255), Line::new(LineKind::PartyBonus, 0, 255), meso(15, 255)]);
    let s = sim.snap();
    assert_eq!(s.meso_gained, 15);
    assert_eq!(s.exp_gained, 0);
}

#[test]
fn unreadable_frames_do_not_break_alignment() {
    let mut sim = Sim::new();
    sim.cmd(Command::Start);
    sim.stack(&[]);
    sim.stack(&[meso(15, 255)]);
    sim.unreadable();
    sim.unreadable();
    sim.stack(&[meso(15, 220), meso(9, 255)]);
    assert_eq!(sim.snap().meso_gained, 24);
}

#[test]
fn long_pickup_sequence_counts_each_line_once() {
    // A stack of up to 5 lines; each pickup appends at the bottom, the
    // oldest scrolls off once full; lines fade 10 per frame pair.
    let pickups = [15, 15, 22, 15, 15, 15, 8, 30, 30, 15, 15, 15, 15, 41];
    let mut sim = Sim::new();
    sim.cmd(Command::Start);
    sim.stack(&[]);
    let mut screen: Vec<Line> = Vec::new();
    for &amount in &pickups {
        for l in &mut screen {
            l.freshness = l.freshness.saturating_sub(40);
        }
        screen.push(meso(amount, 255));
        if screen.len() > 5 {
            screen.remove(0);
        }
        sim.stack(&screen);
    }
    assert_eq!(sim.snap().meso_gained, pickups.iter().sum::<u64>());
}

#[test]
fn stack_baseline_resets_on_resume() {
    let mut sim = Sim::new();
    sim.cmd(Command::Start);
    sim.stack(&[]);
    sim.cmd(Command::Pause);
    sim.stack(&[meso(100, 255)]); // picked up while paused: ignored
    sim.cmd(Command::Resume);
    sim.stack(&[meso(100, 230)]);
    sim.stack(&[meso(100, 210), meso(3, 255)]);
    assert_eq!(sim.snap().meso_gained, 3);
}

#[test]
fn covered_line_is_counted_once_it_becomes_readable() {
    let u = Line::new(LineKind::Unknown, 0, 0);
    let mut sim = Sim::new();
    sim.cmd(Command::Start);
    sim.stack(&[meso(5, 200)]);
    // A new line arrives under a monster…
    sim.stack(&[meso(5, 190), u]);
    assert_eq!(sim.snap().meso_gained, 0);
    // …and is readable once the monster moves.
    sim.stack(&[meso(5, 180), meso(30, 240)]);
    sim.stack(&[meso(5, 170), meso(30, 230)]);
    assert_eq!(sim.snap().meso_gained, 30);
}

#[test]
fn line_covered_later_is_not_counted_again() {
    let u = Line::new(LineKind::Unknown, 0, 0);
    let mut sim = Sim::new();
    sim.cmd(Command::Start);
    sim.stack(&[]);
    sim.stack(&[meso(30, 255)]);
    sim.stack(&[u]);
    sim.stack(&[meso(30, 230)]);
    assert_eq!(sim.snap().meso_gained, 30);
}

#[test]
fn unknown_line_already_on_screen_at_start_is_never_counted() {
    let u = Line::new(LineKind::Unknown, 0, 0);
    let mut sim = Sim::new();
    sim.cmd(Command::Start);
    sim.stack(&[u]);
    sim.stack(&[meso(30, 230)]);
    assert_eq!(sim.snap().meso_gained, 0);
}
