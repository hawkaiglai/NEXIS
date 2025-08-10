//! ASCII banner and welcome screen for IronVeil OS
//! 
//! DEPENDENCIES:
//! - Uses VGA driver for display output
//! - Uses CLI color scheme
//! 
//! INTEGRATION POINTS:
//! - Called during system boot
//! - Uses existing VGA writer infrastructure

use crate::cli::colors;
use crate::vga::{vprintln, vprint};

/// Display the main IronVeil boot banner
pub fn display_boot_banner() {
    vprintln!("");
    vprintln!("{}{}╔══════════════════════════════════════════════════════════════════════════════╗{}", 
             colors::PRIMARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}║                                                                              ║{}", 
             colors::PRIMARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}║{}  ██╗██████╗  ██████╗ ███╗   ██╗██╗   ██╗███████╗██╗██╗                    {}{}║{}", 
             colors::PRIMARY, colors::BOLD, colors::RESET, colors::PRIMARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}║{}  ██║██╔══██╗██╔═══██╗████╗  ██║██║   ██║██╔════╝██║██║                    {}{}║{}", 
             colors::PRIMARY, colors::BOLD, colors::RESET, colors::PRIMARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}║{}  ██║██████╔╝██║   ██║██╔██╗ ██║██║   ██║█████╗  ██║██║                    {}{}║{}", 
             colors::PRIMARY, colors::BOLD, colors::RESET, colors::PRIMARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}║{}  ██║██╔══██╗██║   ██║██║╚██╗██║╚██╗ ██╔╝██╔══╝  ██║██║                    {}{}║{}", 
             colors::PRIMARY, colors::BOLD, colors::RESET, colors::PRIMARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}║{}  ██║██║  ██║╚██████╔╝██║ ╚████║ ╚████╔╝ ███████╗██║███████╗               {}{}║{}", 
             colors::PRIMARY, colors::BOLD, colors::RESET, colors::PRIMARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}║{}  ╚═╝╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═══╝  ╚═══╝  ╚══════╝╚═╝╚══════╝               {}{}║{}", 
             colors::PRIMARY, colors::BOLD, colors::RESET, colors::PRIMARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}║                                                                              ║{}", 
             colors::PRIMARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}║{}                    Privacy-Focused Live Operating System{}                    {}{}║{}", 
             colors::PRIMARY, colors::BOLD, colors::RESET, colors::RESET, colors::PRIMARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}║{}                          Built on Nexis Kernel{}                             {}{}║{}", 
             colors::PRIMARY, colors::BOLD, colors::RESET, colors::RESET, colors::PRIMARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}║                                                                              ║{}", 
             colors::PRIMARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}╚══════════════════════════════════════════════════════════════════════════════╝{}", 
             colors::PRIMARY, colors::BOLD, colors::RESET);
    vprintln!("");
}

/// Display the compact welcome banner for shell startup
pub fn display_welcome_banner() {
    vprintln!("");
    vprintln!("{}{}┌─────────────────────────────────────────────────────────┐{}", 
             colors::SECONDARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}│{}   ██╗██████╗  ██████╗ ███╗   ██╗██╗   ██╗███████╗██╗██╗  {}{}│{}", 
             colors::SECONDARY, colors::BOLD, colors::PRIMARY, colors::SECONDARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}│{}   ██║██╔══██╗██╔═══██╗████╗  ██║██║   ██║██╔════╝██║██║  {}{}│{}", 
             colors::SECONDARY, colors::BOLD, colors::PRIMARY, colors::SECONDARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}│{}   ██║██████╔╝██║   ██║██╔██╗ ██║██║   ██║█████╗  ██║██║  {}{}│{}", 
             colors::SECONDARY, colors::BOLD, colors::PRIMARY, colors::SECONDARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}│{}   ██║██╔══██╗██║   ██║██║╚██╗██║╚██╗ ██╔╝██╔══╝  ██║██║  {}{}│{}", 
             colors::SECONDARY, colors::BOLD, colors::PRIMARY, colors::SECONDARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}│{}   ██║██║  ██║╚██████╔╝██║ ╚████║ ╚████╔╝ ███████╗██║██║  {}{}│{}", 
             colors::SECONDARY, colors::BOLD, colors::PRIMARY, colors::SECONDARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}│{}   ╚═╝╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═══╝  ╚═══╝  ╚══════╝╚═╝╚═╝  {}{}│{}", 
             colors::SECONDARY, colors::BOLD, colors::PRIMARY, colors::SECONDARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}├─────────────────────────────────────────────────────────┤{}", 
             colors::SECONDARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}│{}             Privacy-Focused Live OS v0.1.0-alpha{}         {}{}│{}", 
             colors::SECONDARY, colors::BOLD, colors::RESET, colors::RESET, colors::SECONDARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}│{}                Type 'help' for commands{}                 {}{}│{}", 
             colors::SECONDARY, colors::BOLD, colors::DIM, colors::RESET, colors::SECONDARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}└─────────────────────────────────────────────────────────┘{}", 
             colors::SECONDARY, colors::BOLD, colors::RESET);
    vprintln!("");
}

