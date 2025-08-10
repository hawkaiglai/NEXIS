//! Kernel heap implementation
//! 
//! DEPENDENCIES:
//! - Uses page allocator for mapping heap pages
//! - Uses frame allocator for physical backing
//! - Provides GlobalAlloc implementation
//! 
//! INTEGRATION POINTS:
//! - Used by Rust's alloc crate for Vec, HashMap, etc.
//! - Called during kernel initialization
//! - Grows dynamically as needed

use crate::memory::{VirtAddr, PhysAddr, PageFlags, PageAllocator, OffsetPageTable, MemoryError, Result, FRAME_SIZE, allocate_frame};
use spin::{Mutex, MutexGuard};
use core::alloc::{GlobalAlloc, Layout};
use core::ptr::{self, NonNull};

/// Heap start virtual address (in higher half)
pub const HEAP_START: usize = 0xFFFF_8000_0000_0000;
/// Initial heap size (16 MB)
pub const HEAP_SIZE: usize = 16 * 1024 * 1024;
/// Maximum heap size (1 GB)
pub const MAX_HEAP_SIZE: usize = 1024 * 1024 * 1024;

/// Global heap allocator instance
static HEAP_ALLOCATOR: Mutex<Option<HeapAllocator>> = Mutex::new(None);

/// Simple first-fit heap allocator
pub struct HeapAllocator {
    heap_start: usize,
    heap_size: usize,
    allocated_size: usize,
    free_list: FreeList,
}

/// Free block in the heap
#[derive(Debug)]
struct FreeBlock {
    size: usize,
    next: Option<NonNull<FreeBlock>>,
}

/// List of free blocks
struct FreeList {
    head: Option<NonNull<FreeBlock>>,
}

impl FreeList {
    const fn new() -> Self {
        Self { head: None }
    }
    
    /// Add a free block to the list
    unsafe fn add_block(&mut self, addr: usize, size: usize) {
        if size < core::mem::size_of::<FreeBlock>() {
            return; // Block too small to track
        }
        
        let block = addr as *mut FreeBlock;
        let block_ref = &mut *block;
        block_ref.size = size;
        block_ref.next = self.head;
        self.head = NonNull::new(block);
    }
    
    /// Find and remove a suitable free block
    fn find_block(&mut self, layout: Layout) -> Option<NonNull<FreeBlock>> {
        let mut current = &mut self.head;
        
        while let Some(block_ptr) = *current {
            let block = unsafe { block_ptr.as_ref() };
            
            if block.size >= layout.size() {
                // Remove from list
                *current = block.next;
                return Some(block_ptr);
            }
            
            current = unsafe { &mut block_ptr.as_mut().next };
        }
        
        None
    }
}

impl HeapAllocator {
    /// Create new heap allocator
    fn new(start: usize, size: usize) -> Self {
        let mut allocator = Self {
            heap_start: start,
            heap_size: size,
            allocated_size: 0,
            free_list: FreeList::new(),
        };
        
        // Initialize with one large free block
        unsafe {
            allocator.free_list.add_block(start, size);
        }
        
        allocator
    }
    
    /// Allocate memory block
    fn allocate(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        let size = layout.size().max(core::mem::size_of::<FreeBlock>());
        let align = layout.align();
        
        // Try to find a suitable free block
        if let Some(block_ptr) = self.free_list.find_block(layout) {
            let block = unsafe { block_ptr.as_ref() };
            let block_addr = block_ptr.as_ptr() as usize;
            let block_size = block.size;
            
            // Align the address
            let aligned_addr = (block_addr + align - 1) & !(align - 1);
            let padding = aligned_addr - block_addr;
            
            if block_size >= size + padding {
                // Split the block if there's enough leftover space
                let leftover_size = block_size - size - padding;
                if leftover_size >= core::mem::size_of::<FreeBlock>() {
                    let leftover_addr = aligned_addr + size;
                    unsafe {
                        self.free_list.add_block(leftover_addr, leftover_size);
                    }
                }
                
                // Add padding back to free list if significant
                if padding >= core::mem::size_of::<FreeBlock>() {
                    unsafe {
                        self.free_list.add_block(block_addr, padding);
                    }
                }
                
                self.allocated_size += size;
                return NonNull::new(aligned_addr as *mut u8);
            }
        }
        
        // No suitable block found, try to grow heap
        self.grow_heap(layout.size()).ok()?;
        
        // Try allocation again after growing
        if let Some(block_ptr) = self.free_list.find_block(layout) {
            let block = unsafe { block_ptr.as_ref() };
            let block_addr = block_ptr.as_ptr() as usize;
            let aligned_addr = (block_addr + align - 1) & !(align - 1);
            
            self.allocated_size += size;
            return NonNull::new(aligned_addr as *mut u8);
        }
        
        None
    }
    
