//! Meso counting end to end: every notification crop of a clip read by the
//! parser and fed through the stats engine at 5 fps.

mod common;

use battle_core::stats::{Command, Line, LineKind, Observation, Session, SessionConfig};

/// The clip's 5 fps crops, read and counted. Returns each frame where the
/// meso total went up, with the new total. `None` without extracted frames.
fn count_clip(name: &str) -> Option<Vec<(u32, u64)>> {
    let dir = common::frames_dir(name)?;
    let reader = common::notice_reader();
    let mut session = Session::new(SessionConfig::default());
    session.apply(0, Command::Start).unwrap();
    let mut events = Vec::new();
    let mut total = 0;
    for i in 1.. {
        let path = dir.join("notif").join(format!("{i:04}.png"));
        if !path.exists() {
            break;
        }
        let t_ms = u64::from(i - 1) * 200;
        let stack = reader.read(&common::load_png(&path));
        if std::env::var_os("VERBOSE").is_some() {
            eprintln!("FRAME {i:4} {}", fmt(&stack));
        }
        session.observe(t_ms, Observation::Stack(stack));
        let now = session.snapshot(t_ms).meso_gained;
        if now != total {
            events.push((i, now - total));
            total = now;
        }
    }
    Some(events)
}

fn fmt(stack: &[Line]) -> String {
    let parts: Vec<String> = stack
        .iter()
        .map(|l| match l.kind {
            LineKind::Meso => format!("meso+{}({})", l.amount, l.freshness),
            LineKind::Unknown => format!("?({})", l.freshness),
            k => format!("{k:?}({})", l.freshness),
        })
        .collect();
    parts.join(" | ")
}

/// Exploratory: prints what the tracker counts. Mesos are on hold until the
/// clip's true pickups are confirmed (SPEC §9); slow, so ignored by default.
#[test]
#[ignore]
fn grinding_01_meso_pickups() {
    let Some(events) = count_clip("grinding-01") else {
        eprintln!("skipping: run scripts/extract-crops.sh first");
        return;
    };
    for (frame, amount) in &events {
        println!("frame {frame:4} ({:5.1} s): +{amount}", f64::from(frame - 1) / 5.0);
    }
    println!("total {}", events.iter().map(|e| e.1).sum::<u64>());
}

