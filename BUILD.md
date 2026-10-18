## Building Cork from Source

Currently, there are no pre-built packages or installer scripts yet. You can build Cork from source either with [Nix](#building-with-nix) _**(which we recommend!)**_ or manually with [Cargo](#building-with-cargo).

## Building with Nix

The repository ships a [Nix flake](flake.nix) that provides everything needed for building.

### Prerequisites

- [Nix](https://nixos.org/download/) with flakes enabled (`experimental-features = nix-command flakes`)
- [Git](https://git-scm.com/)

### Build

1. Clone the repository:

```bash
git clone --recursive https://github.com/tachy0nnn/cork.git
cd cork
```

2. Build the package:

```bash
nix build
```

The result is placed in `./result/bin/`.

To build straight from GitHub without cloning, use this command:

```bash
nix build 'git+https://github.com/tachy0nnn/cork?submodules=1'
```

## Building with Cargo

### Prerequisites

- [Rust](https://rustup.rs/)
- GTK4 development libraries

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

### Build

1. Clone the repository:

```bash
git clone --recursive https://github.com/tachy0nnn/cork.git
cd cork
```

2. Build the binary:

```bash
cargo build --release
```
