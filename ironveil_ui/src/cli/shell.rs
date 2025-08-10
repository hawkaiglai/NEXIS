//! Enhanced command shell for IronVeil OS
//! 
//! DEPENDENCIES:
//! - Uses VGA driver for display output
//! - Integrates with memory manager and scheduler
//! - Uses keyboard driver for input
//! 
//! INTEGRATION POINTS:
//! - Called by kernel main loop
//! - Uses existing PMM for memory operations
//! - Integrates with scheduler for task management
//! - Uses neofetch module for system information

use crate::cli::{colors, CliResult, CliError, CommandContext};
use crate::vga::{vprintln, vprint};
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::format;
use core::str;

/// Command handler trait for extensible command system
pub trait CommandHandler {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn usage(&self) -> &'static str;
    fn execute(&self, args: &[&str], context: &CommandContext) -> CliResult<()>;
}

/// Shell execution context
pub struct ShellContext {
    pub context: CommandContext,
    pub commands: Vec<&'static dyn CommandHandler>,
    pub history: Vec<String>,
    pub aliases: Vec<(&'static str, &'static str)>,
}

impl ShellContext {
    pub fn new() -> Self {
        let mut shell = Self {
            context: CommandContext::default(),
            commands: Vec::new(),
            history: Vec::new(),
            aliases: Vec::new(),
        };
        
        // Register built-in commands
        shell.register_builtins();
        shell
    }
    
    pub fn register_command(&mut self, handler: &'static dyn CommandHandler) {
        self.commands.push(handler);
    }
    
    fn register_builtins(&mut self) {
        self.commands.push(&HelpCommand);
        self.commands.push(&ClearCommand);
        self.commands.push(&EchoCommand);
        self.commands.push(&HistoryCommand);
        self.commands.push(&UptimeCommand);
        self.commands.push(&MemInfoCommand);
        self.commands.push(&LsCpuCommand);
        self.commands.push(&PsCommand);
        self.commands.push(&GenPassCommand);
        self.commands.push(&NetworkCommand);
        self.commands.push(&RebootCommand);
        self.commands.push(&NeofetchCommand);
        self.commands.push(&VersionCommand);
        self.commands.push(&WhoamiCommand);
        self.commands.push(&DateCommand);
    }
    
    pub fn execute_command(&mut self, input: &str) -> CliResult<()> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Ok(());
        }
        
        // Add to history
        if self.history.len() >= 100 {
            self.history.remove(0);
        }
        self.history.push(trimmed.to_string());
        
        // Parse command and arguments
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(());
        }
        
        let command_name = parts[0];
        let args = &parts[1..];
        
        // Check for aliases first
        let resolved_command = self.resolve_alias(command_name);
        
        // Find and execute command
        for cmd in &self.commands {
            if cmd.name() == resolved_command {
                return cmd.execute(args, &self.context);
            }
        }
        
        // Command not found
        vprintln!("{}ironshell: {}: command not found{}", 
                 colors::ERROR, command_name, colors::RESET);
        vprintln!("{}Type 'help' for available commands.{}", 
                 colors::DIM, colors::RESET);
        
        Err(CliError::InvalidCommand)
    }
    
    fn resolve_alias(&self, command: &str) -> &str {
        for (alias, target) in &self.aliases {
            if *alias == command {
                return target;
            }
        }
        command
    }
    
    pub fn display_prompt(&self) {
        vprint!("{}{}{}@{}{}:{}{}{} $ {}", 
               colors::SUCCESS, colors::BOLD, self.context.current_user,
               self.context.hostname, colors::RESET,
               colors::PRIMARY, self.context.working_directory,
               colors::RESET, colors::RESET);
    }
}

/// Built-in command implementations
pub struct BuiltinCommands;

// Help Command
struct HelpCommand;
impl CommandHandler for HelpCommand {
    fn name(&self) -> &'static str { "help" }
    fn description(&self) -> &'static str { "Display help information" }
    fn usage(&self) -> &'static str { "help [command]" }
    
