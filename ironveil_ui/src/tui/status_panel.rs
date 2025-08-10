//! Live system status panel implementation
//! 
//! DEPENDENCIES:
//! - Implements UIComponent trait
//! - Uses system information types
//! - Integrates with kernel for live data
//! 
//! INTEGRATION POINTS:
//! - Displays data from nexis_kernel
//! - Shows privacy_core status
//! - Updates in real-time

use alloc::{string::String, vec::Vec, format};
use crate::{
    UIComponent, InputEvent, Result, SystemInfo, MemoryStats, NetworkInterface,
    PrivacyStatus, TorStatus, Rect, Buffer, MacAddr, Ipv4Addr
};
use crate::tui::rust_theme::{RUST_THEME, colors};

pub struct StatusPanel {
    system_info: SystemInfo,
    update_counter: u64,
    scroll_offset: usize,
    max_lines: usize,
}

impl StatusPanel {
    pub fn new() -> Self {
        Self {
            system_info: SystemInfo::default(),
            update_counter: 0,
            scroll_offset: 0,
            max_lines: 18, // Maximum lines to display
        }
    }

    fn update_system_info(&mut self) -> Result<()> {
        // Update uptime
        self.system_info.uptime = self.get_system_uptime();
        
        // Update memory statistics
        self.system_info.memory_usage = self.get_memory_stats()?;
        
        // Update network interfaces
        self.system_info.network_interfaces = self.get_network_interfaces()?;
        
        // Update privacy status
        self.system_info.privacy_status = self.get_privacy_status()?;
        
        self.update_counter += 1;
        Ok(())
    }

    fn get_system_uptime(&self) -> u64 {
        // Implementation would query kernel for uptime
        self.update_counter * 16 // Simulate uptime in milliseconds
    }

    fn get_memory_stats(&self) -> Result<MemoryStats> {
        // Implementation would query kernel memory manager
        Ok(MemoryStats {
            total: 128 * 1024 * 1024, // 128 MB
            used: 64 * 1024 * 1024,   // 64 MB used
            free: 64 * 1024 * 1024,   // 64 MB free
        })
    }

    fn get_network_interfaces(&self) -> Result<Vec<NetworkInterface>> {
        // Implementation would query kernel network drivers
        let mut interfaces = Vec::new();
        
        interfaces.push(NetworkInterface {
            name: "eth0".into(),
            mac_address: MacAddr([0x52, 0x54, 0x00, 0x12, 0x34, 0x56]),
            ip_address: Some(Ipv4Addr([192, 168, 1, 100])),
            status: crate::InterfaceStatus::Up,
        });
        
        Ok(interfaces)
    }

    fn get_privacy_status(&self) -> Result<PrivacyStatus> {
        // Implementation would query privacy_core
        Ok(PrivacyStatus {
            tor_status: TorStatus::Running,
            mac_randomized: true,
            vpn_connected: false,
        })
    }

    fn format_uptime(&self, uptime_ms: u64) -> String {
        let uptime_sec = uptime_ms / 1000;
        let hours = uptime_sec / 3600;
        let minutes = (uptime_sec % 3600) / 60;
        let seconds = uptime_sec % 60;
        
        format!("{}h {}m {}s", hours, minutes, seconds)
    }

    fn format_memory_size(&self, bytes: usize) -> String {
        if bytes >= 1024 * 1024 * 1024 {
            format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
        } else if bytes >= 1024 * 1024 {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        } else if bytes >= 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            format!("{} B", bytes)
        }
    }

    fn format_mac_address(&self, mac: &MacAddr) -> String {
        format!("{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                mac.0[0], mac.0[1], mac.0[2], mac.0[3], mac.0[4], mac.0[5])
    }

    fn format_ip_address(&self, ip: &Ipv4Addr) -> String {
        format!("{}.{}.{}.{}", ip.0[0], ip.0[1], ip.0[2], ip.0[3])
    }

    fn get_status_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        
        // System Information
        lines.push("=== SYSTEM STATUS ===".into());
        lines.push(format!("Uptime: {}", self.format_uptime(self.system_info.uptime)));
        lines.push(format!("Updates: {}", self.update_counter));
        lines.push("".into());
        
        // Memory Information
        lines.push("=== MEMORY ===".into());
        let mem = &self.system_info.memory_usage;
        lines.push(format!("Total:  {}", self.format_memory_size(mem.total)));
        lines.push(format!("Used:   {}", self.format_memory_size(mem.used)));
        lines.push(format!("Free:   {}", self.format_memory_size(mem.free)));
        
        let usage_percent = if mem.total > 0 {
            (mem.used * 100) / mem.total
        } else {
            0
        };
        lines.push(format!("Usage:  {}%", usage_percent));
        lines.push("".into());
        
        // Network Interfaces
        lines.push("=== NETWORK ===".into());
        for interface in &self.system_info.network_interfaces {
            lines.push(format!("Interface: {}", interface.name));
            lines.push(format!("  MAC: {}", self.format_mac_address(&interface.mac_address)));
            
            if let Some(ip) = &interface.ip_address {
                lines.push(format!("  IP:  {}", self.format_ip_address(ip)));
            } else {
                lines.push("  IP:  Not assigned".into());
            }
            
            let status_text = match interface.status {
                crate::InterfaceStatus::Up => "UP",
                crate::InterfaceStatus::Down => "DOWN",
                crate::InterfaceStatus::Unknown => "UNKNOWN",
            };
            lines.push(format!("  Status: {}", status_text));
            lines.push("".into());
        }
        
        // Privacy Status
        lines.push("=== PRIVACY ===".into());
        let privacy = &self.system_info.privacy_status;
        
        let tor_status_text = match privacy.tor_status {
            TorStatus::Stopped => "Stopped",
            TorStatus::Starting => "Starting...",
            TorStatus::Running => "Running",
            TorStatus::Error => "Error",
        };
        lines.push(format!("Tor: {}", tor_status_text));
        
        let mac_status = if privacy.mac_randomized { "Yes" } else { "No" };
        lines.push(format!("MAC Randomized: {}", mac_status));
        
        let vpn_status = if privacy.vpn_connected { "Connected" } else { "Disconnected" };
        lines.push(format!("VPN: {}", vpn_status));
        
        lines
    }
}

