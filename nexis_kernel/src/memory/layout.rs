//! Memory layout definitions and constants
//! 
//! DEPENDENCIES:
//! - Defines memory regions for kernel and user space
//! - Used by all memory management components
//! 
//! INTEGRATION POINTS:
//! - Used by frame allocator for region management
//! - Used by page allocator for virtual address layout
//! - Referenced by linker script symbols

use crate::memory::{VirtAddr, PhysAddr, FRAME_SIZE};

/// Physical memory layout constants
pub mod physical {
    use super::PhysAddr;
    
    /// Start of conventional memory (after BIOS data)
    pub const CONVENTIONAL_START: PhysAddr = PhysAddr(0x500);
    
    /// End of conventional memory (start of EBDA)
    pub const CONVENTIONAL_END: PhysAddr = PhysAddr(0x80000);
    
    /// Start of extended memory (1 MB)
    pub const EXTENDED_START: PhysAddr = PhysAddr(0x100000);
    
    /// Typical end of usable memory (this will be determined from memory map)
    pub const MAX_MEMORY: PhysAddr = PhysAddr(0x100000000); // 4 GB
    
    /// VGA frame buffer
    pub const VGA_BUFFER: PhysAddr = PhysAddr(0xB8000);
    pub const VGA_BUFFER_SIZE: usize = 0x8000;
    
    /// BIOS areas to avoid
    pub const BIOS_START: PhysAddr = PhysAddr(0x80000);
    pub const BIOS_END: PhysAddr = PhysAddr(0x100000);
}

/// Virtual memory layout constants
pub mod virtual_memory {
    use super::VirtAddr;
    
    /// Kernel virtual address space layout (higher half)
    
    /// Kernel code/data start (higher half)
    pub const KERNEL_START: VirtAddr = VirtAddr(0xFFFFFFFF80000000);
    
    /// Kernel heap region
    pub const HEAP_START: VirtAddr = VirtAddr(0xFFFF800000000000);
    pub const HEAP_END: VirtAddr = VirtAddr(0xFFFF900000000000);
    
    /// Kernel stack region
    pub const KERNEL_STACK_START: VirtAddr = VirtAddr(0xFFFF900000000000);
    pub const KERNEL_STACK_END: VirtAddr = VirtAddr(0xFFFFA00000000000);
    
    /// Physical memory direct mapping region
    pub const PHYS_MAP_START: VirtAddr = VirtAddr(0xFFFFA00000000000);
    pub const PHYS_MAP_END: VirtAddr = VirtAddr(0xFFFFB00000000000);
    
    /// Page table mapping region (recursive)
    pub const PAGE_TABLE_START: VirtAddr = VirtAddr(0xFFFFFF0000000000);
    pub const PAGE_TABLE_END: VirtAddr = VirtAddr(0xFFFFFF8000000000);
    
    /// User space layout
    
    /// User code start
    pub const USER_START: VirtAddr = VirtAddr(0x400000);
    
    /// User heap start
    pub const USER_HEAP_START: VirtAddr = VirtAddr(0x10000000);
    pub const USER_HEAP_END: VirtAddr = VirtAddr(0x40000000);
    
    /// User stack top (grows down)
    pub const USER_STACK_TOP: VirtAddr = VirtAddr(0x7FFFFFFFFFFF);
    pub const USER_STACK_SIZE: usize = 8 * 1024 * 1024; // 8 MB default
    
    /// Shared library region
    pub const SHARED_LIB_START: VirtAddr = VirtAddr(0x40000000);
    pub const SHARED_LIB_END: VirtAddr = VirtAddr(0x80000000);
}

/// Memory region descriptors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionType {
    /// Available for use
    Available,
    /// Reserved by system
    Reserved,
    /// ACPI tables
    AcpiTables,
    /// Bad memory
    BadMemory,
    /// Kernel image
    Kernel,
    /// Framebuffer
    Framebuffer,
    /// Allocated by allocator
    Allocated,
}

/// Memory region descriptor
#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    pub start: PhysAddr,
    pub size: usize,
    pub region_type: MemoryRegionType,
}

impl MemoryRegion {
    pub fn new(start: PhysAddr, size: usize, region_type: MemoryRegionType) -> Self {
        Self {
            start,
            size,
            region_type,
        }
    }
    
    pub fn end(&self) -> PhysAddr {
        PhysAddr(self.start.0 + self.size)
    }
    
    pub fn contains(&self, addr: PhysAddr) -> bool {
        addr >= self.start && addr < self.end()
    }
    
    pub fn overlaps(&self, other: &MemoryRegion) -> bool {
        self.start < other.end() && other.start < self.end()
    }
    
