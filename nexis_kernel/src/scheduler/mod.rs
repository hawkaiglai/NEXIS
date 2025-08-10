//! Task scheduler implementation for IronVeil/Nexis kernel
//! 
//! DEPENDENCIES:
//! - Uses core types: TaskId, Result, IronVeilError
//! - Uses memory::PhysicalMemoryManager for stack allocation
//! - Uses interrupt system for preemptive scheduling
//! 
//! INTEGRATION POINTS:
//! - Called by main.rs during kernel initialization
//! - Used by PIT timer interrupt for preemptive scheduling
//! - Integrates with memory manager for task stack allocation
//! - Provides task spawning interface for user tasks
//! 
//! TESTING REQUIREMENTS:
//! - Test task spawning and termination
//! - Test context switching between tasks
//! - Test preemptive scheduling behavior
//! - Test cooperative yielding
//! 
//! PERFORMANCE CONSIDERATIONS:
//! - Context switching must be fast (assembly optimized)
//! - Scheduler should have minimal overhead
//! - Task table access is protected by spinlocks

pub mod task;
pub mod preemptive;
pub mod context_switch;

use crate::memory::PhysicalMemoryManager;
use task::{Task, TaskState, TaskId, TASK_TABLE};
use context_switch::context_switch;
use spin::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    /// Currently running task index
    pub static ref CURRENT: Mutex<Option<usize>> = Mutex::new(None);
    
    /// Round-robin scheduling queue
    pub static ref RUN_QUEUE: Mutex<Vec<usize>> = Mutex::new(Vec::new());
    
    /// Next task ID to allocate
    static ref NEXT_TID: Mutex<u64> = Mutex::new(1);
}

/// Initialize the scheduler subsystem
pub fn init() {
    crate::vga::vprintln!("Scheduler initialized");
}

/// Spawn a new task with the given entry point and stack size
/// 
/// # Arguments
/// * `entry` - Function to execute in the new task
/// * `pmm` - Physical memory manager for stack allocation
/// * `pages` - Number of pages to allocate for the task stack
/// 
/// # Returns
/// * `Some(slot)` - Task table slot index if successful
/// * `None` - If spawning failed (no free slots or memory allocation failed)
pub fn spawn(
    entry: extern "C" fn(), 
    pmm: &PhysicalMemoryManager, 
    pages: usize
) -> Option<usize> {
    let mut table = TASK_TABLE.lock();
    let slot = table.find_free_slot()?;
    
    // Allocate new task ID
    let tid = {
        let mut next_tid = NEXT_TID.lock();
        let id = *next_tid;
        *next_tid += 1;
        TaskId(id)
    };
    
    // Allocate stack for the task
    if let Some((stack_base, stack_size)) = task::alloc_stack(pmm, pages) {
        let rsp = task::prepare_stack(entry, stack_base, stack_size);
        
        let new_task = Task {
            tid,
            rsp,
            stack_base,
            stack_size,
            state: TaskState::Ready,
        };
        
        table.tasks[slot] = new_task;
        
        // Add to run queue
        RUN_QUEUE.lock().push(slot);
        
        Some(slot)
    } else {
        None
    }
}

/// Start the main scheduling loop (never returns)
/// This function begins task execution and handles scheduling
pub fn schedule_loop() -> ! {
    loop {
        // Check for preemptive scheduling request
        preemptive::check_preemption();
        
        // Find next ready task
        let next_idx = find_next_ready_task();
        
        let next = match next_idx {
            Some(i) => i,
            None => {
                // No tasks ready - halt and wait for interrupts
                unsafe { 
                    x86_64::instructions::hlt();
                }
                continue;
            }
        };
        
        // Switch to the next task
        switch_to_task(next);
    }
}

/// Find the next ready task using round-robin scheduling
fn find_next_ready_task() -> Option<usize> {
    let table = TASK_TABLE.lock();
    let current = *CURRENT.lock();
    
    // Start searching from the task after current
    let start_idx = match current {
        Some(idx) => (idx + 1) % task::TaskTable::MAX_TASKS,
        None => 0,
    };
    
    // Round-robin search for ready task
    for i in 0..task::TaskTable::MAX_TASKS {
        let idx = (start_idx + i) % task::TaskTable::MAX_TASKS;
        if table.tasks[idx].state == TaskState::Ready {
            return Some(idx);
        }
    }
    
    None
}

