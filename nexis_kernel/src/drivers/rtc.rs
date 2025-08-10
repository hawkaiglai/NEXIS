//! Real-Time Clock (RTC) driver for Nexis kernel
//! 
//! DEPENDENCIES:
//! - Uses x86_64 crate for port I/O operations
//! - Uses spin::Mutex for thread-safe access
//! - Uses lazy_static for global RTC instance
//! 
//! INTEGRATION POINTS:
//! - Called by interrupt handler for RTC interrupts (IRQ8)
//! - Used by kernel for system timestamps
//! - Integrates with scheduler for time-based operations
//! - Used by logging system for timestamped entries

use spin::Mutex;
use lazy_static::lazy_static;
use x86_64::instructions::port::Port;
use super::{Driver, TimerDriver};

// RTC I/O ports
const RTC_REGISTER_SELECT: u16 = 0x70;
const RTC_DATA_PORT: u16 = 0x71;

// RTC register addresses
const RTC_SECONDS: u8 = 0x00;
const RTC_MINUTES: u8 = 0x02;
const RTC_HOURS: u8 = 0x04;
const RTC_WEEKDAY: u8 = 0x06;
const RTC_DAY: u8 = 0x07;
const RTC_MONTH: u8 = 0x08;
const RTC_YEAR: u8 = 0x09;
const RTC_STATUS_A: u8 = 0x0A;
const RTC_STATUS_B: u8 = 0x0B;
const RTC_STATUS_C: u8 = 0x0C;

// Status register B flags
const RTC_24_HOUR: u8 = 0x02;
const RTC_BINARY: u8 = 0x04;
const RTC_PERIODIC_INTERRUPT: u8 = 0x40;
const RTC_UPDATE_INTERRUPT: u8 = 0x10;

lazy_static! {
    static ref RTC: Mutex<RtcDriver> = Mutex::new(RtcDriver::new());
}

#[derive(Debug, Clone, Copy)]
pub struct DateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub weekday: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl DateTime {
    pub fn new() -> Self {
        Self {
            year: 0,
            month: 0,
            day: 0,
            weekday: 0,
            hour: 0,
            minute: 0,
            second: 0,
        }
    }
    
    /// Convert to Unix timestamp (seconds since January 1, 1970)
    pub fn to_unix_timestamp(&self) -> u64 {
        if self.year < 1970 {
            return 0;
        }
        
        let mut days = 0u64;
        
        // Add days for complete years
        for year in 1970..self.year {
            if is_leap_year(year) {
                days += 366;
            } else {
                days += 365;
            }
        }
        
        // Add days for complete months this year
        for month in 1..self.month {
            days += days_in_month(month, self.year) as u64;
        }
        
        // Add remaining days
        days += (self.day - 1) as u64;
        
        // Convert to seconds
        let seconds = days * 24 * 60 * 60;
        seconds + (self.hour as u64 * 3600) + (self.minute as u64 * 60) + self.second as u64
    }
    
    pub fn format(&self) -> heapless::String<32> {
        let mut s = heapless::String::new();
        use core::fmt::Write;
        let _ = write!(s, "{:04}-{:02}-{:02} {:02}:{:02}:{:02}", 
                     self.year, self.month, self.day,
                     self.hour, self.minute, self.second);
        s
    }
}

fn is_leap_year(year: u16) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn days_in_month(month: u8, year: u16) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => if is_leap_year(year) { 29 } else { 28 },
        _ => 0,
    }
}

pub struct RtcDriver {
    register_port: Port<u8>,
    data_port: Port<u8>,
    initialized: bool,
    boot_time: u64,
    last_update: u64,
    tick_count: u64,
}

impl RtcDriver {
    const fn new() -> Self {
        Self {
            register_port: Port::new(RTC_REGISTER_SELECT),
            data_port: Port::new(RTC_DATA_PORT),
            initialized: false,
            boot_time: 0,
            last_update: 0,
            tick_count: 0,
        }
    }
    
    fn read_register(&mut self, register: u8) -> u8 {
        unsafe {
            // Disable NMI and select register
            self.register_port.write(register | 0x80);
            // Small delay for hardware
            for _ in 0..100 { core::hint::spin_loop(); }
            // Read data
            self.data_port.read()
        }
    }
    
    fn write_register(&mut self, register: u8, value: u8) {
        unsafe {
            // Disable NMI and select register
            self.register_port.write(register | 0x80);
            // Small delay for hardware
            for _ in 0..100 { core::hint::spin_loop(); }
            // Write data
            self.data_port.write(value);
        }
    }
    
