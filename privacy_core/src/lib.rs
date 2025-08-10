#![no_std]
#![no_main]

extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use core::fmt;

pub mod network_hooks;

// Re-export network hooks for convenience
pub use network_hooks::{NetworkHook, NetworkHookManager, PacketAction};
pub use network_hooks::interface_manager::{
    NetworkInterface, InterfaceManager, InterfaceState, InterfaceType
};

/// Core error types for privacy operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IronVeilError {
    OutOfMemory,
    InvalidAddress,
    TaskNotFound,
    NetworkError,
    PrivacyViolation,
    HardwareError,
    InterfaceNotFound,
    InvalidMacAddress,
    TorConnectionFailed,
    InvalidPacket,
    PermissionDenied,
}

impl fmt::Display for IronVeilError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            IronVeilError::OutOfMemory => write!(f, "Out of memory"),
            IronVeilError::InvalidAddress => write!(f, "Invalid address"),
            IronVeilError::TaskNotFound => write!(f, "Task not found"),
            IronVeilError::NetworkError => write!(f, "Network error"),
            IronVeilError::PrivacyViolation => write!(f, "Privacy violation detected"),
            IronVeilError::HardwareError => write!(f, "Hardware error"),
            IronVeilError::InterfaceNotFound => write!(f, "Network interface not found"),
            IronVeilError::InvalidMacAddress => write!(f, "Invalid MAC address"),
            IronVeilError::TorConnectionFailed => write!(f, "Tor connection failed"),
            IronVeilError::InvalidPacket => write!(f, "Invalid packet format"),
            IronVeilError::PermissionDenied => write!(f, "Permission denied"),
        }
    }
}

pub type Result<T> = core::result::Result<T, IronVeilError>;

/// MAC Address representation
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MacAddr(pub [u8; 6]);

impl MacAddr {
    /// Create a new MAC address
    pub const fn new(bytes: [u8; 6]) -> Self {
        Self(bytes)
    }

    /// Create MAC address from slice
    pub fn from_slice(slice: &[u8]) -> Result<Self> {
        if slice.len() != 6 {
            return Err(IronVeilError::InvalidMacAddress);
        }
        let mut bytes = [0u8; 6];
        bytes.copy_from_slice(slice);
        Ok(Self(bytes))
    }

    /// Get MAC address as bytes
    pub fn as_bytes(&self) -> &[u8; 6] {
        &self.0
    }

    /// Check if MAC is multicast
    pub fn is_multicast(&self) -> bool {
        (self.0[0] & 0x01) != 0
    }

    /// Check if MAC is locally administered
    pub fn is_locally_administered(&self) -> bool {
        (self.0[0] & 0x02) != 0
    }

    /// Check if MAC is broadcast
    pub fn is_broadcast(&self) -> bool {
        self.0 == [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
    }

    /// Create a locally administered MAC (for spoofing)
    pub fn locally_administered(mut bytes: [u8; 6]) -> Self {
        bytes[0] &= 0xFE; // Clear multicast bit
        bytes[0] |= 0x02; // Set locally administered bit
        Self(bytes)
    }
}

impl fmt::Display for MacAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
               self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5])
    }
}

/// IPv4 Address representation
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Ipv4Addr(pub [u8; 4]);

impl Ipv4Addr {
    /// Create new IPv4 address
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self([a, b, c, d])
    }

    /// Create from bytes
    pub fn from_bytes(bytes: [u8; 4]) -> Self {
        Self(bytes)
    }

    /// Get as bytes
    pub fn as_bytes(&self) -> &[u8; 4] {
        &self.0
    }

    /// Check if address is private
    pub fn is_private(&self) -> bool {
        match self.0[0] {
            10 => true,
            172 => self.0[1] >= 16 && self.0[1] <= 31,
            192 => self.0[1] == 168,
            _ => false,
        }
    }

    /// Check if address is loopback
    pub fn is_loopback(&self) -> bool {
        self.0[0] == 127
    }
}

impl fmt::Display for Ipv4Addr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}.{}.{}", self.0[0], self.0[1], self.0[2], self.0[3])
    }
}

/// MAC address spoofing trait
pub trait MacSpoofing {
    /// Generate a random MAC address
    fn generate_random_mac(&self) -> MacAddr;
    
    /// Apply MAC address to network interface
    fn apply_mac_to_interface(&mut self, interface: &str, mac: MacAddr) -> Result<()>;
    
    /// Get current MAC address of interface
    fn get_current_mac(&self, interface: &str) -> Result<MacAddr>;
}

/// Tor integration status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TorStatus {
    Stopped,
    Starting,
    Running,
    Error,
}

impl fmt::Display for TorStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TorStatus::Stopped => write!(f, "Stopped"),
            TorStatus::Starting => write!(f, "Starting"),
            TorStatus::Running => write!(f, "Running"),
            TorStatus::Error => write!(f, "Error"),
        }
    }
}

/// Tor integration trait
pub trait TorIntegration {
    /// Start Tor daemon
    fn start_tor_daemon(&mut self) -> Result<()>;
    
    /// Stop Tor daemon
    fn stop_tor_daemon(&mut self) -> Result<()>;
    
    /// Get current Tor status
    fn get_tor_status(&self) -> TorStatus;
    
    /// Route traffic through Tor
    fn route_through_tor(&mut self, destination: Ipv4Addr, port: u16) -> Result<()>;
}