/// Switch to the specified task
fn switch_to_task(next_idx: usize) {
    let mut table = TASK_TABLE.lock();
    let current_idx = *CURRENT.lock();
    
    // Mark next task as running
    table.tasks[next_idx].state = TaskState::Running;
    
    match current_idx {
        Some(cur_idx) if cur_idx != next_idx => {
            // Mark current task as ready (unless it's blocked/terminated)
            if table.tasks[cur_idx].state == TaskState::Running {
                table.tasks[cur_idx].state = TaskState::Ready;
            }
            
            // Perform context switch
            let cur_task = &mut table.tasks[cur_idx];
            let next_task = &mut table.tasks[next_idx];
            
            // Update current task pointer before context switch
            *CURRENT.lock() = Some(next_idx);
            
            unsafe {
                context_switch(&mut cur_task.rsp, next_task.rsp);
            }
        }
        None => {
            // First task to run - just jump to it
            let next_task = &mut table.tasks[next_idx];
            *CURRENT.lock() = Some(next_idx);
            
            unsafe {
                let mut dummy_rsp: usize = 0;
                context_switch(&mut dummy_rsp, next_task.rsp);
            }
        }
        Some(_) => {
            // Current task is the same as next task - nothing to do
        }
    }
}

/// Yield the CPU voluntarily (cooperative scheduling)
pub fn task_yield() {
    let current_idx = match *CURRENT.lock() {
        Some(idx) => idx,
        None => return, // No current task
    };
    
    let mut table = TASK_TABLE.lock();
    
    // Mark current task as ready
    if table.tasks[current_idx].state == TaskState::Running {
        table.tasks[current_idx].state = TaskState::Ready;
    }
    
    // Find next ready task
    drop(table); // Release lock before finding next task
    
    if let Some(next_idx) = find_next_ready_task() {
        switch_to_task(next_idx);
    }
}

/// Block the current task
pub fn block_current_task() {
    if let Some(current_idx) = *CURRENT.lock() {
        let mut table = TASK_TABLE.lock();
        table.tasks[current_idx].state = TaskState::Blocked;
        
        // Remove from run queue
        RUN_QUEUE.lock().retain(|&x| x != current_idx);
        
        drop(table);
        
        // Find next task to run
        if let Some(next_idx) = find_next_ready_task() {
            switch_to_task(next_idx);
        }
    }
}

/// Unblock a task by task ID
pub fn unblock_task(tid: TaskId) -> bool {
    let mut table = TASK_TABLE.lock();
    
    for i in 0..task::TaskTable::MAX_TASKS {
        if table.tasks[i].tid == tid && table.tasks[i].state == TaskState::Blocked {
            table.tasks[i].state = TaskState::Ready;
            RUN_QUEUE.lock().push(i);
            return true;
        }
    }
    
    false
}

/// Terminate the current task
pub fn terminate_current_task() -> ! {
    if let Some(current_idx) = *CURRENT.lock() {
        let mut table = TASK_TABLE.lock();
        let task = &mut table.tasks[current_idx];
        
        // Free the task's stack
        task::free_stack(task.stack_base, task.stack_size);
        
        // Mark as terminated
        task.state = TaskState::Terminated;
        
        // Remove from run queue
        RUN_QUEUE.lock().retain(|&x| x != current_idx);
        
        // Clear current task
        *CURRENT.lock() = None;
        
        drop(table);
        
        // Find next task to run
        if let Some(next_idx) = find_next_ready_task() {
            switch_to_task(next_idx);
        }
    }
    
    // If no tasks available, halt
    loop {
        unsafe {
            x86_64::instructions::hlt();
        }
    }
}

/// Check for pending preemptive scheduling and handle it
pub fn check_and_schedule() {
    preemptive::check_preemption();
}

/// Get the current task ID
pub fn current_task_id() -> Option<TaskId> {
    if let Some(current_idx) = *CURRENT.lock() {
        let table = TASK_TABLE.lock();
        Some(table.tasks[current_idx].tid)
    } else {
        None
    }
}

/// Get scheduler statistics
pub fn get_stats() -> SchedulerStats {
    let table = TASK_TABLE.lock();
    let mut stats = SchedulerStats::default();
    
    for i in 0..task::TaskTable::MAX_TASKS {
        match table.tasks[i].state {
            TaskState::Ready => stats.ready_tasks += 1,
            TaskState::Running => stats.running_tasks += 1,
            TaskState::Blocked => stats.blocked_tasks += 1,
            TaskState::Terminated => stats.terminated_tasks += 1,
        }
    }
    
    stats.total_tasks = stats.ready_tasks + stats.running_tasks + stats.blocked_tasks;
    stats
}

#[derive(Debug, Default)]
pub struct SchedulerStats {
    pub ready_tasks: usize,
    pub running_tasks: usize,
    pub blocked_tasks: usize,
    pub terminated_tasks: usize,
    pub total_tasks: usize,
}
