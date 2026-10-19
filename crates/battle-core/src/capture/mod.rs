//! Screen-capture abstraction. Read-only: implementations may only *read pixels*
//! of the game window. No memory access, no injection, no input.
//!
//! Coordinates come in two flavours and the display scale factor is always
//! explicit (Retina is 2.0, Windows commonly 1.25 / 1.5) — never assume one.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Rectangle in logical points (scale-independent).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RectPt {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Rectangle in physical pixels of a captured frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RectPx {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl RectPt {
    /// Converts to pixels at `scale`, rounding outward so the pixel rect
    /// always covers the whole point rect.
    pub fn to_px(self, scale: f64) -> RectPx {
        let x0 = (self.x * scale).floor().max(0.0);
        let y0 = (self.y * scale).floor().max(0.0);
        let x1 = ((self.x + self.w) * scale).ceil().max(x0);
        let y1 = ((self.y + self.h) * scale).ceil().max(y0);
        RectPx {
            x: x0 as u32,
            y: y0 as u32,
            w: (x1 - x0) as u32,
            h: (y1 - y0) as u32,
        }
    }
}

/// A capturable top-level window.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    /// Platform window id (CGWindowID on macOS, HWND on Windows).
    pub id: u64,
    pub title: String,
    pub owner: String,
    /// Window content bounds on screen, in points.
    pub bounds: RectPt,
    /// Scale factor of the display the window is on.
    pub scale: f64,
    /// Visible on the current Space (a fullscreen game in another Space isn't).
    pub on_screen: bool,
}

/// An RGBA8 image, either a whole window or a sub-rectangle of one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    /// Bytes per row (may exceed `width * 4` due to platform padding).
    pub stride: usize,
    pub rgba: Vec<u8>,
}

impl Frame {
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = y as usize * self.stride + x as usize * 4;
        [self.rgba[i], self.rgba[i + 1], self.rgba[i + 2], self.rgba[i + 3]]
    }

    /// The part of the frame inside `r`, clamped to the frame.
    pub fn crop(&self, r: RectPx) -> Frame {
        let x0 = r.x.min(self.width);
        let y0 = r.y.min(self.height);
        let w = r.w.min(self.width - x0);
        let h = r.h.min(self.height - y0);
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for y in y0..y0 + h {
            let row = y as usize * self.stride + x0 as usize * 4;
            rgba.extend_from_slice(&self.rgba[row..row + w as usize * 4]);
        }
        Frame { width: w, height: h, stride: w as usize * 4, rgba }
    }

    /// Nearest-neighbour resample so one output pixel covers `factor` input
    /// pixels (e.g. a Retina frame at `factor` 2.0 comes out in points).
    pub fn downsample(&self, factor: f64) -> Frame {
        let width = ((f64::from(self.width) / factor).floor() as u32).max(1);
        let height = ((f64::from(self.height) / factor).floor() as u32).max(1);
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            let sy = ((f64::from(y) * factor) as u32).min(self.height - 1);
            for x in 0..width {
                let sx = ((f64::from(x) * factor) as u32).min(self.width - 1);
                rgba.extend_from_slice(&self.pixel(sx, sy));
            }
        }
        Frame { width, height, stride: width as usize * 4, rgba }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaptureError {
    /// The OS has not granted screen-capture permission.
    PermissionDenied,
    WindowNotFound,
    /// The window exists but no frame is available right now (minimised, etc.).
    NoFrame,
    Platform(String),
}

impl fmt::Display for CaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PermissionDenied => write!(f, "screen recording permission not granted"),
            Self::WindowNotFound => write!(f, "game window not found"),
            Self::NoFrame => write!(f, "no frame available"),
            Self::Platform(msg) => write!(f, "capture error: {msg}"),
        }
    }
}

impl std::error::Error for CaptureError {}

/// Source of game-window pixels. One implementation per OS.
pub trait FrameSource {
    /// Windows that can be captured, for the window picker.
    fn list_windows(&mut self) -> Result<Vec<WindowInfo>, CaptureError>;

    /// Selects the window subsequent captures read from.
    fn select_window(&mut self, id: u64) -> Result<WindowInfo, CaptureError>;

    /// Latest frame of the selected window, cropped to `region` (points,
    /// relative to the window's content origin). Implementations should
    /// capture only the sub-rectangle when the platform allows it.
    fn capture_region(&mut self, region: RectPt) -> Result<Frame, CaptureError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_px_rounds_outward_at_fractional_scales() {
        let r = RectPt { x: 10.3, y: 5.0, w: 20.0, h: 7.1 };
        assert_eq!(r.to_px(1.0), RectPx { x: 10, y: 5, w: 21, h: 8 });
        assert_eq!(r.to_px(2.0), RectPx { x: 20, y: 10, w: 41, h: 15 });
        assert_eq!(r.to_px(1.25), RectPx { x: 12, y: 6, w: 26, h: 10 });
    }

    /// 4×3 frame whose red channel is `10 * y + x`, with row padding.
    fn numbered() -> Frame {
        let stride = 4 * 4 + 8;
        let mut rgba = vec![0; stride * 3];
        for y in 0..3 {
            for x in 0..4 {
                rgba[y * stride + x * 4] = (10 * y + x) as u8;
            }
        }
        Frame { width: 4, height: 3, stride, rgba }
    }

    #[test]
    fn crop_clamps_and_drops_padding() {
        let c = numbered().crop(RectPx { x: 1, y: 1, w: 10, h: 1 });
        assert_eq!((c.width, c.height, c.stride), (3, 1, 12));
        assert_eq!([c.pixel(0, 0)[0], c.pixel(2, 0)[0]], [11, 13]);
        assert_eq!(numbered().crop(RectPx { x: 9, y: 9, w: 2, h: 2 }).width, 0);
    }

    #[test]
    fn downsample_picks_every_nth_pixel() {
        let d = numbered().downsample(2.0);
        assert_eq!((d.width, d.height), (2, 1));
        assert_eq!([d.pixel(0, 0)[0], d.pixel(1, 0)[0]], [0, 2]);
        assert_eq!(numbered().downsample(1.0), Frame { stride: 16, rgba: numbered().crop(RectPx { x: 0, y: 0, w: 4, h: 3 }).rgba, ..numbered() });
    }
}
