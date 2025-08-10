//! Network interface management for privacy operations
//! 
//! DEPENDENCIES:
//! - Uses core types: MacAddr, IronVeilError, Result
//! - Integrates with kernel network drivers
//! 
//! INTEGRATION POINTS:
//! - Called by privacy_core for MAC address management
//! - Used by network hooks for interface-specific filtering
//! - Interfaces with kernel driver layer

use crate::{IronVeilError, Result, MacAddr, Ipv4Addr};
use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use spin::Mutex;

/// Network interface type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceType {
    Ethernet,
    Wireless,
    Loopback,
    Virtual,
    Unknown,
}

impl InterfaceType {
    pub fn from_name(name: &str) -> Self {
        match name {
            n if n.starts_with("eth") => InterfaceType::Ethernet,
            n if n.starts_with("wlan") || n.starts_with("wifi") => InterfaceType::Wireless,
            "lo" | "loopback" => InterfaceType::Loopback,
            n if n.starts_with("veth") || n.starts_with("tap") => InterfaceType::Virtual,
            _ => InterfaceType::Unknown,
        }
    }
}

/// Network interface state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceState {
    Down,
    Up,
    Dormant,
    Unknown,
}

/// Network interface statistics
#[derive(Debug, Clone, Default)]
pub struct InterfaceStats {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub errors_sent: u64,
    pub errors_received: u64,
    pub dropped_sent: u64,
    pub dropped_received: u64,
}

/// Network interface representation
#[derive(Debug, Clone)]
pub struct NetworkInterface {
    pub name: String,
    pub interface_type: InterfaceType,
    pub state: InterfaceState,
    pub mac_address: MacAddr,
    pub original_mac: MacAddr,
    pub ip_addresses: Vec<Ipv4Addr>,
    pub mtu: u32,
    pub stats: InterfaceStats,
    pub privacy_enabled: bool,
    pub mac_randomized: bool,
    pub last_mac_change: Option<u64>, // timestamp
}

impl NetworkInterface {
    pub fn new(name: String, mac_address: MacAddr) -> Self {
        let interface_type = InterfaceType::from_name(&name);
        
        Self {
            name,
            interface_type,
            state: InterfaceState::Down,
            mac_address,
            original_mac: mac_address,
            ip_addresses: Vec::new(),
            mtu: 1500, // Default Ethernet MTU
            stats: InterfaceStats::default(),
            privacy_enabled: true,
            mac_randomized: false,
            last_mac_change: None,
        }
    }
    
    pub fn set_mac_address(&mut self, mac: MacAddr, timestamp: Option<u64>) -> Result<()> {
        // Validate MAC address
        crate::validate_mac(&mac)?;
        
        self.mac_address = mac;
        self.mac_randomized = true;
        self.last_mac_change = timestamp;
        
        Ok(())
    }
    
    pub fn restore_original_mac(&mut self) {
        self.mac_address = self.original_mac;
        self.mac_randomized = false;
    }
    
    pub fn add_ip_address(&mut self, ip: Ipv4Addr) {
        if !self.ip_addresses.contains(&ip) {
            self.ip_addresses.push(ip);
        }
    }
    
    pub fn remove_ip_address(&mut self, ip: &Ipv4Addr) {
        self.ip_addresses.retain(|addr| addr != ip);
    }
    
    pub fn is_up(&self) -> bool {
        self.state == InterfaceState::Up
    }
    
    pub fn can_randomize_mac(&self) -> bool {
        match self.interface_type {
            InterfaceType::Loopback => false,
            InterfaceType::Virtual => false,
            _ => self.privacy_enabled,
        }
    }
    
    pub fn update_stats(&mut self, bytes_sent: u64, bytes_received: u64, packets_sent: u64, packets_received: u64) {
        self.stats.bytes_sent += bytes_sent;
        self.stats.bytes_received += bytes_received;
        self.stats.packets_sent += packets_sent;
        self.stats.packets_received += packets_received;
    }
}

/// Interface change event
#[derive(Debug, Clone)]
pub enum InterfaceEvent {
    Added { name: String },
    Removed { name: String },
    StateChanged { name: String, old_state: InterfaceState, new_state: InterfaceState },
    MacChanged { name: String, old_mac: MacAddr, new_mac: MacAddr },
    IpChanged { name: String, ip: Ipv4Addr, added: bool },
}

