//! Task management and task table implementation
//! 
//! DEPENDENCIES:
//! - Uses memory::PhysicalMemoryManager for stack allocation
//! - Uses core types for no_std compatibility
//! 
//! INTEGRATION POINTS:
//! - Used by scheduler for task management
//! - Integrates with memory manager for stack allocation
//! - Called during task creation and destruction
//! 
//! TESTING REQUIREMENTS:
//! - Test task creation and initialization
//! - Test stack allocation and deallocation
//! - Test task state transitions
//! - Test task table management

use crate::memory::{PhysicalMemoryManager, FRAME_SIZE};
use spin::Mutex;
use lazy_static::lazy_static;

/// Maximum number of tasks that can exist simultaneously
const MAX_TASKS: usize = 64;

/// Task identifier
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TaskId(pub u64);

/// Task execution states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Task is ready to be scheduled
    Ready,
    /// Task is currently executing
    Running,
    /// Task is blocked waiting for an event
    Blocked,
    /// Task has finished execution
    Terminated,
}

/// Task control block
#[derive(Debug, Clone)]
pub struct Task {
    /// Unique task identifier
    pub tid: TaskId,
    /// Current stack pointer (saved during context switches)
    pub rsp: usize,
    /// Base address of the task's stack
    pub stack_base: usize,
    /// Size of the task's stack in bytes
    pub stack_size: usize,
    /// Current execution state
    pub state: TaskState,
}

impl Default for Task {
    fn default() -> Self {
        Self {
            tid: TaskId(0),
            rsp: 0,
            stack_base: 0,
            stack_size: 0,
            state: TaskState::Terminated,
        }
    }
}

/// Global task table containing all tasks
pub struct TaskTable {
    pub tasks: [Task; MAX_TASKS],
}

impl TaskTable {
    pub const MAX_TASKS: usize = MAX_TASKS;
    
    /// Create a new empty task table
    pub const fn new() -> Self {
        const EMPTY_TASK: Task = Task {
            tid: TaskId(0),
            rsp: 0,
            stack_base: 0,
            stack_size: 0,
            state: TaskState::Terminated,
        };
        
        Self {
            tasks: [EMPTY_TASK; MAX_TASKS],
        }
    }
    
    /// Find the first free slot in the task table
    pub fn find_free_slot(&self) -> Option<usize> {
        for i in 0..MAX_TASKS {
            if self.tasks[i].state == TaskState::Terminated {
                return Some(i);
            }
        }
        None
    }
    
    /// Get a task by its ID
    pub fn find_task_by_id(&self, tid: TaskId) -> Option<usize> {
        for i in 0..MAX_TASKS {
            if self.tasks[i].tid == tid && self.tasks[i].state != TaskState::Terminated {
                return Some(i);
            }
        }
        None
    }
    
    /// Count tasks in a specific state
    pub fn count_tasks_in_state(&self, state: TaskState) -> usize {
        self.tasks.iter()
            .filter(|task| task.state == state)
            .count()
    }
}

lazy_static! {
    /// Global task table protected by a spinlock
    pub static ref TASK_TABLE: Mutex<TaskTable> = Mutex::new(TaskTable::new());
}

/// Allocate a stack for a task
/// 
/// # Arguments
/// * `pmm` - Physical memory manager
/// * `pages` - Number of pages to allocate for the stack
/// 
/// # Returns
/// * `Some((base, size))` - Stack base address and size if successful
/// * `None` - If allocation failed
pub fn alloc_stack(pmm: &PhysicalMemoryManager, pages: usize) -> Option<(usize, usize)> {
    if pages == 0 {
        return None;
    }
    
    let mut allocated_frames = Vec::new();
    let stack_size = pages * FRAME_SIZE;
    
    // Allocate the required number of contiguous pages
    for _ in 0..pages {
        if let Some(frame) = pmm.alloc_frame() {
            allocated_frames.push(frame.start_address());
        } else {
            // Allocation failed - free what we've allocated so far
            for addr in allocated_frames {
                pmm.free_frame(addr);
            }
            return None;
        }
    }
    
    // For simplicity, we assume the frames are contiguous
    // In a real implementation, you might want to set up virtual memory mapping
    let stack_base = allocated_frames[0];
    
    // Zero the stack memory for security
    unsafe {
        core::ptr::write_bytes(stack_base as *mut u8, 0, stack_size);
    }
    
    Some((stack_base, stack_size))
}

