//! TUI framework integration
//! 
//! DEPENDENCIES:
//! - Uses tui crate for rendering framework
//! - Uses crossterm for terminal control
//! - Implements UIComponent trait
//! 
//! INTEGRATION POINTS:
//! - Called by main UI entry point
//! - Integrates with menu system and status panels
//! - Handles input events and rendering coordination

use alloc::{boxed::Box, vec::Vec};
use crate::{UIComponent, InputEvent, KeyEvent, KeyCode, KeyModifiers, Result, IronVeilError, Rect, Buffer};

pub mod menu;
pub mod status_panel;
pub mod rust_theme;

use menu::IronVeilMenu;
use status_panel::StatusPanel;
use rust_theme::RUST_THEME;

pub struct TUIApplication {
    current_screen: Screen,
    main_menu: IronVeilMenu,
    privacy_menu: IronVeilMenu,
    network_menu: IronVeilMenu,
    status_panel: StatusPanel,
    should_quit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Main,
    Privacy,
    Network,
    SystemInfo,
}

impl TUIApplication {
    pub fn new() -> Self {
        Self {
            current_screen: Screen::Main,
            main_menu: IronVeilMenu::main_menu(),
            privacy_menu: IronVeilMenu::privacy_menu(),
            network_menu: IronVeilMenu::network_menu(),
            status_panel: StatusPanel::new(),
            should_quit: false,
        }
    }

    pub fn run(&mut self) -> ! {
        // Initialize terminal
        self.init_terminal();

        loop {
            // Update components
            let _ = self.update();

            // Render frame
            let _ = self.render();

            // Handle input
            if let Ok(event) = self.get_input() {
                let _ = self.handle_input(event);
            }

            // Check if should quit
            if self.should_quit {
                break;
            }

            // Small delay to prevent busy loop
            self.sleep_ms(16); // ~60 FPS
        }

        self.cleanup_terminal();
        
        // Return to kernel scheduler
        loop {
            core::hint::spin_loop();
        }
    }

    fn init_terminal(&mut self) {
        // Initialize terminal for TUI mode
        // In a real implementation, this would set up crossterm
        self.clear_screen();
        self.show_cursor(false);
    }

    fn cleanup_terminal(&mut self) {
        self.clear_screen();
        self.show_cursor(true);
    }

    fn update(&mut self) -> Result<()> {
        self.status_panel.update()?;
        
        match self.current_screen {
            Screen::Main => self.main_menu.update(),
            Screen::Privacy => self.privacy_menu.update(),
            Screen::Network => self.network_menu.update(),
            Screen::SystemInfo => self.status_panel.update(),
        }
    }

    fn render(&mut self) -> Result<()> {
        // Create main buffer
        let area = Rect::new(0, 0, 80, 25); // Standard VGA text mode
        let mut buffer = Buffer {
            area,
            content: Vec::new(),
        };

        // Render header
        self.render_header(&area, &mut buffer)?;

        // Split screen for menu and status
        let menu_area = Rect::new(0, 3, 60, 20);
        let status_area = Rect::new(60, 3, 20, 20);

        // Render current screen
        match self.current_screen {
            Screen::Main => self.main_menu.render(menu_area, &mut buffer)?,
            Screen::Privacy => self.privacy_menu.render(menu_area, &mut buffer)?,
            Screen::Network => self.network_menu.render(menu_area, &mut buffer)?,
            Screen::SystemInfo => self.status_panel.render(menu_area, &mut buffer)?,
        }

        // Always render status panel
        self.status_panel.render(status_area, &mut buffer)?;

        // Render footer
        self.render_footer(&area, &mut buffer)?;

        // Actually draw to screen (would use VGA writer in real implementation)
        self.draw_buffer(&buffer)?;

        Ok(())
    }

    fn render_header(&self, area: &Rect, buffer: &mut Buffer) -> Result<()> {
        let title = match self.current_screen {
            Screen::Main => "IronVeil - Main Menu",
            Screen::Privacy => "IronVeil - Privacy & Security",
            Screen::Network => "IronVeil - Network Tools",
            Screen::SystemInfo => "IronVeil - System Information",
        };

        // Draw title bar with rust theme colors
        self.draw_text(2, 1, title, RUST_THEME.text, RUST_THEME.primary)?;
        
        Ok(())
    }

    fn render_footer(&self, area: &Rect, buffer: &mut Buffer) -> Result<()> {
        let footer_text = "ESC: Back | ENTER: Select | Q: Quit | TAB: Switch Menu";
        self.draw_text(2, area.height - 1, footer_text, RUST_THEME.text, RUST_THEME.secondary)?;
        Ok(())
    }

    fn handle_input(&mut self, event: InputEvent) -> Result<()> {
        match event {
            InputEvent::Key(key) => self.handle_key(key),
            InputEvent::Mouse(_) => Ok(()), // Mouse not implemented yet
            InputEvent::Resize(_, _) => Ok(()), // Handle resize if needed
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        // Global key handling
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') if key.modifiers.ctrl => {
                self.should_quit = true;
                return Ok(());
            }
            KeyCode::Tab => {
                self.switch_screen();
                return Ok(());
            }
            KeyCode::Escape => {
                self.current_screen = Screen::Main;
                return Ok(());
            }
            _ => {}
        }

        // Pass to current screen
        let input_event = InputEvent::Key(key);
        match self.current_screen {
            Screen::Main => {
                if self.main_menu.handle_input(input_event)? {
                    // Menu handled the input, check if we need to switch screens
                    if let Some(selected) = self.main_menu.get_selected() {
                        match selected {
                            0 => self.current_screen = Screen::Privacy,
                            1 => self.current_screen = Screen::Network,
                            2 => self.current_screen = Screen::SystemInfo,
                            _ => {}
                        }
                    }
                }
            }
            Screen::Privacy => {
                self.privacy_menu.handle_input(input_event)?;
            }
            Screen::Network => {
                self.network_menu.handle_input(input_event)?;
            }
            Screen::SystemInfo => {
                self.status_panel.handle_input(input_event)?;
            }
        }

        Ok(())
    }

    fn switch_screen(&mut self) {
        self.current_screen = match self.current_screen {
            Screen::Main => Screen::Privacy,
            Screen::Privacy => Screen::Network,
            Screen::Network => Screen::SystemInfo,
            Screen::SystemInfo => Screen::Main,
        };
    }

    fn get_input(&self) -> Result<InputEvent> {
        // In real implementation, this would read from keyboard driver
        // For now, simulate some basic input
        Ok(InputEvent::Key(KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers {
                ctrl: false,
                alt: false,
                shift: false,
            },
        }))
    }

    fn draw_buffer(&self, buffer: &Buffer) -> Result<()> {
        // In real implementation, this would write to VGA buffer
        Ok(())
    }

    fn draw_text(&self, x: u16, y: u16, text: &str, fg: (u8, u8, u8), bg: (u8, u8, u8)) -> Result<()> {
        // In real implementation, this would write to VGA buffer with colors
        Ok(())
    }

    fn clear_screen(&self) {
        // Clear VGA screen
    }

    fn show_cursor(&self, show: bool) {
        // Control cursor visibility
    }

    fn sleep_ms(&self, ms: u64) {
        // Sleep for specified milliseconds
        for _ in 0..ms * 1000 {
            core::hint::spin_loop();
        }
    }
}