/// Interface event handler trait
pub trait InterfaceEventHandler {
    fn handle_event(&mut self, event: InterfaceEvent);
}

/// Network interface manager
pub struct InterfaceManager {
    interfaces: Mutex<BTreeMap<String, NetworkInterface>>,
    event_handlers: Mutex<Vec<Box<dyn InterfaceEventHandler>>>,
    auto_randomize: AtomicBool,
    randomize_interval: AtomicU64, // in milliseconds
    next_interface_id: AtomicU64,
}

impl InterfaceManager {
    pub fn new() -> Self {
        Self {
            interfaces: Mutex::new(BTreeMap::new()),
            event_handlers: Mutex::new(Vec::new()),
            auto_randomize: AtomicBool::new(false),
            randomize_interval: AtomicU64::new(300000), // 5 minutes
            next_interface_id: AtomicU64::new(0),
        }
    }
    
    /// Add a new network interface
    pub fn add_interface(&self, name: String, mac_address: MacAddr) -> Result<()> {
        let interface = NetworkInterface::new(name.clone(), mac_address);
        
        let mut interfaces = self.interfaces.lock();
        if interfaces.contains_key(&name) {
            return Err(IronVeilError::NetworkError);
        }
        
        interfaces.insert(name.clone(), interface);
        drop(interfaces);
        
        // Notify event handlers
        self.notify_event(InterfaceEvent::Added { name });
        
        Ok(())
    }
    
    /// Remove a network interface
    pub fn remove_interface(&self, name: &str) -> Result<()> {
        let mut interfaces = self.interfaces.lock();
        
        if interfaces.remove(name).is_none() {
            return Err(IronVeilError::InterfaceNotFound);
        }
        
        drop(interfaces);
        
        // Notify event handlers
        self.notify_event(InterfaceEvent::Removed { name: name.to_string() });
        
        Ok(())
    }
    
    /// Get interface by name
    pub fn get_interface(&self, name: &str) -> Result<NetworkInterface> {
        let interfaces = self.interfaces.lock();
        interfaces.get(name)
            .cloned()
            .ok_or(IronVeilError::InterfaceNotFound)
    }
    
    /// List all interfaces
    pub fn list_interfaces(&self) -> Vec<NetworkInterface> {
        let interfaces = self.interfaces.lock();
        interfaces.values().cloned().collect()
    }
    
    /// Set interface state
    pub fn set_interface_state(&self, name: &str, state: InterfaceState) -> Result<()> {
        let mut interfaces = self.interfaces.lock();
        
        if let Some(interface) = interfaces.get_mut(name) {
            let old_state = interface.state;
            interface.state = state;
            
            drop(interfaces);
            
            // Notify event handlers
            self.notify_event(InterfaceEvent::StateChanged {
                name: name.to_string(),
                old_state,
                new_state: state,
            });
            
            Ok(())
        } else {
            Err(IronVeilError::InterfaceNotFound)
        }
    }
    
    /// Set MAC address for interface
    pub fn set_mac_address(&self, name: &str, mac: MacAddr) -> Result<()> {
        let mut interfaces = self.interfaces.lock();
        
        if let Some(interface) = interfaces.get_mut(name) {
            if !interface.can_randomize_mac() {
                return Err(IronVeilError::PermissionDenied);
            }
            
            let old_mac = interface.mac_address;
            interface.set_mac_address(mac, Some(self.get_current_timestamp()))?;
            
            drop(interfaces);
            
            // Notify event handlers
            self.notify_event(InterfaceEvent::MacChanged {
                name: name.to_string(),
                old_mac,
                new_mac: mac,
            });
            
            // Apply MAC change to hardware (would integrate with actual driver)
            self.apply_mac_to_hardware(name, mac)?;
            
            Ok(())
        } else {
            Err(IronVeilError::InterfaceNotFound)
        }
    }
    
    /// Randomize MAC address for interface
    pub fn randomize_mac_address(&self, name: &str) -> Result<MacAddr> {
        let new_mac = crate::generate_secure_mac();
        self.set_mac_address(name, new_mac)?;
        Ok(new_mac)
    }
    
