//! Physical frame allocator implementation
//! 
//! DEPENDENCIES:
//! - Uses core types: PhysFrame, PhysAddr, MemoryError, Result
//! - Implements FrameAllocator trait
//! 
//! INTEGRATION POINTS:
//! - Called by page_allocator.rs for physical memory
//! - Used by heap.rs for kernel heap expansion
//! - Initialized by main.rs during boot

use crate::memory::{PhysFrame, PhysAddr, MemoryError, Result, FRAME_SIZE};
use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;

const FRAMES_PER_BYTE: usize = 8;

/// Frame allocator trait for abstracting allocation strategies
pub trait FrameAllocator {
    /// Allocate a single physical frame
    fn allocate_frame(&mut self) -> Option<PhysFrame>;
    
    /// Deallocate a physical frame
    fn deallocate_frame(&mut self, frame: PhysFrame);
    
    /// Get number of free frames
    fn free_frame_count(&self) -> usize;
    
    /// Mark a range of frames as used
    fn mark_frames_used(&mut self, start: PhysFrame, count: usize) -> Result<()>;
    
    /// Mark a range of frames as free
    fn mark_frames_free(&mut self, start: PhysFrame, count: usize) -> Result<()>;
}

/// Bitmap-based physical frame allocator
pub struct BitmapFrameAllocator {
    bitmap: &'static mut [u8],
    start_frame: PhysFrame,
    frame_count: usize,
    free_frames: AtomicUsize,
    lock: Mutex<()>,
}

impl BitmapFrameAllocator {
    /// Create new allocator
    /// 
    /// SAFETY: bitmap must point to valid memory that won't be used elsewhere
    pub unsafe fn new(
        bitmap_addr: usize,
        bitmap_size: usize,
        start_frame: PhysFrame,
        frame_count: usize,
    ) -> Result<Self> {
        if bitmap_addr == 0 {
            return Err(MemoryError::InvalidAddress);
        }
        
        let needed_bytes = (frame_count + FRAMES_PER_BYTE - 1) / FRAMES_PER_BYTE;
        if bitmap_size < needed_bytes {
            return Err(MemoryError::OutOfMemory);
        }
        
        // Convert bitmap address to slice
        let bitmap = core::slice::from_raw_parts_mut(
            bitmap_addr as *mut u8,
            needed_bytes,
        );
        
        // Zero initialize bitmap (all frames free)
        ptr::write_bytes(bitmap.as_mut_ptr(), 0, needed_bytes);
        
        Ok(Self {
            bitmap,
            start_frame,
            frame_count,
            free_frames: AtomicUsize::new(frame_count),
            lock: Mutex::new(()),
        })
    }
    
    /// Calculate bit index for a frame
    fn frame_to_bit_index(&self, frame: PhysFrame) -> Option<usize> {
        if frame.0 < self.start_frame.0 {
            return None;
        }
        
        let frame_index = (frame.0 - self.start_frame.0) / FRAME_SIZE;
        if frame_index >= self.frame_count {
            None
        } else {
            Some(frame_index)
        }
    }
    
    /// Convert bit index back to frame
    fn bit_index_to_frame(&self, bit_index: usize) -> PhysFrame {
        PhysFrame(self.start_frame.0 + bit_index * FRAME_SIZE)
    }
    
    /// Set frame as used (set bit)
    fn set_frame_used(&mut self, frame: PhysFrame) -> Result<()> {
        let bit_index = self.frame_to_bit_index(frame)
            .ok_or(MemoryError::InvalidAddress)?;
        
        let byte_index = bit_index / FRAMES_PER_BYTE;
        let bit_offset = bit_index % FRAMES_PER_BYTE;
        let mask = 1u8 << bit_offset;
        
        let _guard = self.lock.lock();
        
        // Check if already used
        if (self.bitmap[byte_index] & mask) != 0 {
            return Err(MemoryError::AlreadyMapped);
        }
        
        // Set bit
        self.bitmap[byte_index] |= mask;
        self.free_frames.fetch_sub(1, Ordering::SeqCst);
        
        Ok(())
    }
    
    /// Set frame as free (clear bit)
    fn set_frame_free(&mut self, frame: PhysFrame) -> Result<()> {
        let bit_index = self.frame_to_bit_index(frame)
            .ok_or(MemoryError::InvalidAddress)?;
        
        let byte_index = bit_index / FRAMES_PER_BYTE;
        let bit_offset = bit_index % FRAMES_PER_BYTE;
        let mask = 1u8 << bit_offset;
        
        let _guard = self.lock.lock();
        
        // Check if already free
        if (self.bitmap[byte_index] & mask) == 0 {
            return Err(MemoryError::NotMapped);
        }
        
        // Clear bit
        self.bitmap[byte_index] &= !mask;
        self.free_frames.fetch_add(1, Ordering::SeqCst);
        
        Ok(())
    }
    
