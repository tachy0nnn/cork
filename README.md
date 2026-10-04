# cork

[![CI](https://github.com/tachy0nnn/cork/actions/workflows/ci.yml/badge.svg)](https://github.com/tachy0nnn/cork/actions/workflows/ci.yml)
[![License: AGPL-3.0-or-later](https://img.shields.io/badge/license-AGPL--3.0--or--later-blue.svg)](LICENSE)

An experimental Roblox runtime for Linux that uses the official Android APK as its client and translates it to run on Linux.

> [!WARNING]
> **Cork** is an **independent** open-source project and is **not affiliated with**, **endorsed by**, or associated with **Roblox Corporation**.

## Building from Source

Currently, there are no pre-built packages or installer scripts available. You can build Cork manually from source using Cargo.

### Prerequisites

Ensure you have [Rust](https://rustup.rs/) (Rust 1.99+) and the GTK 4 development libraries installed:

#### Debian / Ubuntu
```bash
sudo apt install -y build-essential libgtk-4-dev pkg-config
```

#### Arch Linux
```bash
sudo pacman -S git base-devel gtk4 pkgconf
```

#### Fedora / RHEL
```bash
sudo dnf install git gcc gtk4-devel pkg-config
```

### Compilation
1. Clone the repository:
  ```bash
  git clone --recursive https://github.com/tachy0nnn/cork.git
  cd cork
  ```
2. Build the binary:
  ```bash
  cargo build --release
  ```