    /// Randomize MAC addresses for all eligible interfaces
    pub fn randomize_all_mac_addresses(&self) -> Result<Vec<(String, MacAddr)>> {
        let interfaces = self.interfaces.lock();
        let interface_names: Vec<String> = interfaces
            .values()
            .filter(|iface| iface.can_randomize_mac())
            .map(|iface| iface.name.clone())
            .collect();
        drop(interfaces);
        
        let mut results = Vec::new();
        
        for name in interface_names {
            match self.randomize_mac_address(&name) {
                Ok(mac) => results.push((name, mac)),
                Err(_) => continue, // Skip failed randomizations
            }
        }
        
        Ok(results)
    }
    
    /// Add IP address to interface
    pub fn add_ip_address(&self, name: &str, ip: Ipv4Addr) -> Result<()> {
        let mut interfaces = self.interfaces.lock();
        
        if let Some(interface) = interfaces.get_mut(name) {
            interface.add_ip_address(ip);
            
            drop(interfaces);
            
            // Notify event handlers
            self.notify_event(InterfaceEvent::IpChanged {
                name: name.to_string(),
                ip,
                added: true,
            });
            
            Ok(())
        } else {
            Err(IronVeilError::InterfaceNotFound)
        }
    }
    
    /// Remove IP address from interface
    pub fn remove_ip_address(&self, name: &str, ip: &Ipv4Addr) -> Result<()> {
        let mut interfaces = self.interfaces.lock();
        
        if let Some(interface) = interfaces.get_mut(name) {
            interface.remove_ip_address(ip);
            
            drop(interfaces);
            
            // Notify event handlers
            self.notify_event(InterfaceEvent::IpChanged {
                name: name.to_string(),
                ip: *ip,
                added: false,
            });
            
            Ok(())
        } else {
            Err(IronVeilError::InterfaceNotFound)
        }
    }
    
    /// Enable auto MAC randomization
    pub fn enable_auto_randomization(&self, interval_ms: u64) {
        self.auto_randomize.store(true, Ordering::SeqCst);
        self.randomize_interval.store(interval_ms, Ordering::SeqCst);
    }
    
    /// Disable auto MAC randomization
    pub fn disable_auto_randomization(&self) {
        self.auto_randomize.store(false, Ordering::SeqCst);
    }
    
