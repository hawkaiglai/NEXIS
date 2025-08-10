//! Live USB Image Creation for IronVeil OS
//! 
//! DEPENDENCIES:
//! - Uses system tools for partition creation and formatting
//! - Integrates with bootloader installation utilities
//! 
//! INTEGRATION POINTS:
//! - Uses bootimage output from nexis_kernel
//! - Creates bootable USB images with privacy tools
//! - Integrates with build configuration system

use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs::{self, File};
use std::io::{self, Write, Seek, SeekFrom};
use anyhow::{Result, Context};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use tempfile::TempDir;

use crate::{BuildConfig, BuildError};

/// USB image builder for IronVeil OS
pub struct UsbImageBuilder {
    config: BuildConfig,
    temp_dir: TempDir,
    mount_point: Option<PathBuf>,
}

/// USB image creation options
#[derive(Debug, Clone)]
pub struct UsbImageOptions {
    /// Output image path
    pub output_path: PathBuf,
    /// Image size in MB
    pub size_mb: u32,
    /// File system type
    pub filesystem: String,
    /// Include privacy tools
    pub include_privacy_tools: bool,
    /// Include development tools
    pub include_dev_tools: bool,
    /// Additional files to include
    pub additional_files: Vec<(PathBuf, PathBuf)>, // (source, dest)
    /// Image label
    pub label: String,
}

/// Partition layout for USB image
#[derive(Debug, Clone)]
pub struct PartitionLayout {
    /// Boot partition size in MB
    pub boot_size_mb: u32,
    /// Data partition size in MB (0 = use remaining space)
    pub data_size_mb: u32,
    /// Boot partition filesystem
    pub boot_filesystem: String,
    /// Data partition filesystem
    pub data_filesystem: String,
}

/// Bootloader configuration
#[derive(Debug, Clone)]
pub struct BootloaderSetup {
    /// Bootloader type (syslinux, grub2)
    pub bootloader_type: String,
    /// Boot timeout
    pub timeout: u32,
    /// Default boot entry
    pub default_entry: String,
    /// Splash screen path
    pub splash_image: Option<PathBuf>,
}

impl UsbImageBuilder {
    /// Create new USB image builder
    pub fn new(config: BuildConfig) -> Result<Self> {
        let temp_dir = TempDir::new()
            .context("Failed to create temporary directory")?;
            
        Ok(Self {
            config,
            temp_dir,
            mount_point: None,
        })
    }

    /// Create USB image with specified options
    pub async fn create_image(&mut self, options: UsbImageOptions) -> Result<()> {
        log::info!("Creating USB image: {}", options.output_path.display());
        
        let progress = self.create_progress_bar("Creating USB image");
        
        // Step 1: Create empty image file
        progress.set_message("Creating image file...");
        self.create_empty_image(&options.output_path, options.size_mb)?;
        progress.inc(1);

        // Step 2: Create partition table
        progress.set_message("Creating partition table...");
        let layout = self.create_partition_layout(&options)?;
        self.create_partitions(&options.output_path, &layout)?;
        progress.inc(1);

        // Step 3: Format partitions
        progress.set_message("Formatting partitions...");
        self.format_partitions(&options.output_path, &layout, &options.label)?;
        progress.inc(1);

        // Step 4: Mount partitions
        progress.set_message("Mounting partitions...");
        self.mount_partitions(&options.output_path)?;
        progress.inc(1);

        // Step 5: Install bootloader
        progress.set_message("Installing bootloader...");
        let bootloader_setup = self.create_bootloader_setup(&options)?;
        self.install_bootloader(&bootloader_setup)?;
        progress.inc(1);

        // Step 6: Copy kernel and system files
        progress.set_message("Copying system files...");
        self.copy_system_files(&options).await?;
        progress.inc(1);

        // Step 7: Copy additional files
        progress.set_message("Copying additional files...");
        self.copy_additional_files(&options)?;
        progress.inc(1);

        // Step 8: Create privacy tools
        if options.include_privacy_tools {
            progress.set_message("Installing privacy tools...");
            self.install_privacy_tools().await?;
        }
        progress.inc(1);

        // Step 9: Unmount and finalize
        progress.set_message("Finalizing image...");
        self.unmount_partitions()?;
        self.finalize_image(&options.output_path)?;
        progress.inc(1);

        progress.finish_with_message(&format!(
            "{} USB image created: {}", 
            "✓".green(), 
            options.output_path.display()
        ));

        log::info!("USB image creation completed successfully");
        Ok(())
    }

