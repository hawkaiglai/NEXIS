//! Driver subsystem for Nexis kernel
//! 
//! DEPENDENCIES:
//! - Uses x86_64 crate for port I/O operations
//! - Uses spin and lazy_static for synchronization
//! - Integrates with interrupt system
//! 
//! INTEGRATION POINTS:
//! - Called by main.rs during driver initialization
//! - VGA driver used by shell and kernel output
//! - Keyboard driver used by shell for input
//! - RTC driver used for system time and timer interrupts
//! 
//! TESTING REQUIREMENTS:
//! - Each driver must initialize without panicking
//! - VGA output must be visible and correctly formatted
//! - Keyboard input must be properly decoded and queued
//! - RTC must provide accurate time readings

pub mod vga;
pub mod kb;
pub mod rtc;

use crate::logging::define_logger;
define_logger!("drivers");

/// Initialize all system drivers in the correct order
pub fn init_all_drivers() -> Result<(), &'static str> {
    log_info!("Initializing system drivers...");
    
    // VGA must be first for early output
    vga::init().map_err(|_| "Failed to initialize VGA driver")?;
    log_info!("VGA driver initialized");
    
    // Keyboard driver for user input
    kb::init().map_err(|_| "Failed to initialize keyboard driver")?;
    log_info!("Keyboard driver initialized");
    
    // RTC for system time
    rtc::init().map_err(|_| "Failed to initialize RTC driver")?;
    log_info!("RTC driver initialized");
    
    log_info!("All drivers initialized successfully");
    Ok(())
}

/// Driver trait that all drivers must implement
pub trait Driver {
    type Error;
    
    fn name(&self) -> &'static str;
    fn init(&mut self) -> Result<(), Self::Error>;
    fn cleanup(&mut self) -> Result<(), Self::Error>;
    fn is_initialized(&self) -> bool;
}

/// Input driver trait for devices that provide input
pub trait InputDriver: Driver {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error>;
    fn has_input(&self) -> bool;
}

/// Output driver trait for devices that display output
pub trait OutputDriver: Driver {
    fn write(&mut self, buffer: &[u8]) -> Result<usize, Self::Error>;
    fn flush(&mut self) -> Result<(), Self::Error>;
}

/// Timer driver trait for time-related functionality
pub trait TimerDriver: Driver {
    fn get_timestamp(&self) -> u64;
    fn set_alarm(&mut self, timestamp: u64) -> Result<(), Self::Error>;
}
