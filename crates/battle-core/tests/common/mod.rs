//! Shared helpers for fixture-based tests.

#![allow(dead_code)]

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use battle_core::capture::Frame;
use serde_json::Value;

pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// Extracted frames from a recording (gitignored; tests skip without them).
pub fn frames_dir(name: &str) -> Option<PathBuf> {
    let dir = fixtures_dir().join("frames").join(name);
    dir.is_dir().then_some(dir)
}

/// A committed fixture frame: its crops and expected values.
pub struct Fixture {
    pub frame: u32,
    pub expected: Value,
    dir: PathBuf,
}

impl Fixture {
    pub fn load(set: &str, frame: u32) -> Self {
        let dir = fixtures_dir().join(set);
        let json = std::fs::read_to_string(dir.join(format!("{frame:04}.json"))).expect("fixture json");
        Self { frame, expected: serde_json::from_str(&json).expect("valid json"), dir }
    }

    pub fn all(set: &str) -> Vec<Self> {
        let mut frames: Vec<u32> = std::fs::read_dir(fixtures_dir().join(set))
            .expect("fixture dir")
            .filter_map(|e| e.ok()?.file_name().to_str()?.strip_suffix(".json")?.parse().ok())
            .collect();
        frames.sort();
        frames.into_iter().map(|f| Self::load(set, f)).collect()
    }

    /// `region` is `exp`, `level` or `notif`.
    pub fn crop(&self, region: &str) -> Frame {
        load_png(&self.dir.join(format!("{:04}.{region}.png", self.frame)))
    }
}

pub fn load_png(path: &Path) -> Frame {
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
