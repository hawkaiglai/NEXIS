//! Memory management subsystem for Nexis kernel
//! 
//! DEPENDENCIES:
//! - Uses x86_64 crate for page table structures
//! - Uses spin for synchronization
//! - Uses bootloader for memory map information
//! 
//! INTEGRATION POINTS:
//! - Called by main.rs during kernel initialization
//! - Used by scheduler for task stack allocation
//! - Provides heap allocation for kernel data structures
//! 
//! TESTING REQUIREMENTS:
//! - Frame allocation/deallocation stress tests
//! - Page mapping validation tests
//! - Heap allocation correctness tests
//! 
//! PERFORMANCE CONSIDERATIONS:
//! - Bitmap scanning for frame allocation is O(n)
//! - Page table walks are cached by CPU TLB
//! - Heap allocation uses simple first-fit strategy

pub use self::frame_allocator::{BitmapFrameAllocator, FrameAllocator};
pub use self::page_allocator::{PageAllocator, PageFlags, OffsetPageTable};
pub use self::heap::{init_heap, HEAP_START, HEAP_SIZE};
pub use self::layout::*;

mod frame_allocator;
mod page_allocator;
mod heap;
mod layout;

use spin::Mutex;
use lazy_static::lazy_static;

/// Page/frame size — 4 KiB
pub const FRAME_SIZE: usize = 4096;
pub const PAGE_SIZE: usize = FRAME_SIZE;

/// Physical frame address (physical addr aligned to FRAME_SIZE)
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysFrame(pub usize);

impl PhysFrame {
    #[inline]
    pub fn start_address(&self) -> PhysAddr {
        PhysAddr(self.0)
    }
    
    #[inline]
    pub fn containing_address(addr: PhysAddr) -> Self {
        PhysFrame(addr.0 & !(FRAME_SIZE - 1))
    }
}

/// Physical memory address
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysAddr(pub usize);

impl PhysAddr {
    #[inline]
    pub const fn new(addr: usize) -> Self {
        Self(addr)
    }
    
    #[inline]
    pub const fn as_u64(self) -> u64 {
        self.0 as u64
    }
    
    #[inline]
    pub const fn as_usize(self) -> usize {
        self.0
    }
}

/// Virtual memory address  
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtAddr(pub usize);

impl VirtAddr {
    #[inline]
    pub const fn new(addr: usize) -> Self {
        Self(addr)
    }
    
    #[inline]
    pub const fn as_u64(self) -> u64 {
        self.0 as u64
    }
    
    #[inline]
    pub const fn as_usize(self) -> usize {
        self.0
    }
    
    #[inline]
    pub const fn as_ptr<T>(self) -> *const T {
        self.0 as *const T
    }
    
    #[inline]
    pub const fn as_mut_ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }
}

/// Error types for memory operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryError {
    OutOfMemory,
    InvalidAddress,
    AlreadyMapped,
    NotMapped,
    PermissionDenied,
}

pub type Result<T> = core::result::Result<T, MemoryError>;

/// Global frame allocator instance
lazy_static! {
    pub static ref FRAME_ALLOCATOR: Mutex<Option<BitmapFrameAllocator>> = Mutex::new(None);
}

/// Initialize the global frame allocator
pub fn init_frame_allocator(
    bitmap_addr: usize,
    bitmap_size: usize,
    start_frame: PhysFrame,
    frame_count: usize,
) -> Result<()> {
    let allocator = unsafe {
        BitmapFrameAllocator::new(bitmap_addr, bitmap_size, start_frame, frame_count)?
    };
    
    *FRAME_ALLOCATOR.lock() = Some(allocator);
    Ok(())
}

/// Allocate a physical frame using the global allocator
pub fn allocate_frame() -> Option<PhysFrame> {
    FRAME_ALLOCATOR.lock()
        .as_mut()?
        .allocate_frame()
}

/// Deallocate a physical frame using the global allocator
pub fn deallocate_frame(frame: PhysFrame) {
    if let Some(allocator) = FRAME_ALLOCATOR.lock().as_mut() {
        allocator.deallocate_frame(frame);
    }
}

/// Get the number of free frames
pub fn free_frame_count() -> usize {
    FRAME_ALLOCATOR.lock()
        .as_ref()
        .map(|a| a.free_frame_count())
        .unwrap_or(0)
}
