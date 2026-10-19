//! Bitmap-font text reading (Milestone 2): crop → binarize by brightness →
//! segment glyphs by column gaps → match against calibrated templates.
//!
//! Parsers produce [`crate::stats::ExpReading`] for the EXP line and a list of
//! [`crate::stats::Line`] (top → bottom) for the notification stack.