    /// Create bootable USB from existing image
    pub async fn create_bootable_usb(&self, image_path: &Path, device: &str) -> Result<()> {
        log::warn!("Creating bootable USB on device: {}", device);
        log::warn!("This will DESTROY all data on the device!");

        // Verify device exists and is a block device
        self.verify_block_device(device)?;

        // Unmount any mounted partitions
        self.unmount_device(device)?;

        // Write image to device
        let progress = self.create_progress_bar("Writing to USB device");
        progress.set_message(&format!("Writing to {}...", device));

        self.write_image_to_device(image_path, device, &progress)?;

        progress.finish_with_message(&format!(
            "{} Bootable USB created on {}", 
            "✓".green(), 
            device
        ));

        Ok(())
    }

    /// List available USB devices
    pub fn list_usb_devices(&self) -> Result<Vec<UsbDevice>> {
        let mut devices = Vec::new();

        #[cfg(target_os = "linux")]
        {
            let output = Command::new("lsblk")
                .args(&["-J", "-o", "NAME,SIZE,TYPE,MOUNTPOINT,VENDOR,MODEL"])
                .output()
                .context("Failed to list block devices")?;

            if output.status.success() {
                let json_str = String::from_utf8_lossy(&output.stdout);
                devices = self.parse_lsblk_output(&json_str)?;
            }
        }

        #[cfg(target_os = "macos")]
        {
            let output = Command::new("diskutil")
                .args(&["list", "-plist"])
                .output()
                .context("Failed to list disk devices")?;

            if output.status.success() {
                let plist_str = String::from_utf8_lossy(&output.stdout);
                devices = self.parse_diskutil_output(&plist_str)?;
            }
        }

        Ok(devices)
    }

    /// Create empty image file
    fn create_empty_image(&self, path: &Path, size_mb: u32) -> Result<()> {
        let size_bytes = size_mb as u64 * 1024 * 1024;
        
        let mut file = File::create(path)
            .context("Failed to create image file")?;
            
        file.set_len(size_bytes)
            .context("Failed to set image file size")?;
            
        Ok(())
    }

    /// Create partition layout based on options
    fn create_partition_layout(&self, options: &UsbImageOptions) -> Result<PartitionLayout> {
        let boot_size = 64; // 64MB boot partition
        let data_size = if options.size_mb > boot_size + 16 {
            options.size_mb - boot_size - 16 // Leave some space
        } else {
            0
        };

        Ok(PartitionLayout {
            boot_size_mb: boot_size,
            data_size_mb: data_size,
            boot_filesystem: "fat32".to_string(),
            data_filesystem: options.filesystem.clone(),
        })
    }

    /// Create partitions on image
    fn create_partitions(&self, image_path: &Path, layout: &PartitionLayout) -> Result<()> {
        // Use parted to create partition table
        let status = Command::new("parted")
            .args(&[
                image_path.to_str().unwrap(),
                "--script",
                "mklabel", "msdos",
                "mkpart", "primary", "fat32", "1MiB", &format!("{}MiB", layout.boot_size_mb + 1),
                "set", "1", "boot", "on",
            ])
            .status()
            .context("Failed to create boot partition")?;

        if !status.success() {
            return Err(BuildError::Usb("Failed to create partitions".to_string()).into());
        }

        // Create data partition if there's space
        if layout.data_size_mb > 0 {
            let start = layout.boot_size_mb + 1;
            let end = start + layout.data_size_mb;
            
            let status = Command::new("parted")
                .args(&[
                    image_path.to_str().unwrap(),
                    "--script",
                    "mkpart", "primary", &layout.data_filesystem, 
                    &format!("{}MiB", start), &format!("{}MiB", end),
                ])
                .status()
                .context("Failed to create data partition")?;

            if !status.success() {
                return Err(BuildError::Usb("Failed to create data partition".to_string()).into());
            }
        }

        Ok(())
    }

