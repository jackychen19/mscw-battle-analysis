//! Platform-independent core of Battle Analysis.
//!
//! Nothing in this crate touches the OS, the network, or the filesystem
//! (calibration load/save lives in the app crate). OS-specific screen capture
//! implements [`capture::FrameSource`] elsewhere.

pub mod calibration;
pub mod capture;
pub mod parse;