    /// Get frame range for this region
    pub fn frame_range(&self) -> (usize, usize) {
        let start_frame = self.start.0 / FRAME_SIZE;
        let end_frame = (self.end().0 + FRAME_SIZE - 1) / FRAME_SIZE;
        (start_frame, end_frame)
    }
}

/// Memory layout manager
pub struct MemoryLayout {
    regions: [MemoryRegion; MAX_REGIONS],
    region_count: usize,
}

const MAX_REGIONS: usize = 64;

impl MemoryLayout {
    pub const fn new() -> Self {
        Self {
            regions: [MemoryRegion {
                start: PhysAddr(0),
                size: 0,
                region_type: MemoryRegionType::Available,
            }; MAX_REGIONS],
            region_count: 0,
        }
    }
    
    /// Add a memory region
    pub fn add_region(&mut self, region: MemoryRegion) -> Result<(), &'static str> {
        if self.region_count >= MAX_REGIONS {
            return Err("Too many memory regions");
        }
        
        self.regions[self.region_count] = region;
        self.region_count += 1;
        
        // Keep regions sorted by start address
        self.sort_regions();
        
        Ok(())
    }
    
    /// Get all regions
    pub fn regions(&self) -> &[MemoryRegion] {
        &self.regions[..self.region_count]
    }
    
    /// Find region containing address
    pub fn find_region(&self, addr: PhysAddr) -> Option<&MemoryRegion> {
        self.regions().iter().find(|region| region.contains(addr))
    }
    
    /// Get largest available region
    pub fn largest_available_region(&self) -> Option<&MemoryRegion> {
        self.regions()
            .iter()
            .filter(|region| region.region_type == MemoryRegionType::Available)
            .max_by_key(|region| region.size)
    }
    
    /// Get total available memory
    pub fn total_available_memory(&self) -> usize {
        self.regions()
            .iter()
            .filter(|region| region.region_type == MemoryRegionType::Available)
            .map(|region| region.size)
            .sum()
    }
    
    /// Mark region as allocated
    pub fn mark_allocated(&mut self, start: PhysAddr, size: usize) -> Result<(), &'static str> {
        self.add_region(MemoryRegion::new(
            start,
            size,
            MemoryRegionType::Allocated,
        ))
    }
    
    /// Sort regions by start address
    fn sort_regions(&mut self) {
        let regions = &mut self.regions[..self.region_count];
        regions.sort_by_key(|region| region.start.0);
    }
    
    /// Initialize with standard PC memory layout
    pub fn init_standard_layout(&mut self) -> Result<(), &'static str> {
        // Add standard memory regions
        
        // Low memory (0-640K)
        self.add_region(MemoryRegion::new(
            physical::CONVENTIONAL_START,
            640 * 1024 - physical::CONVENTIONAL_START.0,
            MemoryRegionType::Available,
        ))?;
        
        // VGA buffer
        self.add_region(MemoryRegion::new(
            physical::VGA_BUFFER,
            physical::VGA_BUFFER_SIZE,
            MemoryRegionType::Framebuffer,
        ))?;
        
        // BIOS area
        self.add_region(MemoryRegion::new(
            physical::BIOS_START,
            physical::BIOS_END.0 - physical::BIOS_START.0,
            MemoryRegionType::Reserved,
        ))?;
        
        Ok(())
    }
}

/// Kernel layout information from linker
pub struct KernelLayout {
    pub start: VirtAddr,
    pub end: VirtAddr,
    pub text_start: VirtAddr,
    pub text_end: VirtAddr,
    pub rodata_start: VirtAddr,
    pub rodata_end: VirtAddr,
    pub data_start: VirtAddr,
    pub data_end: VirtAddr,
    pub bss_start: VirtAddr,
    pub bss_end: VirtAddr,
}

impl KernelLayout {
    /// Get kernel layout from linker symbols
    pub fn from_linker() -> Self {
        extern "C" {
            static __kernel_start: u8;
            static __kernel_end: u8;
            static __text_start: u8;
            static __text_end: u8;
            static __rodata_start: u8;
            static __rodata_end: u8;
            static __data_start: u8;
            static __data_end: u8;
            static __bss_start: u8;
            static __bss_end: u8;
        }
        
        unsafe {
            Self {
                start: VirtAddr(&__kernel_start as *const _ as usize),
                end: VirtAddr(&__kernel_end as *const _ as usize),
                text_start: VirtAddr(&__text_start as *const _ as usize),
                text_end: VirtAddr(&__text_end as *const _ as usize),
                rodata_start: VirtAddr(&__rodata_start as *const _ as usize),
                rodata_end: VirtAddr(&__rodata_end as *const _ as usize),
                data_start: VirtAddr(&__data_start as *const _ as usize),
                data_end: VirtAddr(&__data_end as *const _ as usize),
                bss_start: VirtAddr(&__bss_start as *const _ as usize),
                bss_end: VirtAddr(&__bss_end as *const _ as usize),
            }
        }
    }
    
