//! Bitmap-font text reading: crop → binarize by brightness → segment glyphs
//! by column gaps → match against templates learned during calibration.
//!
//! Parsers produce [`crate::stats::ExpReading`] for the EXP line and a list of
//! [`crate::stats::Line`] (top → bottom) for the notification stack.

mod bitmap;
mod gray;
mod segment;

pub use bitmap::Bitmap;
pub use gray::Gray;
pub use segment::{column_runs, row_runs};
