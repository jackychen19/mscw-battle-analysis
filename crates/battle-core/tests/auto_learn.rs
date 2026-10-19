//! Learning the EXP font from real crops with nothing typed.

mod common;

use battle_core::calibration::{ExpFontLearner, find_exp_text, find_level};
use battle_core::capture::RectPx;
use battle_core::parse::ExpReader;
use common::Fixture;

#[test]
#[ignore = "grinding-01 is a rescaled video: one character blurs differently per frame (6 and 8 score closer than two 0s), \
            so glyphs can't be grouped. Direct window captures draw each character identically."]
fn learns_the_exp_font_from_grinding_01_without_typing() {
    let fixtures = Fixture::all("grinding-01");
    let mut learner = ExpFontLearner::default();
    for f in &fixtures {
        assert!(learner.add(&f.crop("exp")), "frame {}", f.frame);
    }
    let font = learner.solve().expect("a unique solution");
    let reader = ExpReader { exp_glyphs: font, ..ExpReader::default() };
    for f in &fixtures {
        let want = &f.expected["exp"];
        let want = (want["exp"].as_u64().unwrap(), want["pctHundredths"].as_u64().unwrap() as u32);
        assert_eq!(reader.read_exp(&f.crop("exp")), Some(want), "frame {}", f.frame);
    }
}

#[test]
#[ignore = "dev measurement"]
fn glyph_similarity_spread() {
    use battle_core::parse::{exp_line, format_exp_text, similarity};
    let mut glyphs = Vec::new();
    for f in Fixture::all("grinding-01") {
        let e = &f.expected["exp"];
        let text = format_exp_text(e["exp"].as_u64().unwrap(), e["pctHundredths"].as_u64().unwrap() as u32);
        let line = exp_line(&f.crop("exp")).unwrap();
        if line.glyphs.len() != text.len() {
            eprintln!("frame {}: {} glyphs for {text}", f.frame, line.glyphs.len());
            continue;
        }
        glyphs.extend(text.chars().enumerate().map(|(i, c)| (c, line.glyph(i))));
    }
    let mut same_min = std::collections::BTreeMap::new();
    let mut diff_max = std::collections::BTreeMap::new();
    for (i, (a, ga)) in glyphs.iter().enumerate() {
        for (b, gb) in &glyphs[i + 1..] {
            let s = similarity(ga, gb);
            if a == b {
                let e = same_min.entry(*a).or_insert(1.0f32);
                *e = e.min(s);
            } else {
                let key = if a < b { (*a, *b) } else { (*b, *a) };
                let e = diff_max.entry(key).or_insert(0.0f32);
                *e = e.max(s);
            }
        }
    }
    eprintln!("same-char min: {same_min:?}");
    let mut worst: Vec<_> = diff_max.into_iter().collect();
    worst.sort_by(|a, b| b.1.total_cmp(&a.1));
    eprintln!("closest different pairs: {:?}", &worst[..12]);
}

/// Is `inner` (mostly) inside `outer`?
fn covers(outer: RectPx, inner: RectPx) -> bool {
    outer.x <= inner.x + 3 && outer.y <= inner.y + 3 && outer.x + outer.w + 3 >= inner.x + inner.w && outer.y + outer.h + 3 >= inner.y + inner.h
}

#[test]
fn finds_exp_and_level_on_the_status_bar() {
    // The bottom bar of frame 465, with a streamer's overlay (another LV and
    // EXP readout) on the right as a decoy.
    let bar = common::load_png(&common::fixtures_dir().join("grinding-01/bar-0465.png"));
    let exp = find_exp_text(&bar).expect("EXP line");
    let level = find_level(&bar).expect("level");
    // Where the confirmed fixture crops were cut (video pixels, minus the strip's origin).
    assert!(covers(exp, RectPx { x: 885, y: 66, w: 180, h: 18 }), "{exp:?}");
    assert!(exp.x > 870 && exp.w < 320, "label excluded, not too wide: {exp:?}");
    assert!(covers(level, RectPx { x: 52, y: 80, w: 44, h: 26 }) && level.w < 70, "{level:?}");

    // What was found reads back with the reader taught on the fixtures.
    let reader = common::exp_reader();
    assert_eq!(reader.read_exp(&bar.crop(exp)), Some((422_579, 3684)));
    assert_eq!(reader.read_level(&bar.crop(level)), Some(59));
}
