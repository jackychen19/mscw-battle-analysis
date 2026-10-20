//! Floating-panel behaviour, per OS. `BasicFloating` is Tauri's portable
//! always-on-top; on macOS `MacPanel` turns the window into a non-activating
//! NSPanel that sits over the fullscreen game without taking focus from it.

use tauri::WebviewWindow;

pub trait FloatingWindow {
    fn make_floating(&self, window: &WebviewWindow) -> tauri::Result<()>;
}

/// Portable fallback: always on top, on every workspace.
#[allow(dead_code)] // the fallback for platforms without their own impl yet
pub struct BasicFloating;

impl FloatingWindow for BasicFloating {
    fn make_floating(&self, window: &WebviewWindow) -> tauri::Result<()> {
        window.set_always_on_top(true)?;
        window.set_visible_on_all_workspaces(true)
    }
}

#[cfg(all(target_os = "macos", debug_assertions))]
pub use macos::log_objc_exceptions;
#[cfg(target_os = "macos")]
pub use macos::MacPanel;

#[cfg(target_os = "macos")]
mod macos {
    use objc2::runtime::{AnyObject, NSObject};
    use objc2::{define_class, ClassType, MainThreadOnly};
    use objc2_app_kit::{
        NSPanel, NSPopUpMenuWindowLevel, NSResponder, NSWindow, NSWindowCollectionBehavior,
        NSWindowStyleMask,
    };
    use tauri::WebviewWindow;

    use super::FloatingWindow;

    define_class!(
        // A panel that never becomes key: clicks reach the webview (acceptFirstMouse),
        // but keystrokes stay with the game.
        #[unsafe(super(NSPanel, NSWindow, NSResponder, NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "BattleAnalysisPanel"]
        struct BattlePanel;

        impl BattlePanel {
            #[unsafe(method(canBecomeKeyWindow))]
            fn can_become_key_window(&self) -> bool {
                false
            }

            #[unsafe(method(canBecomeMainWindow))]
            fn can_become_main_window(&self) -> bool {
                false
            }
        }
    );

    /// Debug builds: print every Objective-C exception (name, reason, stack) as it's
    /// thrown, since one that unwinds into Rust aborts with no detail.
    #[cfg(debug_assertions)]
    pub fn log_objc_exceptions() {
        use std::ffi::{c_char, CStr};

        use objc2::msg_send;

        unsafe fn utf8(s: *mut AnyObject) -> String {
            if s.is_null() {
                return String::new();
            }
            unsafe {
                let p: *const c_char = msg_send![&*s, UTF8String];
                CStr::from_ptr(p).to_string_lossy().into_owned()
            }
        }

        unsafe extern "C" fn log(exc: *mut AnyObject) -> *mut AnyObject {
            unsafe {
                let e = &*exc;
                let name: *mut AnyObject = msg_send![e, name];
                let reason: *mut AnyObject = msg_send![e, reason];
                // Filled in only once thrown, so print where we are now instead.
                let stack: *mut AnyObject = msg_send![objc2::class!(NSThread), callStackSymbols];
                let stack: *mut AnyObject = msg_send![&*stack, description];
                eprintln!("[objc exception] {}: {}\n{}", utf8(name), utf8(reason), utf8(stack));
                exc
            }
        }

        unsafe { objc2::ffi::objc_setExceptionPreprocessor(log) };
    }

    /// Non-activating panel over every Space, fullscreen ones included.
    pub struct MacPanel;

    impl FloatingWindow for MacPanel {
        fn make_floating(&self, window: &WebviewWindow) -> tauri::Result<()> {
            // Called from Tauri's `setup`, which runs on the main thread as AppKit requires.
            let ptr = window.ns_window()? as *mut AnyObject;
            // SAFETY: `ptr` is the live NSWindow behind `window`. BattlePanel adds no
            // ivars, so the object's layout still fits; tao's `TaoWindow` overrides are
            // replaced by NSPanel's, and its `focusable` ivar is only touched at
            // creation and by `set_focusable`, which we never call.
            unsafe {
                AnyObject::set_class(&*ptr, BattlePanel::class());
                let panel = &*(ptr as *const NSPanel);
                panel.setStyleMask(panel.styleMask() | NSWindowStyleMask::NonactivatingPanel);
                panel.setFloatingPanel(true);
                panel.setHidesOnDeactivate(false);
                // MapleStory Classic World's window sits at level 26, above
                // the usual floating (3) and status (25) levels.
                panel.setLevel(NSPopUpMenuWindowLevel);
                panel.setCollectionBehavior(
                    NSWindowCollectionBehavior::CanJoinAllSpaces
                        | NSWindowCollectionBehavior::FullScreenAuxiliary
                        | NSWindowCollectionBehavior::Stationary
                        | NSWindowCollectionBehavior::IgnoresCycle,
                );
            }
            Ok(())
        }
    }
}
