// privacy_core/src/mac_spoofing/mod.rs
//! MAC address spoofing and randomization module
//! 
//! DEPENDENCIES:
//! - Uses core types: MacAddr, Result, PrivacyError
//! - Implements MacSpoofing trait for network interface manipulation
//! 
//! INTEGRATION POINTS:
//! - Called by ironveil_ui menu system for MAC randomization
//! - Uses network drivers from nexis_kernel for interface control
//! - Integrates with system entropy sources for randomization

pub mod randomization;

use crate::{MacAddr, Result, PrivacyError};
use spin::Mutex;
use lazy_static::lazy_static;

pub use randomization::{MacRandomizer, OuiDatabase, SystemEntropy};

lazy_static! {
    static ref MAC_SPOOFING_SERVICE: Mutex<Option<MacSpoofingService>> = Mutex::new(None);
}

/// Network interface information
#[derive(Debug, Clone)]
pub struct NetworkInterface {
    pub name: &'static str,
    pub mac: MacAddr,
    pub is_up: bool,
    pub interface_type: InterfaceType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InterfaceType {
    Ethernet,
    Wireless,
    Loopback,
    Unknown,
}

/// MAC spoofing service that manages network interfaces
pub struct MacSpoofingService {
    interfaces: [Option<NetworkInterface>; 8],
    randomizer: MacRandomizer,
}

impl MacSpoofingService {
    pub fn new() -> Self {
        Self {
            interfaces: [None; 8],
            randomizer: MacRandomizer::new(Box::new(SystemEntropy::new())),
        }
    }

    /// Register a network interface
    pub fn register_interface(&mut self, interface: NetworkInterface) -> Result<()> {
        for slot in &mut self.interfaces {
            if slot.is_none() {
                *slot = Some(interface);
                return Ok(());
            }
        }
        Err(PrivacyError::OperationFailed)
    }

    /// Find interface by name
    pub fn find_interface(&self, name: &str) -> Option<&NetworkInterface> {
        self.interfaces.iter()
            .filter_map(|slot| slot.as_ref())
            .find(|iface| iface.name == name)
    }

    /// Find interface by name (mutable)
    pub fn find_interface_mut(&mut self, name: &str) -> Option<&mut NetworkInterface> {
        self.interfaces.iter_mut()
            .filter_map(|slot| slot.as_mut())
            .find(|iface| iface.name == name)
    }

    /// List all registered interfaces
    pub fn list_interfaces(&self) -> impl Iterator<Item = &NetworkInterface> {
        self.interfaces.iter().filter_map(|slot| slot.as_ref())
    }

    /// Randomize MAC address for specific interface
    pub fn randomize_interface_mac(&mut self, interface_name: &str) -> Result<MacAddr> {
        let new_mac = self.randomizer.generate_realistic_mac();
        
        if let Some(interface) = self.find_interface_mut(interface_name) {
            let old_mac = interface.mac;
            interface.mac = new_mac;
            
            // In a real implementation, this would call the network driver
            // to actually set the MAC address on the hardware
            self.apply_mac_to_hardware(interface_name, new_mac)?;
            
            Ok(new_mac)
        } else {
            Err(PrivacyError::NetworkInterfaceNotFound)
        }
    }

    /// Apply MAC address to hardware (placeholder for driver integration)
    fn apply_mac_to_hardware(&self, interface_name: &str, mac: MacAddr) -> Result<()> {
        // This would integrate with the actual network drivers
        // For now, we simulate the operation
        if mac.is_multicast() || mac.is_zero() {
            return Err(PrivacyError::InvalidMacAddress);
        }
        
        // Simulate hardware operation
        Ok(())
    }

    /// Get randomizer for manual MAC generation
    pub fn get_randomizer(&mut self) -> &mut MacRandomizer {
        &mut self.randomizer
    }
}

/// MAC spoofing trait for network interface manipulation
pub trait MacSpoofing {
    /// Generate a random MAC address
    fn generate_random_mac(&mut self) -> MacAddr;
    
    /// Apply MAC address to specific network interface
    fn apply_mac_to_interface(&mut self, interface: &str, mac: MacAddr) -> Result<()>;
    
    /// Get current MAC address of interface
    fn get_current_mac(&self, interface: &str) -> Result<MacAddr>;
    
    /// Randomize MAC for all interfaces
    fn randomize_all_interfaces(&mut self) -> Result<()>;
}

impl MacSpoofing for MacSpoofingService {
    fn generate_random_mac(&mut self) -> MacAddr {
        self.randomizer.generate_realistic_mac()
    }
    
    fn apply_mac_to_interface(&mut self, interface: &str, mac: MacAddr) -> Result<()> {
        if let Some(iface) = self.find_interface_mut(interface) {
            iface.mac = mac;
            self.apply_mac_to_hardware(interface, mac)
        } else {
            Err(PrivacyError::NetworkInterfaceNotFound)
        }
    }
    
    fn get_current_mac(&self, interface: &str) -> Result<MacAddr> {
        self.find_interface(interface)
            .map(|iface| iface.mac)
            .ok_or(PrivacyError::NetworkInterfaceNotFound)
    }
    
    fn randomize_all_interfaces(&mut self) -> Result<()> {
        let interface_names: Vec<&str> = self.list_interfaces()
            .map(|iface| iface.name)
            .collect();
            
        for name in interface_names {
            self.randomize_interface_mac(name)?;
        }
        
        Ok(())
    }
}

/// Initialize MAC spoofing service
pub fn init() -> Result<()> {
    let service = MacSpoofingService::new();
    *MAC_SPOOFING_SERVICE.lock() = Some(service);
    Ok(())
}

/// Get global MAC spoofing service
pub fn get_service() -> Option<impl core::ops::DerefMut<Target = MacSpoofingService>> {
    MAC_SPOOFING_SERVICE.try_lock().ok().and_then(|mut guard| {
        if guard.is_some() {
            Some(spin::MutexGuard::map(guard, |opt| opt.as_mut().unwrap()))
        } else {
            None
        }
    })
}

/// Register a network interface with the MAC spoofing service
pub fn register_interface(interface: NetworkInterface) -> Result<()> {
    if let Some(mut service) = get_service() {
        service.register_interface(interface)
    } else {
        Err(PrivacyError::OperationFailed)
    }
}

/// Randomize MAC address for a specific interface
pub fn randomize_mac(interface_name: &str) -> Result<MacAddr> {
    if let Some(mut service) = get_service() {
        service.randomize_interface_mac(interface_name)
    } else {
        Err(PrivacyError::OperationFailed)
    }
}
