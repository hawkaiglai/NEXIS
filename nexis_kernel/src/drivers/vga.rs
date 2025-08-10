//! VGA text mode driver for Nexis kernel
//! 
//! DEPENDENCIES:
//! - Uses core for no_std compatibility
//! - Uses spin::Mutex for thread-safe access
//! - Uses lazy_static for global instances
//! 
//! INTEGRATION POINTS:
//! - Used by kernel for early boot messages
//! - Used by shell for interactive output
//! - Integrates with serial driver for dual output
//! - Called by panic handler for error display

use lazy_static::lazy_static;
use spin::Mutex;
use uart_16550::SerialPort;
use core::fmt::Write;
use x86_64::instructions::port::Port;
use super::{Driver, OutputDriver};

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;
const VGA_BUFFER_ADDR: usize = 0xb8000;

// VGA color constants
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ColorCode(u8);

impl ColorCode {
    pub fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

lazy_static! {
    pub static ref VGA_WRITER: Mutex<VgaWriter> = Mutex::new(VgaWriter::new());
    pub static ref SERIAL1: Mutex<SerialPort> = {
        let mut sp = unsafe { SerialPort::new(0x3F8) };
        sp.init();
        Mutex::new(sp)
    };
}

pub struct VgaWriter {
    column: usize,
    row: usize,
    color: ColorCode,
    buffer: *mut u8,
    initialized: bool,
}

impl VgaWriter {
    pub const fn new() -> Self {
        Self {
            column: 0,
            row: 0,
            color: ColorCode::new(Color::White, Color::Black),
            buffer: VGA_BUFFER_ADDR as *mut u8,
            initialized: false,
        }
    }

    pub fn init(&mut self) -> Result<(), &'static str> {
        // Test VGA buffer accessibility
        unsafe {
            let test_byte = core::ptr::read_volatile(self.buffer);
            core::ptr::write_volatile(self.buffer, test_byte);
        }
        
        self.initialized = true;
        self.clear_screen();
        Ok(())
    }

    pub fn set_color(&mut self, color: ColorCode) {
        self.color = color;
    }

    pub fn put_char(&mut self, c: char) {
        match c {
            '\n' => { 
                self.new_line(); 
                return; 
            }
            '\r' => { 
                self.column = 0; 
                return; 
            }
            '\t' => {
                // Tab to next 8-character boundary
                let next_tab = (self.column + 8) & !7;
                while self.column < next_tab && self.column < BUFFER_WIDTH {
                    self.put_char(' ');
                }
                return;
            }
            '\x08' => { // Backspace
                if self.column > 0 {
                    self.column -= 1;
                    let offset = (self.row * BUFFER_WIDTH + self.column) * 2;
                    unsafe {
                        core::ptr::write_volatile(self.buffer.add(offset), b' ');
                        core::ptr::write_volatile(self.buffer.add(offset + 1), self.color.0);
                    }
                }
                return;
            }
            _ => {}
        }

        if self.column >= BUFFER_WIDTH { 
            self.new_line(); 
        }

        let offset = (self.row * BUFFER_WIDTH + self.column) * 2;
        unsafe {
            core::ptr::write_volatile(self.buffer.add(offset), c as u8);
            core::ptr::write_volatile(self.buffer.add(offset + 1), self.color.0);
        }
        self.column += 1;
    }

    pub fn write_str(&mut self, s: &str) {
        for c in s.chars() { 
            self.put_char(c); 
        }
    }

    pub fn new_line(&mut self) {
        self.column = 0;
        if self.row + 1 < BUFFER_HEIGHT { 
            self.row += 1; 
        } else {
            self.scroll_up();
        }
    }

