//! Dev tool: teach the EXP reader from a few frames, then read every frame.
//!
//! cargo run -p battle-core --example exp_scan -- fixtures/frames/grinding-01 \
//!     151:419165:3654:59 [N:EXP:PCT_HUNDREDTHS:LEVEL ...]
//!
//! Prints one line per frame: the parsed reading, or the raw glyph text when
//! it didn't parse (useful for spotting misreads).

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use battle_core::capture::Frame;
use battle_core::parse::{Channel, ExpReader, Gray, TextLine, exp_line, level_line, similarity};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((dir, teach)) = args.split_first() else {
        eprintln!("usage: exp_scan <frames dir> N:EXP:PCT:LEVEL ...");
        std::process::exit(2);
    };
    let dir = Path::new(dir);
    // `dump:N[:T]` / `dumplevel:N[:T]` print frame N's binarized EXP or level
    // line (threshold T, default Otsu) and its glyph runs.
    let dump = teach.first().and_then(|s| {
        s.strip_prefix("dump:").map(|r| ("exp", Channel::Luma, r)).or(s.strip_prefix("dumplevel:").map(|r| ("level", Channel::Min, r)))
    });
    if let Some((region, channel, spec)) = dump {
        let mut parts = spec.split(':').map(|v| v.parse::<u64>().expect("number"));
        let g = Gray::from_frame(&load(dir, region, parts.next().expect("frame")), channel);
        let line = match parts.next() {
            Some(t) => TextLine::from_bitmap(g.binarize(t as u8)),
            None => TextLine::from_gray(&g),
        }
        .expect("ink");
        println!("otsu {}, glyphs {:?}", g.otsu_threshold(), line.glyphs);
        line.bitmap.to_ascii().iter().for_each(|row| println!("{row}"));
        return;
    }
    let mut reader = ExpReader::default();
    for spec in teach {
        let v: Vec<u64> = spec.split(':').map(|s| s.parse().expect("number")).collect();
        let [n, exp, pct, level] = v[..] else { panic!("bad teach spec {spec}") };
        let (e, l) = (load(dir, "exp", n), load(dir, "level", n));
        reader.teach_exp(&e, exp, pct as u32).unwrap_or_else(|err| panic!("teach exp {n}: {err}"));
        reader.teach_level(&l, level as u32).unwrap_or_else(|err| panic!("teach level {n}: {err}"));
    }
    let chars: String = reader.exp_glyphs.glyphs.iter().map(|g| g.ch).collect();
    eprintln!("exp templates: {chars:?}");

    // `explain=N` (after the teach specs): top matches for each glyph of frame N.
    if let Some(n) = std::env::var("EXPLAIN").ok().and_then(|v| v.parse::<u64>().ok()) {
        let line = exp_line(&load(dir, "exp", n)).expect("ink");
        for i in 0..line.glyphs.len() {
            let glyph = line.glyph(i);
            let mut scores: Vec<(char, f32)> =
                reader.exp_glyphs.glyphs.iter().map(|t| (t.ch, similarity(&glyph, &t.image))).collect();
            scores.sort_by(|a, b| b.1.total_cmp(&a.1));
            scores.dedup_by_key(|s| s.0);
            let top: Vec<String> = scores.iter().take(3).map(|(c, s)| format!("{c} {s:.3}")).collect();
            println!("glyph {i:2} cols {:?}: {}", line.glyphs[i], top.join(" | "));
        }
        return;
    }

    let (mut ok, mut total) = (0, 0);
    for n in 1.. {
        let path = dir.join("exp").join(format!("{n:04}.png"));
        if !path.exists() {
            break;
        }
        total += 1;
        let (e, l) = (load(dir, "exp", n), load(dir, "level", n));
        let t = (n - 1) as f64 / 5.0;
        match reader.read(&e, &l) {
            Some(r) => {
                ok += 1;
                println!("{n:04} {t:6.1}s  Lv {} {} [{:.2}%]", r.level, r.exp, f64::from(r.pct_hundredths) / 100.0);
            }
            None => println!("{n:04} {t:6.1}s  ?? exp={:?} level={:?}", raw(&reader, &e, false), raw(&reader, &l, true)),
        }
    }
    eprintln!("{ok}/{total} frames read");
}

fn raw(reader: &ExpReader, crop: &Frame, level: bool) -> Option<String> {
    let (line, set) = if level { (level_line(crop)?, &reader.level_glyphs) } else { (exp_line(crop)?, &reader.exp_glyphs) };
    set.read(&line, 0.0).map(|r| format!("{} ({:.2})", r.text, r.score))
}

fn load(dir: &Path, region: &str, n: u64) -> Frame {
    load_png(&dir.join(region).join(format!("{n:04}.png")))
}

fn load_png(path: &Path) -> Frame {
    let file = File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut decoder = png::Decoder::new(BufReader::new(file));
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
