//! Reading the EXP text by shape, with nothing learned, on real captures.

mod common;

use battle_core::calibration::find_regions;
use battle_core::capture::RectPt;
use battle_core::parse::{ExpInk, ExpReader};

/// Status bars cut from screenshots: the ink the bar search should settle on,
/// the EXP text and the level they show.
const BARS: [(&str, ExpInk, (u64, u32), u32); 4] = [
    // Windowed: sharp, each game pixel exactly 2x2.
    ("native-01", ExpInk::Otsu, (117_832, 8498), 33),
    ("native-02", ExpInk::Otsu, (27_066, 1740), 34),
    // Fullscreen, stretched 1.998x to the display and smoothed.
    ("fullscreen-01", ExpInk::Soft, (50_182, 3226), 34),
    // Fullscreen at exactly 2x, the game's own UI still scaled up smoothly.
    ("fullscreen-02", ExpInk::Soft, (69_035, 4438), 34),
];

#[test]
fn reads_the_exp_text_by_shape_on_every_capture() {
    for (name, ink, exp, level) in BARS {
        let bar = common::load_png(&common::fixtures_dir().join(name).join("bar.png"));
        let area = RectPt { x: 0.0, y: 0.0, w: f64::from(bar.width), h: f64::from(bar.height) };
        let regions = find_regions(&bar, area, 1.0).unwrap_or_else(|| panic!("{name}: EXP text and level"));
        assert_eq!(regions.ink, ink, "{name}");
        // Nothing taught: both are read by shape.
        let reader = ExpReader { ink: regions.ink, ..ExpReader::default() };
        assert_eq!(reader.read_exp(&bar.crop(regions.exp_text.to_px(1.0))), Some(exp), "{name}");
        assert_eq!(reader.read_level(&bar.crop(regions.level.to_px(1.0))), Some(level), "{name}");
    }
}
