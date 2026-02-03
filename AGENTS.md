# fastsync - AI Agent Guide

This document provides essential information for AI coding agents working on the fastsync project.

## Project Overview

**fastsync** is a high-performance file transfer tool written in Rust, designed as a modern replacement for rclone, rsync, and scp. It maximizes throughput using parallel TCP streams, optimized socket buffers, and async I/O.

**Current Status**: Functional but minimal implementation. The codebase has extensive documentation describing planned architecture, but only core features are implemented.

## Technology Stack

| Component | Technology | Version |
|-----------|------------|---------|
| Language | Rust | Edition 2021 |
| Async Runtime | Tokio | 1.35+ |
| CLI Parsing | clap | 4.5+ |
| HTTP Client | reqwest | 0.12+ |
| Progress Bars | indicatif | 0.17 |
| Logging | tracing | 0.1 |

## Project Structure

```
fastsync/
├── Cargo.toml              # Rust project configuration
├── Cargo.lock              # Dependency lock file
├── src/
│   ├── main.rs             # CLI entry point, command dispatch
│   ├── lib.rs              # Library exports
│   ├── protocol/
│   │   ├── mod.rs          # Protocol trait definition
│   │   ├── http.rs         # HTTP/HTTPS download implementation
│   │   └── local.rs        # Local filesystem operations
│   ├── progress/
│   │   └── mod.rs          # Progress tracking utilities
│   └── download/
│       └── mod.rs          # High-level download manager
├── examples/
│   └── download.rs         # Example usage of library API
├── docs/
│   ├── ARCHITECTURE.md     # System architecture documentation
│   ├── CONFIGURATION.md    # Configuration reference
│   └── PERFORMANCE.md      # Performance tuning guide
├── COMMANDS.md             # Quick command reference
├── PROJECT_SUMMARY.md      # Project overview and goals
├── CHANGELOG.md            # Version history
└── AGENTS.md               # This file
```

## Build Commands

```bash
# Build debug version
cargo build

# Build release version (optimized)
cargo build --release

# Run the application
cargo run -- <args>

# Run with release optimizations
cargo run --release -- <args>

# Install locally
cargo install --path .
```

## Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture
```

**Note**: The project currently has minimal test coverage. Tests should be added for new protocol implementations.

## Code Style Guidelines

### Rust Conventions

- Follow standard Rust naming conventions (`snake_case` for functions/variables, `PascalCase` for types)
- Use `anyhow::Result` for error handling in application code
- Use `thiserror` for library error types (if extracted)
- Prefer `async/await` over manual futures
- Use `tracing` for logging (already initialized in `main()`)

### Example Code Pattern

```rust
use anyhow::Result;
use tracing::{debug, info, warn};

#[derive(Clone)]
pub struct MyProtocol {
    client: Client,
}

impl MyProtocol {
    pub fn new() -> Self {
        Self { client: Client::new() }
    }
}

#[async_trait]
impl Protocol for MyProtocol {
    async fn download<F>(&self, source: &str, dest: &Path, parallel: usize, progress: F) -> Result<u64>
    where F: Fn(u64) + Send
    {
        info!("Downloading from {}", source);
        // Implementation
        Ok(bytes_transferred)
    }
    
    // ... other trait methods
}
```

## Architecture

### Protocol Trait

All storage backends implement the `Protocol` trait defined in `src/protocol/mod.rs`:

```rust
#[async_trait]
pub trait Protocol: Send + Sync {
    async fn upload(&self, source: &Path, dest: &str) -> anyhow::Result<u64>;
    async fn download<F>(&self, source: &str, dest: &Path, parallel: usize, progress: F) -> anyhow::Result<u64>
    where F: Fn(u64) + Send;
    async fn get_file_info(&self, url: &str) -> anyhow::Result<FileInfo>;
    async fn download_chunk(&self, source: &str, range: Range<u64>) -> anyhow::Result<Bytes>;
}
```

### Current Implementations

1. **LocalProtocol** (`src/protocol/local.rs`)
   - Simple file copy using `tokio::fs`
   - 64KB buffer size
   - Sequential read/write

2. **HttpProtocol** (`src/protocol/http.rs`)
   - Chunked parallel downloads using HTTP Range requests
   - 8MB default chunk size
   - Uses semaphore for concurrency control

3. **SftpProtocol** (`src/protocol/sftp.rs`)
   - SFTP client using russh + russh-sftp
   - Supports rsync/scp syntax: `user@host:path`
   - SSH agent authentication (SSH_AUTH_SOCK) - tried first, with fallback to key files
   - Key-based SSH authentication (~/.ssh/id_ed25519, ~/.ssh/id_rsa, ~/.ssh/id_ecdsa)
   - RSA-SHA2-256 support for modern SSH servers (required by OpenSSH 8.8+)
   - Upload with progress tracking via `upload_with_progress()`
   - Download with progress tracking
   - IPv4/IPv6 selection support
   - Identity file option (-i) for specific key selection
   - Home directory expansion (~ -> /home/user or /root for root user)
   - Comprehensive debug logging for authentication troubleshooting

### CLI Structure

Commands are defined in `src/main.rs` using `clap`:

- `cp` (alias: `copy`) - Copy files (local, SFTP)
  - Local to local: `cp file.txt /backup/`
  - Local to SFTP: `cp file.txt user@host:~/backups/`
  - SFTP to local: `cp user@host:~/file.txt ./`
  - With IP version: `cp -4 file.txt user@host:~/backups/` (force IPv4)
- `sync` - Sync directories (stub)
- `ls` - List files (local only)
- `dl` (alias: `download`) - Download from HTTP/HTTPS URLs
  - With IP version: `dl -6 https://example.com/file.zip` (force IPv6)

