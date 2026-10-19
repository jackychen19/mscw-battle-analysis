//! EXP and level reading against real (blurry, video-compressed) footage.

mod common;

use battle_core::parse::ExpReader;
use battle_core::stats::ExpReading;
use common::Fixture;

/// Frames used for calibration; the rest are held out.
const TEACH: [u32; 6] = [1, 29, 114, 300, 465, 600];

fn expected(f: &Fixture) -> ExpReading {
    let e = &f.expected["exp"];
    let n = |k: &str| e[k].as_u64().unwrap();
    ExpReading::new(n("level") as u32, n("exp"), n("pctHundredths") as u32)
}

fn taught() -> ExpReader {
    let mut reader = ExpReader::default();
    for frame in TEACH {
        let f = Fixture::load("grinding-01", frame);
        let want = expected(&f);
        reader.teach_exp(&f.crop("exp"), want.exp, want.pct_hundredths).unwrap();
        reader.teach_level(&f.crop("level"), want.level).unwrap();
    }
    reader
}

#[test]
fn reads_every_fixture_frame() {
    let reader = taught();
    let fixtures = Fixture::all("grinding-01");
    assert_eq!(fixtures.len(), 10);
    for f in fixtures {
        let got = reader.read(&f.crop("exp"), &f.crop("level"));
        assert_eq!(got, Some(expected(&f)), "frame {}", f.frame);
    }
}

#[test]
fn held_out_frames_cover_every_digit() {
    let held_out: String = Fixture::all("grinding-01")
        .iter()
        .filter(|f| !TEACH.contains(&f.frame))
        .map(|f| expected(f).exp.to_string())
        .collect();
    let missing: Vec<char> = ('0'..='9').filter(|d| !held_out.contains(*d)).collect();
    // 419165, 421770, 422518, 422906: every digit but 3 appears in held-out EXP.
    assert_eq!(missing, vec!['3']);
}

/// Every extracted frame (0–122 s) must read, and readings must be
/// self-consistent: percent agrees with EXP for this level's max, and EXP
/// never decreases (no deaths in this clip). Skips without extracted frames.
#[test]
fn whole_clip_reads_consistently() {
    let Some(dir) = common::frames_dir("grinding-01") else {
        eprintln!("skipping: run scripts/extract-crops.sh first");
        return;
    };
    let reader = taught();
    // Level 59 max lies in 1,146,880..=1,147,034 (from the fixture readings).
    let max = 1_146_950u64;
    let mut prev = 0;
    let mut n = 0;
    for i in 1.. {
        let path = |r: &str| dir.join(r).join(format!("{i:04}.png"));
        if !path("exp").exists() {
            break;
        }
        let r = reader
            .read(&common::load_png(&path("exp")), &common::load_png(&path("level")))
            .unwrap_or_else(|| panic!("frame {i} unreadable"));
        let implied = (r.exp * 10_000 / max) as u32;
        assert!(r.pct_hundredths.abs_diff(implied) <= 1, "frame {i}: {r:?} implies {implied}");
        assert!(r.exp >= prev, "frame {i}: EXP went down to {} from {prev}", r.exp);
        prev = r.exp;
        n += 1;
    }
    assert_eq!(n, 610);
}
