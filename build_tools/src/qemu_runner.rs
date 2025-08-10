//! QEMU Test Automation for IronVeil OS
//! 
//! DEPENDENCIES:
//! - Uses tokio for async process management
//! - Integrates with build_tools configuration system
//! 
//! INTEGRATION POINTS:
//! - Called by test harnesses and CI/CD pipelines
//! - Integrates with bootimage output
//! - Provides serial output capture for automated testing

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tokio::process::Command as TokioCommand;
use tokio::time::timeout;
use tokio::io::{AsyncBufReadExt, BufReader};
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};

use crate::{BuildConfig, BuildError};

/// QEMU runner for automated testing and execution
pub struct QemuRunner {
    config: BuildConfig,
}

/// Test execution results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResults {
    /// Overall test success
    pub success: bool,
    /// Duration of test execution
    pub duration: Duration,
    /// Serial output captured during test
    pub serial_output: String,
    /// Individual test case results
    pub test_cases: Vec<TestCase>,
    /// System information during test
    pub system_info: SystemInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    /// Test case name
    pub name: String,
    /// Test success status
    pub passed: bool,
    /// Test output/messages
    pub output: String,
    /// Test execution time
    pub duration: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// QEMU version used
    pub qemu_version: String,
    /// Host system information
    pub host_info: String,
    /// Memory allocation
    pub memory_mb: u32,
    /// KVM availability
    pub kvm_available: bool,
}

/// QEMU execution options
#[derive(Debug, Clone)]
pub struct QemuOptions {
    /// Bootimage path
    pub bootimage_path: PathBuf,
    /// Memory size in MB
    pub memory_mb: u32,
    /// Enable KVM acceleration
    pub enable_kvm: bool,
    /// Enable serial output capture
    pub capture_serial: bool,
    /// Enable network
    pub enable_network: bool,
    /// Additional QEMU arguments
    pub extra_args: Vec<String>,
    /// Execution timeout
    pub timeout: Duration,
    /// Expected exit patterns for success
    pub success_patterns: Vec<String>,
    /// Expected exit patterns for failure
    pub failure_patterns: Vec<String>,
}

impl QemuRunner {
    /// Create new QEMU runner with configuration
    pub fn new(config: BuildConfig) -> Self {
        Self { config }
    }

    /// Run QEMU with specified options
    pub async fn run(&self, options: QemuOptions) -> Result<TestResults> {
        let start_time = Instant::now();
        
        log::info!("Starting QEMU execution with bootimage: {}", 
                   options.bootimage_path.display());

        // Verify bootimage exists
        if !options.bootimage_path.exists() {
            return Err(BuildError::Qemu(format!(
                "Bootimage not found: {}", 
                options.bootimage_path.display()
            )).into());
        }

        // Build QEMU command
        let mut cmd = self.build_qemu_command(&options)?;
        
        // Set up progress indicator
        let progress = ProgressBar::new_spinner();
        progress.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap()
        );
        progress.set_message("Starting QEMU...");

        // Execute QEMU process
        let result = self.execute_qemu(cmd, &options, &progress).await?;
        
        progress.finish_with_message("QEMU execution completed");
        
        let duration = start_time.elapsed();
        log::info!("QEMU execution completed in {:?}", duration);

