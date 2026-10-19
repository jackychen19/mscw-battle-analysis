//! macOS capture via ScreenCaptureKit: one-shot screenshots of just the
//! requested sub-rectangle of the game window. Read-only pixels; nothing is
//! written to disk.

use battle_core::capture::{CaptureError, Frame, FrameSource, RectPt, RectPx, WindowInfo};
use core_graphics::access::ScreenCaptureAccess;
use screencapturekit::cg::CGRect;
use screencapturekit::error::SCError;
use screencapturekit::prelude::*;
use screencapturekit::screenshot_manager::SCScreenshotManager;

use super::Permission;

pub fn permission() -> Permission {
    if ScreenCaptureAccess.preflight() { Permission::Granted } else { Permission::Denied }
}

/// Shows the system prompt the first time; afterwards the user has to allow
/// the app in System Settings (and usually relaunch it).
pub fn request_permission() -> Permission {
    if ScreenCaptureAccess.request() { Permission::Granted } else { Permission::Denied }
}

struct Selected {
    filter: SCContentFilter,
    info: WindowInfo,
}

#[derive(Default)]
pub struct MacFrameSource {
    selected: Option<Selected>,
}

impl MacFrameSource {
    fn shareable_windows() -> Result<Vec<SCWindow>, CaptureError> {
        let content = SCShareableContent::create()
            .with_on_screen_windows_only(false)
            .with_exclude_desktop_windows(true)
            .get()
            .map_err(map_err)?;
        // Off-screen windows count too: a fullscreen game sits in its own Space.
        // Any window level: MapleStory Classic World's window sits at level 26.
        Ok(content.windows())
    }

    fn window_filter(w: &SCWindow) -> SCContentFilter {
        SCContentFilter::create().with_window(w).build()
    }

    fn info(w: &SCWindow, filter: &SCContentFilter) -> WindowInfo {
        let f = w.frame();
        WindowInfo {
            id: u64::from(w.window_id()),
            title: w.title().unwrap_or_default(),
            owner: w.owning_application().map(|a| a.application_name()).unwrap_or_default(),
            bounds: RectPt { x: f.x, y: f.y, w: f.width, h: f.height },
            scale: f64::from(filter.point_pixel_scale()),
            on_screen: w.is_on_screen(),
        }
    }
}

impl FrameSource for MacFrameSource {
    fn list_windows(&mut self) -> Result<Vec<WindowInfo>, CaptureError> {
        let mut out = Vec::new();
        for w in Self::shareable_windows()? {
            // Untitled or tiny: menu bar strips, helper windows.
            let f = w.frame();
            if w.title().is_none_or(|t| t.is_empty()) || f.width < 50.0 || f.height < 50.0 {
                continue;
            }
            out.push(Self::info(&w, &Self::window_filter(&w)));
        }
        Ok(out)
    }

    fn select_window(&mut self, id: u64) -> Result<WindowInfo, CaptureError> {
        let w = Self::shareable_windows()?
            .into_iter()
            .find(|w| u64::from(w.window_id()) == id)
            .ok_or(CaptureError::WindowNotFound)?;
        let filter = Self::window_filter(&w);
        let info = Self::info(&w, &filter);
        self.selected = Some(Selected { filter, info: info.clone() });
        Ok(info)
    }

    fn capture_region(&mut self, region: RectPt) -> Result<Frame, CaptureError> {
        let sel = self.selected.as_ref().ok_or(CaptureError::WindowNotFound)?;
        let scale = sel.info.scale;
        // Snap to whole pixels so every capture lands on the same pixel grid.
        let px = region.to_px(scale);
        if px.w == 0 || px.h == 0 {
            return Err(CaptureError::Platform("empty capture region".into()));
        }
        // ScreenCaptureKit smooths a source rect that doesn't start and end
        // on whole points (a half-point offset blurs 2x text past reading),
        // so capture the enclosing whole-point rect and crop the pixels.
        let (x0, y0) = ((f64::from(px.x) / scale).floor(), (f64::from(px.y) / scale).floor());
        let (x1, y1) = ((f64::from(px.x + px.w) / scale).ceil(), (f64::from(px.y + px.h) / scale).ceil());
        let config = SCStreamConfiguration::new()
            .with_source_rect(CGRect::new(x0, y0, x1 - x0, y1 - y0))
            .with_width(((x1 - x0) * scale).round() as u32)
            .with_height(((y1 - y0) * scale).round() as u32)
            .with_shows_cursor(false);
        let image = SCScreenshotManager::capture_image(&sel.filter, &config).map_err(map_err)?;
        let (width, height) = (image.width() as u32, image.height() as u32);
        let rgba = image.rgba_data().map_err(map_err)?;
        let whole = Frame { width, height, stride: width as usize * 4, rgba };
        let (dx, dy) = ((f64::from(px.x) - x0 * scale).round() as u32, (f64::from(px.y) - y0 * scale).round() as u32);
        Ok(whole.crop(RectPx { x: dx, y: dy, w: px.w, h: px.h }))
    }
}