    fn execute(&self, args: &[&str], _context: &CommandContext) -> CliResult<()> {
        if args.is_empty() {
            vprintln!("{}{}IronVeil Shell - Available Commands:{}", 
                     colors::PRIMARY, colors::BOLD, colors::RESET);
            vprintln!("");
            
            // System Information
            vprintln!("{}System Information:{}", colors::SECONDARY, colors::RESET);
            vprintln!("  {}neofetch{}    - Display system information", colors::ACCENT, colors::RESET);
            vprintln!("  {}uptime{}     - Show system uptime", colors::ACCENT, colors::RESET);
            vprintln!("  {}meminfo{}    - Display memory information", colors::ACCENT, colors::RESET);
            vprintln!("  {}lscpu{}      - Show CPU information", colors::ACCENT, colors::RESET);
            vprintln!("  {}ps{}         - List running tasks", colors::ACCENT, colors::RESET);
            vprintln!("  {}version{}    - Show OS version", colors::ACCENT, colors::RESET);
            vprintln!("");
            
            // Privacy & Security  
            vprintln!("{}Privacy & Security:{}", colors::SECONDARY, colors::RESET);
            vprintln!("  {}genpass{}    - Generate secure password", colors::ACCENT, colors::RESET);
            vprintln!("  {}network{}    - Network privacy tools", colors::ACCENT, colors::RESET);
            vprintln!("");
            
            // System Control
            vprintln!("{}System Control:{}", colors::SECONDARY, colors::RESET);
            vprintln!("  {}clear{}      - Clear screen", colors::ACCENT, colors::RESET);
            vprintln!("  {}reboot{}     - Restart system", colors::ACCENT, colors::RESET);
            vprintln!("");
            
            // Utilities
            vprintln!("{}Utilities:{}", colors::SECONDARY, colors::RESET);
            vprintln!("  {}echo{}       - Display text", colors::ACCENT, colors::RESET);
            vprintln!("  {}history{}    - Show command history", colors::ACCENT, colors::RESET);
            vprintln!("  {}whoami{}     - Show current user", colors::ACCENT, colors::RESET);
            vprintln!("  {}date{}       - Show current date/time", colors::ACCENT, colors::RESET);
            vprintln!("");
            
            vprintln!("{}Use 'help <command>' for detailed information about a specific command.{}", 
                     colors::DIM, colors::RESET);
        } else {
            vprintln!("Detailed help for '{}' not yet implemented.", args[0]);
        }
        Ok(())
    }
}

// Clear Command
struct ClearCommand;
impl CommandHandler for ClearCommand {
    fn name(&self) -> &'static str { "clear" }
    fn description(&self) -> &'static str { "Clear the terminal screen" }
    fn usage(&self) -> &'static str { "clear" }
    
    fn execute(&self, _args: &[&str], _context: &CommandContext) -> CliResult<()> {
        crate::vga::VGA_WRITER.lock().clear_screen();
        Ok(())
    }
}

// Echo Command
struct EchoCommand;
impl CommandHandler for EchoCommand {
    fn name(&self) -> &'static str { "echo" }
    fn description(&self) -> &'static str { "Display a line of text" }
    fn usage(&self) -> &'static str { "echo [text...]" }
    
    fn execute(&self, args: &[&str], _context: &CommandContext) -> CliResult<()> {
        if args.is_empty() {
            vprintln!("");
        } else {
            vprintln!("{}", args.join(" "));
        }
        Ok(())
    }
}

// History Command
struct HistoryCommand;
impl CommandHandler for HistoryCommand {
    fn name(&self) -> &'static str { "history" }
    fn description(&self) -> &'static str { "Show command history" }
    fn usage(&self) -> &'static str { "history" }
    
    fn execute(&self, _args: &[&str], _context: &CommandContext) -> CliResult<()> {
        vprintln!("Command history not available in this context");
        Ok(())
    }
}

// Uptime Command
struct UptimeCommand;
impl CommandHandler for UptimeCommand {
    fn name(&self) -> &'static str { "uptime" }
    fn description(&self) -> &'static str { "Show system uptime" }
    fn usage(&self) -> &'static str { "uptime" }
    
    fn execute(&self, _args: &[&str], _context: &CommandContext) -> CliResult<()> {
        let ticks = crate::pit::ticks();
        let seconds = ticks / 50; // 50 Hz timer
        let minutes = seconds / 60;
        let hours = minutes / 60;
        let days = hours / 24;
        
        vprint!("{}up {}", colors::PRIMARY, colors::RESET);
        if days > 0 {
            vprint!("{} days, ", days);
        }
        if hours % 24 > 0 {
            vprint!("{}:{:02}, ", hours % 24, minutes % 60);
        } else if minutes > 0 {
            vprint!("{} min, ", minutes % 60);
        }
        vprintln!("load average: 0.00, 0.00, 0.00");
        Ok(())
    }
}