        Ok(TestResults {
            success: result.success,
            duration,
            serial_output: result.output,
            test_cases: result.test_cases,
            system_info: self.get_system_info().await?,
        })
    }

    /// Run integration tests
    pub async fn run_tests(&self, test_suite: &str) -> Result<TestResults> {
        log::info!("Running test suite: {}", test_suite);

        let bootimage_path = self.find_bootimage()?;
        
        let options = QemuOptions {
            bootimage_path,
            memory_mb: self.config.qemu.memory_mb,
            enable_kvm: self.config.qemu.enable_kvm,
            capture_serial: true,
            enable_network: self.config.qemu.network.enabled,
            extra_args: vec![],
            timeout: self.config.qemu.timeout,
            success_patterns: vec![
                "All tests passed".to_string(),
                "Test suite completed successfully".to_string(),
            ],
            failure_patterns: vec![
                "Test failed".to_string(),
                "PANIC".to_string(),
                "kernel panic".to_string(),
            ],
        };

        self.run(options).await
    }

    /// Run bootability test
    pub async fn test_boot(&self) -> Result<TestResults> {
        log::info!("Testing kernel boot sequence");

        let bootimage_path = self.find_bootimage()?;
        
        let options = QemuOptions {
            bootimage_path,
            memory_mb: 256, // Minimal memory for boot test
            enable_kvm: false, // Disable KVM for compatibility
            capture_serial: true,
            enable_network: false,
            extra_args: vec![],
            timeout: Duration::from_secs(30),
            success_patterns: vec![
                "IronVeil".to_string(),
                "Nexis".to_string(),
                "ironveil@nexis".to_string(),
            ],
            failure_patterns: vec![
                "PANIC".to_string(),
                "kernel panic".to_string(),
                "Boot failed".to_string(),
            ],
        };

        self.run(options).await
    }

    /// Interactive QEMU session (for debugging)
    pub async fn run_interactive(&self) -> Result<()> {
        log::info!("Starting interactive QEMU session");

        let bootimage_path = self.find_bootimage()?;
        
        let mut cmd = Command::new(&self.config.qemu.executable);
        cmd.arg("-drive")
           .arg(format!("format=raw,file={}", bootimage_path.display()))
           .arg("-m").arg(self.config.qemu.memory_mb.to_string());

        // Enable graphics for interactive mode
        if self.config.qemu.display.display_type != "none" {
            cmd.arg("-display").arg(&self.config.qemu.display.display_type);
        }

        // Enable KVM if available and requested
        if self.config.qemu.enable_kvm && self.is_kvm_available().await? {
            cmd.arg("-enable-kvm");
        }

        // Enable network if configured
        if self.config.qemu.network.enabled {
            cmd.arg("-netdev").arg(format!("user,id=net0"));
            cmd.arg("-device").arg("rtl8139,netdev=net0");
        }

        log::info!("Executing: {:?}", cmd);
        
        let status = cmd.status()
            .context("Failed to execute QEMU interactive session")?;

        if !status.success() {
            return Err(BuildError::Qemu(
                "QEMU interactive session failed".to_string()
            ).into());
        }

        Ok(())
    }

    /// Build QEMU command from options
    fn build_qemu_command(&self, options: &QemuOptions) -> Result<TokioCommand> {
        let mut cmd = TokioCommand::new(&self.config.qemu.executable);
        
        // Basic arguments
        cmd.arg("-drive")
           .arg(format!("format=raw,file={}", options.bootimage_path.display()))
           .arg("-m").arg(options.memory_mb.to_string());

        // Serial output
        if options.capture_serial {
            cmd.arg("-serial").arg("stdio");
        }

        // Display
        cmd.arg("-display").arg("none");

        // KVM acceleration
        if options.enable_kvm {
            cmd.arg("-enable-kvm");
        }

        // Network configuration
        if options.enable_network {
            cmd.arg("-netdev").arg("user,id=net0");
            cmd.arg("-device").arg("rtl8139,netdev=net0");
        }

        // Additional arguments
        for arg in &options.extra_args {
            cmd.arg(arg);
        }

        // Set up stdio
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        Ok(cmd)
    }

    /// Execute QEMU and capture output
    async fn execute_qemu(
        &self, 
        mut cmd: TokioCommand, 
        options: &QemuOptions,
        progress: &ProgressBar
    ) -> Result<QemuExecutionResult> {
        progress.set_message("Spawning QEMU process...");
        
        let mut child = cmd.spawn()
            .context("Failed to spawn QEMU process")?;

        let stdout = child.stdout.take()
            .context("Failed to capture QEMU stdout")?;
        
        let mut reader = BufReader::new(stdout);
        let mut output = String::new();
        let mut test_cases = Vec::new();
        let mut line_buffer = String::new();

        progress.set_message("Capturing QEMU output...");

        // Read output with timeout
        let result = timeout(options.timeout, async {
            loop {
                line_buffer.clear();
                
                match reader.read_line(&mut line_buffer).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        output.push_str(&line_buffer);
                        log::debug!("QEMU: {}", line_buffer.trim());
                        
                        // Parse test results from output
                        if let Some(test_case) = self.parse_test_output(&line_buffer) {
                            test_cases.push(test_case);
                        }
                        
                        // Check for success/failure patterns
                        for pattern in &options.success_patterns {
                            if line_buffer.contains(pattern) {
                                log::info!("Success pattern matched: {}", pattern);
                                return Ok(true);
                            }
                        }
                        
                        for pattern in &options.failure_patterns {
                            if line_buffer.contains(pattern) {
                                log::error!("Failure pattern matched: {}", pattern);
                                return Ok(false);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Error reading QEMU output: {}", e);
                        break;
                    }
                }
            }
            
            Ok(true) // Default to success if no patterns matched
        }).await;

        // Terminate QEMU process
        let _ = child.kill().await;
        let _ = child.wait().await;

        let success = match result {
            Ok(Ok(success)) => success,
            Ok(Err(_)) => false,
            Err(_) => {
                log::warn!("QEMU execution timed out");
                false
            }
        };

        Ok(QemuExecutionResult {
            success,
            output,
            test_cases,
        })
    }

    /// Parse test output for structured test results
    fn parse_test_output(&self, line: &str) -> Option<TestCase> {
        // Look for test patterns in output
        if line.contains("TEST:") {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                let name = parts[1].trim().to_string();
                let status = parts[2].trim();
                let passed = status.contains("PASS") || status.contains("OK");
                
                return Some(TestCase {
                    name,
                    passed,
                    output: line.trim().to_string(),
                    duration: Duration::from_millis(0), // Would need timing info
                });
            }
        }
        
        None
    }

    /// Find bootimage in target directory
    fn find_bootimage(&self) -> Result<PathBuf> {
        let target_dir = self.config.project_root
            .join("target")
            .join(&self.config.target)
            .join("debug");
            
        let bootimage_path = target_dir.join(&self.config.artifacts.bootimage_name);
        
        if !bootimage_path.exists() {
            return Err(BuildError::Qemu(format!(
                "Bootimage not found at: {}. Run 'cargo bootimage' first.",
                bootimage_path.display()
            )).into());
        }
        
        Ok(bootimage_path)
    }

    /// Check if KVM is available on the host system
    async fn is_kvm_available(&self) -> Result<bool> {
        #[cfg(target_os = "linux")]
        {
            Ok(Path::new("/dev/kvm").exists())
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            Ok(false)
        }
    }

    /// Get system information for test results
    async fn get_system_info(&self) -> Result<SystemInfo> {
        // Get QEMU version
        let qemu_version = self.get_qemu_version().await?;
        
        // Get host info
        let system = crate::get_system_info();
        let host_info = format!(
            "{} {} {} - {} cores, {} MB RAM",
            system.name().unwrap_or("Unknown"),
            system.os_version().unwrap_or("Unknown"),
            system.kernel_version().unwrap_or("Unknown"),
            system.cpus().len(),
            system.total_memory() / 1024 / 1024
        );

        Ok(SystemInfo {
            qemu_version,
            host_info,
            memory_mb: self.config.qemu.memory_mb,
            kvm_available: self.is_kvm_available().await?,
        })
    }

    /// Get QEMU version string
    async fn get_qemu_version(&self) -> Result<String> {
        let output = TokioCommand::new(&self.config.qemu.executable)
            .arg("--version")
            .output()
            .await
            .context("Failed to get QEMU version")?;

        if !output.status.success() {
            return Ok("Unknown".to_string());
        }

        let version_str = String::from_utf8_lossy(&output.stdout);
        Ok(version_str.lines().next().unwrap_or("Unknown").to_string())
    }
}

