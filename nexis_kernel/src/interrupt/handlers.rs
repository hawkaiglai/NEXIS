//! Interrupt service routines for all system interrupts
//! 
//! DEPENDENCIES:
//! - Uses x86_64 crate for interrupt stack frame handling
//! - Integrates with scheduler for preemptive multitasking
//! - Uses driver modules for hardware-specific handling
//! 
//! INTEGRATION POINTS:
//! - Called automatically by CPU when interrupts occur
//! - Timer handler integrates with scheduler
//! - Keyboard handler integrates with keyboard driver
//! - Exception handlers provide debugging information

use x86_64::structures::idt::{InterruptStackFrame, PageFaultErrorCode};
use crate::interrupt::{pic, timer, vectors};
use crate::vga::vprintln;

// ===== CPU EXCEPTION HANDLERS =====

pub extern "x86-interrupt" fn divide_error_handler(stack_frame: InterruptStackFrame) {
    vprintln!("\n*** EXCEPTION: DIVIDE BY ZERO ***");
    vprintln!("Stack frame: {:#?}", stack_frame);
    panic!("Divide by zero exception");
}

pub extern "x86-interrupt" fn debug_handler(stack_frame: InterruptStackFrame) {
    vprintln!("\n*** EXCEPTION: DEBUG ***");
    vprintln!("Stack frame: {:#?}", stack_frame);
}

pub extern "x86-interrupt" fn nmi_handler(stack_frame: InterruptStackFrame) {
    vprintln!("\n*** NON-MASKABLE INTERRUPT ***");
    vprintln!("Stack frame: {:#?}", stack_frame);
    panic!("Non-maskable interrupt received");
}

pub extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    vprintln!("\n*** BREAKPOINT ***");
    vprintln!("Stack frame: {:#?}", stack_frame);
}

pub extern "x86-interrupt" fn overflow_handler(stack_frame: InterruptStackFrame) {
    vprintln!("\n*** EXCEPTION: OVERFLOW ***");
    vprintln!("Stack frame: {:#?}", stack_frame);
    panic!("Overflow exception");
}

pub extern "x86-interrupt" fn bound_range_exceeded_handler(stack_frame: InterruptStackFrame) {
    vprintln!("\n*** EXCEPTION: BOUND RANGE EXCEEDED ***");
    vprintln!("Stack frame: {:#?}", stack_frame);
    panic!("Bound range exceeded exception");
}

pub extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    vprintln!("\n*** EXCEPTION: INVALID OPCODE ***");
    vprintln!("Stack frame: {:#?}", stack_frame);
    vprintln!("Instruction pointer: 0x{:x}", stack_frame.instruction_pointer.as_u64());
    panic!("Invalid opcode exception");
}

pub extern "x86-interrupt" fn device_not_available_handler(stack_frame: InterruptStackFrame) {
    vprintln!("\n*** EXCEPTION: DEVICE NOT AVAILABLE ***");
    vprintln!("Stack frame: {:#?}", stack_frame);
    panic!("Device not available exception");
}

pub extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    vprintln!("\n*** FATAL: DOUBLE FAULT ***");
    vprintln!("Error code: 0x{:x}", error_code);
    vprintln!("Stack frame: {:#?}", stack_frame);
    panic!("Double fault - system halted");
}

pub extern "x86-interrupt" fn invalid_tss_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    vprintln!("\n*** EXCEPTION: INVALID TSS ***");
    vprintln!("Error code: 0x{:x}", error_code);
    vprintln!("Stack frame: {:#?}", stack_frame);
    panic!("Invalid TSS exception");
}

pub extern "x86-interrupt" fn segment_not_present_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    vprintln!("\n*** EXCEPTION: SEGMENT NOT PRESENT ***");
    vprintln!("Error code: 0x{:x}", error_code);
    vprintln!("Stack frame: {:#?}", stack_frame);
    panic!("Segment not present exception");
}

