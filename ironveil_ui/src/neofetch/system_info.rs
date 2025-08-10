//! System information display implementation
//! 
//! DEPENDENCIES:
//! - Uses VGA driver and CLI color system
//! - Integrates with memory manager for stats
//! - Uses scheduler and PIT for system information
//! 
//! INTEGRATION POINTS:
//! - Called by neofetch command
//! - Uses kernel subsystems for data collection

use crate::cli::colors;
use crate::vga::{vprintln, vprint};
use crate::neofetch::DisplayOptions;

/// System information structure
#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub os_name: &'static str,
    pub os_version: &'static str,
    pub kernel_name: &'static str,
    pub kernel_version: &'static str,
    pub architecture: &'static str,
    pub hostname: &'static str,
    pub username: &'static str,
    pub uptime_seconds: u64,
    pub memory: MemoryInfo,
    pub hardware: HardwareInfo,
}

/// Memory information structure
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    pub total_kb: usize,
    pub used_kb: usize,
    pub free_kb: usize,
    pub total_frames: usize,
    pub used_frames: usize,
    pub free_frames: usize,
}

/// Hardware information structure
#[derive(Debug, Clone)]
pub struct HardwareInfo {
    pub cpu_vendor: &'static str,
    pub cpu_model: &'static str,
    pub cpu_cores: u8,
    pub cpu_architecture: &'static str,
}

impl SystemInfo {
    /// Collect current system information
    pub fn collect() -> Self {
        let memory = MemoryInfo::collect();
        let hardware = HardwareInfo::detect();
        let uptime = crate::pit::ticks() / 50; // Convert 50Hz ticks to seconds
        
        Self {
            os_name: "IronVeil OS",
            os_version: "0.1.0-alpha",
            kernel_name: "Nexis",
            kernel_version: "0.1.0",
            architecture: "x86_64",
            hostname: "ironveil",
            username: "root",
            uptime_seconds: uptime,
            memory,
            hardware,
        }
    }
}

impl MemoryInfo {
    /// Collect current memory information from PMM
    pub fn collect() -> Self {
        unsafe {
            let pmm = &crate::PMM;
            let free_frames = pmm.free_frames();
            let total_frames = pmm.total_frames();
            let used_frames = total_frames - free_frames;
            
            let frame_size = crate::memory::FRAME_SIZE;
            let total_kb = (total_frames * frame_size) / 1024;
            let used_kb = (used_frames * frame_size) / 1024;
            let free_kb = (free_frames * frame_size) / 1024;
            
            Self {
                total_kb,
                used_kb,
                free_kb,
                total_frames,
                used_frames,
                free_frames,
            }
        }
    }
    
    /// Calculate memory usage percentage
    pub fn usage_percent(&self) -> u8 {
        if self.total_kb == 0 {
            0
        } else {
            ((self.used_kb * 100) / self.total_kb) as u8
        }
    }
}

impl HardwareInfo {
    /// Detect hardware information (simulated for now)
    pub fn detect() -> Self {
        Self {
            cpu_vendor: "Unknown",
            cpu_model: "IronVeil Virtual CPU",
            cpu_cores: 1,
            cpu_architecture: "x86_64",
        }
    }
}

/// Display full system information with ASCII logo
pub fn display_system_info() {
    let options = DisplayOptions::default();
    display_system_info_with_options(&options);
}

/// Display system information with custom options
pub fn display_system_info_with_options(options: &DisplayOptions) {
    let info = SystemInfo::collect();
    
    vprintln!("");
    
    if options.show_logo {
        display_logo_and_info(&info);
    } else {
        display_info_only(&info);
    }
    
    if options.show_colors {
        display_color_palette();
    }
    
    vprintln!("");
}

/// Display ASCII logo alongside system information
fn display_logo_and_info(info: &SystemInfo) {
    let logo_lines = get_ascii_logo();
    let info_lines = format_system_info(info);
    
    // Display logo and info side by side
    let max_lines = logo_lines.len().max(info_lines.len());
    
    for i in 0..max_lines {
        // Print logo line (or padding)
        if i < logo_lines.len() {
            vprint!("{}", logo_lines[i]);
        } else {
            vprint!("{:<30}", ""); // Logo width padding
        }
        
        // Print info line
        if i < info_lines.len() {
            vprint!("  {}", info_lines[i]);
        }
        
        vprintln!("");
    }
}

