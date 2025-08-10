#![no_std]

extern crate alloc;
use alloc::{boxed::Box, string::String, vec::Vec, collections::VecDeque};
use core::fmt;

// Re-export main components
pub mod tui;

pub use tui::{
    menu::{IronVeilMenu, MenuItem, MenuAction},
    status_panel::StatusPanel,
    rust_theme::{Theme, RUST_THEME, colors},
};

// Core types and traits used across UI system
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysAddr(pub usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtAddr(pub usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PhysFrame(pub usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TaskId(pub u64);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MacAddr(pub [u8; 6]);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Ipv4Addr(pub [u8; 4]);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IronVeilError {
    OutOfMemory,
    InvalidAddress,
    TaskNotFound,
    NetworkError,
    PrivacyViolation,
    HardwareError,
    UIError,
    InvalidInput,
}

pub type Result<T> = core::result::Result<T, IronVeilError>;

// UI Component trait
pub trait UIComponent {
    fn render(&self, area: Rect, buf: &mut Buffer) -> Result<()>;
    fn handle_input(&mut self, event: InputEvent) -> Result<bool>; // true if handled
    fn update(&mut self) -> Result<()>;
}

// Menu trait
pub trait Menu: UIComponent {
    fn add_item(&mut self, label: &str, action: MenuAction);
    fn get_selected(&self) -> Option<usize>;
    fn select_next(&mut self);
    fn select_previous(&mut self);
    fn execute_selected(&mut self) -> Result<()>;
}

// Input event types
#[derive(Debug, Clone)]
pub enum InputEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
}

#[derive(Debug, Clone)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

#[derive(Debug, Clone)]
pub enum KeyCode {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Escape,
    Tab,
    Backspace,
    Delete,
    Char(char),
    F(u8),
}

#[derive(Debug, Clone)]
pub struct KeyModifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

#[derive(Debug, Clone)]
pub struct MouseEvent {
    pub x: u16,
    pub y: u16,
    pub button: MouseButton,
}

#[derive(Debug, Clone)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

// Basic geometry types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self { x, y, width, height }
    }
}

// Basic buffer for rendering
#[derive(Debug)]
pub struct Buffer {
    pub area: Rect,
    pub content: Vec<Cell>,
}

#[derive(Debug, Clone)]
pub struct Cell {
    pub symbol: char,
    pub fg: Color,
    pub bg: Color,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

// System information types
#[derive(Debug, Clone, Default)]
pub struct SystemInfo {
    pub uptime: u64,
    pub memory_usage: MemoryStats,
    pub network_interfaces: Vec<NetworkInterface>,
    pub privacy_status: PrivacyStatus,
}

#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    pub total: usize,
    pub used: usize,
    pub free: usize,
}

#[derive(Debug, Clone)]
pub struct NetworkInterface {
    pub name: String,
    pub mac_address: MacAddr,
    pub ip_address: Option<Ipv4Addr>,
    pub status: InterfaceStatus,
}

#[derive(Debug, Clone)]
pub enum InterfaceStatus {
    Up,
    Down,
    Unknown,
}

#[derive(Debug, Clone, Default)]
pub struct PrivacyStatus {
    pub tor_status: TorStatus,
    pub mac_randomized: bool,
    pub vpn_connected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TorStatus {
    Stopped,
    Starting,
    Running,
    Error,
}

impl Default for TorStatus {
    fn default() -> Self {
        TorStatus::Stopped
    }
}

// Main UI entry point
pub fn main_ui_loop() -> ! {
    // This would be called as a task from the kernel
    // Initialize TUI system and run main loop
    let mut app = tui::TUIApplication::new();
    app.run();
}
