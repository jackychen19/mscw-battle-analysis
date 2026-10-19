use serde::{Deserialize, Serialize};

/// One parsed EXP line plus level, e.g. `LV. 59` and `EXP. 440,834 [38.43%]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpReading {
    pub level: u32,
    pub exp: u64,
    /// Percent in hundredths: 38.43% is 3843.
    pub pct_hundredths: u32,
}

impl ExpReading {
    pub const fn new(level: u32, exp: u64, pct_hundredths: u32) -> Self {
        Self { level, exp, pct_hundredths }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LineKind {
    /// `You have gained mesos (+15)`
    Meso,
    /// `You received EXP (+78)` — cross-check only, never the EXP source.
    Exp,
    /// `Bonus EXP for PARTY (+23)` — cross-check only.
    PartyBonus,
    /// Any other notification. Still takes part in stack alignment.
    Other,
    /// A line is there but couldn't be read (faded, or a sprite is drawn over
    /// it). Matches any line when aligning stacks.
    Unknown,
}

/// One line of the notification stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Line {
    pub kind: LineKind,
    /// The `(+N)` amount. For [`LineKind::Other`], a parser-supplied
    /// fingerprint of the text so different lines don't compare equal.
    pub amount: u64,
    /// Text brightness, 255 = fully opaque. Lines only fade as they age, so a
    /// line that got brighter between frames must be a different line.
    pub freshness: u8,
}

impl Line {
    pub const fn new(kind: LineKind, amount: u64, freshness: u8) -> Self {
        Self { kind, amount, freshness }
    }

    /// Whether two lines could be the same line: same text, or either is
    /// [`LineKind::Unknown`].
    pub fn same_text(&self, other: &Line) -> bool {
        self.is_unknown() || other.is_unknown() || (self.kind == other.kind && self.amount == other.amount)
    }

    pub fn is_unknown(&self) -> bool {
        self.kind == LineKind::Unknown
    }
}
