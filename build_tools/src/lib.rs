//! Build Tools Library for IronVeil OS
//! 
//! DEPENDENCIES:
//! - Uses standard Rust ecosystem crates for system interaction
//! - Integrates with existing bootimage and QEMU toolchain
//! 
//! INTEGRATION POINTS:
//! - Called by workspace build scripts and CI/CD pipelines
//! - Interfaces with QEMU for testing and validation
//! - Creates bootable USB images for live deployment
//! 
//! TESTING REQUIREMENTS:
//! - Unit tests for configuration parsing
//! - Integration tests with QEMU
//! - USB image validation tests

use std::path::PathBuf;
use std::time::Duration;
use anyhow::Result;
use serde::{Deserialize, Serialize};

pub mod qemu_runner;
pub mod live_usb;

/// Global configuration for build tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Project root directory
    pub project_root: PathBuf,
    /// Target architecture configuration
    pub target: String,
    /// QEMU configuration
    pub qemu: QemuConfig,
    /// USB image configuration  
    pub usb: UsbConfig,
    /// Build artifacts configuration
    pub artifacts: ArtifactsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QemuConfig {
    /// QEMU executable path (defaults to qemu-system-x86_64)
    pub executable: String,
    /// Default memory size in MB
    pub memory_mb: u32,
    /// Enable KVM acceleration if available
    pub enable_kvm: bool,
    /// Enable serial console output
    pub serial_output: bool,
    /// Network configuration
    pub network: NetworkConfig,
    /// Display configuration
    pub display: DisplayConfig,
    /// Timeout for operations
    pub timeout: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Enable network interface
    pub enabled: bool,
    /// Network mode (user, tap, bridge)
    pub mode: String,
    /// Port forwards (host_port -> guest_port)
    pub port_forwards: Vec<(u16, u16)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    /// Display type (gtk, sdl, vnc, none)
    pub display_type: String,
    /// VNC port if using VNC display
    pub vnc_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbConfig {
    /// USB image size in MB
    pub size_mb: u32,
    /// File system type (fat32, ext4)
    pub filesystem: String,
    /// Boot sector configuration
    pub bootloader: BootloaderConfig,
    /// Additional files to include
    pub include_files: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootloaderConfig {
    /// Bootloader type (syslinux, grub2)
    pub bootloader_type: String,
    /// Boot timeout in seconds
    pub timeout: u32,
    /// Boot menu entries
    pub menu_entries: Vec<MenuEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuEntry {
    /// Menu entry label
    pub label: String,
    /// Kernel path
    pub kernel: String,
    /// Kernel command line arguments
    pub cmdline: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactsConfig {
    /// Build output directory
    pub output_dir: PathBuf,
    /// Kernel binary name
    pub kernel_name: String,
    /// Bootimage name
    pub bootimage_name: String,
    /// USB image name
    pub usb_image_name: String,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            project_root: PathBuf::from("."),
            target: "x86_64-nexis".to_string(),
            qemu: QemuConfig::default(),
            usb: UsbConfig::default(),
            artifacts: ArtifactsConfig::default(),
        }
    }
}

impl Default for QemuConfig {
    fn default() -> Self {
        Self {
            executable: "qemu-system-x86_64".to_string(),
            memory_mb: 512,
            enable_kvm: true,
            serial_output: true,
            network: NetworkConfig::default(),
            display: DisplayConfig::default(),
            timeout: Duration::from_secs(30),
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: "user".to_string(),
            port_forwards: vec![],
        }
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            display_type: "none".to_string(),
            vnc_port: None,
        }
    }
}

impl Default for UsbConfig {
    fn default() -> Self {
        Self {
            size_mb: 256,
            filesystem: "fat32".to_string(),
            bootloader: BootloaderConfig::default(),
            include_files: vec![],
        }
    }
}

impl Default for BootloaderConfig {
    fn default() -> Self {
        Self {
            bootloader_type: "syslinux".to_string(),
            timeout: 5,
            menu_entries: vec![
                MenuEntry {
                    label: "IronVeil OS".to_string(),
                    kernel: "/ironveil.bin".to_string(),
                    cmdline: "quiet".to_string(),
                },
            ],
        }
    }
}

impl Default for ArtifactsConfig {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("target/build_artifacts"),
            kernel_name: "nexis".to_string(),
            bootimage_name: "bootimage-nexis.bin".to_string(),
            usb_image_name: "ironveil-live.img".to_string(),
        }
    }
}

