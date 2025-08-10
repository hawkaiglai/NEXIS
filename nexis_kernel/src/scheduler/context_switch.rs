//! Context switching wrapper for assembly implementation
//! 
//! DEPENDENCIES:
//! - Uses external assembly function context_switch from context.S
//! - Links with assembly module built by build.rs
//! 
//! INTEGRATION POINTS:
//! - Called by scheduler for task switching
//! - Uses assembly routine for actual register switching
//! - Provides safe Rust interface to unsafe assembly
//! 
//! TESTING REQUIREMENTS:
//! - Test context switch preserves registers correctly
//! - Test stack pointer manipulation
//! - Test switching between different tasks
//! 
//! PERFORMANCE CONSIDERATIONS:
//! - Context switch must be as fast as possible
//! - Assembly implementation optimized for x86_64
//! - Minimal register save/restore overhead

/// External assembly function for context switching
/// 
/// This function is implemented in context.S and performs the actual
/// low-level context switch between tasks.
/// 
/// # Safety
/// This function is unsafe because it:
/// - Manipulates raw stack pointers
/// - Changes execution context
/// - Requires valid stack setups
/// 
/// # Arguments
/// * `old_rsp_ptr` - Pointer where to save the current stack pointer
/// * `new_rsp` - New stack pointer to switch to
extern "C" {
    fn context_switch(old_rsp_ptr: *mut usize, new_rsp: usize);
}

/// Safe wrapper for context switching between tasks
/// 
/// This function provides a safe interface to the assembly context_switch
/// routine while maintaining the performance characteristics.
/// 
/// # Safety
/// This function is unsafe because the caller must ensure:
/// - `old_rsp_ptr` points to valid memory that can be written to
/// - `new_rsp` points to a valid stack with proper register save frame
/// - The new stack has been properly initialized with task entry point
/// 
/// # Arguments
/// * `old_rsp_ptr` - Mutable reference to store current stack pointer
/// * `new_rsp` - Stack pointer of task to switch to
pub unsafe fn context_switch_safe(old_rsp_ptr: &mut usize, new_rsp: usize) {
    context_switch(old_rsp_ptr as *mut usize, new_rsp);
}

/// Perform context switch between two tasks
/// 
/// This is the main interface used by the scheduler to switch between tasks.
/// It handles the low-level details of saving and restoring processor state.
/// 
/// # Safety
/// This function is unsafe and should only be called by the scheduler.
/// The caller must ensure that both stack pointers are valid and properly
/// initialized.
/// 
/// # Arguments
/// * `current_rsp` - Pointer to current task's stack pointer storage
/// * `next_rsp` - Stack pointer of task to switch to
pub unsafe fn switch_tasks(current_rsp: &mut usize, next_rsp: usize) {
    // Call the assembly context switch routine
    context_switch(current_rsp as *mut usize, next_rsp);
}

/// Initialize a new task's stack for context switching
/// 
/// This function sets up the initial register save frame on a new task's
/// stack so that when context_switch is called for the first time, the
/// task will begin executing at the specified entry point.
/// 
/// # Arguments
/// * `stack_top` - Top of the allocated stack
/// * `entry_point` - Function where task execution should begin
/// 
/// # Returns
/// * Stack pointer ready for context switching
pub fn init_task_context(stack_top: usize, entry_point: extern "C" fn()) -> usize {
    let mut sp = stack_top;
    
    // Align to 16-byte boundary as required by System V ABI
    sp &= !0xF;
    
    unsafe {
        // Set up the stack frame that context_switch expects to restore
        
        // Push the entry point as the return address
        sp -= core::mem::size_of::<usize>();
        *(sp as *mut usize) = entry_point as usize;
        
        // Push saved callee-saved registers in the order context_switch expects
        // The assembly code restores in reverse order: r15, r14, r13, r12, rbx, rbp
        
        // rbp (frame pointer)
        sp -= core::mem::size_of::<usize>();
        *(sp as *mut usize) = 0;
        
        // rbx (base register)
        sp -= core::mem::size_of::<usize>();
        *(sp as *mut usize) = 0;
        
        // r12 (general purpose)
        sp -= core::mem::size_of::<usize>();
        *(sp as *mut usize) = 0;
        
        // r13 (general purpose)
        sp -= core::mem::size_of::<usize>();
        *(sp as *mut usize) = 0;
        
        // r14 (general purpose)
        sp -= core::mem::size_of::<usize>();
        *(sp as *mut usize) = 0;
        
        // r15 (general purpose)
        sp -= core::mem::size_of::<usize>();
        *(sp as *mut usize) = 0;
    }
    
    sp
}