    fn scroll_up(&mut self) {
        // Move all lines up by one
        for r in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let src = ((r * BUFFER_WIDTH) + col) * 2;
                let dst = (((r - 1) * BUFFER_WIDTH) + col) * 2;
                unsafe {
                    let ch = core::ptr::read_volatile(self.buffer.add(src));
                    let color = core::ptr::read_volatile(self.buffer.add(src + 1));
                    core::ptr::write_volatile(self.buffer.add(dst), ch);
                    core::ptr::write_volatile(self.buffer.add(dst + 1), color);
                }
            }
        }
        
        // Clear last line
        self.clear_line(BUFFER_HEIGHT - 1);
    }

    fn clear_line(&mut self, line: usize) {
        let start = line * BUFFER_WIDTH * 2;
        for col in 0..BUFFER_WIDTH {
            unsafe {
                core::ptr::write_volatile(self.buffer.add(start + col * 2), b' ');
                core::ptr::write_volatile(self.buffer.add(start + col * 2 + 1), self.color.0);
            }
        }
    }

    pub fn clear_screen(&mut self) {
        for r in 0..BUFFER_HEIGHT {
            for c in 0..BUFFER_WIDTH {
                let offset = (r * BUFFER_WIDTH + c) * 2;
                unsafe {
                    core::ptr::write_volatile(self.buffer.add(offset), b' ');
                    core::ptr::write_volatile(self.buffer.add(offset + 1), self.color.0);
                }
            }
        }
        self.row = 0; 
        self.column = 0;
    }

    pub fn get_cursor_position(&self) -> (usize, usize) {
        (self.column, self.row)
    }

    pub fn set_cursor_position(&mut self, col: usize, row: usize) {
        if col < BUFFER_WIDTH && row < BUFFER_HEIGHT {
            self.column = col;
            self.row = row;
        }
    }
}

impl core::fmt::Write for VgaWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_str(s);
        Ok(())
    }
}

impl Driver for VgaWriter {
    type Error = &'static str;
    
    fn name(&self) -> &'static str {
        "VGA Text Mode Driver"
    }
    
    fn init(&mut self) -> Result<(), Self::Error> {
        self.init()
    }
    
    fn cleanup(&mut self) -> Result<(), Self::Error> {
        self.clear_screen();
        Ok(())
    }
    
    fn is_initialized(&self) -> bool {
        self.initialized
    }
}

impl OutputDriver for VgaWriter {
    fn write(&mut self, buffer: &[u8]) -> Result<usize, Self::Error> {
        let s = core::str::from_utf8(buffer).map_err(|_| "Invalid UTF-8")?;
        self.write_str(s);
        Ok(buffer.len())
    }
    
    fn flush(&mut self) -> Result<(), Self::Error> {
        // VGA writes are immediate, no buffering
        Ok(())
    }
}

// Serial port functions
pub fn serial_print(args: core::fmt::Arguments) {
    let mut s = SERIAL1.lock();
    let _ = s.write_fmt(args);
}

pub fn serial_println(args: core::fmt::Arguments) {
    serial_print(args);
    serial_print(format_args!("\n"));
}

// Combined VGA + Serial output functions
pub fn vprintln_impl(args: core::fmt::Arguments) {
    {
        let mut v = VGA_WRITER.lock();
        let _ = v.write_fmt(args);
        v.put_char('\n');
    }
    serial_print(args);
    serial_print(format_args!("\n"));
}

// Macros for easy printing
macro_rules! sprintln {
    ($($arg:tt)*) => (crate::drivers::vga::serial_print(format_args!($($arg)*)));
}

macro_rules! sprint {
    ($($arg:tt)*) => (crate::drivers::vga::serial_print(format_args!($($arg)*)));
}

macro_rules! vprintln {
    ($($arg:tt)*) => (crate::drivers::vga::vprintln_impl(format_args!($($arg)*)));
}

macro_rules! vprint {
    ($($arg:tt)*) => ({
        let mut v = crate::drivers::vga::VGA_WRITER.lock();
        let _ = v.write_fmt(format_args!($($arg)*));
        let _ = crate::drivers::vga::serial_print(format_args!($($arg)*));
    });
}

pub(crate) use sprintln;
pub(crate) use sprint;
pub(crate) use vprintln;
pub(crate) use vprint;

// Driver initialization function
pub fn init() -> Result<(), &'static str> {
    VGA_WRITER.lock().init()
}