pub extern "x86-interrupt" fn stack_segment_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    vprintln!("\n*** EXCEPTION: STACK SEGMENT FAULT ***");
    vprintln!("Error code: 0x{:x}", error_code);
    vprintln!("Stack frame: {:#?}", stack_frame);
    panic!("Stack segment fault exception");
}

pub extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    vprintln!("\n*** EXCEPTION: GENERAL PROTECTION FAULT ***");
    vprintln!("Error code: 0x{:x}", error_code);
    vprintln!("Stack frame: {:#?}", stack_frame);
    
    // Decode error code for more information
    if error_code != 0 {
        let external = (error_code & 1) != 0;
        let table = (error_code >> 1) & 3;
        let index = error_code >> 3;
        
        vprintln!("Error details:");
        vprintln!("  External: {}", external);
        vprintln!("  Table: {} ({})", table, match table {
            0 => "GDT",
            1 => "IDT", 
            2 => "LDT",
            3 => "IDT",
            _ => "Unknown",
        });
        vprintln!("  Index: 0x{:x}", index);
    }
    
    panic!("General protection fault");
}

pub extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    vprintln!("\n*** EXCEPTION: PAGE FAULT ***");
    vprintln!("Accessed Address: 0x{:x}", Cr2::read().as_u64());
    vprintln!("Error Code: {:?}", error_code);
    vprintln!("Stack frame: {:#?}", stack_frame);
    
    // Decode error code
    vprintln!("Error details:");
    vprintln!("  Caused by: {}", if error_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION) {
        "protection violation"
    } else {
        "page not present"
    });
    vprintln!("  Access type: {}", if error_code.contains(PageFaultErrorCode::CAUSED_BY_WRITE) {
        "write"
    } else {
        "read"
    });
    vprintln!("  Origin: {}", if error_code.contains(PageFaultErrorCode::USER_MODE) {
        "user mode"
    } else {
        "kernel mode"
    });
    vprintln!("  Instruction fetch: {}", if error_code.contains(PageFaultErrorCode::INSTRUCTION_FETCH) {
        "yes"
    } else {
        "no"
    });
    
    panic!("Page fault exception");
}

pub extern "x86-interrupt" fn x87_floating_point_handler(stack_frame: InterruptStackFrame) {
    vprintln!("\n*** EXCEPTION: x87 FLOATING POINT ***");
    vprintln!("Stack frame: {:#?}", stack_frame);
    panic!("x87 floating point exception");
}

pub extern "x86-interrupt" fn alignment_check_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    vprintln!("\n*** EXCEPTION: ALIGNMENT CHECK ***");
    vprintln!("Error code: 0x{:x}", error_code);
    vprintln!("Stack frame: {:#?}", stack_frame);
    panic!("Alignment check exception");
}

pub extern "x86-interrupt" fn machine_check_handler(stack_frame: InterruptStackFrame) -> ! {
    vprintln!("\n*** FATAL: MACHINE CHECK ***");
    vprintln!("Stack frame: {:#?}", stack_frame);
    panic!("Machine check exception - hardware error");
}

pub extern "x86-interrupt" fn simd_floating_point_handler(stack_frame: InterruptStackFrame) {
    vprintln!("\n*** EXCEPTION: SIMD FLOATING POINT ***");
    vprintln!("Stack frame: {:#?}", stack_frame);
    panic!("SIMD floating point exception");
}

pub extern "x86-interrupt" fn virtualization_handler(stack_frame: InterruptStackFrame) {
    vprintln!("\n*** EXCEPTION: VIRTUALIZATION ***");
    vprintln!("Stack frame: {:#?}", stack_frame);
    panic!("Virtualization exception");
}

pub extern "x86-interrupt" fn security_exception_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    vprintln!("\n*** EXCEPTION: SECURITY ***");
    vprintln!("Error code: 0x{:x}", error_code);
    vprintln!("Stack frame: {:#?}", stack_frame);
    panic!("Security exception");
}

// ===== HARDWARE INTERRUPT HANDLERS =====

