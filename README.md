# IronVeil OS

[![Build Status](https://github.com/c0mr4de-laugh4l0t/IronVeil/workflows/CI/badge.svg)](https://github.com/c0mr4de-laugh4l0t/IronVeil/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust Version](https://img.shields.io/badge/rust-nightly--2024--01--01-orange.svg)](https://forge.rust-lang.org/)

**IronVeil** is a privacy-focused live USB operating system built on the Nexis kernel, written entirely in Rust. Designed for security professionals, privacy advocates, and anyone who values digital anonymity.

## 🔒 Key Features

- **Memory Safety**: Built in Rust with `#![no_std]` for maximum control
- **Privacy First**: All network traffic routed through privacy stack
- **Live USB**: No persistent storage by default
- **MAC Randomization**: Automatic MAC address randomization on boot
- **Tor Integration**: Built-in Tor routing capabilities
- **Modular Design**: Clean separation between kernel and userland components

## 🏗️ Architecture

IronVeil consists of four main components:

- **`nexis_kernel`**: Core kernel with memory management, scheduling, and drivers
- **`ironveil_ui`**: TUI-based user interface with Rust-themed design
- **`privacy_core`**: Privacy and security functionality (MAC spoofing, Tor, etc.)
- **`build_tools`**: Build automation and USB image creation tools

## 🚀 Quick Start

### Prerequisites

```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install required tools
cargo install bootimage cargo-binutils

# Install system dependencies (Ubuntu/Debian)
sudo apt update
sudo apt install qemu-system-x86 lld -y

# Add required Rust components
rustup target add x86_64-unknown-none
rustup component add rust-src llvm-tools-preview
Building
bash


# Clone the repository
git clone https://github.com/c0mr4de-laugh4l0t/IronVeil.git
cd IronVeil

# Build the kernel
cargo kernel

# Create bootable image
cargo bootimg

# Run in QEMU
cargo run-kernel
Development Commands
bash


# Build all components
cargo build --workspace

# Run tests
cargo test-all

# Lint code
cargo check-all

# Format code
cargo fmt-all

# Run with debugging
cargo run-debug

# Run without KVM (for containers/CI)
cargo run-nokvm
🛠️ Development Setup
VS Code Extensions (Recommended)
json


{
  "recommendations": [
    "rust-lang.rust-analyzer",
    "tamasfe.even-better-toml",
    "usernamehw.errorlens",
    "serayuzgur.crates",
    "ms-vscode.vscode-github-codespaces"
  ]
}
Workspace Structure
angelscript


IronVeil/
├── nexis_kernel/           # Core kernel implementation
│   ├── src/
│   │   ├── main.rs         # Boot entry point
│   │   ├── memory/         # Memory management
│   │   ├── interrupt/      # Interrupt handling
│   │   ├── scheduler/      # Task scheduling
│   │   ├── drivers/        # Hardware drivers
│   │   └── asm/           # Assembly code
│   └── x86_64-nexis.json  # Target specification
├── ironveil_ui/           # User interface
│   └── src/
│       ├── tui/           # Terminal UI components
│       ├── cli/           # Command-line interface
│       └── neofetch/      # System information
├── privacy_core/          # Privacy functionality
│   └── src/
│       ├── network_hooks/ # Network interception
│       ├── mac_spoofing/  # MAC randomization
│       └── ip_randomization/ # IP privacy
├── build_tools/           # Build automation
│   └── src/
│       ├── qemu_runner.rs # QEMU automation
│       └── live_usb.rs    # USB image creation
└── workspace_config/      # Workspace configuration
    ├── Cargo.toml         # Workspace dependencies
    ├── rust-toolchain.toml # Rust toolchain
    └── .cargo/config.toml # Build configuration
🎨 Design Philosophy
Rust-Themed UI
IronVeil uses a distinctive Rust-inspired color scheme:

Rust Orange (#CE422B): Primary interaction color
Rust Brown (#8C3024): Secondary elements
Iron Gray (#2D2D2D): Accent color
Dark Background (#181818): Main background
Bright White (#FFFFFF): Text color
Privacy by Design
No data persistence without explicit user consent
All network traffic subject to privacy filtering
Automatic MAC address randomization
Built-in Tor integration
Secure memory wiping on shutdown
🧪 Testing
Unit Tests
bash


# Run kernel tests
cargo test --package nexis_kernel

# Run UI tests  
cargo test --package ironveil_ui

# Run privacy tests
cargo test --package privacy_core
Integration Tests
bash


# Run in QEMU for integration testing
cargo run-kernel

# Test specific functionality
cargo test --package nexis_kernel --test memory_integration
Hardware Testing
bash


# Create USB image for hardware testing
cargo run --package build_tools --bin create_usb -- /dev/sdX

# Warning: This will overwrite the target device!
📚 Documentation
Architecture Guide: docs/architecture.md
API Reference: docs/api/
Build Guide: docs/building.md
Contributing: CONTRIBUTING.md
Security: SECURITY.md
🤝 Contributing
We welcome contributions! Please see our Contributing Guide for details.

Development Workflow
Fork the repository
Create a feature branch: git checkout -b feature/amazing-feature
Make your changes following our coding standards
Run tests: cargo test-all
Run linting: cargo check-all
Commit changes: git commit -m 'Add amazing feature'
Push to branch: git push origin feature/amazing-feature
Open a Pull Request
Code Style
Follow Rust standard formatting: cargo fmt-all
Pass all lints: cargo clippy --workspace -- -D warnings
Include tests for new functionality
Document public APIs
Follow the Single Source of Truth (SSOT) document
🔒 Security
IronVeil is designed with security as a primary concern. Please see our Security Policy for:

Reporting security vulnerabilities
Security architecture overview
Threat model and mitigations
Security testing procedures
📄 License
This project is dual-licensed under:

MIT License
Apache License 2.0
You may choose either license at your option.

🌟 Acknowledgments
Philipp Oppermann for the excellent "Writing an OS in Rust" blog series
The Rust community for creating amazing #![no_std] libraries
Bootloader project for kernel loading
QEMU project for virtualization and testing
📞 Support
Documentation: https://c0mr4de-laugh4l0t.github.io/Nexis-Website-/
Issues: https://github.com/c0mr4de-laugh4l0t/IronVeil/issues
Discussions: https://github.com/c0mr4de-laugh4l0t/IronVeil/discussions
Matrix Chat: #ironveil:matrix.org
⚠️ Disclaimer: IronVeil is experimental software. While designed with privacy and security in mind, it should not be relied upon for critical security applications without thorough testing and auditing.
