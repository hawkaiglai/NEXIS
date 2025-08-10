//! Keyboard driver for Nexis kernel using PS/2 interface
//! 
//! DEPENDENCIES:
//! - Uses pc-keyboard crate for scancode translation
//! - Uses spin::Mutex for thread-safe queue access
//! - Uses x86_64::instructions::hlt for power-efficient waiting
//! 
//! INTEGRATION POINTS:
//! - Called by keyboard interrupt handler (IRQ1)
//! - Used by shell for user input
//! - Integrates with VGA driver for echo output

use spin::Mutex;
use lazy_static::lazy_static;
use pc_keyboard::{Keyboard, layouts, ScancodeSet1, DecodedKey, HandleControl};
use x86_64::instructions::hlt;
use core::str;
use super::{Driver, InputDriver};

// Simple PRNG for demo purposes
pub struct XorShift64 { 
    state: u64 
}

impl XorShift64 {
    pub fn new(seed: u64) -> Self { 
        Self { state: if seed == 0 { 1 } else { seed } } 
    }
    
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
    
    pub fn next_u8(&mut self) -> u8 { 
        (self.next_u64() & 0xFF) as u8 
    }
    
    pub fn next_range_u8(&mut self, low: u8, high: u8) -> u8 {
        if high <= low { return low; }
        let range = (high - low + 1) as u64;
        let r = self.next_u64() % range;
        low + r as u8
    }
}

const BUF_SIZE: usize = 1024;
const LINE_BUF_SIZE: usize = 256;

lazy_static! {
    static ref SCANCODE_QUEUE: Mutex<ScancodeQueue> = Mutex::new(ScancodeQueue::new());
    static ref KEYBOARD_STATE: Mutex<KeyboardState> = Mutex::new(KeyboardState::new());
}

struct ScancodeQueue {
    buf: [u8; BUF_SIZE],
    head: usize,
    tail: usize,
    count: usize,
}

impl ScancodeQueue {
    const fn new() -> Self {
        Self { 
            buf: [0; BUF_SIZE], 
            head: 0, 
            tail: 0,
            count: 0,
        }
    }
    
    fn push(&mut self, sc: u8) -> bool {
        if self.count >= BUF_SIZE {
            return false; // Buffer full
        }
        
        self.buf[self.head] = sc;
        self.head = (self.head + 1) % BUF_SIZE;
        self.count += 1;
        true
    }
    
    fn pop(&mut self) -> Option<u8> {
        if self.count == 0 {
            return None;
        }
        
        let sc = self.buf[self.tail];
        self.tail = (self.tail + 1) % BUF_SIZE;
        self.count -= 1;
        Some(sc)
    }
    
    fn len(&self) -> usize {
        self.count
    }
    
    fn is_empty(&self) -> bool {
        self.count == 0
    }
}

struct KeyboardState {
    keyboard: Keyboard<layouts::Us104Key, ScancodeSet1>,
    line_buffer: [u8; LINE_BUF_SIZE],
    line_length: usize,
    initialized: bool,
}

impl KeyboardState {
    fn new() -> Self {
        Self {
            keyboard: Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore),
            line_buffer: [0; LINE_BUF_SIZE],
            line_length: 0,
            initialized: false,
        }
    }
}

pub struct Kb;

