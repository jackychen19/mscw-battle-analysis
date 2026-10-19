//! Per-OS [`FrameSource`] implementations. Only macOS for now; other
//! platforms get a stub that finds no windows.

use battle_core::capture::FrameSource;
use serde::Serialize;

#[cfg(target_os = "macos")]
mod macos;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Permission {
    Granted,
    Denied,
}

#[cfg(target_os = "macos")]
pub use macos::{permission, request_permission};

#[cfg(target_os = "macos")]
pub fn frame_source() -> Box<dyn FrameSource + Send> {
    Box::new(macos::MacFrameSource::default())
}

#[cfg(not(target_os = "macos"))]
pub fn permission() -> Permission {
    Permission::Granted
}

#[cfg(not(target_os = "macos"))]
pub fn request_permission() -> Permission {
    Permission::Granted
}

#[cfg(not(target_os = "macos"))]
pub fn frame_source() -> Box<dyn FrameSource + Send> {
    Box::new(Unsupported)
}

#[cfg(not(target_os = "macos"))]
struct Unsupported;

#[cfg(not(target_os = "macos"))]
impl FrameSource for Unsupported {
    fn list_windows(&mut self) -> Result<Vec<battle_core::capture::WindowInfo>, battle_core::capture::CaptureError> {
        Ok(Vec::new())
    }

    fn select_window(&mut self, _: u64) -> Result<battle_core::capture::WindowInfo, battle_core::capture::CaptureError> {
        Err(battle_core::capture::CaptureError::WindowNotFound)
    }

    fn capture_region(&mut self, _: battle_core::capture::RectPt) -> Result<battle_core::capture::Frame, battle_core::capture::CaptureError> {
        Err(battle_core::capture::CaptureError::WindowNotFound)
    }
}