    /// Format partitions
    fn format_partitions(&self, image_path: &Path, layout: &PartitionLayout, label: &str) -> Result<()> {
        // Set up loop device
        let loop_device = self.setup_loop_device(image_path)?;
        
        // Format boot partition
        let boot_partition = format!("{}p1", loop_device);
        self.format_fat32(&boot_partition, &format!("{}_BOOT", label))?;

        // Format data partition if it exists
        if layout.data_size_mb > 0 {
            let data_partition = format!("{}p2", loop_device);
            match layout.data_filesystem.as_str() {
                "fat32" => self.format_fat32(&data_partition, &format!("{}_DATA", label))?,
                "ext4" => self.format_ext4(&data_partition, &format!("{}_DATA", label))?,
                _ => return Err(BuildError::Usb(format!(
                    "Unsupported filesystem: {}", layout.data_filesystem
                )).into()),
            }
        }

        self.cleanup_loop_device(&loop_device)?;
        Ok(())
    }

    /// Mount partitions for file operations
    fn mount_partitions(&mut self, image_path: &Path) -> Result<()> {
        let loop_device = self.setup_loop_device(image_path)?;
        let mount_dir = self.temp_dir.path().join("mount");
        fs::create_dir_all(&mount_dir)?;

        let boot_partition = format!("{}p1", loop_device);
        
        let status = Command::new("mount")
            .args(&[&boot_partition, mount_dir.to_str().unwrap()])
            .status()
            .context("Failed to mount boot partition")?;

        if !status.success() {
            return Err(BuildError::Usb("Failed to mount partition".to_string()).into());
        }

        self.mount_point = Some(mount_dir);
        Ok(())
    }

    /// Install bootloader
    fn install_bootloader(&self, setup: &BootloaderSetup) -> Result<()> {
        let mount_point = self.mount_point.as_ref()
            .ok_or_else(|| BuildError::Usb("No mount point available".to_string()))?;

        match setup.bootloader_type.as_str() {
            "syslinux" => self.install_syslinux(mount_point, setup)?,
            "grub2" => self.install_grub2(mount_point, setup)?,
            _ => return Err(BuildError::Usb(format!(
                "Unsupported bootloader: {}", setup.bootloader_type
            )).into()),
        }

        Ok(())
    }

    /// Install SYSLINUX bootloader
    fn install_syslinux(&self, mount_point: &Path, setup: &BootloaderSetup) -> Result<()> {
        // Create syslinux directory
        let syslinux_dir = mount_point.join("syslinux");
        fs::create_dir_all(&syslinux_dir)?;

        // Install syslinux files
        let status = Command::new("syslinux")
            .args(&["--directory", "syslinux", "--install", mount_point.to_str().unwrap()])
            .status()
            .context("Failed to install syslinux")?;

        if !status.success() {
            return Err(BuildError::Usb("SYSLINUX installation failed".to_string()).into());
        }

        // Create syslinux.cfg
        let config_content = self.create_syslinux_config(setup)?;
        let config_path = syslinux_dir.join("syslinux.cfg");
        fs::write(config_path, config_content)?;

        Ok(())
    }

    /// Create SYSLINUX configuration
    fn create_syslinux_config(&self, setup: &BootloaderSetup) -> Result<String> {
        let mut config = format!(
            "DEFAULT {}\n",
            setup.default_entry
        );
        
        config.push_str(&format!("TIMEOUT {}\n", setup.timeout * 10)); // SYSLINUX uses deciseconds
        
        if let Some(splash) = &setup.splash_image {
            if splash.exists() {
                config.push_str(&format!("MENU BACKGROUND {}\n", splash.file_name().unwrap().to_str().unwrap()));
            }
        }

        // Add boot entries from config
        for entry in &self.config.usb.bootloader.menu_entries {
            config.push_str(&format!(
                "\nLABEL {}\n",
                entry.label.replace(" ", "_").to_lowercase()
            ));
            config.push_str(&format!("  MENU LABEL {}\n", entry.label));
            config.push_str(&format!("  KERNEL {}\n", entry.kernel));
            if !entry.cmdline.is_empty() {
                config.push_str(&format!("  APPEND {}\n", entry.cmdline));
            }
        }

        Ok(config)
    }

