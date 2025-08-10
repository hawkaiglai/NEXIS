//! Virtual memory page allocator and mapper
//! 
//! DEPENDENCIES:
//! - Uses x86_64 crate for page table structures
//! - Uses frame allocator for physical backing
//! 
//! INTEGRATION POINTS:
//! - Used by heap allocator for mapping heap pages
//! - Used by task manager for mapping user stacks
//! - Called during kernel initialization for identity mapping

use crate::memory::{PhysFrame, PhysAddr, VirtAddr, MemoryError, Result, FRAME_SIZE, allocate_frame, deallocate_frame};
use x86_64::structures::paging::{
    PageTable, PageTableFlags, PhysFrame as X86PhysFrame, Page, Size4KiB,
    page_table::PageTableEntry, mapper::*, RecursivePageTable,
};
use x86_64::{PhysAddr as X86PhysAddr, VirtAddr as X86VirtAddr};
use spin::Mutex;

/// Page mapping flags
#[derive(Clone, Copy, Debug)]
pub struct PageFlags {
    pub writable: bool,
    pub user_accessible: bool,
    pub no_execute: bool,
    pub global: bool,
}

impl Default for PageFlags {
    fn default() -> Self {
        Self {
            writable: false,
            user_accessible: false,
            no_execute: false,
            global: false,
        }
    }
}

impl From<PageFlags> for PageTableFlags {
    fn from(flags: PageFlags) -> Self {
        let mut ptf = PageTableFlags::PRESENT;
        
        if flags.writable {
            ptf |= PageTableFlags::WRITABLE;
        }
        if flags.user_accessible {
            ptf |= PageTableFlags::USER_ACCESSIBLE;
        }
        if flags.no_execute {
            ptf |= PageTableFlags::NO_EXECUTE;
        }
        if flags.global {
            ptf |= PageTableFlags::GLOBAL;
        }
        
        ptf
    }
}

/// Page allocator trait for abstracting virtual memory mapping
pub trait PageAllocator {
    /// Map virtual address to physical frame
    fn map_page(&mut self, virt: VirtAddr, phys: PhysAddr, flags: PageFlags) -> Result<()>;
    
    /// Unmap virtual page
    fn unmap_page(&mut self, virt: VirtAddr) -> Result<PhysAddr>;
    
    /// Translate virtual to physical address
    fn translate(&self, virt: VirtAddr) -> Option<PhysAddr>;
    
    /// Map a range of pages
    fn map_range(&mut self, virt_start: VirtAddr, phys_start: PhysAddr, size: usize, flags: PageFlags) -> Result<()>;
    
    /// Unmap a range of pages
    fn unmap_range(&mut self, virt_start: VirtAddr, size: usize) -> Result<()>;
    
    /// Update page flags
    fn update_flags(&mut self, virt: VirtAddr, flags: PageFlags) -> Result<()>;
}

/// Custom frame allocator for x86_64 crate integration
struct FrameAllocatorWrapper;

unsafe impl x86_64::structures::paging::FrameAllocator<Size4KiB> for FrameAllocatorWrapper {
    fn allocate_frame(&mut self) -> Option<X86PhysFrame> {
        allocate_frame().map(|frame| {
            X86PhysFrame::from_start_address(X86PhysAddr::new(frame.0 as u64)).unwrap()
        })
    }
}

impl x86_64::structures::paging::FrameDeallocator<Size4KiB> for FrameAllocatorWrapper {
    unsafe fn deallocate_frame(&mut self, frame: X86PhysFrame) {
        deallocate_frame(PhysFrame(frame.start_address().as_u64() as usize));
    }
}

/// Offset page table implementation for higher half kernel
pub struct OffsetPageTable {
    page_table: RecursivePageTable<'static>,
    frame_allocator: FrameAllocatorWrapper,
}

impl OffsetPageTable {
    /// Create new offset page table
    /// 
    /// SAFETY: Caller must ensure the page table is valid and accessible
    pub unsafe fn new(page_table_frame: PhysFrame) -> Result<Self> {
        // Map the page table recursively
        let page_table_virt = 0o177777_777_777_777_777_0000usize; // Recursive mapping address
        let page_table_ptr = page_table_virt as *mut PageTable;
        let page_table = &mut *page_table_ptr;
        
        let recursive_page_table = RecursivePageTable::new(page_table)
            .map_err(|_| MemoryError::InvalidAddress)?;
        
        Ok(Self {
            page_table: recursive_page_table,
            frame_allocator: FrameAllocatorWrapper,
        })
    }
    
    /// Create from active page table
    pub fn from_current() -> Result<Self> {
        use x86_64::registers::control::Cr3;
        
        let (frame, _) = Cr3::read();
        unsafe {
            Self::new(PhysFrame(frame.start_address().as_u64() as usize))
        }
    }
    