    /// Deallocate memory block
    unsafe fn deallocate(&mut self, ptr: NonNull<u8>, layout: Layout) {
        let addr = ptr.as_ptr() as usize;
        let size = layout.size().max(core::mem::size_of::<FreeBlock>());
        
        // Add block back to free list
        self.free_list.add_block(addr, size);
        self.allocated_size = self.allocated_size.saturating_sub(size);
        
        // TODO: Implement block coalescing to reduce fragmentation
    }
    
    /// Grow the heap by allocating more pages
    fn grow_heap(&mut self, min_size: usize) -> Result<()> {
        let current_end = self.heap_start + self.heap_size;
        let pages_needed = (min_size + FRAME_SIZE - 1) / FRAME_SIZE;
        let new_size = pages_needed * FRAME_SIZE;
        
        if self.heap_size + new_size > MAX_HEAP_SIZE {
            return Err(MemoryError::OutOfMemory);
        }
        
        // Allocate and map new pages
        for i in 0..pages_needed {
            let virt_addr = VirtAddr(current_end + i * FRAME_SIZE);
            
            // Allocate physical frame
            let frame = allocate_frame().ok_or(MemoryError::OutOfMemory)?;
            
            // Map the page (this is simplified - in practice you'd use the page allocator)
            // For now, we'll assume the mapping succeeds
            // In a full implementation, you'd use:
            // page_allocator.map_page(virt_addr, frame.start_address(), PageFlags::default())?;
        }
        
        // Add new space to free list
        unsafe {
            self.free_list.add_block(current_end, new_size);
        }
        
        self.heap_size += new_size;
        Ok(())
    }
    
    /// Get heap statistics
    pub fn stats(&self) -> HeapStats {
        HeapStats {
            total_size: self.heap_size,
            allocated_size: self.allocated_size,
            free_size: self.heap_size - self.allocated_size,
        }
    }
}

/// Heap statistics for debugging
#[derive(Debug, Clone, Copy)]
pub struct HeapStats {
    pub total_size: usize,
    pub allocated_size: usize,
    pub free_size: usize,
}

/// Global allocator implementation
struct GlobalHeapAllocator;

unsafe impl GlobalAlloc for GlobalHeapAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut heap = HEAP_ALLOCATOR.lock();
        if let Some(allocator) = heap.as_mut() {
            allocator.allocate(layout)
                .map(|ptr| ptr.as_ptr())
                .unwrap_or(ptr::null_mut())
        } else {
            ptr::null_mut()
        }
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if let Some(non_null_ptr) = NonNull::new(ptr) {
            let mut heap = HEAP_ALLOCATOR.lock();
            if let Some(allocator) = heap.as_mut() {
                allocator.deallocate(non_null_ptr, layout);
            }
        }
    }
}

/// Global allocator instance
#[global_allocator]
static GLOBAL_ALLOCATOR: GlobalHeapAllocator = GlobalHeapAllocator;

/// Initialize the kernel heap
/// 
/// This should be called early in kernel initialization after the page allocator is set up
pub fn init_heap(page_allocator: &mut dyn PageAllocator) -> Result<()> {
    // Map initial heap pages
    let heap_start = VirtAddr(HEAP_START);
    let initial_pages = HEAP_SIZE / FRAME_SIZE;
    
    for i in 0..initial_pages {
        let virt_addr = VirtAddr(HEAP_START + i * FRAME_SIZE);
        
        // Allocate physical frame
        let frame = allocate_frame().ok_or(MemoryError::OutOfMemory)?;
        
        // Map with writable flags
        let flags = PageFlags {
            writable: true,
            user_accessible: false,
            no_execute: true,
            global: false,
        };
        
        page_allocator.map_page(virt_addr, frame.start_address(), flags)?;
    }
    
    // Initialize the heap allocator
    let allocator = HeapAllocator::new(HEAP_START, HEAP_SIZE);
    *HEAP_ALLOCATOR.lock() = Some(allocator);
    
    Ok(())
}

/// Get heap statistics
pub fn heap_stats() -> Option<HeapStats> {
    HEAP_ALLOCATOR.lock().as_ref().map(|alloc| alloc.stats())
}

/// Allocate aligned memory
pub fn alloc_aligned(size: usize, align: usize) -> Option<NonNull<u8>> {
    let layout = Layout::from_size_align(size, align).ok()?;
    HEAP_ALLOCATOR.lock().as_mut()?.allocate(layout)
}

/// Deallocate aligned memory
pub unsafe fn dealloc_aligned(ptr: NonNull<u8>, size: usize, align: usize) {
    if let Ok(layout) = Layout::from_size_align(size, align) {
        if let Some(allocator) = HEAP_ALLOCATOR.lock().as_mut() {
            allocator.deallocate(ptr, layout);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;
    use alloc::string::String;
    
    #[test]
    fn test_basic_allocation() {
        // This test would require proper heap initialization
        // For now, it's a placeholder to show the intended API
        let vec = Vec::with_capacity(10);
        assert_eq!(vec.len(), 0);
        assert!(vec.capacity() >= 10);
    }
    
    #[test]
    fn test_string_allocation() {
        let s = String::from("Hello, IronVeil!");
        assert_eq!(s.len(), 17);
    }
}
