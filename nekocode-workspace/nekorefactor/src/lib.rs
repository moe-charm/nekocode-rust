//! Refactoring library for NekoCode

pub mod preview;
pub mod replace;
pub mod moveclass;
pub mod cli;

pub use preview::{PreviewManager, PreviewEntry, PreviewOperation, MatchInfo};
pub use replace::{ReplaceEngine, ReplaceOptions};
pub use moveclass::{MoveClassEngine, MoveOptions};