impl UIComponent for StatusPanel {
    fn render(&self, area: Rect, buffer: &mut Buffer) -> Result<()> {
        // Draw border
        self.draw_border(area)?;
        
        // Draw title
        let title = "System Status";
        self.draw_text_centered(area.x, area.y + 1, area.width, title,
                               RUST_THEME.text, RUST_THEME.secondary)?;
        
        // Get status lines
        let lines = self.get_status_lines();
        let start_y = area.y + 3;
        let visible_lines = core::cmp::min(self.max_lines, area.height as usize - 4);
        
        // Draw status lines
        for i in 0..visible_lines {
            let line_index = self.scroll_offset + i;
            if line_index >= lines.len() {
                break;
            }
            
            let line = &lines[line_index];
            let y = start_y + i as u16;
            
            // Determine color based on line content
            let (fg, bg) = if line.starts_with("===") {
                (RUST_THEME.primary, RUST_THEME.background)
            } else if line.is_empty() {
                (RUST_THEME.text, RUST_THEME.background)
            } else if line.starts_with("  ") {
                (RUST_THEME.text, RUST_THEME.background)
            } else {
                (RUST_THEME.accent, RUST_THEME.background)
            };
            
            // Truncate line if too long
            let max_width = (area.width - 4) as usize;
            let display_line = if line.len() > max_width {
                &line[..max_width]
            } else {
                line
            };
            
            self.draw_text(area.x + 2, y, display_line, fg, bg)?;
        }
        
        // Draw scroll indicators
        if self.scroll_offset > 0 {
            self.draw_text(area.x + area.width - 2, start_y, "↑",
                          RUST_THEME.accent, RUST_THEME.background)?;
        }
        
        if self.scroll_offset + visible_lines < lines.len() {
            let indicator_y = start_y + visible_lines as u16 - 1;
            self.draw_text(area.x + area.width - 2, indicator_y, "↓",
                          RUST_THEME.accent, RUST_THEME.background)?;
        }
        
        Ok(())
    }

    fn handle_input(&mut self, event: InputEvent) -> Result<bool> {
        match event {
            InputEvent::Key(key) => {
                match key.code {
                    crate::KeyCode::Up => {
                        if self.scroll_offset > 0 {
                            self.scroll_offset -= 1;
                        }
                        Ok(true)
                    }
                    crate::KeyCode::Down => {
                        let lines = self.get_status_lines();
                        if self.scroll_offset + self.max_lines < lines.len() {
                            self.scroll_offset += 1;
                        }
                        Ok(true)
                    }
                    crate::KeyCode::Char('r') | crate::KeyCode::Char('R') => {
                        // Force refresh
                        self.update_system_info()?;
                        Ok(true)
                    }
                    _ => Ok(false)
                }
            }
            _ => Ok(false)
        }
    }

    fn update(&mut self) -> Result<()> {
        // Update system information periodically
        if self.update_counter % 60 == 0 { // Update every ~1 second at 60 FPS
            self.update_system_info()?;
        } else {
            self.update_counter += 1;
        }
        Ok(())
    }
}

impl StatusPanel {
    fn draw_border(&self, area: Rect) -> Result<()> {
        // Implementation would draw border using VGA writer
        Ok(())
    }

    fn draw_text(&self, x: u16, y: u16, text: &str, fg: (u8, u8, u8), bg: (u8, u8, u8)) -> Result<()> {
        // Implementation would write to VGA buffer
        Ok(())
    }

    fn draw_text_centered(&self, x: u16, y: u16, width: u16, text: &str,
                         fg: (u8, u8, u8), bg: (u8, u8, u8)) -> Result<()> {
        let text_len = text.len() as u16;
        let start_x = if text_len < width {
            x + (width - text_len) / 2
        } else {
            x
        };
        self.draw_text(start_x, y, text, fg, bg)
    }
}
