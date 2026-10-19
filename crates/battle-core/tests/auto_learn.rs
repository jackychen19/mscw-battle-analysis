//! Learning the EXP font from real crops with nothing typed.

mod common;

use battle_core::calibration::ExpFontLearner;
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