/// Display only system information (no logo)
fn display_info_only(info: &SystemInfo) {
    let info_lines = format_system_info(info);
    for line in info_lines {
        vprintln!("  {}", line);
    }
}

/// Get ASCII logo lines
fn get_ascii_logo() -> [&'static str; 12] {
    [
        "      ██╗██████╗  ██████╗ ███╗   ██╗",
        "      ██║██╔══██╗██╔═══██╗████╗  ██║",
        "      ██║██████╔╝██║   ██║██╔██╗ ██║",
        "      ██║██╔══██╗██║   ██║██║╚██╗██║",
        "      ██║██║  ██║╚██████╔╝██║ ╚████║",
        "      ╚═╝╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═══╝",
        "",
        "  ██╗   ██╗███████╗██╗██╗     ",
        "  ██║   ██║██╔════╝██║██║     ",
        "  ██║   ██║█████╗  ██║██║     ",
        "  ╚██╗ ██╔╝██╔══╝  ██║██║     ",
        "   ╚████╔╝ ███████╗██║███████╗",
    ]
}

/// Format system information into display lines
fn format_system_info(info: &SystemInfo) -> [String; 15] {
    let uptime_str = format_uptime(info.uptime_seconds);
    let memory_bar = format_memory_bar(&info.memory);
    
    [
        format!("{}{}OS:{} {}", colors::PRIMARY, colors::BOLD, colors::RESET, info.os_name),
        format!("{}{}Host:{} {}", colors::PRIMARY, colors::BOLD, colors::RESET, info.hostname),
        format!("{}{}Kernel:{} {} {}", colors::PRIMARY, colors::BOLD, colors::RESET, info.kernel_name, info.kernel_version),
        format!("{}{}Uptime:{} {}", colors::PRIMARY, colors::BOLD, colors::RESET, uptime_str),
        format!("{}{}Shell:{} IronShell", colors::PRIMARY, colors::BOLD, colors::RESET),
        format!("{}{}Terminal:{} VGA Text Mode", colors::PRIMARY, colors::BOLD, colors::RESET),
        format!("{}{}CPU:{} {} ({})", colors::PRIMARY, colors::BOLD, colors::RESET, 
                info.hardware.cpu_model, info.hardware.cpu_cores),
        format!("{}{}Memory:{} {} / {} KB {}", colors::PRIMARY, colors::BOLD, colors::RESET,
                info.memory.used_kb, info.memory.total_kb, memory_bar),
        format!("{}{}Architecture:{} {}", colors::PRIMARY, colors::BOLD, colors::RESET, info.architecture),
        format!("{}{}Security:{} Privacy Mode Active", colors::PRIMARY, colors::BOLD, colors::RESET),
        format!("{}{}Network:{} MAC Randomized", colors::PRIMARY, colors::BOLD, colors::RESET),
        format!("{}{}Storage:{} Live Mode (No Persistence)", colors::PRIMARY, colors::BOLD, colors::RESET),
        "".to_string(),
        format!("{}{}Privacy Features:{}", colors::SECONDARY, colors::BOLD, colors::RESET),
        format!("  • Network traffic routing • MAC spoofing • Secure memory"),
    ]
}

/// Format uptime into human-readable string
fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;
    
    if days > 0 {
        format!("{} days, {}:{:02}:{:02}", days, hours, minutes, secs)
    } else if hours > 0 {
        format!("{}:{:02}:{:02}", hours, minutes, secs)
    } else if minutes > 0 {
        format!("{}:{:02}", minutes, secs)
    } else {
        format!("{} seconds", secs)
    }
}