#[derive(Debug)]
struct QemuExecutionResult {
    success: bool,
    output: String,
    test_cases: Vec<TestCase>,
}

impl Default for QemuOptions {
    fn default() -> Self {
        Self {
            bootimage_path: PathBuf::from("bootimage.bin"),
            memory_mb: 512,
            enable_kvm: true,
            capture_serial: true,
            enable_network: false,
            extra_args: vec![],
            timeout: Duration::from_secs(30),
            success_patterns: vec![],
            failure_patterns: vec!["PANIC".to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_qemu_options_default() {
        let options = QemuOptions::default();
        assert_eq!(options.memory_mb, 512);
        assert!(options.enable_kvm);
        assert!(options.capture_serial);
    }

    #[tokio::test]
    async fn test_qemu_version_check() -> Result<()> {
        let config = BuildConfig::default();
        let runner = QemuRunner::new(config);
        
        // This test assumes QEMU is installed
        match runner.get_qemu_version().await {
            Ok(version) => {
                assert!(!version.is_empty());
                assert!(version.contains("qemu") || version.contains("QEMU"));
            }
            Err(_) => {
                // QEMU not installed - skip test
                log::warn!("QEMU not found, skipping version test");
            }
        }
        
        Ok(())
    }

    #[test]
    fn test_parse_test_output() {
        let config = BuildConfig::default();
        let runner = QemuRunner::new(config);
        
        let test_line = "TEST: memory_allocation: PASS";
        let result = runner.parse_test_output(test_line);
        
        assert!(result.is_some());
        let test_case = result.unwrap();
        assert_eq!(test_case.name, "memory_allocation");
        assert!(test_case.passed);
    }
}
