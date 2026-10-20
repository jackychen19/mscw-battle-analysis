//! Dev tool: run the status bar search on whole-window PNGs.
//!
//! cargo run -p battle-core --example locate_scan -- shot.png [more.png ...]
//!
//! Prints where the EXP text and level were found in each (or `None`) and
//! what the level reads.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use battle_core::calibration::{find_exp_text, find_level_near};
use battle_core::capture::Frame;
use battle_core::parse::ExpReader;

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: locate_scan <window.png> ...");
        std::process::exit(2);
    }
    for path in &paths {
        let frame = load_png(Path::new(path));
        let exp = find_exp_text(&frame);
        let level = exp.and_then(|e| find_level_near(&frame, e));
        let reads = level.and_then(|l| ExpReader::default().read_level(&frame.crop(l)));
        println!("{path} ({}x{}): exp {exp:?}, level {level:?} reads {reads:?}", frame.width, frame.height);
    }
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
