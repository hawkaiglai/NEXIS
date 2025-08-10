//! Rust-colored theming system
//! 
//! DEPENDENCIES:
//! - Uses core color definitions from SSOT
//! - No external dependencies
//! 
//! INTEGRATION POINTS:
//! - Used by all UI components for consistent theming
//! - Defines color palette and styling standards

/// Color constants matching the Rust brand and IronVeil aesthetic
pub mod colors {
    pub const RUST_ORANGE: (u8, u8, u8) = (206, 66, 43);     // #CE422B - Primary Rust color
    pub const RUST_BROWN: (u8, u8, u8) = (140, 48, 36);      // #8C3024 - Darker Rust shade
    pub const IRON_GRAY: (u8, u8, u8) = (45, 45, 45);        // #2D2D2D - Iron/metal gray
    pub const BRIGHT_WHITE: (u8, u8, u8) = (255, 255, 255);  // #FFFFFF - Pure white text
    pub const DARK_GRAY: (u8, u8, u8) = (24, 24, 24);        // #181818 - Dark background
    pub const SUCCESS_GREEN: (u8, u8, u8) = (40, 167, 69);   // #28A745 - Success states
    pub const WARNING_YELLOW: (u8, u8, u8) = (255, 193, 7);  // #FFC107 - Warning states
    pub const ERROR_RED: (u8, u8, u8) = (220, 53, 69);       // #DC3545 - Error states
    pub const ACCENT_BLUE: (u8, u8, u8) = (52, 144, 220);    // #3490DC - Accent/links
    pub const MUTED_GRAY: (u8, u8, u8) = (108, 117, 125);    // #6C757D - Muted text
}

/// Theme definition structure
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    /// Primary interaction color (buttons, selections, etc.)
    pub primary: (u8, u8, u8),
    /// Secondary color for less prominent elements
    pub secondary: (u8, u8, u8),
    /// Main background color
    pub background: (u8, u8, u8),
    /// Primary text color
    pub text: (u8, u8, u8),
    /// Accent color for highlights and emphasis
    pub accent: (u8, u8, u8),
    /// Success state color
    pub success: (u8, u8, u8),
    /// Warning state color
    pub warning: (u8, u8, u8),
    /// Error state color
    pub error: (u8, u8, u8),
    /// Muted/disabled text color
    pub muted: (u8, u8, u8),
}

/// Main Rust-themed color scheme for IronVeil
pub const RUST_THEME: Theme = Theme {
    primary: colors::RUST_ORANGE,
    secondary: colors::RUST_BROWN,
    background: colors::DARK_GRAY,
    text: colors::BRIGHT_WHITE,
    accent: colors::IRON_GRAY,
    success: colors::SUCCESS_GREEN,
    warning: colors::WARNING_YELLOW,
    error: colors::ERROR_RED,
    muted: colors::MUTED_GRAY,
};

/// Alternative light theme (for potential future use)
pub const RUST_LIGHT_THEME: Theme = Theme {
    primary: colors::RUST_ORANGE,
    secondary: colors::RUST_BROWN,
    background: colors::BRIGHT_WHITE,
    text: colors::DARK_GRAY,
    accent: colors::IRON_GRAY,
    success: colors::SUCCESS_GREEN,
    warning: colors::WARNING_YELLOW,
    error: colors::ERROR_RED,
    muted: colors::MUTED_GRAY,
};

/// High contrast theme for accessibility
pub const HIGH_CONTRAST_THEME: Theme = Theme {
    primary: colors::BRIGHT_WHITE,
    secondary: colors::MUTED_GRAY,
    background: (0, 0, 0),           // Pure black
    text: colors::BRIGHT_WHITE,
    accent: colors::RUST_ORANGE,
    success: colors::SUCCESS_GREEN,
    warning: colors::WARNING_YELLOW,
    error: colors::ERROR_RED,
    muted: colors::MUTED_GRAY,
};

/// Status-specific color mappings
pub mod status_colors {
    use super::colors;
    
    /// Network interface status colors
    pub const INTERFACE_UP: (u8, u8, u8) = colors::SUCCESS_GREEN;
    pub const INTERFACE_DOWN: (u8, u8, u8) = colors::ERROR_RED;
    pub const INTERFACE_UNKNOWN: (u8, u8, u8) = colors::WARNING_YELLOW;
    