    fn wait_for_update_complete(&mut self) {
        // Wait until update is not in progress
        while self.read_register(RTC_STATUS_A) & 0x80 != 0 {
            core::hint::spin_loop();
        }
    }
    
    fn bcd_to_binary(bcd: u8) -> u8 {
        (bcd & 0x0F) + ((bcd >> 4) * 10)
    }
    
    fn binary_to_bcd(binary: u8) -> u8 {
        ((binary / 10) << 4) | (binary % 10)
    }
    
    pub fn read_datetime(&mut self) -> DateTime {
        self.wait_for_update_complete();
        
        let status_b = self.read_register(RTC_STATUS_B);
        let binary_mode = (status_b & RTC_BINARY) != 0;
        let hour_24 = (status_b & RTC_24_HOUR) != 0;
        
        let mut datetime = DateTime::new();
        
        // Read time values
        datetime.second = self.read_register(RTC_SECONDS);
        datetime.minute = self.read_register(RTC_MINUTES);
        datetime.hour = self.read_register(RTC_HOURS);
        datetime.weekday = self.read_register(RTC_WEEKDAY);
        datetime.day = self.read_register(RTC_DAY);
        datetime.month = self.read_register(RTC_MONTH);
        datetime.year = self.read_register(RTC_YEAR) as u16;
        
        // Convert from BCD if necessary
        if !binary_mode {
            datetime.second = Self::bcd_to_binary(datetime.second);
            datetime.minute = Self::bcd_to_binary(datetime.minute);
            datetime.hour = Self::bcd_to_binary(datetime.hour);
            datetime.day = Self::bcd_to_binary(datetime.day);
            datetime.month = Self::bcd_to_binary(datetime.month);
            datetime.year = Self::bcd_to_binary(datetime.year as u8) as u16;
        }
        
        // Handle 12/24 hour format
        if !hour_24 && datetime.hour & 0x80 != 0 {
            // PM in 12-hour format
            datetime.hour = ((datetime.hour & 0x7F) % 12) + 12;
        }
        
        // Convert 2-digit year to 4-digit (assume 21st century)
        if datetime.year < 100 {
            datetime.year += 2000;
        }
        
        datetime
    }
    
    pub fn set_datetime(&mut self, datetime: &DateTime) -> Result<(), &'static str> {
        if !self.initialized {
            return Err("RTC not initialized");
        }
        
        self.wait_for_update_complete();
        
        let status_b = self.read_register(RTC_STATUS_B);
        let binary_mode = (status_b & RTC_BINARY) != 0;
        
        // Convert values based on mode
        let second = if binary_mode { datetime.second } else { Self::binary_to_bcd(datetime.second) };
        let minute = if binary_mode { datetime.minute } else { Self::binary_to_bcd(datetime.minute) };
        let hour = if binary_mode { datetime.hour } else { Self::binary_to_bcd(datetime.hour) };
        let day = if binary_mode { datetime.day } else { Self::binary_to_bcd(datetime.day) };
        let month = if binary_mode { datetime.month } else { Self::binary_to_bcd(datetime.month) };
        let year = if binary_mode { 
            (datetime.year % 100) as u8 
        } else { 
            Self::binary_to_bcd((datetime.year % 100) as u8) 
        };
        
        // Disable updates while setting time
        let status_b_new = status_b | 0x80;
        self.write_register(RTC_STATUS_B, status_b_new);
        
        // Set time values
        self.write_register(RTC_SECONDS, second);
        self.write_register(RTC_MINUTES, minute);
        self.write_register(RTC_HOURS, hour);
        self.write_register(RTC_DAY, day);
        self.write_register(RTC_MONTH, month);
        self.write_register(RTC_YEAR, year);
        
        // Re-enable updates
        self.write_register(RTC_STATUS_B, status_b);
        
        Ok(())
    }
    
    pub fn enable_interrupts(&mut self) -> Result<(), &'static str> {
        if !self.initialized {
            return Err("RTC not initialized");
        }
        
        // Read current status
        let mut status_b = self.read_register(RTC_STATUS_B);
        
        // Enable periodic interrupts (every second)
        status_b |= RTC_PERIODIC_INTERRUPT;
        
        // Set interrupt rate to 1024 Hz (fastest)
        let mut status_a = self.read_register(RTC_STATUS_A);
        status_a = (status_a & 0xF0) | 0x06; // 1024 Hz
        
        self.write_register(RTC_STATUS_A, status_a);
        self.write_register(RTC_STATUS_B, status_b);
        
        // Clear any pending interrupts
        self.read_register(RTC_STATUS_C);
        
        Ok(())
    }
    
    pub fn disable_interrupts(&mut self) {
        let mut status_b = self.read_register(RTC_STATUS_B);
        status_b &= !(RTC_PERIODIC_INTERRUPT | RTC_UPDATE_INTERRUPT);
        self.write_register(RTC_STATUS_B, status_b);
    }
    
    /// Called by interrupt handler
    pub fn handle_interrupt(&mut self) {
        // Read status C to clear interrupt
        let _status_c = self.read_register(RTC_STATUS_C);
        self.tick_count += 1;
    }
    
    pub fn get_tick_count(&self) -> u64 {
        self.tick_count
    }
    
    /// Get system uptime in seconds
    pub fn get_uptime(&self) -> u64 {
        if self.boot_time == 0 {
            return 0;
        }
        
        let current = self.read_datetime();
        current.to_unix_timestamp().saturating_sub(self.boot_time)
    }
}

