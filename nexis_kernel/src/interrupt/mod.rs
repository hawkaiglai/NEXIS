//! Interrupt management subsystem
//! 
//! DEPENDENCIES:
//! - Uses x86_64 crate for low-level interrupt handling
//! - Uses lazy_static for static interrupt structures
//! - Uses spin for interrupt-safe locking
//! 
//! INTEGRATION POINTS:
//! - Called by main.rs during kernel initialization
//! - Used by drivers for hardware interrupt registration
//! - Integrates with scheduler for preemptive multitasking
//! 
//! TESTING REQUIREMENTS:
//! - IDT loading and handler registration
//! - PIC remapping and interrupt routing
//! - Timer interrupt frequency and scheduling
//! 
//! PERFORMANCE CONSIDERATIONS:
//! - Minimal overhead in interrupt handlers
//! - Fast context switching for timer interrupts
//! - Interrupt-safe data structures

#![no_std]

pub mod idt;
pub mod pic;
pub mod timer;
pub mod handlers;

// Re-export main functionality
pub use idt::{init_idt, register_interrupt_handler};
pub use pic::{remap_pic, enable_irq, disable_irq, send_eoi};
pub use timer::{init_timer, get_tick_count};

use x86_64::instructions::interrupts;

/// Initialize the complete interrupt subsystem
pub fn init() {
    // 1. Initialize IDT with handlers
    idt::init_idt();
    
    // 2. Remap PIC to avoid conflicts with CPU exceptions
    pic::remap_pic();
    
    // 3. Initialize timer for preemptive scheduling
    timer::init_timer(50); // 50 Hz for good responsiveness
    
    // 4. Enable keyboard and timer interrupts
    pic::enable_irq(0); // Timer (IRQ0)
    pic::enable_irq(1); // Keyboard (IRQ1)
}

/// Enable interrupts globally
pub fn enable_interrupts() {
    interrupts::enable();
}

/// Disable interrupts globally
pub fn disable_interrupts() {
    interrupts::disable();
}

/// Execute closure with interrupts disabled
pub fn without_interrupts<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    interrupts::without_interrupts(f)
}

// Standard interrupt vectors following SSOT
pub mod vectors {
    pub const DIVIDE_ERROR: u8 = 0;
    pub const DEBUG: u8 = 1;
    pub const NMI: u8 = 2;
    pub const BREAKPOINT: u8 = 3;
    pub const OVERFLOW: u8 = 4;
    pub const BOUND_RANGE_EXCEEDED: u8 = 5;
    pub const INVALID_OPCODE: u8 = 6;
    pub const DEVICE_NOT_AVAILABLE: u8 = 7;
    pub const DOUBLE_FAULT: u8 = 8;
    pub const INVALID_TSS: u8 = 10;
    pub const SEGMENT_NOT_PRESENT: u8 = 11;
    pub const STACK_SEGMENT_FAULT: u8 = 12;
    pub const GENERAL_PROTECTION_FAULT: u8 = 13;
    pub const PAGE_FAULT: u8 = 14;
    pub const X87_FLOATING_POINT: u8 = 16;
    pub const ALIGNMENT_CHECK: u8 = 17;
    pub const MACHINE_CHECK: u8 = 18;
    pub const SIMD_FLOATING_POINT: u8 = 19;
    pub const VIRTUALIZATION: u8 = 20;
    pub const SECURITY_EXCEPTION: u8 = 30;
    
    // Hardware interrupts (after PIC remapping)
    pub const TIMER: u8 = 32;
    pub const KEYBOARD: u8 = 33;
    pub const CASCADE: u8 = 34;
    pub const COM2: u8 = 35;
    pub const COM1: u8 = 36;
    pub const LPT2: u8 = 37;
    pub const FLOPPY: u8 = 38;
    pub const LPT1: u8 = 39;
    pub const RTC: u8 = 40;
    pub const FREE1: u8 = 41;
    pub const FREE2: u8 = 42;
    pub const FREE3: u8 = 43;
    pub const PS2_MOUSE: u8 = 44;
    pub const FPU: u8 = 45;
    pub const PRIMARY_ATA: u8 = 46;
    pub const SECONDARY_ATA: u8 = 47;
    
    // System call vector
    pub const SYSCALL: u8 = 0x80;
}

// Error types for interrupt management
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptError {
    InvalidVector,
    HandlerAlreadyRegistered,
    PicInitializationFailed,
    TimerInitializationFailed,
}

pub type Result<T> = core::result::Result<T, InterruptError>;