    /// Tor status colors
    pub const TOR_RUNNING: (u8, u8, u8) = colors::SUCCESS_GREEN;
    pub const TOR_STARTING: (u8, u8, u8) = colors::WARNING_YELLOW;
    pub const TOR_STOPPED: (u8, u8, u8) = colors::MUTED_GRAY;
    pub const TOR_ERROR: (u8, u8, u8) = colors::ERROR_RED;
    
    /// VPN status colors
    pub const VPN_CONNECTED: (u8, u8, u8) = colors::SUCCESS_GREEN;
    pub const VPN_DISCONNECTED: (u8, u8, u8) = colors::ERROR_RED;
    pub const VPN_CONNECTING: (u8, u8, u8) = colors::WARNING_YELLOW;
    
    /// Privacy feature colors
    pub const PRIVACY_ENABLED: (u8, u8, u8) = colors::SUCCESS_GREEN;
    pub const PRIVACY_DISABLED: (u8, u8, u8) = colors::ERROR_RED;
    pub const PRIVACY_PARTIAL: (u8, u8, u8) = colors::WARNING_YELLOW;
}

/// Style definitions for different UI elements
pub mod styles {
    use super::Theme;
    
    #[derive(Debug, Clone, Copy)]
    pub struct Style {
        pub fg: (u8, u8, u8),
        pub bg: (u8, u8, u8),
        pub bold: bool,
        pub italic: bool,
        pub underline: bool,
    }
    
    impl Style {
        pub const fn new(fg: (u8, u8, u8), bg: (u8, u8, u8)) -> Self {
            Self {
                fg,
                bg,
                bold: false,
                italic: false,
                underline: false,
            }
        }
        
        pub const fn bold(mut self) -> Self {
            self.bold = true;
            self
        }
        
        pub const fn italic(mut self) -> Self {
            self.italic = true;
            self
        }
        
        pub const fn underline(mut self) -> Self {
            self.underline = true;
            self
        }
    }
    
    /// Generate common styles for a theme
    pub const fn theme_styles(theme: &Theme) -> ThemeStyles {
        ThemeStyles {
            normal: Style::new(theme.text, theme.background),
            selected: Style::new(theme.background, theme.primary).bold(),
            disabled: Style::new(theme.muted, theme.background),
            error: Style::new(theme.error, theme.background).bold(),
            warning: Style::new(theme.warning, theme.background),
            success: Style::new(theme.success, theme.background),
            header: Style::new(theme.text, theme.primary).bold(),
            border: Style::new(theme.accent, theme.background),
            accent: Style::new(theme.accent, theme.background),
        }
    }
    
    #[derive(Debug, Clone, Copy)]
    pub struct ThemeStyles {
        pub normal: Style,
        pub selected: Style,
        pub disabled: Style,
        pub error: Style,
        pub warning: Style,
        pub success: Style,
        pub header: Style,
        pub border: Style,
        pub accent: Style,
    }
}

/// Predefined styles for the Rust theme
pub const RUST_STYLES: styles::ThemeStyles = styles::theme_styles(&RUST_THEME);

/// Utility functions for color manipulation
pub mod utils {
    /// Darken a color by reducing all components by a factor
    pub const fn darken(color: (u8, u8, u8), factor: f32) -> (u8, u8, u8) {
        (
            (color.0 as f32 * factor) as u8,
            (color.1 as f32 * factor) as u8,
            (color.2 as f32 * factor) as u8,
        )
    }
    
    /// Lighten a color by increasing all components towards white
    pub const fn lighten(color: (u8, u8, u8), factor: f32) -> (u8, u8, u8) {
        (
            (color.0 as f32 + (255.0 - color.0 as f32) * factor) as u8,
            (color.1 as f32 + (255.0 - color.1 as f32) * factor) as u8,
            (color.2 as f32 + (255.0 - color.2 as f32) * factor) as u8,
        )
    }
    
    /// Convert RGB to grayscale using luminance formula
    pub const fn to_grayscale(color: (u8, u8, u8)) -> u8 {
        // Using standard luminance weights: 0.299*R + 0.587*G + 0.114*B
        (0.299 * color.0 as f32 + 0.587 * color.1 as f32 + 0.114 * color.2 as f32) as u8
    }
    
