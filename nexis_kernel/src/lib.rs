#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

//! IronVeil OS Nexis Kernel Library
//! 
//! DEPENDENCIES:
//! - bootloader crate for boot protocol
//! - x86_64 crate for hardware abstractions
//! - spin for no_std synchronization
//! - lazy_static for static initialization
//! 
//! INTEGRATION POINTS:
//! - Entry point for bootloader
//! - Provides core kernel services to all modules
//! - Manages global system state
//! 
//! TESTING REQUIREMENTS:
//! - Integration tests for module interactions
//! - Memory safety verification
//! - Interrupt handling validation

extern crate alloc;

// Core kernel modules
pub mod memory;
pub mod interrupt;
pub mod scheduler;
pub mod drivers;
pub mod task;

// Re-export commonly used types
pub use crate::memory::{
    PhysAddr, VirtAddr, PhysFrame, 
    FrameAllocator, PageAllocator, PageFlags,
    PhysicalMemoryManager, FRAME_SIZE
};

pub use crate::interrupt::{
    InterruptManager, InterruptHandler, InterruptHandlerWithError,
    interrupts
};

pub use crate::scheduler::{
    Task, TaskId, TaskState, Scheduler
};

pub use crate::task::{
    prepare_stack, TaskTable
};

// Global error type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IronVeilError {
    OutOfMemory,
    InvalidAddress,
    TaskNotFound,
    NetworkError,
    PrivacyViolation,
    HardwareError,
    InvalidInput,
    PermissionDenied,
    ResourceBusy,
    NotImplemented,
}

impl core::fmt::Display for IronVeilError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            IronVeilError::OutOfMemory => write!(f, "Out of memory"),
            IronVeilError::InvalidAddress => write!(f, "Invalid address"),
            IronVeilError::TaskNotFound => write!(f, "Task not found"),
            IronVeilError::NetworkError => write!(f, "Network error"),
            IronVeilError::PrivacyViolation => write!(f, "Privacy violation"),
            IronVeilError::HardwareError => write!(f, "Hardware error"),
            IronVeilError::InvalidInput => write!(f, "Invalid input"),
            IronVeilError::PermissionDenied => write!(f, "Permission denied"),
            IronVeilError::ResourceBusy => write!(f, "Resource busy"),
            IronVeilError::NotImplemented => write!(f, "Not implemented"),
        }
    }
}

pub type Result<T> = core::result::Result<T, IronVeilError>;

// Global system state
use spin::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    /// Global physical memory manager
    pub static ref PHYSICAL_MEMORY_MANAGER: Mutex<PhysicalMemoryManager> = 
        Mutex::new(PhysicalMemoryManager::new_uninit());
    
    /// Global interrupt manager
    pub static ref INTERRUPT_MANAGER: Mutex<interrupt::IdtManager> = 
        Mutex::new(interrupt::IdtManager::new());
    
    /// Global task scheduler
    pub static ref SCHEDULER: Mutex<scheduler::RoundRobinScheduler> = 
        Mutex::new(scheduler::RoundRobinScheduler::new());
        
    /// System uptime in timer ticks
    pub static ref SYSTEM_UPTIME: Mutex<u64> = Mutex::new(0);
    
    /// System entropy pool for random number generation
    pub static ref ENTROPY_POOL: Mutex<[u64; 4]> = Mutex::new([0; 4]);
}

// Kernel initialization functions
pub fn init_memory(boot_info: &bootloader::BootInfo) -> Result<()> {
    // Initialize physical memory manager based on boot info
    let mut pmm = PHYSICAL_MEMORY_MANAGER.lock();
    
    // Use memory map from bootloader to set up frame allocator
    memory::init_frame_allocator(&mut pmm, boot_info)?;
    
    // Initialize kernel heap
    memory::init_heap()?;
    
    drivers::vga::vprintln!("Memory management initialized");
    Ok(())
}