/// Context switching statistics and debugging information
#[derive(Debug, Default)]
pub struct ContextSwitchStats {
    /// Total number of context switches performed
    pub total_switches: u64,
    /// Number of switches to new tasks
    pub new_task_switches: u64,
    /// Number of switches between existing tasks
    pub task_to_task_switches: u64,
}

static mut CONTEXT_STATS: ContextSwitchStats = ContextSwitchStats {
    total_switches: 0,
    new_task_switches: 0,
    task_to_task_switches: 0,
};

/// Record a context switch for statistics
pub fn record_context_switch(is_new_task: bool) {
    unsafe {
        CONTEXT_STATS.total_switches += 1;
        if is_new_task {
            CONTEXT_STATS.new_task_switches += 1;
        } else {
            CONTEXT_STATS.task_to_task_switches += 1;
        }
    }
}

/// Get context switching statistics
pub fn get_context_switch_stats() -> ContextSwitchStats {
    unsafe { CONTEXT_STATS.clone() }
}

/// Reset context switching statistics
pub fn reset_context_switch_stats() {
    unsafe {
        CONTEXT_STATS = ContextSwitchStats::default();
    }
}

/// Validate that a stack pointer appears to be properly set up
/// 
/// This function performs basic sanity checks on a stack pointer to help
/// debug context switching issues.
/// 
/// # Arguments
/// * `sp` - Stack pointer to validate
/// * `stack_base` - Base address of the stack
/// * `stack_size` - Size of the stack in bytes
/// 
/// # Returns
/// * `true` if the stack pointer appears valid
/// * `false` if there are obvious problems
pub fn validate_stack_pointer(sp: usize, stack_base: usize, stack_size: usize) -> bool {
    // Check that SP is within the stack bounds
    if sp < stack_base || sp >= stack_base + stack_size {
        return false;
    }
    
    // Check that SP is properly aligned
    if sp & 0xF != 0 {
        return false;
    }
    
    // Check that there's reasonable space on the stack
    let used_space = (stack_base + stack_size) - sp;
    if used_space > stack_size - 256 {  // Leave at least 256 bytes free
        return false;
    }
    
    true
}

/// Context switch debug information
#[derive(Debug)]
pub struct ContextSwitchDebug {
    pub from_task_sp: usize,
    pub to_task_sp: usize,
    pub switch_count: u64,
    pub timestamp: u64,
}

/// Enable context switch debugging
static mut DEBUG_ENABLED: bool = false;

/// Enable or disable context switch debugging
pub fn set_debug_enabled(enabled: bool) {
    unsafe {
        DEBUG_ENABLED = enabled;
    }
}

/// Check if context switch debugging is enabled
pub fn is_debug_enabled() -> bool {
    unsafe { DEBUG_ENABLED }
}

/// Log context switch for debugging
pub fn debug_context_switch(from_sp: usize, to_sp: usize) {
    if is_debug_enabled() {
        let debug_info = ContextSwitchDebug {
            from_task_sp: from_sp,
            to_task_sp: to_sp,
            switch_count: unsafe { CONTEXT_STATS.total_switches },
            timestamp: crate::pit::ticks(),
        };
        
        crate::vga::vprintln!(
            "Context switch #{}: 0x{:x} -> 0x{:x} at tick {}",
            debug_info.switch_count,
            debug_info.from_task_sp,
            debug_info.to_task_sp,
            debug_info.timestamp
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_stack_pointer_validation() {
        let stack_base = 0x1000;
        let stack_size = 4096;
        
        // Valid stack pointer
        assert!(validate_stack_pointer(0x1800, stack_base, stack_size));
        
        // Below stack base
        assert!(!validate_stack_pointer(0x800, stack_base, stack_size));
        
        // Above stack top
        assert!(!validate_stack_pointer(0x2100, stack_base, stack_size));
        
        // Misaligned
        assert!(!validate_stack_pointer(0x1801, stack_base, stack_size));
    }
    
    #[test]
    fn test_context_init() {
        extern "C" fn test_entry() {}
        
        let stack_top = 0x2000;
        let sp = init_task_context(stack_top, test_entry);
        
        // Stack pointer should be below top
        assert!(sp < stack_top);
        
        // Should be aligned
        assert_eq!(sp & 0xF, 0);
        
        // Should have space for saved registers
        assert!(sp >= stack_top - 8 * core::mem::size_of::<usize>());
    }
}