### Global Options

- `-p, --parallel <N>` - Number of parallel streams
- `-r, --resume` - Resume partial downloads
- `-4, --ipv4` - Use IPv4 only
- `-6, --ipv6` - Use IPv6 only
- `-i, --identity <PATH>` - SSH private key file for SFTP authentication

## Adding New Features

### Adding a New Protocol

1. Create new file in `src/protocol/` (e.g., `sftp.rs`)
2. Implement the `Protocol` trait
3. Add module declaration in `src/protocol/mod.rs`
4. Export type in `src/protocol/mod.rs`
5. Update CLI in `src/main.rs` to use the new protocol

### Adding a New Command

1. Add variant to `Commands` enum in `src/main.rs`
2. Add argument definitions to the variant
3. Add match arm in `main()` function
4. Implement handler function

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime |
| `anyhow` | Error handling |
| `clap` | CLI parsing |
| `reqwest` | HTTP client |
| `indicatif` | Progress bars |
| `colored` | Terminal colors |
| `async-trait` | Async traits |
| `futures` | Async utilities |
| `bytes` | Byte buffer types |
| `tracing` / `tracing-subscriber` | Logging |
| `serde` / `toml` | Configuration |
| `dirs` | Config directories |
| `walkdir` | Directory traversal |
| `chrono` | Date/time handling |
| `humansize` | Human-readable sizes |
| `url` | URL parsing |

## Current Limitations

1. **Protocol Support**: HTTP, local file copy, and SFTP are implemented
2. **Sync Command**: Only lists files, doesn't actually sync
3. **Configuration**: Config file support is documented but not implemented
4. **SFTP Password Authentication**: Password-based auth not yet implemented (key-based auth works)
5. **Resume Support**: Partially implemented for HTTP downloads

## Documentation vs. Implementation Gap

**Important**: The documentation files (`ARCHITECTURE.md`, `CONFIGURATION.md`, `PERFORMANCE.md`) describe a fully-featured system that does not yet exist. The actual implementation is minimal:

- Documentation describes: SFTP, S3, Azure, GCS support
- Actual implementation: HTTP, local filesystem only
- Documentation describes: Configuration files, profiles, encryption
- Actual implementation: CLI arguments only

When implementing features, use the documentation as a specification but verify against actual code.

## Security Considerations

1. **SSH Agent** - Uses SSH_AUTH_SOCK environment variable to access SSH agent for authentication
2. **SSH Keys** - Reads private keys from ~/.ssh/ directory (id_ed25519, id_rsa, id_ecdsa)
3. **No credential storage** is currently implemented (no password caching)
4. **No encryption at rest** for transferred files
5. HTTPS certificate validation is handled by `reqwest` (Rustls with native roots)
6. Host key verification accepts any key (for now - known_hosts not yet implemented)

## Common Tasks

### Run Local Copy
```bash
cargo run -- cp file.txt /backup/
```

### Download with Parallel Streams
```bash
cargo run -- dl https://example.com/file.zip -p 16
```

### Upload to SFTP (IPv4 only)
```bash
cargo run -- cp file.txt user@server:~/backups/ -4
```

### Upload with specific SSH key
```bash
cargo run -- cp -i ~/.ssh/my_server_key file.txt user@server:~/backups/
```

### Download with IPv6
```bash
cargo run -- dl https://example.com/file.zip -6
```

### Force IPv4 for SFTP download
```bash
cargo run -- cp user@server:~/file.txt ./downloads/ --ipv4
```

### Check Code
```bash
cargo check
```

### Format Code
```bash
cargo fmt
```

### Lint
```bash
cargo clippy
```

## Cross-Compilation

### Option 1: Using Cross (Recommended - Easiest)

The `cross` tool handles all the complexity of cross-compilation using Docker:

```bash
# Install cross
cargo install cross

# Build for ARM64 (AArch64)
cross build --release --target aarch64-unknown-linux-gnu

# Build for ARMv7 (32-bit)
cross build --release --target armv7-unknown-linux-gnueabihf

# Build for ARM64 with musl (static linking, no dependencies)
cross build --release --target aarch64-unknown-linux-musl
```

Binaries will be in `target/<target-triple>/release/fastsync`