fn map_err(e: SCError) -> CaptureError {
    match e {
        SCError::PermissionDenied(_) => CaptureError::PermissionDenied,
        SCError::WindowNotFound(_) => CaptureError::WindowNotFound,
        _ if permission() == Permission::Denied => CaptureError::PermissionDenied,
        other => CaptureError::Platform(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use battle_core::calibration::{find_game_window, find_regions};
    use battle_core::parse::{ExpReader, exp_line};

    use super::*;

    /// Needs the game open and Screen Recording permission. Checks that a
    /// region capture matches the same pixels cut from a whole-window
    /// capture, and prints what the app would read. Saves nothing.
    #[test]
    #[ignore = "needs the game running"]
    fn live_game_window() {
        // A bare test binary has no window-server connection; this opens one.
        let _ = core_graphics::display::CGDisplay::main();
        let mut src = MacFrameSource::default();
        let windows = src.list_windows().unwrap();
        let game = find_game_window(&windows, None).expect("game window").clone();
        println!("game: {game:?}");
        let game = src.select_window(game.id).unwrap();
        let area = RectPt { x: 0.0, y: 0.0, w: game.bounds.w, h: game.bounds.h };
        let frame = src.capture_region(area).unwrap();
        let regions = find_regions(&frame, area, game.scale).expect("EXP text and level");
        println!("regions: {regions:?}");
        for (name, r) in [("exp", regions.exp_text), ("level", regions.level)] {
            let own = src.capture_region(r).unwrap();
            let cut = frame.crop(r.to_px(game.scale));
            assert_eq!((own.width, own.height), (cut.width, cut.height), "{name} size");
            let diff: u64 = (0..own.height)
                .flat_map(|y| (0..own.width).map(move |x| (x, y)))
                .map(|(x, y)| (0..3).map(|i| u64::from(own.pixel(x, y)[i].abs_diff(cut.pixel(x, y)[i]))).sum::<u64>())
                .sum();
            let mean = diff as f64 / f64::from(own.width * own.height * 3);
            let reader = ExpReader::default();
            println!(
                "{name}: mean diff vs whole-window cut {mean:.2}; glyphs {:?}; level by shape {:?}",
                exp_line(&own).map(|l| l.glyphs.len()),
                reader.read_level(&own)
            );
        }
    }

    /// Needs the game open while EXP changes (kills). Learns the EXP font from
    /// the live window the way the app does, for up to a minute.
    #[test]
    #[ignore = "needs the game running"]
    fn live_learning() {
        use battle_core::calibration::ExpFontLearner;
        let _ = core_graphics::display::CGDisplay::main();
        let mut src = MacFrameSource::default();
        let game = find_game_window(&src.list_windows().unwrap(), None).expect("game window").clone();
        let game = src.select_window(game.id).unwrap();
        let area = RectPt { x: 0.0, y: 0.0, w: game.bounds.w, h: game.bounds.h };
        let regions = find_regions(&src.capture_region(area).unwrap(), area, game.scale).expect("regions");
        let mut learner = ExpFontLearner::default();
        let start = std::time::Instant::now();
        while start.elapsed() < std::time::Duration::from_secs(60) {
            let crop = src.capture_region(regions.exp_text).unwrap();
            let before = learner.sample_count();
            learner.add(&crop);
            if learner.sample_count() > before {
                let t = std::time::Instant::now();
                let solved = learner.solve();
                println!("{:>4.1}s: {} samples, solve took {:?}: {}", start.elapsed().as_secs_f32(), learner.sample_count(), t.elapsed(), solved.is_some());
                if let Some(font) = solved {
                    let reader = ExpReader { exp_glyphs: font, ..ExpReader::default() };
                    let level = src.capture_region(regions.level).unwrap();
                    println!("reads: {:?}", reader.read(&src.capture_region(regions.exp_text).unwrap(), &level));
                    return;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        panic!("not solved in a minute:\n{}", learner.describe());
    }
}