pub fn init_interrupts() -> Result<()> {
    let mut idt = INTERRUPT_MANAGER.lock();
    
    // Set up IDT with exception and interrupt handlers
    idt.init()?;
    
    // Initialize and remap PIC
    interrupt::pic::init_and_remap()?;
    
    // Enable interrupts globally
    x86_64::instructions::interrupts::enable();
    
    drivers::vga::vprintln!("Interrupt system initialized");
    Ok(())
}

pub fn init_drivers() -> Result<()> {
    // Initialize VGA driver
    drivers::vga::init()?;
    
    // Initialize keyboard driver
    drivers::keyboard::init()?;
    
    // Initialize timer (PIT)
    drivers::timer::init(50)?; // 50 Hz
    
    // Initialize RTC if available
    if let Err(_) = drivers::rtc::init() {
        drivers::vga::vprintln!("Warning: RTC initialization failed");
    }
    
    drivers::vga::vprintln!("Device drivers initialized");
    Ok(())
}

pub fn init_scheduler() -> Result<()> {
    let mut scheduler = SCHEDULER.lock();
    scheduler.init()?;
    
    drivers::vga::vprintln!("Task scheduler initialized");
    Ok(())
}

// System call interface
#[repr(u64)]
#[derive(Debug, Clone, Copy)]
pub enum SyscallNumber {
    Exit = 0,
    Write = 1,
    Read = 2,
    Open = 3,
    Close = 4,
    Yield = 5,
    GetPid = 6,
    Sleep = 7,
    GetTime = 8,
    AllocateMemory = 9,
    FreeMemory = 10,
    CreateTask = 11,
    KillTask = 12,
    WaitTask = 13,
    SendMessage = 14,
    ReceiveMessage = 15,
}

// System call handler (called from assembly)
#[no_mangle]
pub extern "C" fn syscall_handler_rust(
    syscall_num: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
    arg6: u64,
) -> u64 {
    use SyscallNumber::*;
    
    let syscall = match syscall_num {
        0 => Exit,
        1 => Write,
        2 => Read,
        3 => Open,
        4 => Close,
        5 => Yield,
        6 => GetPid,
        7 => Sleep,
        8 => GetTime,
        9 => AllocateMemory,
        10 => FreeMemory,
        11 => CreateTask,
        12 => KillTask,
        13 => WaitTask,
        14 => SendMessage,
        15 => ReceiveMessage,
        _ => return u64::MAX, // Invalid syscall
    };
    
    match handle_syscall(syscall, arg1, arg2, arg3, arg4, arg5, arg6) {
        Ok(result) => result,
        Err(_) => u64::MAX, // Error indicator
    }
}

fn handle_syscall(
    syscall: SyscallNumber,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    _arg5: u64,
    _arg6: u64,
) -> Result<u64> {
    use SyscallNumber::*;
    
    match syscall {
        Exit => {
            let exit_code = arg1 as i32;
            scheduler::terminate_current_task(exit_code);
            Ok(0) // Never reached
        }
        
        Write => {
            let fd = arg1;
            let buf_ptr = arg2 as *const u8;
            let len = arg3 as usize;
            
            if fd == 1 || fd == 2 { // stdout/stderr
                let slice = unsafe { core::slice::from_raw_parts(buf_ptr, len) };
                if let Ok(s) = core::str::from_utf8(slice) {
                    drivers::vga::vprint!("{}", s);
                    Ok(len as u64)
                } else {
                    Err(IronVeilError::InvalidInput)
                }
            } else {
                Err(IronVeilError::NotImplemented)
            }
        }
        
        Yield => {
            scheduler::task_yield();
            Ok(0)
        }
        
        GetPid => {
            Ok(scheduler::get_current_task_id().0)
        }
        
        GetTime => {
            Ok(*SYSTEM_UPTIME.lock())
        }
        
        AllocateMemory => {
            let size = arg1 as usize;
            let align = arg2 as usize;
            
            match memory::allocate_pages(size, align) {
                Ok(addr) => Ok(addr.0 as u64),
                Err(e) => Err(e),
            }
        }
        
        FreeMemory => {
            let addr = VirtAddr(arg1 as usize);
            let size = arg2 as usize;
            
            memory::deallocate_pages(addr, size)?;
            Ok(0)
        }
        
        CreateTask => {
            let entry_point = VirtAddr(arg1 as usize);
            let stack_size = arg2 as usize;
            
            match scheduler::spawn_task(entry_point, stack_size) {
                Ok(task_id) => Ok(task_id.0),
                Err(e) => Err(e),
            }
        }
        
        Sleep => {
            let ticks = arg1;
            scheduler::sleep_current_task(ticks);
            Ok(0)
        }
        
        _ => Err(IronVeilError::NotImplemented),
    }
}

