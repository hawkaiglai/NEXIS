// privacy_core/src/mac_spoofing/randomization.rs
//! MAC address randomization implementation
//! 
//! DEPENDENCIES:
//! - Uses core types: MacAddr, Result, PrivacyError
//! - Implements entropy generation for secure randomization
//! 
//! INTEGRATION POINTS:
//! - Called by MAC spoofing service for address generation
//! - Uses system entropy sources (CPU timestamp, memory addresses)
//! - Provides realistic OUI database for believable MAC addresses

use crate::{MacAddr, Result, PrivacyError};

/// OUI (Organizationally Unique Identifier) database for realistic MACs
pub struct OuiDatabase {
    common_ouis: &'static [[u8; 3]],
    vendor_ouis: &'static [[u8; 3]],
}

impl OuiDatabase {
    pub const fn new() -> Self {
        Self {
            common_ouis: &[
                [0x00, 0x50, 0x56], // VMware
                [0x08, 0x00, 0x27], // VirtualBox  
                [0x52, 0x54, 0x00], // QEMU
                [0x00, 0x0C, 0x29], // VMware ESX
                [0x00, 0x1C, 0x42], // Parallels
                [0x00, 0x15, 0x5D], // Microsoft Hyper-V
                [0x00, 0x16, 0x3E], // Xen
                [0x00, 0x1B, 0x21], // Intel Corporation
                [0x00, 0x22, 0x15], // Cisco Systems
                [0x00, 0x24, 0x81], // Hewlett Packard
                [0x00, 0x26, 0x99], // Dell Inc.
                [0x00, 0x1E, 0x68], // Realtek Semiconductor
                [0x00, 0x19, 0x99], // Belkin International
                [0x00, 0x1F, 0x3F], // D-Link Corporation
                [0x00, 0x13, 0x02], // Netgear
                [0x00, 0x18, 0x39], // Cisco-Linksys
            ],
            vendor_ouis: &[
                [0x00, 0x11, 0x43], // Apple Inc
                [0x00, 0x23, 0x12], // Apple Inc
                [0x00, 0x25, 0x00], // Apple Inc
                [0x00, 0x26, 0x08], // Apple Inc
                [0x28, 0x18, 0x78], // Apple Inc
                [0x2C, 0x36, 0xF8], // Apple Inc
                [0x00, 0x03, 0x93], // Apple Inc
                [0x00, 0x05, 0x02], // Apple Inc
                [0x00, 0x0A, 0x27], // Apple Inc
                [0x00, 0x0A, 0x95], // Apple Inc
                [0x00, 0x0D, 0x93], // Apple Inc
                [0x00, 0x11, 0x24], // Apple Inc
                [0x00, 0x14, 0x51], // Apple Inc
                [0x00, 0x16, 0xCB], // Apple Inc
                [0x00, 0x17, 0xF2], // Apple Inc
                [0x00, 0x19, 0xE3], // Apple Inc
            ],
        }
    }
    
    pub fn get_random_oui(&self, rng: &mut dyn RandomGenerator) -> [u8; 3] {
        let use_vendor = rng.next_u32() % 100 < 30; // 30% chance to use vendor OUI
        
        let ouis = if use_vendor {
            self.vendor_ouis
        } else {
            self.common_ouis
        };
        
        let index = rng.next_u32() as usize % ouis.len();
        ouis[index]
    }
    
    pub fn get_realistic_oui(&self, rng: &mut dyn RandomGenerator) -> [u8; 3] {
        // Prefer common virtualization OUIs for believability
        let index = rng.next_u32() as usize % self.common_ouis.len();
        self.common_ouis[index]
    }
}

/// MAC address randomizer with realistic OUI selection
pub struct MacRandomizer {
    oui_db: OuiDatabase,
    entropy_source: Box<dyn RandomGenerator>,
    generation_counter: u64,
}

impl MacRandomizer {
    pub fn new(entropy_source: Box<dyn RandomGenerator>) -> Self {
        Self {
            oui_db: OuiDatabase::new(),
            entropy_source,
            generation_counter: 0,
        }
    }
    
    /// Generate MAC with realistic OUI (most believable)
    pub fn generate_realistic_mac(&mut self) -> MacAddr {
        let oui = self.oui_db.get_realistic_oui(&mut *self.entropy_source);
        self.generate_mac_with_oui(oui)
    }
    
