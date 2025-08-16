//! CLI handling for nekomcp

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "nekomcp")]
#[command(about = "MCP (Model Context Protocol) server for NekoCode", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    
    /// Verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the MCP server
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value = "3000")]
        port: u16,
        
        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        
        /// Enable CORS for web clients
        #[arg(long)]
        cors: bool,
    },
    
    /// Test MCP server functionality
    Test {
        /// Test specific functionality
        #[arg(short, long)]
        function: Option<String>,
        
        /// Test data directory
        #[arg(short, long)]
        data: Option<PathBuf>,
    },
    
    /// Check MCP server health
    Health {
        /// Server URL to check
        #[arg(long, default_value = "http://127.0.0.1:3000")]
        url: String,
    },
    
    /// Show MCP server capabilities
    Capabilities,
    
    /// Generate MCP server configuration
    Config {
        /// Output configuration file
        #[arg(short, long)]
        output: Option<PathBuf>,
        
        /// Configuration format (json, yaml)
        #[arg(long, default_value = "json")]
        format: String,
    },
}