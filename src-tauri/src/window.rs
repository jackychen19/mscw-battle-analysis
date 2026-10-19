//! Floating-panel behaviour, per OS. Milestone 1 uses Tauri's portable
//! always-on-top; Milestone 4 adds a macOS NSPanel implementation that joins
//! fullscreen Spaces without stealing focus.

use tauri::WebviewWindow;

pub trait FloatingWindow {
    fn make_floating(&self, window: &WebviewWindow) -> tauri::Result<()>;
}

/// Portable fallback: always on top, on every workspace.
pub struct BasicFloating;

impl FloatingWindow for BasicFloating {
    fn make_floating(&self, window: &WebviewWindow) -> tauri::Result<()> {
        window.set_always_on_top(true)?;
        window.set_visible_on_all_workspaces(true)
    }
}