    /// Generate MAC with random OUI selection
    pub fn generate_random_mac(&mut self) -> MacAddr {
        let oui = self.oui_db.get_random_oui(&mut *self.entropy_source);
        self.generate_mac_with_oui(oui)
    }
    
    /// Generate completely random MAC (less believable)
    pub fn generate_pure_random_mac(&mut self) -> MacAddr {
        let mut mac = [0u8; 6];
        
        for i in 0..6 {
            mac[i] = self.entropy_source.next_u8();
        }
        
        self.fix_mac_flags(&mut mac);
        self.generation_counter += 1;
        
        MacAddr(mac)
    }
    
    /// Generate MAC with specific OUI
    fn generate_mac_with_oui(&mut self, oui: [u8; 3]) -> MacAddr {
        let mut mac = [0u8; 6];
        
        // Set OUI (first 3 bytes)
        mac[0..3].copy_from_slice(&oui);
        
        // Generate random NIC-specific part (last 3 bytes)
        for i in 3..6 {
            mac[i] = self.entropy_source.next_u8();
        }
        
        // Ensure the OUI flags are correct (in case the OUI data is wrong)
        self.fix_mac_flags(&mut mac);
        self.generation_counter += 1;
        
        MacAddr(mac)
    }
    
    /// Fix MAC address flags to ensure valid unicast, locally administered address
    fn fix_mac_flags(&self, mac: &mut [u8; 6]) {
        // Clear multicast bit (bit 0 of first byte)
        mac[0] &= 0xFE;
        
        // Set locally administered bit (bit 1 of first byte) 
        mac[0] |= 0x02;
    }
    
    /// Generate MAC with vendor-style pattern
    pub fn generate_vendor_style_mac(&mut self, vendor_pattern: VendorPattern) -> MacAddr {
        let mut mac = match vendor_pattern {
            VendorPattern::Apple => {
                let oui = self.oui_db.vendor_ouis[self.entropy_source.next_u32() as usize % 16];
                self.generate_mac_with_oui(oui)
            }
            VendorPattern::Intel => {
                self.generate_mac_with_oui([0x00, 0x1B, 0x21])
            }
            VendorPattern::Cisco => {
                self.generate_mac_with_oui([0x00, 0x22, 0x15])
            }
            VendorPattern::Dell => {
                self.generate_mac_with_oui([0x00, 0x26, 0x99])
            }
            VendorPattern::Hp => {
                self.generate_mac_with_oui([0x00, 0x24, 0x81])
            }
        };
        
        mac
    }
    