    /// Get kernel size in bytes
    pub fn size(&self) -> usize {
        self.end.0 - self.start.0
    }
    
    /// Get kernel physical address (assuming identity mapping for now)
    pub fn physical_start(&self) -> PhysAddr {
        PhysAddr(self.start.0)
    }
    
    /// Check if virtual address is within kernel
    pub fn contains_virtual(&self, addr: VirtAddr) -> bool {
        addr >= self.start && addr < self.end
    }
}

/// Address space layout randomization (ASLR) helpers
pub mod aslr {
    use super::VirtAddr;
    use crate::kb::XorShift64;
    
    /// Simple ASLR implementation
    pub struct AslrManager {
        rng: XorShift64,
    }
    
    impl AslrManager {
        pub fn new(seed: u64) -> Self {
            Self {
                rng: XorShift64::new(seed),
            }
        }
        
        /// Randomize user space base address
        pub fn randomize_user_base(&mut self) -> VirtAddr {
            let base = super::virtual_memory::USER_START.0;
            let offset = (self.rng.next_u64() % 0x10000000) as usize; // Up to 256 MB offset
            VirtAddr(base + offset)
        }
        
        /// Randomize heap base address
        pub fn randomize_heap_base(&mut self) -> VirtAddr {
            let base = super::virtual_memory::USER_HEAP_START.0;
            let offset = (self.rng.next_u64() % 0x1000000) as usize; // Up to 16 MB offset
            VirtAddr(base + offset)
        }
        
        /// Randomize stack base address
        pub fn randomize_stack_base(&mut self) -> VirtAddr {
            let base = super::virtual_memory::USER_STACK_TOP.0 - super::virtual_memory::USER_STACK_SIZE;
            let offset = (self.rng.next_u64() % 0x1000000) as usize; // Up to 16 MB offset
            VirtAddr(base - offset)
        }
    }
}

/// Utility functions for address calculations
pub mod utils {
    use super::{VirtAddr, PhysAddr, FRAME_SIZE};
    
    /// Align address up to frame boundary
    pub fn align_up(addr: usize) -> usize {
        (addr + FRAME_SIZE - 1) & !(FRAME_SIZE - 1)
    }
    
    /// Align address down to frame boundary
    pub fn align_down(addr: usize) -> usize {
        addr & !(FRAME_SIZE - 1)
    }
    
    /// Convert frames to bytes
    pub fn frames_to_bytes(frames: usize) -> usize {
        frames * FRAME_SIZE
    }
    
    /// Convert bytes to frames (rounded up)
    pub fn bytes_to_frames(bytes: usize) -> usize {
        (bytes + FRAME_SIZE - 1) / FRAME_SIZE
    }
    
    /// Check if address is frame-aligned
    pub fn is_aligned(addr: usize) -> bool {
        addr % FRAME_SIZE == 0
    }
    
    /// Calculate offset within frame
    pub fn frame_offset(addr: usize) -> usize {
        addr % FRAME_SIZE
    }
    
    /// Get frame number from address
    pub fn addr_to_frame(addr: usize) -> usize {
        addr / FRAME_SIZE
    }
    
    /// Get address from frame number
    pub fn frame_to_addr(frame: usize) -> usize {
        frame * FRAME_SIZE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_memory_region() {
        let region = MemoryRegion::new(
            PhysAddr(0x100000),
            0x200000,
            MemoryRegionType::Available,
        );
        
        assert_eq!(region.start, PhysAddr(0x100000));
        assert_eq!(region.end(), PhysAddr(0x300000));
        assert!(region.contains(PhysAddr(0x200000)));
        assert!(!region.contains(PhysAddr(0x400000)));
    }
    
    #[test]
    fn test_address_alignment() {
        assert_eq!(utils::align_up(0x1234), 0x2000);
        assert_eq!(utils::align_down(0x1234), 0x1000);
        assert!(utils::is_aligned(0x2000));
        assert!(!utils::is_aligned(0x1234));
    }
    
    #[test]
    fn test_frame_conversion() {
        assert_eq!(utils::bytes_to_frames(4096), 1);
        assert_eq!(utils::bytes_to_frames(4097), 2);
        assert_eq!(utils::frames_to_bytes(2), 8192);
    }
}
