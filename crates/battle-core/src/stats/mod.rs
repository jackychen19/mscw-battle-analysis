//! Session statistics: pure logic, no I/O, no clocks. Callers pass explicit
//! millisecond timestamps so every scenario is reproducible in tests.

mod engine;
mod filter;
mod level;
mod meso_align;
mod reading;
mod rolling;
mod snapshot;

#[cfg(test)]
mod tests;

pub use engine::{Command, Observation, Session, SessionConfig, SessionState, TransitionError};
pub use filter::{ReadingFilter, StackFilter};
pub use level::{LevelTable, MaxEstimator, MaxExp};
pub use meso_align::{Alignment, DEFAULT_FRESHNESS_TOLERANCE, align};
pub use reading::{ExpReading, Line, LineKind};
pub use rolling::{DEFAULT_ROLLING_WINDOW_MS, Rolling};
pub use snapshot::Snapshot;
