# Rust WiFi Kicker

A macOS tool to monitor, analyze and limit the bandwidth (upload/download) of devices on your local network without physical or administrative access.

## Requirements

- macOS (tested on macOS Ventura)
- Rust toolchain
- Root privileges (sudo access)

## Installation

```bash
# Clone the repository
git clone <your-repo>
cd rust-wifi-kicker

# Build the project
cargo build --release
```

## Usage

The tool must be run with sudo privileges:

### Scan for devices on your network

```bash
sudo ./target/release/rust-wifi-kicker scan --interface en0
```

### Monitor a specific device

```bash
sudo ./target/release/rust-wifi-kicker monitor --ip 192.168.1.100
```

### Limit bandwidth for a device

```bash
# Limit both upload and download
sudo ./target/release/rust-wifi-kicker limit --ip 192.168.1.100 --upload 1000 --download 1000

# Limit only upload
sudo ./target/release/rust-wifi-kicker limit --ip 192.168.1.100 --upload 1000

# Limit only download
sudo ./target/release/rust-wifi-kicker limit --ip 192.168.1.100 --download 1000
```

Speed limits are specified in KB/s (kilobytes per second).

## Security Note

This tool requires root privileges as it uses macOS's Packet Filter (PF) system for traffic management. Use with caution.
