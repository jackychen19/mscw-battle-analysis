//! Calibration data: which window, where the text regions are, and the glyph
//! templates learned from the user's own screen. Stored as small local files
//! by the app (Milestone 3); this module only defines the format.

use serde::{Deserialize, Serialize};

use crate::capture::RectPt;

pub const CALIBRATION_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Calibration {
    pub version: u32,
    pub window: WindowMatch,
    pub regions: Regions,
    pub templates: TemplateSet,
}

/// How to find the game window again on next launch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowMatch {
    pub title: String,
    pub owner: String,
}

/// Text regions, in points relative to the game window's content origin.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Regions {
    pub exp_text: RectPt,
    pub level: RectPt,
    pub notifications: RectPt,
}

/// A binarized image: `bits[y * width + x]` is true for text pixels.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bitmap {
    pub width: u32,
    pub height: u32,
    pub bits: Vec<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlyphTemplate {
    pub ch: char,
    pub bitmap: Bitmap,
}

/// Templates are captured at the user's actual scale factor and only valid there.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateSet {
    pub scale: f64,
    pub glyphs: Vec<GlyphTemplate>,
    /// Whole-image template of `You have gained mesos (+`.
    pub meso_prefix: Option<Bitmap>,
}
