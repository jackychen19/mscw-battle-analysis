//! Dev tool: teach the notification reader from fixture frames, then read
//! every fixture and compare with its expected lines.
//!
//! cargo run -p battle-core --example notice_scan -- <bottom_top> <line_height> <pitch> <right> <teach frames...>

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use battle_core::capture::{Frame, RectPx};
use battle_core::parse::{Channel, Gray, NoticeReader, StackLayout, TextLine, similarity};
use battle_core::stats::{Line, LineKind};
use serde_json::Value;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let layout = StackLayout {
        bottom_top: args[0].parse().unwrap(),
        line_height: args[1].parse().unwrap(),
        pitch: args[2].parse().unwrap(),
        right: args[3].parse().unwrap(),
    };
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/grinding-01");
    let mut reader = NoticeReader::new(layout);
    for frame in &args[4..] {
        let n: u32 = frame.parse().unwrap();
        let lines: Vec<Option<(LineKind, u64)>> = expected(&dir, n).iter().rev().map(clean).collect();
        if let Err(e) = reader.teach(&crop(&dir, n), &lines) {
            eprintln!("teach {n}: {e}");
        }
    }
    let chars: String = reader.glyphs.glyphs.iter().map(|g| g.ch).collect();
    eprintln!("glyphs {chars:?}, prefixes {:?}", reader.prefixes.iter().map(|p| (p.kind, p.image.width(), p.gap)).collect::<Vec<_>>());

    // EXPLAIN=frame:slot prints that slot's glyph runs and best matches.
    if let Some((n, slot)) = std::env::var("EXPLAIN").ok().and_then(|v| {
        let (a, b) = v.split_once(':')?;
        Some((a.parse::<u32>().ok()?, b.parse::<u32>().ok()?))
    }) {
        let gray = Gray::from_frame(&crop(&dir, n), Channel::MinRG);
        let top = (layout.bottom_top - slot as f32 * layout.pitch).round() as u32 - 4;
        let band = gray.crop(RectPx { x: 0, y: top, w: layout.right, h: layout.line_height + 8 });
        let line = TextLine::from_gray(&band).unwrap();
        println!("otsu {} band rows {}", band.otsu_threshold(), line.bitmap.height());
        for (i, run) in line.glyphs.iter().enumerate().rev().take(7) {
            let glyph = line.glyph(i);
            let mut scores: Vec<(char, f32)> = reader.glyphs.glyphs.iter().map(|t| (t.ch, similarity(&glyph, &t.image))).collect();
            scores.sort_by(|a, b| b.1.total_cmp(&a.1));
            scores.dedup_by_key(|s| s.0);
            let top: Vec<String> = scores.iter().take(3).map(|(c, s)| format!("{c} {s:.3}")).collect();
            println!("run {run:?} w{}: {}", run.end - run.start, top.join(" | "));
        }
        for ch in ['3', '8'] {
            for t in reader.glyphs.glyphs.iter().filter(|t| t.ch == ch) {
                let g = &t.image;
                let thr = g.otsu_threshold();
                println!("template {ch} {}x{} (otsu {thr})", g.width(), g.height());
                g.binarize(thr).to_ascii().iter().for_each(|r| println!("   {r}"));
            }
        }
        let rows = line.bitmap.to_ascii();
        let from = line.glyphs.iter().rev().nth(6).map_or(0, |r| r.start) as usize;
        rows.iter().for_each(|r| println!("{}", &r[from..]));
        return;
    }

    for n in [1, 29, 114, 151, 300, 380, 450, 465, 490, 600] {
        let got: Vec<String> = reader.read(&crop(&dir, n)).iter().map(fmt).collect();
        let want: Vec<String> = expected(&dir, n).iter().map(|l| {
            let kind = l["kind"].as_str().unwrap();
            let amount = if kind == "meso" { l["amount"].as_u64().map_or(String::new(), |a| format!("+{a}")) } else { String::new() };
            let faded = if l.get("faded").is_some() || l.get("note").is_some() { "~" } else { "" };
            format!("{faded}{kind}{amount}")
        }).collect();
        println!("{n:4} got  {}\n     want {}", got.join(" | "), want.join(" | "));
    }
}

/// A line clean enough to teach from.
fn clean(l: &Value) -> Option<(LineKind, u64)> {
    if l.get("faded").is_some() || l.get("note").is_some() {
        return None;
    }
    let kind = match l["kind"].as_str()? {
        "meso" => LineKind::Meso,
        "exp" => LineKind::Exp,
        "partyBonus" => LineKind::PartyBonus,
        _ => return None,
    };
    Some((kind, l["amount"].as_u64()?))
}

fn fmt(l: &Line) -> String {
    match l.kind {
        LineKind::Unknown => format!("?({})", l.freshness),
        LineKind::Meso => format!("meso+{}({})", l.amount, l.freshness),
        LineKind::Exp => format!("exp({})", l.freshness),
        LineKind::PartyBonus => format!("partyBonus({})", l.freshness),
        LineKind::Other => format!("other({})", l.freshness),
    }
}

fn expected(dir: &Path, n: u32) -> Vec<Value> {
    let v: Value = serde_json::from_str(&std::fs::read_to_string(dir.join(format!("{n:04}.json"))).unwrap()).unwrap();
    v["notifications"].as_array().unwrap().clone()
}

fn crop(dir: &Path, n: u32) -> Frame {
    load_png(&dir.join(format!("{n:04}.notif.png")))
}

fn load_png(path: &PathBuf) -> Frame {
    let mut decoder = png::Decoder::new(BufReader::new(File::open(path).expect("open png")));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().expect("png header");
    let mut buf = vec![0; reader.output_buffer_size().expect("png size")];
    let info = reader.next_frame(&mut buf).expect("png data");
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => buf[..info.buffer_size()].to_vec(),
        png::ColorType::Rgb => buf[..info.buffer_size()].chunks(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        other => panic!("unsupported png colour type {other:?}"),
    };
    Frame { width: info.width, height: info.height, stride: info.width as usize * 4, rgba }
}