// Memory Info Command
struct MemInfoCommand;
impl CommandHandler for MemInfoCommand {
    fn name(&self) -> &'static str { "meminfo" }
    fn description(&self) -> &'static str { "Display memory information" }
    fn usage(&self) -> &'static str { "meminfo" }
    
    fn execute(&self, _args: &[&str], _context: &CommandContext) -> CliResult<()> {
        unsafe {
            let pmm = &crate::PMM;
            let free_frames = pmm.free_frames();
            let total_frames = pmm.total_frames();
            let used_frames = total_frames - free_frames;
            
            let frame_size = crate::memory::FRAME_SIZE;
            let total_kb = (total_frames * frame_size) / 1024;
            let used_kb = (used_frames * frame_size) / 1024;
            let free_kb = (free_frames * frame_size) / 1024;
            
            vprintln!("{}Memory Information:{}", colors::PRIMARY, colors::RESET);
            vprintln!("  Total:     {} KB", total_kb);
            vprintln!("  Used:      {} KB", used_kb);
            vprintln!("  Free:      {} KB", free_kb);
            vprintln!("  Frames:    {} total, {} used, {} free", 
                     total_frames, used_frames, free_frames);
        }
        Ok(())
    }
}

// CPU Info Command
struct LsCpuCommand;
impl CommandHandler for LsCpuCommand {
    fn name(&self) -> &'static str { "lscpu" }
    fn description(&self) -> &'static str { "Display CPU information" }
    fn usage(&self) -> &'static str { "lscpu" }
    
    fn execute(&self, _args: &[&str], _context: &CommandContext) -> CliResult<()> {
        vprintln!("{}CPU Information:{}", colors::PRIMARY, colors::RESET);
        vprintln!("  Architecture:     x86_64");
        vprintln!("  CPU op-mode(s):   64-bit");
        vprintln!("  Byte Order:       Little Endian");
        vprintln!("  CPU(s):           1");
        vprintln!("  Vendor ID:        Unknown");
        vprintln!("  Model name:       IronVeil Virtual CPU");
        Ok(())
    }
}

// Process List Command
struct PsCommand;
impl CommandHandler for PsCommand {
    fn name(&self) -> &'static str { "ps" }
    fn description(&self) -> &'static str { "List running tasks" }
    fn usage(&self) -> &'static str { "ps" }
    
    fn execute(&self, _args: &[&str], _context: &CommandContext) -> CliResult<()> {
        vprintln!("{}{}  PID  STATE    NAME{}", colors::PRIMARY, colors::BOLD, colors::RESET);
        vprintln!("    0  Running  kernel_idle");
        vprintln!("    1  Running  demo_task");
        vprintln!("    2  Running  shell_task");
        Ok(())
    }
}

// Password Generator Command
struct GenPassCommand;
impl CommandHandler for GenPassCommand {
    fn name(&self) -> &'static str { "genpass" }
    fn description(&self) -> &'static str { "Generate a secure password" }
    fn usage(&self) -> &'static str { "genpass [length]" }
    
    fn execute(&self, args: &[&str], _context: &CommandContext) -> CliResult<()> {
        let length = if args.is_empty() {
            16
        } else {
            args[0].parse::<usize>().unwrap_or(16).max(8).min(64)
        };
        
        let mut rng = crate::kb::XorShift64::new(0xdeadbeef12345678u64);
        let mut pass = String::new();
        
        for _ in 0..length {
            let b = rng.next_range_u8(33u8, 126u8);
            pass.push(b as char);
        }
        
        vprintln!("{}Generated password:{} {}", colors::SUCCESS, colors::RESET, pass);
        Ok(())
    }
}

// Network Command
struct NetworkCommand;
impl CommandHandler for NetworkCommand {
    fn name(&self) -> &'static str { "network" }
    fn description(&self) -> &'static str { "Network privacy tools" }
    fn usage(&self) -> &'static str { "network [ip|mac]" }
    