pub extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Update timer tick count and system time
    timer::timer_tick();
    
    // Check if we should reschedule tasks
    if timer::should_reschedule() {
        // Set flag for scheduler to check at safe points
        crate::pit::NEED_RESCHED.store(true, core::sync::atomic::Ordering::SeqCst);
    }
    
    // Send EOI to PIC
    pic::send_eoi(0); // Timer is IRQ0
}

pub extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;
    
    // Read scancode from keyboard controller
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    
    // Push scancode to keyboard driver queue
    crate::kb::Kb::push_scancode(scancode);
    
    // Send EOI to PIC
    pic::send_eoi(1); // Keyboard is IRQ1
}

pub extern "x86-interrupt" fn rtc_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Handle Real-Time Clock interrupt
    // For now, just acknowledge the interrupt
    
    // Read RTC register C to clear interrupt flag
    use x86_64::instructions::port::Port;
    unsafe {
        let mut cmos_select = Port::new(0x70);
        let mut cmos_data = Port::new(0x71);
        
        cmos_select.write(0x0C); // Select register C
        let _status = cmos_data.read(); // Read to clear interrupt
    }
    
    // Send EOI to PIC
    pic::send_eoi(8); // RTC is IRQ8 (becomes IRQ8 after remapping)
}

pub extern "x86-interrupt" fn syscall_handler(stack_frame: InterruptStackFrame) {
    // System call handler - to be implemented later
    vprintln!("System call received");
    vprintln!("Stack frame: {:#?}", stack_frame);
    
    // For now, just return
    // In a full implementation, this would:
    // 1. Extract system call number from registers
    // 2. Extract arguments from registers
    // 3. Dispatch to appropriate system call handler
    // 4. Return result in registers
}

// ===== HELPER FUNCTIONS =====

/// Common exception handler for debugging
fn log_exception(name: &str, stack_frame: &InterruptStackFrame, error_code: Option<u64>) {
    vprintln!("\n*** EXCEPTION: {} ***", name);
    if let Some(code) = error_code {
        vprintln!("Error code: 0x{:x}", code);
    }
    vprintln!("Instruction pointer: 0x{:x}", stack_frame.instruction_pointer.as_u64());
    vprintln!("Stack pointer: 0x{:x}", stack_frame.stack_pointer.as_u64());
    vprintln!("CPU flags: 0x{:x}", stack_frame.cpu_flags);
}

/// Get interrupt statistics for debugging
#[derive(Debug, Clone, Copy)]
pub struct InterruptStats {
    pub timer_interrupts: u64,
    pub keyboard_interrupts: u64,
    pub page_faults: u64,
    pub general_protection_faults: u64,
}

// Global interrupt counters for statistics
use core::sync::atomic::{AtomicU64, Ordering};

static TIMER_INTERRUPT_COUNT: AtomicU64 = AtomicU64::new(0);
static KEYBOARD_INTERRUPT_COUNT: AtomicU64 = AtomicU64::new(0);
static PAGE_FAULT_COUNT: AtomicU64 = AtomicU64::new(0);
static GPF_COUNT: AtomicU64 = AtomicU64::new(0);

pub fn get_interrupt_stats() -> InterruptStats {
    InterruptStats {
        timer_interrupts: TIMER_INTERRUPT_COUNT.load(Ordering::SeqCst),
        keyboard_interrupts: KEYBOARD_INTERRUPT_COUNT.load(Ordering::SeqCst),
        page_faults: PAGE_FAULT_COUNT.load(Ordering::SeqCst),
        general_protection_faults: GPF_COUNT.load(Ordering::SeqCst),
    }
}

pub fn reset_interrupt_stats() {
    TIMER_INTERRUPT_COUNT.store(0, Ordering::SeqCst);
    KEYBOARD_INTERRUPT_COUNT.store(0, Ordering::SeqCst);
    PAGE_FAULT_COUNT.store(0, Ordering::SeqCst);
    GPF_COUNT.store(0, Ordering::SeqCst);
}
