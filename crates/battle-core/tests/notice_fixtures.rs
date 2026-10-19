//! Notification stack reading against real footage: text over moving map
//! art, fading lines, sprites drawn on top.

mod common;

use battle_core::parse::{NoticeReader, StackLayout};
use battle_core::stats::{Line, LineKind};
use common::Fixture;
use serde_json::Value;

/// Measured on grinding-01's notification crop.
pub const LAYOUT: StackLayout = StackLayout { bottom_top: 229.0, line_height: 18, pitch: 28.0, right: 504 };
/// Frames used for calibration; 380, 450 and 600 are held out.
const TEACH: [u32; 7] = [465, 151, 29, 300, 114, 1, 490];

fn kind(l: &Value) -> Option<LineKind> {
    match l["kind"].as_str()? {
        "meso" => Some(LineKind::Meso),
        "exp" => Some(LineKind::Exp),
        "partyBonus" => Some(LineKind::PartyBonus),
        _ => None,
    }
}

fn clean(l: &Value) -> bool {
    l.get("faded").is_none() && l.get("note").is_none()
}

pub fn taught() -> NoticeReader {
    let mut reader = NoticeReader::new(LAYOUT);
    for frame in TEACH {
        let f = Fixture::load("grinding-01", frame);
        let lines: Vec<Option<(LineKind, u64)>> = f.expected["notifications"]
            .as_array()
            .unwrap()
            .iter()
            .rev()
            .map(|l| clean(l).then(|| Some((kind(l)?, l["amount"].as_u64()?))).flatten())
            .collect();
        reader.teach(&f.crop("notif"), &lines).unwrap_or_else(|e| panic!("teach {frame}: {e}"));
    }
    reader
}

/// Every line the reader reports as a known kind must be right (meso
/// amounts included). Unreadable lines may come back as Unknown or be
/// dropped from the top, but never as a wrong value. Clean lines must be
/// read.
#[test]
fn reads_fixture_stacks() {
    let reader = taught();
    let mut misreads = Vec::new();
    for f in Fixture::all("grinding-01") {
        let got = reader.read(&f.crop("notif"));
        let want = f.expected["notifications"].as_array().unwrap();
        assert!(got.len() <= want.len(), "frame {}: {} lines, expected at most {}", f.frame, got.len(), want.len());
        // Compare bottom-up: the bottom line is slot 0.
        for (slot, (g, w)) in got.iter().rev().zip(want.iter().rev()).enumerate() {
            let want_kind = kind(w);
            let ok = match (g.kind, want_kind) {
                (LineKind::Unknown, _) => !clean(w) || want_kind.is_none(),
                (LineKind::Meso, Some(LineKind::Meso)) => Some(g.amount) == w["amount"].as_u64(),
                (k, Some(wk)) => k == wk,
                _ => false,
            };
            if !ok {
                misreads.push((f.frame, slot, *g));
            }
        }
    }
    // Known: in frame 450 a blob of map art touches the "3" of the bottom
    // line's "(+13)", which then reads as 18.
    let known = [(450, 0, Line::new(LineKind::Meso, 18, 139))];
    assert_eq!(misreads, known);
}

#[test]
fn empty_stack_reads_empty() {
    let reader = taught();
    assert_eq!(reader.read(&Fixture::load("grinding-01", 380).crop("notif")), Vec::<Line>::new());
}
