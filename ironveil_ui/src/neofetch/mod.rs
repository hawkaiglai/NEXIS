//! System information display module (neofetch-like functionality)
//! 
//! DEPENDENCIES:
//! - Uses VGA driver for display output
//! - Integrates with memory manager and system stats
//! - Uses CLI color scheme
//! 
//! INTEGRATION POINTS:
//! - Called by CLI shell commands
//! - Uses kernel system information
//! - Integrates with hardware detection

#![no_std]

pub mod system_info;

pub use system_info::{
    display_system_info,
    display_ascii_logo,
    SystemInfo,
    HardwareInfo,
    MemoryInfo
};

/// Neofetch-style information categories
#[derive(Debug, Clone)]
pub enum InfoCategory {
    OS,
    Kernel,
    Uptime,
    Memory,
    CPU,
    Hardware,
    Network,
    Security,
}

/// System information display options
#[derive(Debug, Clone)]
pub struct DisplayOptions {
    pub show_logo: bool,
    pub show_colors: bool,
    pub categories: &'static [InfoCategory],
    pub compact_mode: bool,
}

impl Default for DisplayOptions {
    fn default() -> Self {
        Self {
            show_logo: true,
            show_colors: true,
            categories: &[
                InfoCategory::OS,
                InfoCategory::Kernel,
                InfoCategory::Uptime,
                InfoCategory::Memory,
                InfoCategory::CPU,
                InfoCategory::Hardware,
                InfoCategory::Security,
            ],
            compact_mode: false,
        }
    }
}