    /// Check if frame is used
    pub fn is_frame_used(&self, frame: PhysFrame) -> bool {
        if let Some(bit_index) = self.frame_to_bit_index(frame) {
            let byte_index = bit_index / FRAMES_PER_BYTE;
            let bit_offset = bit_index % FRAMES_PER_BYTE;
            let mask = 1u8 << bit_offset;
            
            let _guard = self.lock.lock();
            (self.bitmap[byte_index] & mask) != 0
        } else {
            false
        }
    }
    
    /// Find first free frame using linear scan
    fn find_free_frame(&self) -> Option<usize> {
        let _guard = self.lock.lock();
        
        let bytes_to_scan = (self.frame_count + FRAMES_PER_BYTE - 1) / FRAMES_PER_BYTE;
        
        for byte_index in 0..bytes_to_scan {
            let byte_val = self.bitmap[byte_index];
            if byte_val != 0xFF {
                // Found a byte with at least one free bit
                for bit_offset in 0..FRAMES_PER_BYTE {
                    let bit_index = byte_index * FRAMES_PER_BYTE + bit_offset;
                    if bit_index >= self.frame_count {
                        break;
                    }
                    
                    let mask = 1u8 << bit_offset;
                    if (byte_val & mask) == 0 {
                        return Some(bit_index);
                    }
                }
            }
        }
        
        None
    }
}

impl FrameAllocator for BitmapFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        // Check if any frames available
        if self.free_frames.load(Ordering::SeqCst) == 0 {
            return None;
        }
        
        // Find first free frame
        let bit_index = self.find_free_frame()?;
        
        // Mark as used
        let frame = self.bit_index_to_frame(bit_index);
        self.set_frame_used(frame).ok()?;
        
        Some(frame)
    }
    
    fn deallocate_frame(&mut self, frame: PhysFrame) {
        let _ = self.set_frame_free(frame);
    }
    
    fn free_frame_count(&self) -> usize {
        self.free_frames.load(Ordering::SeqCst)
    }
    
    fn mark_frames_used(&mut self, start: PhysFrame, count: usize) -> Result<()> {
        for i in 0..count {
            let frame = PhysFrame(start.0 + i * FRAME_SIZE);
            self.set_frame_used(frame)?;
        }
        Ok(())
    }
    
    fn mark_frames_free(&mut self, start: PhysFrame, count: usize) -> Result<()> {
        for i in 0..count {
            let frame = PhysFrame(start.0 + i * FRAME_SIZE);
            self.set_frame_free(frame)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_allocate_single_frame() {
        let mut bitmap = [0u8; 1024];
        let mut allocator = unsafe {
            BitmapFrameAllocator::new(
                bitmap.as_mut_ptr() as usize,
                bitmap.len(),
                PhysFrame(0x100000),
                8192,
            ).unwrap()
        };
        
        let frame = allocator.allocate_frame().unwrap();
        assert_eq!(frame.0, 0x100000);
        assert_eq!(allocator.free_frame_count(), 8191);
    }
    
    #[test]
    fn test_deallocate_frame() {
        let mut bitmap = [0u8; 1024];
        let mut allocator = unsafe {
            BitmapFrameAllocator::new(
                bitmap.as_mut_ptr() as usize,
                bitmap.len(),
                PhysFrame(0x100000),
                8192,
            ).unwrap()
        };
        
        let frame = allocator.allocate_frame().unwrap();
        allocator.deallocate_frame(frame);
        assert_eq!(allocator.free_frame_count(), 8192);
    }
    
    #[test]
    fn test_allocate_all_frames() {
        let mut bitmap = [0u8; 1];
        let mut allocator = unsafe {
            BitmapFrameAllocator::new(
                bitmap.as_mut_ptr() as usize,
                bitmap.len(),
                PhysFrame(0x100000),
                8,
            ).unwrap()
        };
        
        // Allocate all frames
        for _ in 0..8 {
            assert!(allocator.allocate_frame().is_some());
        }
        
        // Should fail on 9th allocation
        assert!(allocator.allocate_frame().is_none());
        assert_eq!(allocator.free_frame_count(), 0);
    }
}
