//! Build script for build_tools crate
//! 
//! This build script handles:
//! - Embedding resource files into the binary
//! - Platform-specific compilation flags
//! - Tool availability checking

use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=configs/");
    println!("cargo:rerun-if-changed=scripts/");

    // Check for required system tools
    check_system_dependencies();
    
    // Embed configuration files
    embed_config_files();
    
    // Embed scripts
    embed_scripts();
    
    // Set platform-specific flags
    set_platform_flags();
}

fn check_system_dependencies() {
    let tools = [
        ("qemu-system-x86_64", "QEMU emulator"),
        ("dd", "disk utility"),
    ];
    
    for (tool, description) in &tools {
        if which::which(tool).is_err() {
            println!("cargo:warning={} not found in PATH - {} functionality will be limited", tool, description);
        }
    }
    
    // Check for optional tools
    let optional_tools = [
        ("parted", "partition management"),
        ("syslinux", "bootloader installation"),
        ("mkfs.fat", "FAT filesystem creation"),
        ("mkfs.ext4", "ext4 filesystem creation"),
        ("losetup", "loop device management"),
    ];
    
    for (tool, description) in &optional_tools {
        if which::which(tool).is_err() {
            println!("cargo:warning=Optional tool {} not found - {} will not be available", tool, description);
        }
    }
}

fn embed_config_files() {
    // Create configs directory if it doesn't exist
    let config_dir = Path::new("configs");
    if !config_dir.exists() {
        std::fs::create_dir_all(config_dir).expect("Failed to create configs directory");
        
        // Create default configuration files
        create_default_configs(config_dir);
    }
}

fn embed_scripts() {
    // Create scripts directory if it doesn't exist
    let scripts_dir = Path::new("scripts");
    if !scripts_dir.exists() {
        std::fs::create_dir_all(scripts_dir).expect("Failed to create scripts directory");
        
        // Create default scripts
        create_default_scripts(scripts_dir);
    }
}

fn create_default_configs(config_dir: &Path) {
    // Create torrc configuration
    let torrc_content = r#"
# Tor configuration for IronVeil OS
SocksPort 9050
ControlPort 9051
DataDirectory /tmp/tor
ExitPolicy reject *:*
DNSPort 5353
AutomapHostsOnResolve 1
VirtualAddrNetworkIPv4 10.192.0.0/10
"#;
    std::fs::write(config_dir.join("torrc"), torrc_content)
        .expect("Failed to create torrc");

    // Create VPN configuration template
    let vpn_template = r#"
# OpenVPN configuration template for IronVeil OS
client
dev tun
proto udp
remote YOUR_VPN_SERVER 1194
resolv-retry infinite
nobind
persist-key
persist-tun
ca ca.crt
cert client.crt
key client.key
comp-lzo
verb 3
"#;
    std::fs::write(config_dir.join("vpn.conf.template"), vpn_template)
        .expect("Failed to create VPN template");

    // Create DNS over HTTPS configuration
    let doh_config = r#"
# DNS over HTTPS configuration for IronVeil OS
upstream 1.1.1.1:443
upstream 8.8.8.8:443
listen 127.0.0.1:53
timeout 5
"#;
    std::fs::write(config_dir.join("doh.conf"), doh_config)
        .expect("Failed to create DoH config");
}

fn create_default_scripts(scripts_dir: &Path) {
    // Create MAC randomization script
    let mac_script = r#"#!/bin/bash
# MAC Address Randomization Script for IronVeil OS

set -e

INTERFACE=${1:-eth0}

if [ "$EUID" -ne 0 ]; then
    echo "This script must be run as root"
    exit 1
fi

# Generate random MAC address
# Set locally administered bit and clear multicast bit
RANDOM_MAC=$(printf "02:%02x:%02x:%02x:%02x:%02x" \
    $((RANDOM % 256)) \
    $((RANDOM % 256)) \
    $((RANDOM % 256)) \
    $((RANDOM % 256)) \
    $((RANDOM % 256)))

echo "Randomizing MAC address for interface $INTERFACE"
echo "New MAC: $RANDOM_MAC"

# Bring interface down
ip link set $INTERFACE down

# Set new MAC address
ip link set dev $INTERFACE address $RANDOM_MAC

# Bring interface back up
ip link set $INTERFACE up

echo "MAC address randomized successfully"
"#;
    std::fs::write(scripts_dir.join("randomize_mac.sh"), mac_script)
        .expect("Failed to create MAC randomization script");

    // Create network security script
    let security_script = r#"#!/bin/bash
# Network Security Setup Script for IronVeil OS

set -e

echo "Setting up network security..."

# Enable IP forwarding for Tor transparent proxy
echo 1 > /proc/sys/net/ipv4/ip_forward

# Flush existing iptables rules
iptables -F
iptables -t nat -F

# Set default policies
iptables -P INPUT DROP
iptables -P FORWARD DROP
iptables -P OUTPUT DROP

# Allow loopback traffic
iptables -A INPUT -i lo -j ACCEPT
iptables -A OUTPUT -o lo -j ACCEPT

# Allow established connections
iptables -A INPUT -m state --state ESTABLISHED,RELATED -j ACCEPT
iptables -A OUTPUT -m state --state ESTABLISHED,RELATED -j ACCEPT

# Redirect DNS queries to Tor
iptables -t nat -A OUTPUT -p udp --dport 53 -j REDIRECT --to-ports 5353

# Redirect TCP traffic to Tor transparent proxy
iptables -t nat -A OUTPUT -p tcp --syn -j REDIRECT --to-ports 9040

echo "Network security rules applied"
"#;
    std::fs::write(scripts_dir.join("setup_security.sh"), security_script)
        .expect("Failed to create security setup script");
}

fn set_platform_flags() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    
    match target_os.as_str() {
        "linux" => {
            println!("cargo:rustc-cfg=platform_linux");
        }
        "macos" => {
            println!("cargo:rustc-cfg=platform_macos");
        }
        "windows" => {
            println!("cargo:rustc-cfg=platform_windows");
        }
        _ => {}
    }
    
    // Check for sudo availability on Unix systems
    if target_os != "windows" {
        if which::which("sudo").is_ok() {
            println!("cargo:rustc-cfg=has_sudo");
        }
    }
}
