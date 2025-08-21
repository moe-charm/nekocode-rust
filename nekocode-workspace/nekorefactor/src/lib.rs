//! Refactoring library for NekoCode

pub mod preview;
pub mod replace;
pub mod moveclass;
pub mod strip_comments;
pub mod edit_history;
pub mod cli;
pub mod split_file;
pub mod smart;
pub mod language_detection;

pub use preview::{PreviewManager, PreviewEntry, PreviewOperation, MatchInfo};
pub use replace::{ReplaceEngine, ReplaceOptions};
pub use moveclass::{MoveClassEngine, MoveOptions};
pub use strip_comments::{CommentStripper, StripOptions, StripStats};
pub use edit_history::{EditHistory, EditEntry, EditOperation, get_history, record_edit};
pub use split_file::{FileSplitter, SplitBy, SplitResult, SplitFileInfo};
pub use language_detection::LanguageDetector;