// Panic handler
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // Disable interrupts to prevent further issues
    x86_64::instructions::interrupts::disable();
    
    drivers::vga::vprintln!("\n\n*** KERNEL PANIC ***");
    
    if let Some(location) = info.location() {
        drivers::vga::vprintln!(
            "Panic at {}:{}:{}", 
            location.file(), 
            location.line(), 
            location.column()
        );
    }
    
    if let Some(message) = info.message() {
        drivers::vga::vprintln!("Message: {}", message);
    }
    
    if let Some(payload) = info.payload().downcast_ref::<&str>() {
        drivers::vga::vprintln!("Payload: {}", payload);
    }
    
    // Dump system state for debugging
    dump_system_state();
    
    drivers::vga::vprintln!("\nSystem halted. Please restart.");
    
    // Halt forever
    loop {
        x86_64::instructions::hlt();
    }
}

fn dump_system_state() {
    drivers::vga::vprintln!("\n--- System State Dump ---");
    
    // Memory information
    let pmm = PHYSICAL_MEMORY_MANAGER.lock();
    drivers::vga::vprintln!(
        "Memory: {} free frames of {} total", 
        pmm.free_frames(), 
        pmm.total_frames()
    );
    drop(pmm);
    
    // Uptime
    let uptime = *SYSTEM_UPTIME.lock();
    drivers::vga::vprintln!("Uptime: {} ticks", uptime);
    
    // Current task info
    if let Some(current_task) = scheduler::get_current_task_info() {
        drivers::vga::vprintln!(
            "Current task: ID={}, state={:?}", 
            current_task.id.0, 
            current_task.state
        );
    } else {
        drivers::vga::vprintln!("No current task");
    }
    
    drivers::vga::vprintln!("--- End Dump ---");
}

// Kernel heap allocator error handler
#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("Allocation error: {:?}", layout);
}

// Global allocator (defined in memory module)
#[global_allocator]
static ALLOCATOR: memory::heap::LockedHeap = memory::heap::LockedHeap::empty();

// Runtime entropy collection
pub fn collect_entropy() {
    let mut pool = ENTROPY_POOL.lock();
    
    // Collect entropy from various sources
    let rdtsc = unsafe { core::arch::x86_64::_rdtsc() };
    let stack_addr = &pool as *const _ as u64;
    let uptime = *SYSTEM_UPTIME.lock();
    
    // Simple mixing function
    pool[0] ^= rdtsc;
    pool[1] ^= stack_addr;
    pool[2] ^= uptime;
    pool[3] ^= rdtsc.wrapping_mul(0x9e3779b97f4a7c15);
    
    // Rotate the pool
    let temp = pool[0];
    pool[0] = pool[1];
    pool[1] = pool[2];
    pool[2] = pool[3];
    pool[3] = temp;
}

// Get random number from entropy pool
pub fn get_random_u64() -> u64 {
    collect_entropy();
    let pool = ENTROPY_POOL.lock();
    pool[0] ^ pool[1] ^ pool[2] ^ pool[3]
}

// Timer tick handler (called from interrupt handlers)
pub fn system_tick() {
    // Increment uptime
    *SYSTEM_UPTIME.lock() += 1;
    
    // Collect entropy
    collect_entropy();
    
    // Update scheduler time slice
    scheduler::timer_tick();
}

// Module-specific initialization for testing
#[cfg(test)]
pub fn init_for_testing() -> Result<()> {
    // Minimal initialization for unit tests
    Ok(())
}
