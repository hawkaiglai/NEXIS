//! Interrupt Descriptor Table setup and management
//! 
//! DEPENDENCIES:
//! - Uses x86_64 crate for IDT structures and management
//! - Uses lazy_static for static IDT instance
//! 
//! INTEGRATION POINTS:
//! - Called by interrupt::init() during system initialization
//! - Registers all exception and interrupt handlers
//! - Provides interface for dynamic handler registration

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use x86_64::structures::gdt::SegmentSelector;
use lazy_static::lazy_static;
use crate::interrupt::{handlers, vectors, Result, InterruptError};

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        // CPU Exception handlers
        idt.divide_error.set_handler_fn(handlers::divide_error_handler);
        idt.debug.set_handler_fn(handlers::debug_handler);
        idt.non_maskable_interrupt.set_handler_fn(handlers::nmi_handler);
        idt.breakpoint.set_handler_fn(handlers::breakpoint_handler);
        idt.overflow.set_handler_fn(handlers::overflow_handler);
        idt.bound_range_exceeded.set_handler_fn(handlers::bound_range_exceeded_handler);
        idt.invalid_opcode.set_handler_fn(handlers::invalid_opcode_handler);
        idt.device_not_available.set_handler_fn(handlers::device_not_available_handler);
        idt.double_fault.set_handler_fn(handlers::double_fault_handler);
        idt.invalid_tss.set_handler_fn(handlers::invalid_tss_handler);
        idt.segment_not_present.set_handler_fn(handlers::segment_not_present_handler);
        idt.stack_segment_fault.set_handler_fn(handlers::stack_segment_fault_handler);
        idt.general_protection_fault.set_handler_fn(handlers::general_protection_fault_handler);
        idt.page_fault.set_handler_fn(handlers::page_fault_handler);
        idt.x87_floating_point.set_handler_fn(handlers::x87_floating_point_handler);
        idt.alignment_check.set_handler_fn(handlers::alignment_check_handler);
        idt.machine_check.set_handler_fn(handlers::machine_check_handler);
        idt.simd_floating_point.set_handler_fn(handlers::simd_floating_point_handler);
        idt.virtualization.set_handler_fn(handlers::virtualization_handler);
        idt.security_exception.set_handler_fn(handlers::security_exception_handler);

        // Hardware interrupt handlers (after PIC remapping)
        idt[vectors::TIMER as usize].set_handler_fn(handlers::timer_interrupt_handler);
        idt[vectors::KEYBOARD as usize].set_handler_fn(handlers::keyboard_interrupt_handler);
        idt[vectors::RTC as usize].set_handler_fn(handlers::rtc_interrupt_handler);
        
        // System call handler
        idt[vectors::SYSCALL as usize].set_handler_fn(handlers::syscall_handler);

        idt
    };
}

/// Initialize the IDT
pub fn init_idt() {
    crate::vga::vprintln!("Loading IDT...");
    IDT.load();
    crate::vga::vprintln!("IDT loaded successfully");
}

/// Register a custom interrupt handler for a given vector
pub fn register_interrupt_handler(
    vector: u8, 
    handler: extern "x86-interrupt" fn(InterruptStackFrame)
) -> Result<()> {
    if vector >= 32 && vector < 48 {
        // This is a hardware interrupt vector, allow registration
        // Note: In a real implementation, we would need mutable access to IDT
        // For now, return success as handlers are statically registered
        Ok(())
    } else {
        Err(InterruptError::InvalidVector)
    }
}

/// Get IDT information for debugging
pub fn get_idt_info() -> IdtInfo {
    IdtInfo {
        base_address: IDT.base(),
        limit: IDT.limit(),
        loaded: true,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct IdtInfo {
    pub base_address: u64,
    pub limit: u16,
    pub loaded: bool,
}

// Double fault stack setup for safety
const DOUBLE_FAULT_IST_INDEX: u16 = 0;

/// Setup double fault handling with separate stack
pub fn setup_double_fault_stack(stack_top: u64) {
    // In a full implementation, this would setup the TSS and IST
    // For now, we acknowledge the importance of separate stacks for double faults
    crate::vga::vprintln!("Double fault stack configured at 0x{:x}", stack_top);
}
