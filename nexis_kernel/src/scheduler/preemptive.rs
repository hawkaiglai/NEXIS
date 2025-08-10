//! Preemptive scheduling implementation using PIT timer
//! 
//! DEPENDENCIES:
//! - Uses pit module for timer management
//! - Uses scheduler module for task switching
//! - Uses interrupt system for timer callbacks
//! 
//! INTEGRATION POINTS:
//! - Called by PIT timer interrupt handler
//! - Integrates with main scheduler for task switching
//! - Used by tasks to check for preemption requests
//! 
//! TESTING REQUIREMENTS:
//! - Test preemption timing accuracy
//! - Test that preemption doesn't break critical sections
//! - Test time slice management
//! - Test priority-based preemption

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use crate::pit;

/// Number of timer ticks per time slice (for round-robin scheduling)
const TIME_SLICE_TICKS: u64 = 10; // ~200ms at 50Hz

/// Global preemption control
static PREEMPTION_ENABLED: AtomicBool = AtomicBool::new(true);

/// Current time slice counter
static TIME_SLICE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Flag indicating a preemption is pending
static PREEMPTION_PENDING: AtomicBool = AtomicBool::new(false);

/// Total number of preemptions that have occurred
static PREEMPTION_COUNT: AtomicU64 = AtomicU64::new(0);

/// Initialize the preemptive scheduler
pub fn init() {
    reset_time_slice();
    enable_preemption();
    crate::vga::vprintln!("Preemptive scheduler initialized (time slice: {} ticks)", TIME_SLICE_TICKS);
}

/// Enable preemptive scheduling
pub fn enable_preemption() {
    PREEMPTION_ENABLED.store(true, Ordering::SeqCst);
}

/// Disable preemptive scheduling (for critical sections)
pub fn disable_preemption() {
    PREEMPTION_ENABLED.store(false, Ordering::SeqCst);
}

/// Check if preemption is currently enabled
pub fn is_preemption_enabled() -> bool {
    PREEMPTION_ENABLED.load(Ordering::SeqCst)
}

/// Reset the time slice counter for the current task
pub fn reset_time_slice() {
    TIME_SLICE_COUNTER.store(0, Ordering::SeqCst);
}

/// Handle timer tick - called by PIT interrupt handler
pub fn handle_timer_tick() {
    // Increment global tick counter
    let tick_count = pit::tick();
    
    // Only handle preemption if it's enabled
    if !is_preemption_enabled() {
        return;
    }
    
    // Increment time slice counter
    let slice_ticks = TIME_SLICE_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
    
    // Check if time slice has expired
    if slice_ticks >= TIME_SLICE_TICKS {
        request_preemption();
    }
}

/// Request a preemption (set the pending flag)
pub fn request_preemption() {
    PREEMPTION_PENDING.store(true, Ordering::SeqCst);
    // Also set the global NEED_RESCHED flag for compatibility
    crate::pit::NEED_RESCHED.store(true, Ordering::SeqCst);
}

/// Check if preemption is pending
pub fn is_preemption_pending() -> bool {
    PREEMPTION_PENDING.load(Ordering::SeqCst)
}

/// Clear the preemption pending flag
pub fn clear_preemption_pending() {
    PREEMPTION_PENDING.store(false, Ordering::SeqCst);
    crate::pit::NEED_RESCHED.store(false, Ordering::SeqCst);
}

/// Check for pending preemption and handle it
/// This is called from safe points in the code
pub fn check_preemption() {
    // Check both our flag and the legacy PIT flag
    let our_flag = PREEMPTION_PENDING.swap(false, Ordering::SeqCst);
    let pit_flag = crate::pit::NEED_RESCHED.swap(false, Ordering::SeqCst);
    
    if our_flag || pit_flag {
        handle_preemption();
    }
}

/// Handle a preemption request
fn handle_preemption() {
    // Only preempt if preemption is enabled
    if !is_preemption_enabled() {
        // Re-set the flag to try again later
        request_preemption();
        return;
    }
    
    // Increment preemption counter
    PREEMPTION_COUNT.fetch_add(1, Ordering::SeqCst);
    
    // Reset time slice for next task
    reset_time_slice();
    
    // Trigger a task switch by yielding
    super::task_yield();
}

