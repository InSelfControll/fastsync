# fastsync

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge)](https://opensource.org/licenses/MIT)

A high-performance file transfer tool written in Rust with support for HTTP/HTTPS, SFTP, and local file operations. Features progress bars, resume capability, parallel downloads, and modern SSH authentication.

## Features

- **Multi-Protocol Support**
  - HTTP/HTTPS downloads with resume capability
  - SFTP (SSH File Transfer Protocol) upload and download
  - Local file copy operations

- **Modern SSH Authentication**
  - SSH agent support
  - Identity file support (`-i` flag)
  - RSA-SHA2-256 for modern OpenSSH servers (8.8+)
  - Automatic fallback to SSH agent keys

- **User-Friendly Interface**
  - Beautiful progress bars with transfer speed
  - Colored output for better readability
  - Human-readable file sizes
  - Resume interrupted downloads

- **Network Options**
  - IPv4/IPv6 selection (`-4`/`-6` flags)
  - Parallel download support
  - Configurable connection parameters

- **Cross-Platform**
  - Linux x86_64, ARM64 (aarch64), ARMv7
  - Static linking with rustls (no OpenSSL dependency)

## Installation

### Download Pre-built Binaries

Download the latest release for your platform:

| Platform | Architecture | Download |
|----------|-------------|----------|
| Linux | x86_64 | [fastsync-linux-x86_64.tar.gz](https://github.com/InSelfControll/fastsync/releases/latest) |
| Linux | ARM64 | [fastsync-linux-aarch64.tar.gz](https://github.com/InSelfControll/fastsync/releases/latest) |
| Linux | ARMv7 | [fastsync-linux-armv7.tar.gz](https://github.com/InSelfControll/fastsync/releases/latest) |

```bash
# Example: Install on Linux x86_64
curl -L -o fastsync.tar.gz https://github.com/InSelfControll/fastsync/releases/latest/download/fastsync-linux-x86_64.tar.gz
tar xzf fastsync.tar.gz
chmod +x fastsync-linux-x86_64
sudo mv fastsync-linux-x86_64 /usr/local/bin/fastsync
```

### Build from Source

```bash
# Clone the repository
git clone https://github.com/InSelfControll/fastsync.git
cd fastsync

# Build with cargo
cargo build --release

# Binary will be at target/release/fastsync
```

#### Cross-Compilation for ARM

```bash
# Install cross tool
cargo install cross

# Build for ARM64 (Raspberry Pi 4, etc.)
cross build --target aarch64-unknown-linux-gnu --release

# Build for ARMv7 (Raspberry Pi 3, etc.)
cross build --target armv7-unknown-linux-gnueabihf --release
```

## Usage

### Copy Command (`cp`)

Copy files between local filesystem, HTTP/HTTPS URLs, and SFTP servers.

```bash
# Download from HTTP/HTTPS
fastsync cp https://example.com/file.iso ./downloads/

# Upload to SFTP
fastsync cp ./local-file.txt user@remote:/path/to/destination/

# Download from SFTP
fastsync cp user@remote:/path/to/remote-file.txt ./local-destination/

# Copy locally
fastsync cp ./source.txt ./destination.txt
```

#### SFTP Authentication Options

```bash
# Use specific identity file
fastsync cp -i ~/.ssh/my_key ./file.txt user@remote:/path/

# Use SSH agent (default)
fastsync cp ./file.txt user@remote:/path/

# Force IPv4
fastsync cp -4 user@remote:/file.txt ./

# Force IPv6
fastsync cp -6 user@remote:/file.txt ./
```

### Sync Command (`sync`)

Synchronize directories between locations.

```bash
# Sync local directory to SFTP
fastsync sync ./local-dir/ user@remote:/remote/dir/

# Sync with parallel transfers
fastsync sync -p 8 ./local-dir/ user@remote:/remote/dir/
```

### List Command (`ls`)

List remote directory contents.

```bash
# List SFTP directory
fastsync ls user@remote:/path/to/dir/
```

### Download Command (`dl`)

Optimized for batch downloads with resume support.

```bash
# Download with resume capability
fastsync dl -r https://example.com/large-file.iso ./

# Parallel download (if server supports it)
fastsync dl -p 8 https://example.com/file.zip ./
```

## Use Cases

### System Administrators

- **Server Deployment**: Quickly transfer configuration files and scripts to multiple servers
- **Backup Operations**: Sync directories to remote SFTP servers with progress tracking
- **Log Collection**: Download logs from multiple servers for analysis

### Developers

- **CI/CD Pipelines**: Transfer build artifacts to deployment servers
- **Remote Development**: Sync code changes to remote development environments
- **Asset Distribution**: Download large binary assets and dependencies

### DevOps Engineers

- **Infrastructure Automation**: Script file transfers as part of infrastructure provisioning
- **Container Image Building**: Download dependencies during Docker builds
- **Multi-Cloud File Sync**: Transfer files between different cloud providers via SFTP

### Home Users & Hobbyists

- **Raspberry Pi**: Transfer files to/from ARM-based SBCs (ARM64 and ARMv7 binaries available)
- **Media Servers**: Upload/download media files to home servers
- **Remote Backup**: Backup important files to remote SFTP servers

## Examples

### Example 1: Deploy Website to Remote Server

```bash
# Sync website files to production server
fastsync sync -p 4 ./website/ deploy@webserver:/var/www/html/
```

### Example 2: Download Large ISO with Resume

```bash
# Download Ubuntu ISO with resume capability
fastsync dl -r https://releases.ubuntu.com/24.04/ubuntu-24.04-desktop-amd64.iso ./downloads/
```

### Example 3: Backup Directory to SFTP

```bash
# Daily backup with specific SSH key
fastsync cp -i ~/.ssh/backup_key -r ./important-data/ backup@storage.example.com:/backups/$(date +%Y%m%d)/
```

### Example 4: Raspberry Pi File Transfer

```bash
# On your ARM64 device (Raspberry Pi 4/5)
wget https://github.com/InSelfControll/fastsync/releases/latest/download/fastsync-linux-aarch64.tar.gz
tar xzf fastsync-linux-aarch64.tar.gz

# Transfer files
./fastsync-linux-aarch64 cp ~/data.log server@192.168.1.100:/backups/
```

## Configuration

Create `~/.config/fastsync/config.toml` for default settings:

```toml
[defaults]
parallel = 4
resume = true

[sftp]
timeout = 30
keepalive = true
```

## Architecture

fastsync is built with Rust and uses:

- **tokio**: Async runtime for high-performance I/O
- **reqwest**: HTTP client with rustls TLS
- **russh + russh-sftp**: Pure Rust SSH/SFTP implementation
- **indicatif**: Beautiful progress bars
- **clap**: Command-line argument parsing

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for detailed technical documentation.

## Performance

fastsync is designed for high-performance file transfers:

- Async I/O for maximum throughput
- Parallel chunk downloads where supported
- Zero-copy operations where possible
- Efficient progress tracking with minimal overhead

See [docs/PERFORMANCE.md](docs/PERFORMANCE.md) for benchmarks and optimization tips.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Built with [Rust](https://www.rust-lang.org/)
- SSH/SFTP powered by [russh](https://github.com/warp-tech/russh)
- HTTP client by [reqwest](https://github.com/seanmonstar/reqwest)
