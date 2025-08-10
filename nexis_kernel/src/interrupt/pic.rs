//! Programmable Interrupt Controller (PIC) management
//! 
//! DEPENDENCIES:
//! - Uses x86_64 crate for port I/O operations
//! 
//! INTEGRATION POINTS:
//! - Called by interrupt::init() for PIC initialization
//! - Used by interrupt handlers to send EOI (End of Interrupt)
//! - Used by drivers to enable/disable specific IRQ lines

use x86_64::instructions::port::Port;
use spin::Mutex;

// PIC ports
const PIC1_COMMAND: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_COMMAND: u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;

// PIC commands
const PIC_EOI: u8 = 0x20;
const ICW1_ICW4: u8 = 0x01;
const ICW1_SINGLE: u8 = 0x02;
const ICW1_INTERVAL4: u8 = 0x04;
const ICW1_LEVEL: u8 = 0x08;
const ICW1_INIT: u8 = 0x10;
const ICW4_8086: u8 = 0x01;
const ICW4_AUTO: u8 = 0x02;
const ICW4_BUF_SLAVE: u8 = 0x08;
const ICW4_BUF_MASTER: u8 = 0x0C;
const ICW4_SFNM: u8 = 0x10;

// PIC offsets after remapping
const PIC1_OFFSET: u8 = 32;
const PIC2_OFFSET: u8 = 40;

static PIC_LOCK: Mutex<()> = Mutex::new(());

/// PIC management structure
pub struct Pics {
    pic1_command: Port<u8>,
    pic1_data: Port<u8>,
    pic2_command: Port<u8>,
    pic2_data: Port<u8>,
    pic1_offset: u8,
    pic2_offset: u8,
}

impl Pics {
    pub const fn new(pic1_offset: u8, pic2_offset: u8) -> Self {
        Self {
            pic1_command: Port::new(PIC1_COMMAND),
            pic1_data: Port::new(PIC1_DATA),
            pic2_command: Port::new(PIC2_COMMAND),
            pic2_data: Port::new(PIC2_DATA),
            pic1_offset,
            pic2_offset,
        }
    }

    /// Initialize and remap the PICs
    pub unsafe fn remap(&mut self) {
        let _lock = PIC_LOCK.lock();

        // Save current masks
        let pic1_mask = self.pic1_data.read();
        let pic2_mask = self.pic2_data.read();

        // Start initialization sequence
        self.pic1_command.write(ICW1_INIT | ICW1_ICW4);
        io_wait();
        self.pic2_command.write(ICW1_INIT | ICW1_ICW4);
        io_wait();

        // Set vector offsets
        self.pic1_data.write(self.pic1_offset);
        io_wait();
        self.pic2_data.write(self.pic2_offset);
        io_wait();

        // Configure cascade
        self.pic1_data.write(4); // PIC2 is at IRQ2
        io_wait();
        self.pic2_data.write(2); // Cascade identity
        io_wait();

        // Set mode
        self.pic1_data.write(ICW4_8086);
        io_wait();
        self.pic2_data.write(ICW4_8086);
        io_wait();

        // Restore masks (initially disable all interrupts)
        self.pic1_data.write(0xFF); // Mask all PIC1 interrupts
        self.pic2_data.write(0xFF); // Mask all PIC2 interrupts
    }

    /// Send End of Interrupt signal
    pub unsafe fn send_eoi(&mut self, irq: u8) {
        let _lock = PIC_LOCK.lock();
        
        if irq >= 8 {
            // IRQ came from PIC2, send EOI to both
            self.pic2_command.write(PIC_EOI);
        }
        // Always send EOI to PIC1
        self.pic1_command.write(PIC_EOI);
    }

    /// Enable a specific IRQ line
    pub unsafe fn enable_irq(&mut self, irq: u8) {
        let _lock = PIC_LOCK.lock();
        
        if irq < 8 {
            // PIC1 IRQ
            let mut mask = self.pic1_data.read();
            mask &= !(1 << irq);
            self.pic1_data.write(mask);
        } else if irq < 16 {
            // PIC2 IRQ
            let mut mask = self.pic2_data.read();
            mask &= !(1 << (irq - 8));
            self.pic2_data.write(mask);
            
            // Also enable cascade on PIC1
            let mut pic1_mask = self.pic1_data.read();
            pic1_mask &= !(1 << 2); // Enable IRQ2 (cascade)
            self.pic1_data.write(pic1_mask);
        }
    }

    /// Disable a specific IRQ line
    pub unsafe fn disable_irq(&mut self, irq: u8) {
        let _lock = PIC_LOCK.lock();
        
        if irq < 8 {
            // PIC1 IRQ
            let mut mask = self.pic1_data.read();
            mask |= 1 << irq;
            self.pic1_data.write(mask);
        } else if irq < 16 {
            // PIC2 IRQ
            let mut mask = self.pic2_data.read();
            mask |= 1 << (irq - 8);
            self.pic2_data.write(mask);
        }
    }

    /// Get current IRQ mask for debugging
    pub unsafe fn get_mask(&mut self) -> (u8, u8) {
        let _lock = PIC_LOCK.lock();
        (self.pic1_data.read(), self.pic2_data.read())
    }
}

static mut PICS: Pics = Pics::new(PIC1_OFFSET, PIC2_OFFSET);

/// Remap the PICs to avoid conflicts with CPU exceptions
pub fn remap_pic() {
    unsafe {
        crate::vga::vprintln!("Remapping PIC...");
        PICS.remap();
        crate::vga::vprintln!("PIC remapped successfully");
    }
}

/// Send End of Interrupt for the given IRQ
pub fn send_eoi(irq: u8) {
    unsafe {
        PICS.send_eoi(irq);
    }
}

/// Enable a specific IRQ line
pub fn enable_irq(irq: u8) {
    unsafe {
        PICS.enable_irq(irq);
        crate::vga::vprintln!("Enabled IRQ {}", irq);
    }
}

/// Disable a specific IRQ line
pub fn disable_irq(irq: u8) {
    unsafe {
        PICS.disable_irq(irq);
        crate::vga::vprintln!("Disabled IRQ {}", irq);
    }
}

/// Get PIC mask status for debugging
pub fn get_pic_masks() -> (u8, u8) {
    unsafe {
        PICS.get_mask()
    }
}

/// Small delay for PIC operations
unsafe fn io_wait() {
    // Write to unused port to create small delay
    let mut port = Port::new(0x80);
    port.write(0u8);
}

/// Initialize PIC with default configuration
pub fn init_pic() {
    remap_pic();
    
    // Initially, all IRQs are disabled
    // Drivers will enable their specific IRQs as needed
    crate::vga::vprintln!("PIC initialization complete");
}