impl Kb {
    pub fn init() -> Result<(), &'static str> {
        let mut state = KEYBOARD_STATE.lock();
        state.initialized = true;
        Ok(())
    }
    
    /// Called by interrupt handler to queue scancodes
    pub fn push_scancode(sc: u8) {
        SCANCODE_QUEUE.lock().push(sc);
    }
    
    /// Blocking read of a single scancode; sleeps with `hlt` if none available
    fn read_scancode_blocking() -> u8 {
        loop {
            if let Some(sc) = SCANCODE_QUEUE.lock().pop() {
                return sc;
            }
            hlt(); // Wait for next interrupt
        }
    }
    
    /// Non-blocking read of scancode
    pub fn read_scancode() -> Option<u8> {
        SCANCODE_QUEUE.lock().pop()
    }
    
    /// Get number of queued scancodes
    pub fn queue_len() -> usize {
        SCANCODE_QUEUE.lock().len()
    }
    
    /// Read a complete line from keyboard input (blocking)
    /// Returns a static string slice for simplicity in no_std environment
    pub fn read_line_irq() -> &'static str {
        static mut LINE_BUF: [u8; LINE_BUF_SIZE] = [0; LINE_BUF_SIZE];
        let mut len = 0usize;
        
        let mut state = KEYBOARD_STATE.lock();
        
        loop {
            drop(state); // Release lock while waiting for input
            let sc = Self::read_scancode_blocking();
            state = KEYBOARD_STATE.lock();
            
            if let Ok(Some(event)) = state.keyboard.add_byte(sc) {
                if let Some(key) = state.keyboard.process_keyevent(event) {
                    match key {
                        DecodedKey::Unicode(ch) => {
                            match ch {
                                '\r' | '\n' => {
                                    // Echo newline to VGA/serial
                                    crate::drivers::vga::VGA_WRITER.lock().put_char('\n');
                                    crate::drivers::vga::sprint!("\n");
                                    
                                    // Null-terminate and return
                                    unsafe {
                                        LINE_BUF[len] = 0;
                                        let s = core::str::from_utf8_unchecked(&LINE_BUF[..len]);
                                        return s;
                                    }
                                }
                                '\x08' => { // Backspace
                                    if len > 0 {
                                        len -= 1;
                                        // Echo backspace sequence to VGA/serial
                                        crate::drivers::vga::VGA_WRITER.lock().put_char('\x08');
                                        crate::drivers::vga::sprint!("\x08 \x08");
                                    }
                                }
                                ch if ch >= ' ' && ch <= '~' => { // Printable ASCII
                                    if len < LINE_BUF_SIZE - 1 {
                                        unsafe { LINE_BUF[len] = ch as u8; }
                                        len += 1;
                                        
                                        // Echo character to VGA/serial
                                        crate::drivers::vga::VGA_WRITER.lock().put_char(ch);
                                        crate::drivers::vga::sprint!("{}", ch);
                                    }
                                }
                                _ => {
                                    // Ignore other control characters
                                }
                            }
                        }
                        DecodedKey::RawKey(raw_key) => {
                            // Handle special keys that might come as raw keys
                            match raw_key {
                                pc_keyboard::KeyCode::Enter => {
                                    crate::drivers::vga::VGA_WRITER.lock().put_char('\n');
                                    crate::drivers::vga::sprint!("\n");
                                    
                                    unsafe {
                                        LINE_BUF[len] = 0;
                                        let s = core::str::from_utf8_unchecked(&LINE_BUF[..len]);
                                        return s;
                                    }
                                }
                                pc_keyboard::KeyCode::Backspace => {
                                    if len > 0 {
                                        len -= 1;
                                        crate::drivers::vga::VGA_WRITER.lock().put_char('\x08');
                                        crate::drivers::vga::sprint!("\x08 \x08");
                                    }
                                }
                                _ => {
                                    // Ignore other raw keys
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// Read a single character (blocking)
    pub fn read_char() -> char {
        let mut state = KEYBOARD_STATE.lock();
        
        loop {
            drop(state);
            let sc = Self::read_scancode_blocking();
            state = KEYBOARD_STATE.lock();
            
            if let Ok(Some(event)) = state.keyboard.add_byte(sc) {
                if let Some(key) = state.keyboard.process_keyevent(event) {
                    if let DecodedKey::Unicode(ch) = key {
                        return ch;
                    }
                }
            }
        }
    }
    
    /// Check if keyboard input is available
    pub fn has_input() -> bool {
        !SCANCODE_QUEUE.lock().is_empty()
    }
}

impl Driver for Kb {
    type Error = &'static str;
    
    fn name(&self) -> &'static str {
        "PS/2 Keyboard Driver"
    }
    
    fn init(&mut self) -> Result<(), Self::Error> {
        Self::init()
    }
    
    fn cleanup(&mut self) -> Result<(), Self::Error> {
        // Clear the scancode queue
        let mut queue = SCANCODE_QUEUE.lock();
        queue.head = 0;
        queue.tail = 0;
        queue.count = 0;
        Ok(())
    }
    
    fn is_initialized(&self) -> bool {
        KEYBOARD_STATE.lock().initialized
    }
}

impl InputDriver for Kb {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        let mut bytes_read = 0;
        
        while bytes_read < buffer.len() {
            if let Some(sc) = Self::read_scancode() {
                buffer[bytes_read] = sc;
                bytes_read += 1;
            } else {
                break;
            }
        }
        
        Ok(bytes_read)
    }
    
    fn has_input(&self) -> bool {
        Self::has_input()
    }
}

// Driver initialization function
pub fn init() -> Result<(), &'static str> {
    Kb::init()
}
