//! PIT (Programmable Interval Timer) for preemptive scheduling
//! 
//! DEPENDENCIES:
//! - Uses x86_64 crate for port I/O operations
//! - Uses core::sync::atomic for thread-safe tick counting
//! 
//! INTEGRATION POINTS:
//! - Called by interrupt::init() for timer initialization
//! - Timer interrupt handler calls scheduler for preemption
//! - Tick counting used by scheduler and system time

use x86_64::instructions::port::Port;
use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};

// PIT constants
const PIT_FREQUENCY: u32 = 1_193_182; // Base frequency in Hz
const PIT_CHANNEL0_PORT: u16 = 0x40;
const PIT_COMMAND_PORT: u16 = 0x43;

// PIT command byte: Channel 0, Access mode lobyte/hibyte, Operating mode 2 (rate generator), Binary mode
const PIT_COMMAND: u8 = 0b00110100;

// Global timer state
static TICK_COUNT: AtomicU64 = AtomicU64::new(0);
static TIMER_FREQUENCY: AtomicU64 = AtomicU64::new(0);
static NEED_RESCHEDULE: AtomicBool = AtomicBool::new(false);

// Time tracking
static SYSTEM_TIME_MS: AtomicU64 = AtomicU64::new(0);
static LAST_TICK_TIME: AtomicU64 = AtomicU64::new(0);

/// Initialize the PIT timer with specified frequency
pub fn init_timer(frequency_hz: u32) {
    if frequency_hz == 0 || frequency_hz > PIT_FREQUENCY {
        panic!("Invalid timer frequency: {}", frequency_hz);
    }

    let divisor = PIT_FREQUENCY / frequency_hz;
    if divisor > 65535 {
        panic!("Timer frequency too low: {}", frequency_hz);
    }

    unsafe {
        let mut command_port = Port::<u8>::new(PIT_COMMAND_PORT);
        let mut channel0_port = Port::<u8>::new(PIT_CHANNEL0_PORT);

        // Send command byte
        command_port.write(PIT_COMMAND);

        // Send divisor (low byte first, then high byte)
        channel0_port.write((divisor & 0xFF) as u8);
        channel0_port.write((divisor >> 8) as u8);
    }

    // Store frequency for time calculations
    TIMER_FREQUENCY.store(frequency_hz as u64, Ordering::SeqCst);
    
    crate::vga::vprintln!("Timer initialized at {} Hz (divisor: {})", frequency_hz, divisor);
}

/// Called by timer interrupt handler - increments tick count and sets reschedule flag
pub fn timer_tick() {
    let tick = TICK_COUNT.fetch_add(1, Ordering::SeqCst);
    let frequency = TIMER_FREQUENCY.load(Ordering::SeqCst);
    
    // Update system time in milliseconds
    if frequency > 0 {
        let time_ms = (tick * 1000) / frequency;
        SYSTEM_TIME_MS.store(time_ms, Ordering::SeqCst);
    }
    
    // Set reschedule flag for preemptive multitasking
    NEED_RESCHEDULE.store(true, Ordering::SeqCst);
    
    // Store last tick time for delta calculations
    LAST_TICK_TIME.store(tick, Ordering::SeqCst);
}

/// Get current tick count
pub fn get_tick_count() -> u64 {
    TICK_COUNT.load(Ordering::SeqCst)
}

/// Get system uptime in milliseconds
pub fn get_uptime_ms() -> u64 {
    SYSTEM_TIME_MS.load(Ordering::SeqCst)
}

/// Get system uptime in seconds
pub fn get_uptime_seconds() -> u64 {
    get_uptime_ms() / 1000
}

/// Check if scheduler should run (and clear the flag)
pub fn should_reschedule() -> bool {
    NEED_RESCHEDULE.swap(false, Ordering::SeqCst)
}

/// Force a reschedule on next timer interrupt
pub fn request_reschedule() {
    NEED_RESCHEDULE.store(true, Ordering::SeqCst);
}

/// Get timer frequency in Hz
pub fn get_timer_frequency() -> u64 {
    TIMER_FREQUENCY.load(Ordering::SeqCst)
}

/// Sleep for approximately the given number of ticks
pub fn sleep_ticks(ticks: u64) {
    let start_tick = get_tick_count();
    let target_tick = start_tick + ticks;
    
    while get_tick_count() < target_tick {
        x86_64::instructions::hlt();
    }
}

/// Sleep for approximately the given number of milliseconds
pub fn sleep_ms(ms: u64) {
    let frequency = get_timer_frequency();
    if frequency > 0 {
        let ticks = (ms * frequency) / 1000;
        sleep_ticks(ticks);
    }
}

/// Get time elapsed since last timer configuration
pub fn get_elapsed_ticks() -> u64 {
    get_tick_count()
}

/// Reset timer statistics (useful for benchmarking)
pub fn reset_timer_stats() {
    TICK_COUNT.store(0, Ordering::SeqCst);
    SYSTEM_TIME_MS.store(0, Ordering::SeqCst);
    LAST_TICK_TIME.store(0, Ordering::SeqCst);
    NEED_RESCHEDULE.store(false, Ordering::SeqCst);
}

/// Timer diagnostics for debugging
#[derive(Debug, Clone, Copy)]
pub struct TimerStats {
    pub tick_count: u64,
    pub uptime_ms: u64,
    pub frequency_hz: u64,
    pub reschedule_pending: bool,
}

pub fn get_timer_stats() -> TimerStats {
    TimerStats {
        tick_count: get_tick_count(),
        uptime_ms: get_uptime_ms(),
        frequency_hz: get_timer_frequency(),
        reschedule_pending: NEED_RESCHEDULE.load(Ordering::SeqCst),
    }
}

/// Time-based utilities
pub mod time {
    use super::*;
    
    /// Convert ticks to milliseconds
    pub fn ticks_to_ms(ticks: u64) -> u64 {
        let frequency = get_timer_frequency();
        if frequency > 0 {
            (ticks * 1000) / frequency
        } else {
            0
        }
    }
    
    /// Convert milliseconds to ticks
    pub fn ms_to_ticks(ms: u64) -> u64 {
        let frequency = get_timer_frequency();
        if frequency > 0 {
            (ms * frequency) / 1000
        } else {
            0
        }
    }
    
    /// Get a timestamp in milliseconds since boot
    pub fn timestamp_ms() -> u64 {
        get_uptime_ms()
    }
}