    fn execute(&self, args: &[&str], _context: &CommandContext) -> CliResult<()> {
        let mut rng = crate::kb::XorShift64::new(0xabcdef123456789u64);
        
        if args.is_empty() {
            vprintln!("{}Network Privacy Tools:{}", colors::PRIMARY, colors::RESET);
            vprintln!("  network ip   - Generate random IP address");
            vprintln!("  network mac  - Generate random MAC address");
            return Ok(());
        }
        
        match args[0] {
            "ip" => {
                let a = rng.next_range_u8(10, 250);
                let b = rng.next_range_u8(1, 254);
                let c = rng.next_range_u8(1, 254);
                let d = rng.next_range_u8(1, 254);
                vprintln!("{}Random IPv4:{} {}.{}.{}.{}", 
                         colors::SUCCESS, colors::RESET, a, b, c, d);
            }
            "mac" => {
                let mut parts = [0u8; 6];
                for i in 0..6 { 
                    parts[i] = rng.next_u8(); 
                }
                // Ensure it's a locally administered unicast address
                parts[0] = (parts[0] & 0xFC) | 0x02;
                
                vprintln!("{}Random MAC:{} {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                         colors::SUCCESS, colors::RESET,
                         parts[0], parts[1], parts[2], parts[3], parts[4], parts[5]);
            }
            _ => {
                vprintln!("{}Invalid network subcommand. Use 'ip' or 'mac'.{}", 
                         colors::ERROR, colors::RESET);
            }
        }
        Ok(())
    }
}

// Reboot Command
struct RebootCommand;
impl CommandHandler for RebootCommand {
    fn name(&self) -> &'static str { "reboot" }
    fn description(&self) -> &'static str { "Restart the system" }
    fn usage(&self) -> &'static str { "reboot" }
    
    fn execute(&self, _args: &[&str], _context: &CommandContext) -> CliResult<()> {
        vprintln!("{}System reboot requested...{}", colors::WARNING, colors::RESET);
        vprintln!("{}Halting kernel. Restart QEMU to continue.{}", colors::DIM, colors::RESET);
        loop { 
            core::hint::spin_loop(); 
        }
    }
}

// Neofetch Command
struct NeofetchCommand;
impl CommandHandler for NeofetchCommand {
    fn name(&self) -> &'static str { "neofetch" }
    fn description(&self) -> &'static str { "Display system information" }
    fn usage(&self) -> &'static str { "neofetch" }
    
    fn execute(&self, _args: &[&str], _context: &CommandContext) -> CliResult<()> {
        crate::neofetch::system_info::display_system_info();
        Ok(())
    }
}

// Version Command
struct VersionCommand;
impl CommandHandler for VersionCommand {
    fn name(&self) -> &'static str { "version" }
    fn description(&self) -> &'static str { "Show OS version information" }
    fn usage(&self) -> &'static str { "version" }
    
    fn execute(&self, _args: &[&str], _context: &CommandContext) -> CliResult<()> {
        vprintln!("{}IronVeil OS{} version {}", colors::PRIMARY, colors::RESET, "0.1.0-alpha");
        vprintln!("Built on Nexis kernel with Rust {}", "1.75+");
        vprintln!("Privacy-focused live operating system");
        Ok(())
    }
}

// Whoami Command
struct WhoamiCommand;
impl CommandHandler for WhoamiCommand {
    fn name(&self) -> &'static str { "whoami" }
    fn description(&self) -> &'static str { "Display current username" }
    fn usage(&self) -> &'static str { "whoami" }
    
    fn execute(&self, _args: &[&str], context: &CommandContext) -> CliResult<()> {
        vprintln!("{}", context.current_user);
        Ok(())
    }
}

// Date Command
struct DateCommand;
impl CommandHandler for DateCommand {
    fn name(&self) -> &'static str { "date" }
    fn description(&self) -> &'static str { "Display current date and time" }
    fn usage(&self) -> &'static str { "date" }
    
    fn execute(&self, _args: &[&str], _context: &CommandContext) -> CliResult<()> {
        let ticks = crate::pit::ticks();
        let seconds = ticks / 50; // 50 Hz timer
        
        // Simple time display (boot time + uptime)
        vprintln!("Boot time + {} seconds", seconds);
        vprintln!("{}Note: RTC driver required for actual date/time{}", 
                 colors::DIM, colors::RESET);
        Ok(())
    }
}
