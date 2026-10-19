//! Dev tool: per-row ink counts of a crop, to find text line bands.
//!
//! cargo run -p battle-core --example row_profile -- <png> [threshold]

use std::fs::File;
use std::io::BufReader;

use battle_core::capture::Frame;
use battle_core::parse::{Channel, Gray};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let frame = load_png(&args[0]);
    let channel = if std::env::var("CHANNEL").as_deref() == Ok("minrg") { Channel::MinRG } else { Channel::Luma };
    let gray = Gray::from_frame(&frame, channel);
    let t = args.get(1).map_or_else(|| gray.otsu_threshold(), |t| t.parse().expect("threshold"));
    let ink = gray.binarize(t);
    println!("{}x{}, threshold {t}", ink.width(), ink.height());
    for y in 0..ink.height() {
        let row: Vec<u32> = (0..ink.width()).filter(|&x| ink.get(x, y)).collect();
        let (first, last) = (row.first().copied().unwrap_or(0), row.last().copied().unwrap_or(0));
        println!("{y:3} {:4} {first:4}..{last:4} {}", row.len(), "#".repeat(row.len() / 4));
    }
}

fn load_png(path: &str) -> Frame {
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