/// Load build configuration from file or create default
pub fn load_config<P: AsRef<std::path::Path>>(config_path: P) -> Result<BuildConfig> {
    let path = config_path.as_ref();
    
    if path.exists() {
        let content = std::fs::read_to_string(path)?;
        let config: BuildConfig = toml::from_str(&content)?;
        Ok(config)
    } else {
        log::info!("Config file not found, creating default at {}", path.display());
        let config = BuildConfig::default();
        save_config(&config, path)?;
        Ok(config)
    }
}

/// Save build configuration to file
pub fn save_config<P: AsRef<std::path::Path>>(config: &BuildConfig, config_path: P) -> Result<()> {
    let content = toml::to_string_pretty(config)?;
    std::fs::write(config_path, content)?;
    Ok(())
}

/// Check if required tools are available in PATH
pub fn check_dependencies() -> Result<()> {
    let required_tools = [
        "cargo",
        "rustc",
        "qemu-system-x86_64",
        "dd",
    ];
    
    let optional_tools = [
        "syslinux",
        "mkfs.fat",
        "mkfs.ext4",
        "parted",
    ];
    
    log::info!("Checking required dependencies...");
    
    for tool in &required_tools {
        match which::which(tool) {
            Ok(path) => log::debug!("Found {}: {}", tool, path.display()),
            Err(_) => {
                anyhow::bail!("Required tool '{}' not found in PATH", tool);
            }
        }
    }
    
    log::info!("Checking optional dependencies...");
    
    for tool in &optional_tools {
        match which::which(tool) {
            Ok(path) => log::debug!("Found {}: {}", tool, path.display()),
            Err(_) => log::warn!("Optional tool '{}' not found in PATH", tool),
        }
    }
    
    Ok(())
}

/// Utility function to find project root by looking for Cargo.toml
pub fn find_project_root() -> Result<PathBuf> {
    let mut current = std::env::current_dir()?;
    
    loop {
        if current.join("Cargo.toml").exists() {
            return Ok(current);
        }
        
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => anyhow::bail!("Could not find project root (no Cargo.toml found)"),
        }
    }
}

/// Get system information for debugging and logging
pub fn get_system_info() -> sysinfo::System {
    let mut system = sysinfo::System::new_all();
    system.refresh_all();
    system
}

/// Common error types for build operations
#[derive(thiserror::Error, Debug)]
pub enum BuildError {
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("QEMU error: {0}")]
    Qemu(String),
    
    #[error("USB creation error: {0}")]
    Usb(String),
    
    #[error("File system error: {0}")]
    Filesystem(String),
    
    #[error("Process execution error: {0}")]
    Process(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_default_config_creation() {
        let config = BuildConfig::default();
        assert_eq!(config.target, "x86_64-nexis");
        assert_eq!(config.qemu.memory_mb, 512);
        assert_eq!(config.usb.size_mb, 256);
    }
    
    #[test]
    fn test_config_serialization() -> Result<()> {
        let config = BuildConfig::default();
        let toml_str = toml::to_string(&config)?;
        let parsed: BuildConfig = toml::from_str(&toml_str)?;
        
        assert_eq!(config.target, parsed.target);
        assert_eq!(config.qemu.memory_mb, parsed.qemu.memory_mb);
        
        Ok(())
    }
    
    #[test]
    fn test_config_file_operations() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("build_config.toml");
        
        let original_config = BuildConfig::default();
        save_config(&original_config, &config_path)?;
        
        let loaded_config = load_config(&config_path)?;
        assert_eq!(original_config.target, loaded_config.target);
        
        Ok(())
    }
    
    #[test]
    fn test_find_project_root() {
        // This test assumes we're running from within a Rust project
        let result = find_project_root();
        assert!(result.is_ok(), "Should find project root");
    }
}