/// Format memory usage bar
fn format_memory_bar(memory: &MemoryInfo) -> String {
    let usage_percent = memory.usage_percent();
    let bar_width = 20;
    let filled = (usage_percent as usize * bar_width) / 100;
    let empty = bar_width - filled;
    
    let mut bar = String::new();
    bar.push('[');
    
    // Add filled portion
    for _ in 0..filled {
        bar.push('█');
    }
    
    // Add empty portion
    for _ in 0..empty {
        bar.push('░');
    }
    
    bar.push(']');
    bar.push(' ');
    bar.push_str(&format!("{}%", usage_percent));
    
    if usage_percent > 80 {
        format!("{}{}{}", colors::ERROR, bar, colors::RESET)
    } else if usage_percent > 60 {
        format!("{}{}{}", colors::WARNING, bar, colors::RESET)
    } else {
        format!("{}{}{}", colors::SUCCESS, bar, colors::RESET)
    }
}

/// Display color palette for testing
fn display_color_palette() {
    vprintln!("");
    vprintln!("{}{}Colors:{}", colors::PRIMARY, colors::BOLD, colors::RESET);
    
    vprint!("  ");
    vprint!("{}███{}", colors::PRIMARY, colors::RESET);
    vprint!("{}███{}", colors::SECONDARY, colors::RESET);
    vprint!("{}███{}", colors::SUCCESS, colors::RESET);
    vprint!("{}███{}", colors::WARNING, colors::RESET);
    vprint!("{}███{}", colors::ERROR, colors::RESET);
    vprint!("{}███{}", colors::ACCENT, colors::RESET);
    vprintln!("");
}

/// Display compact ASCII logo (for smaller displays)
pub fn display_ascii_logo() {
    let logo_lines = get_ascii_logo();
    for line in &logo_lines {
        vprintln!("{}{}{}", colors::PRIMARY, line, colors::RESET);
    }
}

/// Display detailed hardware information
pub fn display_hardware_info() {
    let hardware = HardwareInfo::detect();
    
    vprintln!("{}{}Hardware Information:{}", colors::PRIMARY, colors::BOLD, colors::RESET);
    vprintln!("  CPU Vendor:     {}", hardware.cpu_vendor);
    vprintln!("  CPU Model:      {}", hardware.cpu_model);
    vprintln!("  CPU Cores:      {}", hardware.cpu_cores);
    vprintln!("  Architecture:   {}", hardware.cpu_architecture);
    vprintln!("  Instruction Set: x86_64");
    vprintln!("  Endianness:     Little Endian");
}

/// Display detailed memory information
pub fn display_memory_info() {
    let memory = MemoryInfo::collect();
    
    vprintln!("{}{}Memory Information:{}", colors::PRIMARY, colors::BOLD, colors::RESET);
    vprintln!("  Total Memory:   {} KB ({} MB)", memory.total_kb, memory.total_kb / 1024);
    vprintln!("  Used Memory:    {} KB ({} MB)", memory.used_kb, memory.used_kb / 1024);
    vprintln!("  Free Memory:    {} KB ({} MB)", memory.free_kb, memory.free_kb / 1024);
    vprintln!("  Usage:          {}%", memory.usage_percent());
    vprintln!("");
    vprintln!("  Frame Details:");
    vprintln!("    Total Frames: {}", memory.total_frames);
    vprintln!("    Used Frames:  {}", memory.used_frames);
    vprintln!("    Free Frames:  {}", memory.free_frames);
    vprintln!("    Frame Size:   {} bytes", crate::memory::FRAME_SIZE);
}

/// Display privacy and security status
pub fn display_security_status() {
    vprintln!("{}{}Security & Privacy Status:{}", colors::PRIMARY, colors::BOLD, colors::RESET);
    vprintln!("  {}{}●{} Live Mode Active (No Persistence)", colors::SUCCESS, colors::BOLD, colors::RESET);
    vprintln!("  {}{}●{} MAC Address Randomized", colors::SUCCESS, colors::BOLD, colors::RESET);
    vprintln!("  {}{}○{} Tor Integration (Planned)", colors::WARNING, colors::BOLD, colors::RESET);
    vprintln!("  {}{}○{} VPN Support (Planned)", colors::WARNING, colors::BOLD, colors::RESET);
    vprintln!("  {}{}●{} Secure Memory Allocation", colors::SUCCESS, colors::BOLD, colors::RESET);
    vprintln!("  {}{}●{} No Persistent Storage", colors::SUCCESS, colors::BOLD, colors::RESET);
}