impl Driver for RtcDriver {
    type Error = &'static str;
    
    fn name(&self) -> &'static str {
        "Real-Time Clock Driver"
    }
    
    fn init(&mut self) -> Result<(), Self::Error> {
        // Configure RTC for 24-hour binary mode
        let mut status_b = self.read_register(RTC_STATUS_B);
        status_b |= RTC_24_HOUR | RTC_BINARY;
        self.write_register(RTC_STATUS_B, status_b);
        
        // Record boot time
        let boot_datetime = self.read_datetime();
        self.boot_time = boot_datetime.to_unix_timestamp();
        self.last_update = self.boot_time;
        
        self.initialized = true;
        Ok(())
    }
    
    fn cleanup(&mut self) -> Result<(), Self::Error> {
        self.disable_interrupts();
        self.initialized = false;
        Ok(())
    }
    
    fn is_initialized(&self) -> bool {
        self.initialized
    }
}

impl TimerDriver for RtcDriver {
    fn get_timestamp(&self) -> u64 {
        // For performance, we could cache the last read time and estimate based on ticks
        // For now, always read from hardware
        let mut driver = RTC.lock();
        let datetime = driver.read_datetime();
        datetime.to_unix_timestamp()
    }
    
    fn set_alarm(&mut self, _timestamp: u64) -> Result<(), Self::Error> {
        // RTC alarm functionality would be implemented here
        // For now, just return an error
        Err("RTC alarms not implemented")
    }
}

// Public interface functions
pub fn init() -> Result<(), &'static str> {
    RTC.lock().init()
}

pub fn get_current_time() -> DateTime {
    RTC.lock().read_datetime()
}

pub fn set_current_time(datetime: &DateTime) -> Result<(), &'static str> {
    RTC.lock().set_datetime(datetime)
}

pub fn get_timestamp() -> u64 {
    let driver = RTC.lock();
    let datetime = driver.read_datetime();
    datetime.to_unix_timestamp()
}

pub fn get_uptime() -> u64 {
    RTC.lock().get_uptime()
}

pub fn enable_interrupts() -> Result<(), &'static str> {
    RTC.lock().enable_interrupts()
}

pub fn disable_interrupts() {
    RTC.lock().disable_interrupts()
}

/// Called by interrupt handler (IRQ8)
pub fn handle_interrupt() {
    RTC.lock().handle_interrupt();
}

pub fn get_tick_count() -> u64 {
    RTC.lock().get_tick_count()
}

/// Format current time as string
pub fn format_current_time() -> heapless::String<32> {
    get_current_time().format()
}

/// Get a simple timestamp for logging (milliseconds since boot)
pub fn get_simple_timestamp() -> u64 {
    // Use tick count as millisecond approximation
    // This assumes 1024 Hz interrupt rate, so divide by ~1 for ms
    get_tick_count()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_datetime_creation() {
        let dt = DateTime::new();
        assert_eq!(dt.year, 0);
        assert_eq!(dt.month, 0);
    }
    
    #[test]
    fn test_leap_year() {
        assert!(is_leap_year(2000));
        assert!(is_leap_year(2004));
        assert!(!is_leap_year(1900));
        assert!(!is_leap_year(2001));
    }
    
    #[test]
    fn test_days_in_month() {
        assert_eq!(days_in_month(1, 2023), 31);
        assert_eq!(days_in_month(2, 2023), 28);
        assert_eq!(days_in_month(2, 2024), 29);
        assert_eq!(days_in_month(4, 2023), 30);
    }
    
    #[test]
    fn test_bcd_conversion() {
        assert_eq!(RtcDriver::bcd_to_binary(0x23), 23);
        assert_eq!(RtcDriver::bcd_to_binary(0x59), 59);
        assert_eq!(RtcDriver::binary_to_bcd(23), 0x23);
        assert_eq!(RtcDriver::binary_to_bcd(59), 0x59);
    }
}