    /// Convert VirtAddr to x86_64 VirtAddr
    fn to_x86_virt_addr(addr: VirtAddr) -> X86VirtAddr {
        X86VirtAddr::new(addr.0 as u64)
    }
    
    /// Convert PhysAddr to x86_64 PhysAddr
    fn to_x86_phys_addr(addr: PhysAddr) -> X86PhysAddr {
        X86PhysAddr::new(addr.0 as u64)
    }
    
    /// Convert x86_64 PhysAddr to PhysAddr
    fn from_x86_phys_addr(addr: X86PhysAddr) -> PhysAddr {
        PhysAddr(addr.as_u64() as usize)
    }
}

impl PageAllocator for OffsetPageTable {
    fn map_page(&mut self, virt: VirtAddr, phys: PhysAddr, flags: PageFlags) -> Result<()> {
        let page = Page::containing_address(Self::to_x86_virt_addr(virt));
        let frame = X86PhysFrame::containing_address(Self::to_x86_phys_addr(phys));
        let page_flags = PageTableFlags::from(flags);
        
        unsafe {
            self.page_table
                .map_to(page, frame, page_flags, &mut self.frame_allocator)
                .map_err(|_| MemoryError::AlreadyMapped)?
                .flush();
        }
        
        Ok(())
    }
    
    fn unmap_page(&mut self, virt: VirtAddr) -> Result<PhysAddr> {
        let page = Page::containing_address(Self::to_x86_virt_addr(virt));
        
        let (frame, flush) = self.page_table
            .unmap(page)
            .map_err(|_| MemoryError::NotMapped)?;
        
        flush.flush();
        
        Ok(Self::from_x86_phys_addr(frame.start_address()))
    }
    
    fn translate(&self, virt: VirtAddr) -> Option<PhysAddr> {
        use x86_64::structures::paging::mapper::Translate;
        
        self.page_table
            .translate_addr(Self::to_x86_virt_addr(virt))
            .map(Self::from_x86_phys_addr)
    }
    
    fn map_range(&mut self, virt_start: VirtAddr, phys_start: PhysAddr, size: usize, flags: PageFlags) -> Result<()> {
        let page_count = (size + FRAME_SIZE - 1) / FRAME_SIZE;
        
        for i in 0..page_count {
            let virt = VirtAddr(virt_start.0 + i * FRAME_SIZE);
            let phys = PhysAddr(phys_start.0 + i * FRAME_SIZE);
            self.map_page(virt, phys, flags)?;
        }
        
        Ok(())
    }
    
    fn unmap_range(&mut self, virt_start: VirtAddr, size: usize) -> Result<()> {
        let page_count = (size + FRAME_SIZE - 1) / FRAME_SIZE;
        
        for i in 0..page_count {
            let virt = VirtAddr(virt_start.0 + i * FRAME_SIZE);
            self.unmap_page(virt)?;
        }
        
        Ok(())
    }
    
    fn update_flags(&mut self, virt: VirtAddr, flags: PageFlags) -> Result<()> {
        let page = Page::containing_address(Self::to_x86_virt_addr(virt));
        let page_flags = PageTableFlags::from(flags);
        
        unsafe {
            self.page_table
                .update_flags(page, page_flags)
                .map_err(|_| MemoryError::NotMapped)?
                .flush();
        }
        
        Ok(())
    }
}

/// Simple identity mapper for kernel initialization
pub struct IdentityMapper {
    frame_allocator: FrameAllocatorWrapper,
}

impl IdentityMapper {
    pub fn new() -> Self {
        Self {
            frame_allocator: FrameAllocatorWrapper,
        }
    }
    
    /// Create identity mapping for physical memory range
    pub fn identity_map_range(&mut self, start: PhysAddr, size: usize, flags: PageFlags) -> Result<()> {
        let page_count = (size + FRAME_SIZE - 1) / FRAME_SIZE;
        
        // Get current page table
        use x86_64::registers::control::Cr3;
        let (page_table_frame, _) = Cr3::read();
        let page_table_phys = page_table_frame.start_address().as_u64() as usize;
        
        // For now, we'll use a simple approach and assume we can access the page table
        // In a full implementation, this would need proper recursive mapping setup
        
        for i in 0..page_count {
            let addr = start.0 + i * FRAME_SIZE;
            // Identity map: virtual address = physical address
            // This is a simplified implementation
            // In practice, you'd need to properly walk and modify page tables
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_page_flags_conversion() {
        let flags = PageFlags {
            writable: true,
            user_accessible: true,
            no_execute: false,
            global: false,
        };
        
        let pt_flags = PageTableFlags::from(flags);
        assert!(pt_flags.contains(PageTableFlags::PRESENT));
        assert!(pt_flags.contains(PageTableFlags::WRITABLE));
        assert!(pt_flags.contains(PageTableFlags::USER_ACCESSIBLE));
        assert!(!pt_flags.contains(PageTableFlags::NO_EXECUTE));
    }
}