    /// Check if auto randomization should run
    pub fn should_auto_randomize(&self, current_timestamp: u64) -> bool {
        if !self.auto_randomize.load(Ordering::SeqCst) {
            return false;
        }
        
        let interval = self.randomize_interval.load(Ordering::SeqCst);
        let interfaces = self.interfaces.lock();
        
        for interface in interfaces.values() {
            if let Some(last_change) = interface.last_mac_change {
                if current_timestamp.saturating_sub(last_change) >= interval {
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Add event handler
    pub fn add_event_handler(&self, handler: Box<dyn InterfaceEventHandler>) {
        self.event_handlers.lock().push(handler);
    }
    
    /// Update interface statistics
    pub fn update_interface_stats(&self, name: &str, bytes_sent: u64, bytes_received: u64, packets_sent: u64, packets_received: u64) -> Result<()> {
        let mut interfaces = self.interfaces.lock();
        
        if let Some(interface) = interfaces.get_mut(name) {
            interface.update_stats(bytes_sent, bytes_received, packets_sent, packets_received);
            Ok(())
        } else {
            Err(IronVeilError::InterfaceNotFound)
        }
    }
    
    /// Get interface statistics
    pub fn get_interface_stats(&self, name: &str) -> Result<InterfaceStats> {
        let interfaces = self.interfaces.lock();
        
        if let Some(interface) = interfaces.get(name) {
            Ok(interface.stats.clone())
        } else {
            Err(IronVeilError::InterfaceNotFound)
        }
    }
    
    /// Restore original MAC addresses for all interfaces
    pub fn restore_all_original_macs(&self) -> Result<()> {
        let interfaces = self.interfaces.lock();
        let interface_names: Vec<String> = interfaces.keys().cloned().collect();
        drop(interfaces);
        
        for name in interface_names {
            self.restore_original_mac(&name)?;
        }
        
        Ok(())
    }
    
    /// Restore original MAC address for specific interface
    pub fn restore_original_mac(&self, name: &str) -> Result<()> {
        let mut interfaces = self.interfaces.lock();
        
        if let Some(interface) = interfaces.get_mut(name) {
            let original_mac = interface.original_mac;
            interface.restore_original_mac();
            
            drop(interfaces);
            
            // Apply MAC change to hardware
            self.apply_mac_to_hardware(name, original_mac)?;
            
            Ok(())
        } else {
            Err(IronVeilError::InterfaceNotFound)
        }
    }
    
    /// Private helper methods
    
    fn notify_event(&self, event: InterfaceEvent) {
        let mut handlers = self.event_handlers.lock();
        for handler in handlers.iter_mut() {
            handler.handle_event(event.clone());
        }
    }
    
    fn apply_mac_to_hardware(&self, _name: &str, _mac: MacAddr) -> Result<()> {
        // This would integrate with actual hardware drivers
        // For now, this is a placeholder
        
        // Real implementation would:
        // 1. Find the network driver for this interface
        // 2. Call driver's set_mac_address function
        // 3. Verify the change was applied
        // 4. Handle any hardware-specific requirements
        
        #[cfg(feature = "logging")]
        log::info!("Applied MAC address {} to interface {}", _mac, _name);
        
        Ok(())
    }
    
    fn get_current_timestamp(&self) -> u64 {
        // This would get actual timestamp from kernel
        // For now, return a placeholder
        0
    }
}

/// Simple event handler for logging
pub struct LoggingEventHandler;

impl InterfaceEventHandler for LoggingEventHandler {
    fn handle_event(&mut self, event: InterfaceEvent) {
        #[cfg(feature = "logging")]
        match event {
            InterfaceEvent::Added { name } => {
                log::info!("Network interface added: {}", name);
            },
            InterfaceEvent::Removed { name } => {
                log::info!("Network interface removed: {}", name);
            },
            InterfaceEvent::StateChanged { name, old_state, new_state } => {
                log::info!("Interface {} state changed: {:?} -> {:?}", name, old_state, new_state);
            },
            InterfaceEvent::MacChanged { name, old_mac, new_mac } => {
                log::info!("Interface {} MAC changed: {} -> {}", name, old_mac, new_mac);
            },
            InterfaceEvent::IpChanged { name, ip, added } => {
                let action = if added { "added" } else { "removed" };
                log::info!("Interface {} IP {}: {}", name, action, ip);
            },
        }
        
        // For no_std environments without logging, could use serial output
        #[cfg(not(feature = "logging"))]
        match event {
            InterfaceEvent::MacChanged { name, new_mac, .. } => {
                // Example: notify VGA/serial for important events
                // crate::vga::vprintln!("MAC randomized on {}: {}", name, new_mac);
            },
            _ => {},
        }
    }
}

/// Create a default interface manager with logging
pub fn create_default_interface_manager() -> InterfaceManager {
    let manager = InterfaceManager::new();
    
    // Add logging event handler
    let logging_handler = Box::new(LoggingEventHandler);
    manager.add_event_handler(logging_handler);
    
    manager
}

/// Scan and discover network interfaces (integration point with kernel)
pub fn discover_interfaces(manager: &InterfaceManager) -> Result<()> {
    // This would integrate with kernel driver discovery
    // For now, add some mock interfaces for testing
    
    // Example: Ethernet interface
    let eth_mac = MacAddr::new([0x00, 0x50, 0x56, 0x12, 0x34, 0x56]);
    manager.add_interface("eth0".to_string(), eth_mac)?;
    
    // Example: Wireless interface  
    let wifi_mac = MacAddr::new([0x00, 0x1C, 0x42, 0xAB, 0xCD, 0xEF]);
    manager.add_interface("wlan0".to_string(), wifi_mac)?;
    
    // Loopback interface
    let lo_mac = MacAddr::new([0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    manager.add_interface("lo".to_string(), lo_mac)?;
    
    #[cfg(feature = "logging")]
    log::info!("Network interface discovery completed");
    
    Ok(())
}