    /// Get generation statistics
    pub fn get_stats(&self) -> MacGenerationStats {
        MacGenerationStats {
            total_generated: self.generation_counter,
            entropy_health: self.entropy_source.health_check(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum VendorPattern {
    Apple,
    Intel,
    Cisco,
    Dell,
    Hp,
}

#[derive(Debug, Clone)]
pub struct MacGenerationStats {
    pub total_generated: u64,
    pub entropy_health: EntropyHealth,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EntropyHealth {
    Excellent,
    Good,
    Fair,
    Poor,
    Critical,
}

/// Random number generation trait for entropy sources
pub trait RandomGenerator {
    fn next_u8(&mut self) -> u8;
    fn next_u32(&mut self) -> u32;
    fn next_u64(&mut self) -> u64;
    fn health_check(&self) -> EntropyHealth;
}

/// System entropy source using multiple hardware sources
pub struct SystemEntropy {
    state: [u64; 4],
    counter: u64,
    cycles_since_init: u64,
}

impl SystemEntropy {
    pub fn new() -> Self {
        let mut entropy = Self {
            state: [0; 4],
            counter: 0,
            cycles_since_init: 0,
        };
        entropy.reseed();
        entropy
    }
    
    /// Reseed entropy pool from multiple sources
    fn reseed(&mut self) {
        // Use CPU timestamp counter as primary entropy
        let tsc = Self::read_tsc();
        
        // Use memory addresses as secondary entropy
        let stack_addr = &self as *const _ as usize as u64;
        let heap_entropy = Self::get_heap_entropy();
        
        // Use interrupt timing if available
        let timing_entropy = Self::get_timing_entropy();
        
        // Use hardware random if available (RDRAND instruction)
        let hw_random = Self::try_hardware_random();
        
        // Combine all entropy sources using a mixing function
        self.state[0] ^= tsc.wrapping_add(self.counter);
        self.state[1] ^= stack_addr.wrapping_add(timing_entropy);
        self.state[2] ^= heap_entropy.wrapping_add(hw_random);
        self.state[3] ^= (tsc >> 32).wrapping_add(stack_addr >> 16);
        
        // Apply avalanche mixing
        for _ in 0..3 {
            self.mix_state();
        }
        
        self.counter = self.counter.wrapping_add(1);
    }
    
    /// Mix internal state for better distribution
    fn mix_state(&mut self) {
        for i in 0..4 {
            self.state[i] = self.state[i].wrapping_mul(0x9E3779B97F4A7C15);
            self.state[i] ^= self.state[i] >> 30;
            self.state[i] = self.state[i].wrapping_mul(0xBF58476D1CE4E5B9);
            self.state[i] ^= self.state[i] >> 27;
            self.state[i] = self.state[i].wrapping_mul(0x94D049BB133111EB);
            self.state[i] ^= self.state[i] >> 31;
        }
    }
    
    /// Read CPU timestamp counter
    fn read_tsc() -> u64 {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::x86_64::_rdtsc()
        }
        
        #[cfg(not(target_arch = "x86_64"))]
        {
            // Fallback for other architectures
            0xDEADBEEFCAFEBABE
        }
    }
    
    /// Get entropy from heap/memory layout
    fn get_heap_entropy() -> u64 {
        // Use various memory addresses as entropy
        let addr1 = Self::read_tsc as *const _ as usize as u64;
        let addr2 = SystemEntropy::new as *const _ as usize as u64;
        addr1.wrapping_add(addr2).wrapping_mul(0x517CC1B727220A95)
    }
    
    /// Get timing-based entropy
    fn get_timing_entropy() -> u64 {
        let start = Self::read_tsc();
        // Small delay with variable timing
        for i in 0..100 {
            core::hint::black_box(i);
        }
        let end = Self::read_tsc();
        end.wrapping_sub(start)
    }
    
    /// Try to use hardware random number generator
    fn try_hardware_random() -> u64 {
        #[cfg(target_arch = "x86_64")]
        {
            // Try RDRAND instruction
            let mut result = 0u64;
            unsafe {
                if core::arch::x86_64::_rdrand64_step(&mut result) == 1 {
                    return result;
                }
            }
        }
        
        // Fallback: use timestamp and mixing
        Self::read_tsc().wrapping_mul(0x9E3779B97F4A7C15)
    }
    
    /// Xorshift algorithm for internal state advancement
    fn xorshift(&mut self) -> u64 {
        let mut x = self.state[0];
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state[0] = x;
        
        // Rotate state
        self.state.rotate_left(1);
        self.cycles_since_init = self.cycles_since_init.wrapping_add(1);
        
        // Periodic reseeding for long-running systems
        if self.cycles_since_init % 1000 == 0 {
            self.reseed();
        }
        
        x
    }
}

impl RandomGenerator for SystemEntropy {
    fn next_u8(&mut self) -> u8 {
        (self.xorshift() & 0xFF) as u8
    }
    
    fn next_u32(&mut self) -> u32 {
        (self.xorshift() & 0xFFFFFFFF) as u32
    }
    
    fn next_u64(&mut self) -> u64 {
        self.xorshift()
    }
    
    fn health_check(&self) -> EntropyHealth {
        // Simple health check based on state diversity
        let diversity = self.state.iter()
            .map(|&x| x.count_ones())
            .sum::<u32>();
            
        match diversity {
            200..=256 => EntropyHealth::Excellent,
            150..=199 => EntropyHealth::Good,
            100..=149 => EntropyHealth::Fair,
            50..=99 => EntropyHealth::Poor,
            _ => EntropyHealth::Critical,
        }
    }
}

/// Simple linear congruential generator for testing/fallback
pub struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }
}

impl RandomGenerator for SimpleRng {
    fn next_u8(&mut self) -> u8 {
        (self.next_u32() & 0xFF) as u8
    }
    
    fn next_u32(&mut self) -> u32 {
        (self.next_u64() & 0xFFFFFFFF) as u32
    }
    
    fn next_u64(&mut self) -> u64 {
        // Linear congruential generator constants from Numerical Recipes
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        self.state
    }
    
    fn health_check(&self) -> EntropyHealth {
        // LCG has predictable patterns, so always fair
        EntropyHealth::Fair
    }
}