    /// Calculate color contrast ratio (simplified)
    pub fn contrast_ratio(color1: (u8, u8, u8), color2: (u8, u8, u8)) -> f32 {
        let l1 = to_grayscale(color1) as f32 / 255.0;
        let l2 = to_grayscale(color2) as f32 / 255.0;
        
        let lighter = l1.max(l2);
        let darker = l1.min(l2);
        
        (lighter + 0.05) / (darker + 0.05)
    }
    
    /// Check if color combination meets WCAG AA accessibility standards
    pub fn is_accessible(fg: (u8, u8, u8), bg: (u8, u8, u8)) -> bool {
        contrast_ratio(fg, bg) >= 4.5
    }
}

/// ASCII art and decorative elements using the theme
pub mod art {
    use super::colors;
    
    /// IronVeil ASCII logo
    pub const IRONVEIL_LOGO: &str = r#"
 ██╗██████╗  ██████╗ ███╗   ██╗██╗   ██╗███████╗██╗██╗     
 ██║██╔══██╗██╔═══██╗████╗  ██║██║   ██║██╔════╝██║██║     
 ██║██████╔╝██║   ██║██╔██╗ ██║██║   ██║█████╗  ██║██║     
 ██║██╔══██╗██║   ██║██║╚██╗██║╚██╗ ██╔╝██╔══╝  ██║██║     
 ██║██║  ██║╚██████╔╝██║ ╚████║ ╚████╔╝ ███████╗██║███████╗
 ╚═╝╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═══╝  ╚═══╝  ╚══════╝╚═╝╚══════╝
"#;
    
    /// Privacy shield icon
    pub const PRIVACY_SHIELD: &str = r#"
    ╭─────╮
   ╱       ╲
  ╱    🛡️    ╲
 ╱           ╲
╱             ╲
╲             ╱
 ╲           ╱
  ╲         ╱
   ╲_______╱
"#;
    
    /// Network activity indicator
    pub const NETWORK_ACTIVITY: [&str; 4] = [
        "📡 ●○○",
        "📡 ○●○", 
        "📡 ○○●",
        "📡 ●●●",
    ];
    
    /// Progress bar characters
    pub const PROGRESS_CHARS: [char; 9] = [
        ' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'
    ];
    
    /// Box drawing characters for borders
    pub mod borders {
        pub const HORIZONTAL: char = '─';
        pub const VERTICAL: char = '│';
        pub const TOP_LEFT: char = '╭';
        pub const TOP_RIGHT: char = '╮';
        pub const BOTTOM_LEFT: char = '╰';
        pub const BOTTOM_RIGHT: char = '╯';
        pub const CROSS: char = '┼';
        pub const T_UP: char = '┴';
        pub const T_DOWN: char = '┬';
        pub const T_LEFT: char = '┤';
        pub const T_RIGHT: char = '├';
    }
}

/// Theme manager for runtime theme switching
pub struct ThemeManager {
    current_theme: Theme,
    available_themes: &'static [(&'static str, Theme)],
}

impl ThemeManager {
    pub const fn new() -> Self {
        Self {
            current_theme: RUST_THEME,
            available_themes: &[
                ("Rust Dark", RUST_THEME),
                ("Rust Light", RUST_LIGHT_THEME),
                ("High Contrast", HIGH_CONTRAST_THEME),
            ],
        }
    }
    
    pub fn get_current_theme(&self) -> &Theme {
        &self.current_theme
    }
    
    pub fn set_theme_by_name(&mut self, name: &str) -> bool {
        for (theme_name, theme) in self.available_themes {
            if *theme_name == name {
                self.current_theme = *theme;
                return true;
            }
        }
        false
    }
    
    pub fn get_available_themes(&self) -> &[(&'static str, Theme)] {
        self.available_themes
    }
    
    pub fn cycle_theme(&mut self) {
        // Find current theme index and switch to next
        for (i, (_, theme)) in self.available_themes.iter().enumerate() {
            // Compare themes by their primary color (simple comparison)
            if theme.primary.0 == self.current_theme.primary.0 &&
               theme.primary.1 == self.current_theme.primary.1 &&
               theme.primary.2 == self.current_theme.primary.2 {
                let next_index = (i + 1) % self.available_themes.len();
                self.current_theme = self.available_themes[next_index].1;
                break;
            }
        }
    }
}