/// Enter a critical section (disable preemption)
/// Returns the previous preemption state
pub fn enter_critical_section() -> bool {
    let was_enabled = is_preemption_enabled();
    disable_preemption();
    was_enabled
}

/// Exit a critical section (restore preemption state)
pub fn exit_critical_section(was_enabled: bool) {
    if was_enabled {
        enable_preemption();
        // Check for pending preemption after re-enabling
        check_preemption();
    }
}

/// Get current time slice progress (0.0 to 1.0)
pub fn get_time_slice_progress() -> f32 {
    let current = TIME_SLICE_COUNTER.load(Ordering::SeqCst);
    (current as f32) / (TIME_SLICE_TICKS as f32)
}

/// Get the number of ticks remaining in current time slice
pub fn get_remaining_ticks() -> u64 {
    let current = TIME_SLICE_COUNTER.load(Ordering::SeqCst);
    if current >= TIME_SLICE_TICKS {
        0
    } else {
        TIME_SLICE_TICKS - current
    }
}

/// Get preemption statistics
pub fn get_preemption_stats() -> PreemptionStats {
    PreemptionStats {
        total_preemptions: PREEMPTION_COUNT.load(Ordering::SeqCst),
        time_slice_ticks: TIME_SLICE_TICKS,
        current_slice_ticks: TIME_SLICE_COUNTER.load(Ordering::SeqCst),
        preemption_enabled: is_preemption_enabled(),
        preemption_pending: is_preemption_pending(),
    }
}

/// Preemption statistics structure
#[derive(Debug, Clone)]
pub struct PreemptionStats {
    pub total_preemptions: u64,
    pub time_slice_ticks: u64,
    pub current_slice_ticks: u64,
    pub preemption_enabled: bool,
    pub preemption_pending: bool,
}

/// RAII guard for critical sections
pub struct CriticalSection {
    was_enabled: bool,
}

impl CriticalSection {
    /// Enter a critical section
    pub fn enter() -> Self {
        Self {
            was_enabled: enter_critical_section(),
        }
    }
}

impl Drop for CriticalSection {
    fn drop(&mut self) {
        exit_critical_section(self.was_enabled);
    }
}

/// Macro for executing code in a critical section
#[macro_export]
macro_rules! critical_section {
    ($code:block) => {
        {
            let _guard = $crate::scheduler::preemptive::CriticalSection::enter();
            $code
        }
    };
}

/// Yield with preemption check
/// This is a convenience function that checks for preemption before yielding
pub fn yield_with_preemption_check() {
    check_preemption();
    super::task_yield();
}

/// Sleep for approximately the specified number of timer ticks
/// This is a cooperative sleep that yields the CPU
pub fn sleep_ticks(ticks: u64) {
    let start_time = pit::ticks();
    
    while pit::ticks() - start_time < ticks {
        check_preemption();
        super::task_yield();
    }
}

/// Priority levels for tasks (future enhancement)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    High = 0,
    Normal = 1,
    Low = 2,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

/// Calculate time slice based on priority
pub fn time_slice_for_priority(priority: Priority) -> u64 {
    match priority {
        Priority::High => TIME_SLICE_TICKS / 2,     // Shorter slices for high priority
        Priority::Normal => TIME_SLICE_TICKS,       // Standard time slice
        Priority::Low => TIME_SLICE_TICKS * 2,      // Longer slices for low priority
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_preemption_enable_disable() {
        // Test initial state
        assert!(is_preemption_enabled());
        
        // Test disable
        disable_preemption();
        assert!(!is_preemption_enabled());
        
        // Test enable
        enable_preemption();
        assert!(is_preemption_enabled());
    }
    
    #[test]
    fn test_critical_section() {
        enable_preemption();
        
        let was_enabled = enter_critical_section();
        assert!(was_enabled);
        assert!(!is_preemption_enabled());
        
        exit_critical_section(was_enabled);
        assert!(is_preemption_enabled());
    }
    
    #[test]
    fn test_time_slice_calculation() {
        assert_eq!(time_slice_for_priority(Priority::High), TIME_SLICE_TICKS / 2);
        assert_eq!(time_slice_for_priority(Priority::Normal), TIME_SLICE_TICKS);
        assert_eq!(time_slice_for_priority(Priority::Low), TIME_SLICE_TICKS * 2);
    }
}