/// Free a task's stack
/// 
/// # Arguments
/// * `stack_base` - Base address of the stack
/// * `stack_size` - Size of the stack in bytes
pub fn free_stack(stack_base: usize, stack_size: usize) {
    // In a real implementation, you would free the physical frames
    // For now, we'll just mark the memory as available
    // This requires access to the PMM, which should be passed in
    
    // Clear the stack memory for security
    unsafe {
        core::ptr::write_bytes(stack_base as *mut u8, 0, stack_size);
    }
    
    // Note: In the actual implementation, you'd need to free the frames
    // back to the physical memory manager
}

/// Prepare a stack for a new task
/// 
/// This function sets up the initial stack frame for a task so that when
/// context_switch is called, the task will begin executing at the specified
/// entry point.
/// 
/// # Arguments
/// * `entry` - Entry point function for the task
/// * `stack_base` - Base address of the allocated stack
/// * `stack_size` - Size of the stack in bytes
/// 
/// # Returns
/// * Initial stack pointer value for the task
pub fn prepare_stack(entry: extern "C" fn(), stack_base: usize, stack_size: usize) -> usize {
    let mut sp = stack_base + stack_size;
    
    // Align stack pointer to 16-byte boundary (required by System V ABI)
    sp &= !0xF;
    
    unsafe {
        // Push entry point as return address
        sp -= core::mem::size_of::<usize>();
        (sp as *mut usize).write_volatile(entry as usize);
        
        // Push saved registers in the order they will be popped by context_switch
        // These match the order in context.S: rbp, rbx, r12, r13, r14, r15
        
        // Push rbp placeholder
        sp -= core::mem::size_of::<usize>();
        (sp as *mut usize).write_volatile(0);
        
        // Push rbx placeholder
        sp -= core::mem::size_of::<usize>();
        (sp as *mut usize).write_volatile(0);
        
        // Push r12 placeholder
        sp -= core::mem::size_of::<usize>();
        (sp as *mut usize).write_volatile(0);
        
        // Push r13 placeholder
        sp -= core::mem::size_of::<usize>();
        (sp as *mut usize).write_volatile(0);
        
        // Push r14 placeholder
        sp -= core::mem::size_of::<usize>();
        (sp as *mut usize).write_volatile(0);
        
        // Push r15 placeholder
        sp -= core::mem::size_of::<usize>();
        (sp as *mut usize).write_volatile(0);
    }
    
    sp
}

/// Create a kernel task with the specified entry point
/// 
/// # Arguments
/// * `entry` - Function to execute
/// * `stack_pages` - Number of pages for the stack
/// * `pmm` - Physical memory manager
/// 
/// # Returns
/// * `Some(task)` - Created task if successful
/// * `None` - If creation failed
pub fn create_kernel_task(
    entry: extern "C" fn(),
    stack_pages: usize,
    pmm: &PhysicalMemoryManager,
) -> Option<Task> {
    if let Some((stack_base, stack_size)) = alloc_stack(pmm, stack_pages) {
        let rsp = prepare_stack(entry, stack_base, stack_size);
        
        Some(Task {
            tid: TaskId(0), // Will be set by scheduler
            rsp,
            stack_base,
            stack_size,
            state: TaskState::Ready,
        })
    } else {
        None
    }
}

/// Task-specific error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskError {
    /// No free task slots available
    NoFreeSlots,
    /// Stack allocation failed
    StackAllocationFailed,
    /// Invalid task ID
    InvalidTaskId,
    /// Task is in wrong state for operation
    InvalidState,
}

pub type TaskResult<T> = core::result::Result<T, TaskError>;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_task_table_creation() {
        let table = TaskTable::new();
        
        // All tasks should be in terminated state initially
        for i in 0..MAX_TASKS {
            assert_eq!(table.tasks[i].state, TaskState::Terminated);
        }
    }
    
    #[test]
    fn test_find_free_slot() {
        let mut table = TaskTable::new();
        
        // Should find slot 0 first
        assert_eq!(table.find_free_slot(), Some(0));
        
        // Mark slot 0 as ready
        table.tasks[0].state = TaskState::Ready;
        
        // Should find slot 1 next
        assert_eq!(table.find_free_slot(), Some(1));
    }
    
    #[test]
    fn test_stack_preparation() {
        // Mock stack allocation
        let stack_base = 0x1000;
        let stack_size = 4096;
        
        let rsp = prepare_stack(test_entry_point, stack_base, stack_size);
        
        // Stack pointer should be below the top of the stack
        assert!(rsp < stack_base + stack_size);
        assert!(rsp >= stack_base);
        
        // Should be 16-byte aligned
        assert_eq!(rsp & 0xF, 0);
    }
    
    extern "C" fn test_entry_point() {
        // Test entry point
    }
}
