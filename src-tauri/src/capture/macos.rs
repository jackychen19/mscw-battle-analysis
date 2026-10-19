//! macOS capture via ScreenCaptureKit: one-shot screenshots of just the
//! requested sub-rectangle of the game window. Read-only pixels; nothing is
//! written to disk.

use battle_core::capture::{CaptureError, Frame, FrameSource, RectPt, WindowInfo};
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
            .with_on_screen_windows_only(true)
            .with_exclude_desktop_windows(true)
            .get()
            .map_err(map_err)?;
        // Layer 0 = ordinary app windows (no menu bar, dock, overlays).
        Ok(content.windows().into_iter().filter(|w| w.window_layer() == 0).collect())
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
        }
    }
}

impl FrameSource for MacFrameSource {
    fn list_windows(&mut self) -> Result<Vec<WindowInfo>, CaptureError> {
        let mut out = Vec::new();
        for w in Self::shareable_windows()? {
            if w.title().is_none_or(|t| t.is_empty()) {
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
        let source = CGRect::new(
            f64::from(px.x) / scale,
            f64::from(px.y) / scale,
            f64::from(px.w) / scale,
            f64::from(px.h) / scale,
        );
        let config = SCStreamConfiguration::new()
            .with_source_rect(source)
            .with_width(px.w)
            .with_height(px.h)
            .with_shows_cursor(false);
        let image = SCScreenshotManager::capture_image(&sel.filter, &config).map_err(map_err)?;
        let (width, height) = (image.width() as u32, image.height() as u32);
        let rgba = image.rgba_data().map_err(map_err)?;
        Ok(Frame { width, height, stride: width as usize * 4, rgba })
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