**Note:** The project is configured to use `rustls-tls` instead of `native-tls` (OpenSSL) for better cross-compilation support. This is set in `Cargo.toml`:
```toml
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "stream", "http2"] }

### Option 2: Manual Cross-Compilation

#### Step 1: Install Rust Targets

```bash
# 64-bit ARM (ARM64/AArch64) - most modern ARM devices (Raspberry Pi 4, etc.)
rustup target add aarch64-unknown-linux-gnu
rustup target add aarch64-unknown-linux-musl

# 32-bit ARM (ARMv7) - older ARM devices (Raspberry Pi 3, etc.)
rustup target add armv7-unknown-linux-gnueabihf
rustup target add armv7-unknown-linux-musleabihf
```

#### Step 2: Install Cross-Compilation Tools

**On Ubuntu/Debian:**
```bash
# For ARM64 (AArch64)
sudo apt-get install gcc-aarch64-linux-gnu

# For ARMv7 (32-bit)
sudo apt-get install gcc-arm-linux-gnueabihf

# For musl targets (static linking)
sudo apt-get install musl-tools
```

**On Fedora:**
```bash
sudo dnf install gcc-aarch64-linux-gnu arm-linux-gnueabihf-gcc
```

#### Step 3: Configure Cargo for Cross-Compilation

The project includes `.cargo/config.toml` with linker configurations:

```toml
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"

[target.armv7-unknown-linux-gnueabihf]
linker = "arm-linux-gnueabihf-gcc"
```

If your cross-compilers have different names, update this file accordingly.

#### Step 4: Build

```bash
# Build for ARM64 (glibc)
cargo build --release --target aarch64-unknown-linux-gnu

# Build for ARMv7 (glibc)
cargo build --release --target armv7-unknown-linux-gnueabihf

# Build for ARM64 (musl - static linking)
# Note: Requires musl cross-compiler setup
CC=aarch64-linux-musl-gcc cargo build --release --target aarch64-unknown-linux-musl
```

### Check Binary Architecture

```bash
file target/aarch64-unknown-linux-gnu/release/fastsync
# Output: ELF 64-bit LSB executable, ARM aarch64, version 1 (GNU/Linux)

file target/armv7-unknown-linux-gnueabihf/release/fastsync
# Output: ELF 32-bit LSB executable, ARM, EABI5 version 1 (GNU/Linux)
```

### Copy to Target Device

```bash
# Copy to Raspberry Pi or other ARM device
scp target/aarch64-unknown-linux-gnu/release/fastsync pi@raspberrypi.local:/usr/local/bin/

# Make executable and test
ssh pi@raspberrypi.local "chmod +x /usr/local/bin/fastsync && fastsync --version"
```

### Quick Reference: Common ARM Targets

| Target | Architecture | Use Case |
|--------|-------------|----------|
| `aarch64-unknown-linux-gnu` | ARM64 64-bit | Modern ARM devices (RPi 4, newer SBCs) |
| `aarch64-unknown-linux-musl` | ARM64 static | Modern ARM, no libc dependencies |
| `armv7-unknown-linux-gnueabihf` | ARMv7 32-bit | Older ARM (RPi 3, older SBCs) |
| `armv7-unknown-linux-musleabihf` | ARMv7 static | Older ARM, no libc dependencies |
| `aarch64-linux-android` | ARM64 Android | Android devices |
| `armv7-linux-androideabi` | ARMv7 Android | Older Android devices |

## Troubleshooting

### SFTP Authentication Issues

If you see "Authentication failed" when trying to connect via SFTP:

#### Key Authentication Flow

1. If `-i <keyfile>` is specified: **Only that key is tried**
2. Otherwise: First 3 SSH agent keys are tried
3. Otherwise: Default key files (`~/.ssh/id_ed25519`, `~/.ssh/id_rsa`)

**Important:** SSH servers limit authentication attempts (usually 3-6). If too many keys fail, the connection is closed.

#### Debugging Steps

1. **Check which keys are available:**
   ```bash
   ssh-add -l
   ```

2. **Try a specific key file (recommended):**
   ```bash
   fastsync cp -i ~/.ssh/my_server_key file.txt user@server:~/
   ```

3. **Verify SSH key is in authorized_keys:**
   ```bash
   ssh-add -L | grep my_key
   ssh user@server 'cat ~/.ssh/authorized_keys' | grep my_key
   ```

4. **Check key permissions:**
   ```bash
   ls -la ~/.ssh/
   chmod 600 ~/.ssh/id_ed25519
   ```

5. **Test with regular SSH first:**
   ```bash
   ssh -i ~/.ssh/my_server_key user@server
   ```

5. **Test with regular SSH first:**
   ```bash
   ssh -v user@server
   ```

6. **Enable full debug logging:**
   ```bash
   RUST_LOG=debug fastsync cp file.txt user@server:~/ 2>&1 | less
   ```

## License

MIT OR Apache-2.0

---

*Last updated: 2026-02-02*