/// Display ASCII art logo (alternative compact version)
pub fn display_compact_logo() {
    vprintln!("{}{}  ███████╗██████╗  ██████╗ ███╗   ██╗{}", 
             colors::PRIMARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}  ██╔════╝██╔══██╗██╔═══██╗████╗  ██║{}", 
             colors::PRIMARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}  █████╗  ██████╔╝██║   ██║██╔██╗ ██║{}", 
             colors::PRIMARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}  ██╔══╝  ██╔══██╗██║   ██║██║╚██╗██║{}", 
             colors::PRIMARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}  ██║     ██║  ██║╚██████╔╝██║ ╚████║{}", 
             colors::PRIMARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}  ╚═╝     ╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═══╝{}", 
             colors::PRIMARY, colors::BOLD, colors::RESET);
    vprintln!("");
    vprintln!("{}{}      ██╗   ██╗███████╗██╗██╗     {}", 
             colors::SECONDARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}      ██║   ██║██╔════╝██║██║     {}", 
             colors::SECONDARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}      ██║   ██║█████╗  ██║██║     {}", 
             colors::SECONDARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}      ╚██╗ ██╔╝██╔══╝  ██║██║     {}", 
             colors::SECONDARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}       ╚████╔╝ ███████╗██║███████╗{}", 
             colors::SECONDARY, colors::BOLD, colors::RESET);
    vprintln!("{}{}        ╚═══╝  ╚══════╝╚═╝╚══════╝{}", 
             colors::SECONDARY, colors::BOLD, colors::RESET);
}

/// Display boot messages with status indicators
pub fn display_boot_status() {
    vprintln!("");
    vprintln!("{}{}┌─ Boot Status ─────────────────────────────────────────┐{}", 
             colors::ACCENT, colors::BOLD, colors::RESET);
    
    print_status_line("Kernel", "Loaded", true);
    print_status_line("Memory Manager", "Initialized", true);
    print_status_line("Interrupt Controller", "Active", true);
    print_status_line("Keyboard Driver", "Ready", true);
    print_status_line("VGA Driver", "Active", true);
    print_status_line("Scheduler", "Running", true);
    print_status_line("Privacy Stack", "Loading", false);
    print_status_line("Network Interface", "Detecting", false);
    
    vprintln!("{}{}└───────────────────────────────────────────────────────┘{}", 
             colors::ACCENT, colors::BOLD, colors::RESET);
    vprintln!("");
}

/// Helper function to print status lines with indicators
fn print_status_line(component: &str, status: &str, success: bool) {
    let indicator = if success { "●" } else { "○" };
    let color = if success { colors::SUCCESS } else { colors::WARNING };
    
    vprintln!("{}{}│{} {}{} {:<25} {:>20} {}{}{}│{}", 
             colors::ACCENT, colors::BOLD, colors::RESET,
             color, indicator, component, status, colors::RESET,
             colors::ACCENT, colors::BOLD, colors::RESET);
}

/// Display privacy notice and warnings
pub fn display_privacy_notice() {
    vprintln!("");
    vprintln!("{}{}┌─ Privacy Notice ──────────────────────────────────────┐{}", 
             colors::WARNING, colors::BOLD, colors::RESET);
    vprintln!("{}{}│{}                       IMPORTANT{}                       {}{}│{}", 
             colors::WARNING, colors::BOLD, colors::RESET, colors::RESET, colors::WARNING, colors::BOLD, colors::RESET);
    vprintln!("{}{}│                                                      │{}", 
             colors::WARNING, colors::BOLD, colors::RESET);
    vprintln!("{}{}│{} This is a privacy-focused live operating system.{}    {}{}│{}", 
             colors::WARNING, colors::BOLD, colors::RESET, colors::RESET, colors::WARNING, colors::BOLD, colors::RESET);
    vprintln!("{}{}│{} • No data is written to permanent storage{}          {}{}│{}", 
             colors::WARNING, colors::BOLD, colors::RESET, colors::RESET, colors::WARNING, colors::BOLD, colors::RESET);
    vprintln!("{}{}│{} • Network traffic routing through privacy stack{}    {}{}│{}", 
             colors::WARNING, colors::BOLD, colors::RESET, colors::RESET, colors::WARNING, colors::BOLD, colors::RESET);
    vprintln!("{}{}│{} • MAC addresses are randomized on boot{}             {}{}│{}", 
             colors::WARNING, colors::BOLD, colors::RESET, colors::RESET, colors::WARNING, colors::BOLD, colors::RESET);
    vprintln!("{}{}│{} • Use 'help' for privacy and security commands{}     {}{}│{}", 
             colors::WARNING, colors::BOLD, colors::RESET, colors::RESET, colors::WARNING, colors::BOLD, colors::RESET);
    vprintln!("{}{}│                                                      │{}", 
             colors::WARNING, colors::BOLD, colors::RESET);
    vprintln!("{}{}└──────────────────────────────────────────────────────┘{}", 
             colors::WARNING, colors::BOLD, colors::RESET);
    vprintln!("");
}