/// Privacy configuration
#[derive(Debug, Clone)]
pub struct PrivacyConfig {
    pub mac_randomization_enabled: bool,
    pub tor_enabled: bool,
    pub packet_filtering_enabled: bool,
    pub dns_over_tor: bool,
    pub block_ipv6: bool,
    pub randomize_mac_on_boot: bool,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            mac_randomization_enabled: true,
            tor_enabled: false,
            packet_filtering_enabled: true,
            dns_over_tor: false,
            block_ipv6: true,
            randomize_mac_on_boot: true,
        }
    }
}

/// Privacy status information
#[derive(Debug, Clone)]
pub struct PrivacyStatus {
    pub mac_randomized: bool,
    pub current_mac: Option<MacAddr>,
    pub tor_status: TorStatus,
    pub packets_filtered: u64,
    pub privacy_violations_detected: u64,
    pub last_mac_change: Option<u64>, // timestamp
}

impl Default for PrivacyStatus {
    fn default() -> Self {
        Self {
            mac_randomized: false,
            current_mac: None,
            tor_status: TorStatus::Stopped,
            packets_filtered: 0,
            privacy_violations_detected: 0,
            last_mac_change: None,
        }
    }
}

/// Random number generator trait for privacy operations
pub trait RandomGenerator {
    fn next_u8(&mut self) -> u8;
    fn next_u32(&mut self) -> u32;
    fn next_u64(&mut self) -> u64;
    
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        for byte in dest.iter_mut() {
            *byte = self.next_u8();
        }
    }
}

/// Simple XorShift random number generator
pub struct XorShiftRng {
    state: u64,
}

impl XorShiftRng {
    pub fn new(seed: u64) -> Self {
        Self { 
            state: if seed == 0 { 1 } else { seed }
        }
    }
    
    pub fn from_entropy() -> Self {
        // Use timestamp and memory address as entropy source
        let entropy = unsafe {
            let ptr = &0u8 as *const u8 as u64;
            ptr.wrapping_mul(0x9e3779b97f4a7c15)
        };
        Self::new(entropy)
    }
}

impl RandomGenerator for XorShiftRng {
    fn next_u64(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }
    
    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }
    
    fn next_u8(&mut self) -> u8 {
        self.next_u64() as u8
    }
}

/// Privacy command types for inter-module communication
#[derive(Debug, Clone)]
pub enum PrivacyCommand {
    RandomizeMac { interface: String },
    StartTor,
    StopTor,
    EnablePacketFiltering,
    DisablePacketFiltering,
    SetMacAddress { interface: String, mac: MacAddr },
    BlockDomain { domain: String },
    AllowDomain { domain: String },
}

/// Status update types for UI communication
#[derive(Debug, Clone)]
pub enum StatusUpdate {
    MacRandomized { interface: String, new_mac: MacAddr },
    TorStatusChanged { status: TorStatus },
    PacketFiltered { reason: String },
    PrivacyViolationDetected { description: String },
    InterfaceStateChanged { interface: String, state: String },
}

/// Initialize privacy core subsystems
pub fn init() -> Result<()> {
    // Initialize network hooks
    network_hooks::init()?;
    
    // Log initialization
    #[cfg(feature = "logging")]
    log::info!("Privacy core initialized successfully");
    
    Ok(())
}

/// Get default privacy configuration
pub fn default_config() -> PrivacyConfig {
    PrivacyConfig::default()
}

/// Create a secure random MAC address
pub fn generate_secure_mac() -> MacAddr {
    let mut rng = XorShiftRng::from_entropy();
    let mut bytes = [0u8; 6];
    rng.fill_bytes(&mut bytes);
    MacAddr::locally_administered(bytes)
}

/// Validate MAC address format
pub fn validate_mac(mac: &MacAddr) -> Result<()> {
    if mac.is_multicast() {
        return Err(IronVeilError::InvalidMacAddress);
    }
    
    // Check for all-zero MAC
    if mac.as_bytes().iter().all(|&b| b == 0) {
        return Err(IronVeilError::InvalidMacAddress);
    }
    
    Ok(())
}

/// Common OUI (Organizationally Unique Identifier) database
pub mod oui {
    use super::MacAddr;
    
    /// Common OUIs for realistic MAC generation
    pub const COMMON_OUIS: &[[u8; 3]] = &[
        [0x00, 0x50, 0x56], // VMware
        [0x08, 0x00, 0x27], // VirtualBox
        [0x52, 0x54, 0x00], // QEMU
        [0x00, 0x0C, 0x29], // VMware ESX
        [0x00, 0x1C, 0x42], // Parallels
        [0x00, 0x15, 0x5D], // Microsoft Hyper-V
        [0x00, 0x16, 0x3E], // Xen
        [0x00, 0x1B, 0x21], // Intel
        [0x00, 0x19, 0x99], // Cisco
        [0x00, 0x24, 0xD6], // Broadcom
    ];
    
    /// Get a random OUI from the database
    pub fn random_oui(rng: &mut dyn super::RandomGenerator) -> [u8; 3] {
        let index = (rng.next_u32() as usize) % COMMON_OUIS.len();
        COMMON_OUIS[index]
    }
    
    /// Generate MAC with realistic OUI
    pub fn generate_realistic_mac(rng: &mut dyn super::RandomGenerator) -> MacAddr {
        let oui = random_oui(rng);
        let mut bytes = [0u8; 6];
        
        // Set OUI
        bytes[0..3].copy_from_slice(&oui);
        
        // Generate random NIC part
        bytes[3] = rng.next_u8();
        bytes[4] = rng.next_u8();
        bytes[5] = rng.next_u8();
        
        MacAddr::locally_administered(bytes)
    }
}
