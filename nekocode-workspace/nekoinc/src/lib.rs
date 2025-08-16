//! NekoInc - Incremental analysis and watch functionality

pub mod incremental;
pub mod watch;
pub mod cli;

pub use incremental::{
    ChangeDetector, FileChange, ChangeType, FileMetadata,
    IncrementalAnalyzer, IncrementalSummary
};

pub use watch::{
    FileWatcher, WatchConfig, WatchStatus, WatchState,
    WatchManager
};

pub use cli::Cli;