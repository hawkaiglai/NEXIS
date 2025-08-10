//! Command Line Interface module for IronVeil OS
//! 
//! DEPENDENCIES:
//! - Uses core types and no_std environment
//! - Integrates with VGA driver for output
//! - Uses existing memory and scheduler infrastructure
//! 
//! INTEGRATION POINTS:
//! - Called by main kernel shell loop
//! - Uses VGA writer for display output
//! - Integrates with memory manager for system stats
//! - Uses scheduler for task management commands

#![no_std]

pub mod shell;
pub mod banner;

pub use shell::{CommandHandler, ShellContext, BuiltinCommands};
pub use banner::{display_boot_banner, display_welcome_banner};

/// CLI color scheme following IronVeil theme
pub mod colors {
    pub const PRIMARY: &str = "\x1b[38;2;206;66;43m";     // Rust orange
    pub const SECONDARY: &str = "\x1b[38;2;140;48;36m";   // Rust brown  
    pub const ACCENT: &str = "\x1b[38;2;45;45;45m";       // Iron gray
    pub const SUCCESS: &str = "\x1b[38;2;40;167;69m";     // Success green
    pub const WARNING: &str = "\x1b[38;2;255;193;7m";     // Warning yellow
    pub const ERROR: &str = "\x1b[38;2;220;53;69m";       // Error red
    pub const RESET: &str = "\x1b[0m";                    // Reset
    pub const BOLD: &str = "\x1b[1m";                     // Bold
    pub const DIM: &str = "\x1b[2m";                      // Dim
}

/// Result type for CLI operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    InvalidCommand,
    InvalidArguments,
    InsufficientPermissions,
    SystemError,
    OutOfMemory,
}

pub type CliResult<T> = Result<T, CliError>;

/// Command execution context
#[derive(Debug, Clone)]
pub struct CommandContext {
    pub current_user: &'static str,
    pub hostname: &'static str,
    pub working_directory: &'static str,
    pub environment_vars: &'static [(&'static str, &'static str)],
}

impl Default for CommandContext {
    fn default() -> Self {
        Self {
            current_user: "root",
            hostname: "ironveil",
            working_directory: "/",
            environment_vars: &[
                ("HOME", "/root"),
                ("PATH", "/bin:/usr/bin:/sbin:/usr/sbin"),
                ("SHELL", "/bin/ironshell"),
                ("TERM", "ironveil-256color"),
            ],
        }
    }
}
