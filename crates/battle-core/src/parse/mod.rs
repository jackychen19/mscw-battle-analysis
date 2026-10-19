//! Bitmap-font text reading: crop → binarize by brightness → segment glyphs
//! by column gaps → match against templates learned during calibration.
//!
//! Parsers produce [`crate::stats::ExpReading`] for the EXP line and a list of
//! [`crate::stats::Line`] (top → bottom) for the notification stack.

mod bitmap;
mod exp;
mod glyph;
mod gray;
mod segment;
#[cfg(test)]
mod test_font;

pub use bitmap::Bitmap;
pub use exp::{ExpReader, format_exp_text, parse_exp_text, parse_level_text};
pub use glyph::{DEFAULT_MIN_SCORE, GlyphSet, GlyphTemplate, LineRead, TeachError, line_band, similarity};
pub use gray::{Channel, Gray};
pub use segment::{column_runs, row_runs};