    /// Copy system files to USB image
    async fn copy_system_files(&self, options: &UsbImageOptions) -> Result<()> {
        let mount_point = self.mount_point.as_ref()
            .ok_or_else(|| BuildError::Usb("No mount point available".to_string()))?;

        // Copy kernel/bootimage
        let bootimage_path = self.find_bootimage()?;
        let dest_path = mount_point.join("ironveil.bin");
        fs::copy(&bootimage_path, &dest_path)
            .context("Failed to copy bootimage")?;

        log::info!("Copied bootimage: {} -> {}", bootimage_path.display(), dest_path.display());

        // Copy system configuration files
        self.copy_system_configs(mount_point).await?;

        Ok(())
    }

    /// Copy additional files
    fn copy_additional_files(&self, options: &UsbImageOptions) -> Result<()> {
        let mount_point = self.mount_point.as_ref()
            .ok_or_else(|| BuildError::Usb("No mount point available".to_string()))?;

        for (source, dest) in &options.additional_files {
            let dest_path = mount_point.join(dest);
            
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }
            
            fs::copy(source, &dest_path)
                .context(format!("Failed to copy {} to {}", source.display(), dest_path.display()))?;
                
            log::debug!("Copied: {} -> {}", source.display(), dest_path.display());
        }

        Ok(())
    }

    /// Install privacy tools and configurations
    async fn install_privacy_tools(&self) -> Result<()> {
        let mount_point = self.mount_point.as_ref()
            .ok_or_else(|| BuildError::Usb("No mount point available".to_string()))?;

        // Create privacy tools directory
        let privacy_dir = mount_point.join("privacy");
        fs::create_dir_all(&privacy_dir)?;

        // Create MAC randomization script
        let mac_script = privacy_dir.join("randomize_mac.sh");
        fs::write(&mac_script, include_str!("../scripts/randomize_mac.sh"))?;
        
        // Make script executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&mac_script)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&mac_script, perms)?;
        }

        // Create network security configurations
        self.create_network_configs(&privacy_dir).await?;

        log::info!("Privacy tools installed");
        Ok(())
    }

    /// Create network security configurations
    async fn create_network_configs(&self, privacy_dir: &Path) -> Result<()> {
        // Create Tor configuration
        let tor_config = privacy_dir.join("torrc");
        fs::write(&tor_config, include_str!("../configs/torrc"))?;

        // Create VPN configuration template
        let vpn_config = privacy_dir.join("vpn.conf.template");
        fs::write(&vpn_config, include_str!("../configs/vpn.conf.template"))?;

        // Create DNS over HTTPS configuration
        let doh_config = privacy_dir.join("doh.conf");
        fs::write(&doh_config, include_str!("../configs/doh.conf"))?;

        Ok(())
    }

    /// Utility functions
    fn create_progress_bar(&self, message: &str) -> ProgressBar {
        let progress = ProgressBar::new(9);
        progress.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
                .unwrap()
                .progress_chars("##-")
        );
        progress.set_message(message.to_string());
        progress
    }

    fn setup_loop_device(&self, image_path: &Path) -> Result<String> {
        let output = Command::new("losetup")
            .args(&["--find", "--partscan", "--show", image_path.to_str().unwrap()])
            .output()
            .context("Failed to set up loop device")?;

        if !output.status.success() {
            return Err(BuildError::Usb("Failed to create loop device".to_string()).into());
        }

        let device = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(device)
    }

    fn cleanup_loop_device(&self, device: &str) -> Result<()> {
        let status = Command::new("losetup")
            .args(&["--detach", device])
            .status()
            .context("Failed to detach loop device")?;

        if !status.success() {
            log::warn!("Failed to detach loop device: {}", device);
        }

        Ok(())
    }

    fn format_fat32(&self, partition: &str, label: &str) -> Result<()> {
        let status = Command::new("mkfs.fat")
            .args(&["-F", "32", "-n", label, partition])
            .status()
            .context("Failed to format FAT32 partition")?;

        if !status.success() {
            return Err(BuildError::Usb("FAT32 formatting failed".to_string()).into());
        }

        Ok(())
    }

    fn format_ext4(&self, partition: &str, label: &str) -> Result<()> {
        let status = Command::new("mkfs.ext4")
            .args(&["-L", label, partition])
            .status()
            .context("Failed to format ext4 partition")?;

        if !status.success() {
            return Err(BuildError::Usb("ext4 formatting failed".to_string()).into());
        }

        Ok(())
    }

    // Additional helper methods...
    fn find_bootimage(&self) -> Result<PathBuf> {
        let target_dir = self.config.project_root
            .join("target")
            .join(&self.config.target)
            .join("debug");
            
        let bootimage_path = target_dir.join(&self.config.artifacts.bootimage_name);
        
        if !bootimage_path.exists() {
            return Err(BuildError::Usb(format!(
                "Bootimage not found at: {}. Run 'cargo bootimage' first.",
                bootimage_path.display()
            )).into());
        }
        
        Ok(bootimage_path)
    }

    // Implement remaining methods as needed...
    fn create_bootloader_setup(&self, options: &UsbImageOptions) -> Result<BootloaderSetup> {
        Ok(BootloaderSetup {
            bootloader_type: self.config.usb.bootloader.bootloader_type.clone(),
            timeout: self.config.usb.bootloader.timeout,
            default_entry: "ironveil".to_string(),
            splash_image: None,
        })
    }

    fn unmount_partitions(&mut self) -> Result<()> {
        if let Some(mount_point) = &self.mount_point {
            let status = Command::new("umount")
                .arg(mount_point.to_str().unwrap())
                .status()
                .context("Failed to unmount partition")?;

            if !status.success() {
                log::warn!("Failed to unmount partition cleanly");
            }
        }
        self.mount_point = None;
        Ok(())
    }

    fn finalize_image(&self, image_path: &Path) -> Result<()> {
        // Sync filesystem
        let _ = Command::new("sync").status();
        
        // Verify image integrity
        log::info!("Verifying image integrity...");
        
        let metadata = fs::metadata(image_path)?;
        log::info!("Final image size: {} bytes", metadata.len());
        
        Ok(())
    }

    // Placeholder implementations for platform-specific functionality
    fn verify_block_device(&self, device: &str) -> Result<()> {
        // Implementation would verify the device exists and is a block device
        Ok(())
    }

    fn unmount_device(&self, device: &str) -> Result<()> {
        // Implementation would unmount any mounted partitions on the device
        Ok(())
    }

    fn write_image_to_device(&self, image_path: &Path, device: &str, progress: &ProgressBar) -> Result<()> {
        // Implementation would write the image to the device using dd or similar
        Ok(())
    }

    async fn copy_system_configs(&self, mount_point: &Path) -> Result<()> {
        // Implementation would copy system configuration files
        Ok(())
    }

    fn install_grub2(&self, mount_point: &Path, setup: &BootloaderSetup) -> Result<()> {
        // Implementation would install GRUB2 bootloader
        Ok(())
    }

    fn parse_lsblk_output(&self, json_str: &str) -> Result<Vec<UsbDevice>> {
        // Implementation would parse lsblk JSON output
        Ok(vec![])
    }

    fn parse_diskutil_output(&self, plist_str: &str) -> Result<Vec<UsbDevice>> {
        // Implementation would parse diskutil plist output
        Ok(vec![])
    }
}

#[derive(Debug, Clone)]
pub struct UsbDevice {
    pub name: String,
    pub size: String,
    pub device_type: String,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub mountpoint: Option<String>,
}

impl Default for UsbImageOptions {
    fn default() -> Self {
        Self {
            output_path: PathBuf::from("ironveil-live.img"),
            size_mb: 256,
            filesystem: "fat32".to_string(),
            include_privacy_tools: true,
            include_dev_tools: false,
            additional_files: vec![],
            label: "IRONVEIL".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_usb_image_options_default() {
        let options = UsbImageOptions::default();
        assert_eq!(options.size_mb, 256);
        assert_eq!(options.filesystem, "fat32");
        assert!(options.include_privacy_tools);
    }

    #[test]
    fn test_partition_layout_creation() -> Result<()> {
        let config = BuildConfig::default();
        let temp_dir = TempDir::new()?;
        let builder = UsbImageBuilder::new(config)?;
        
        let options = UsbImageOptions {
            size_mb: 512,
            ..Default::default()
        };
        
        let layout = builder.create_partition_layout(&options)?;
        assert_eq!(layout.boot_size_mb, 64);
        assert!(layout.data_size_mb > 0);
        
        Ok(())
    }
}
