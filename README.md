# Rust WiFi Kicker for macOS

A macOS tool for monitoring and managing network bandwidth of devices on your local network. This tool uses macOS's Packet Filter (PF) system to control network traffic.

## Features

- 🔍 Network scanning and device discovery
- 📊 Real-time traffic monitoring
- 🚫 Block specific devices
- 🎚️ Bandwidth limiting
- 💾 Persistent rules (survive reboots)
- 📱 Device tracking

## Requirements

- macOS (tested on macOS Ventura)
- Rust toolchain
- Root privileges (sudo access)
- Packet Filter (PF) enabled

## Installation

```bash
# Clone the repository
git clone <your-repo>
cd rust-wifi-kicker

# Build the project
cargo build --release
```

## Usage

The tool must be run with sudo privileges. Here are the available commands:

### Scan for devices on your network

```bash
# Default interface (en0)
sudo ./target/release/rust-wifi-kicker scan

# Specific interface
sudo ./target/release/rust-wifi-kicker scan --interface en1
```

### Monitor a device

```bash
# Basic monitoring
sudo ./target/release/rust-wifi-kicker monitor --ip 192.168.1.100

# Persistent monitoring (survives reboots)
sudo ./target/release/rust-wifi-kicker monitor --ip 192.168.1.100 --persistent
```

### Limit bandwidth for a device

```bash
# Limit both upload and download
sudo ./target/release/rust-wifi-kicker limit --ip 192.168.1.100 --upload 1000 --download 1000

# Limit only upload
sudo ./target/release/rust-wifi-kicker limit --ip 192.168.1.100 --upload 1000

# Limit only download
sudo ./target/release/rust-wifi-kicker limit --ip 192.168.1.100 --download 1000

# Make limits persistent
sudo ./target/release/rust-wifi-kicker limit --ip 192.168.1.100 --upload 1000 --download 1000 --persistent
```

Speed limits are specified in KB/s (kilobytes per second).

### Remove rules for a device

```bash
sudo ./target/release/rust-wifi-kicker remove --ip 192.168.1.100
```

### Show current status

```bash
sudo ./target/release/rust-wifi-kicker status
```

## macOS-Specific Notes

1. **Packet Filter (PF)**

   - This tool uses macOS's built-in Packet Filter system
   - PF rules are stored in `/etc/pf.conf`
   - Persistent rules are stored in `/etc/pf.anchors/com.wifi-kicker`

2. **Network Interfaces**

   - Default WiFi interface is usually `en0`
   - Use `ifconfig` to list available interfaces

3. **Permissions**

   - Tool requires root privileges
   - First run may request permissions for network monitoring

4. **Persistence**
   - Use the `--persistent` flag to make rules survive reboots
   - Persistent rules are automatically loaded at startup

## Security Note

This tool requires root privileges as it uses macOS's Packet Filter (PF) system for traffic management. Use with caution and responsibility.

## Troubleshooting

1. **PF not enabled**

   ```bash
   # Enable PF
   sudo pfctl -e
   ```

2. **Rules not applying**

   ```bash
   # Check current rules
   sudo pfctl -sr

   # Check current states
   sudo pfctl -ss

   # Flush all rules and states
   sudo pfctl -F all
   ```

3. **Interface not found**
   ```bash
   # List available interfaces
   ifconfig
   ```
