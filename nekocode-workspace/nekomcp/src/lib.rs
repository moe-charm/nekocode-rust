//! NekoMCP - MCP (Model Context Protocol) server for NekoCode
//! 
//! This library provides MCP server functionality to expose NekoCode's
//! code analysis capabilities to AI assistants like Claude Code.

pub mod cli;
pub mod server;

pub use cli::*;
pub use server::*;

use anyhow::Result;

/// Initialize the NekoMCP library
pub fn init() -> Result<()> {
    env_logger::init();
    log::info!("NekoMCP initialized");
    Ok(())
}

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");