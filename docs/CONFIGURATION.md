# fastsync Configuration Reference

Complete reference for fastsync configuration options, profiles, and environment variables.

---

## Table of Contents

- [Configuration File Locations](#configuration-file-locations)
- [Configuration File Format](#configuration-file-format)
- [All Configuration Options](#all-configuration-options)
- [Profile Presets](#profile-presets)
- [Environment Variables](#environment-variables)
- [Example Configurations](#example-configurations)

---

## Configuration File Locations

fastsync searches for configuration files in the following order (later files override earlier ones):

1. **System-wide configuration**: `/etc/fastsync/config.toml`
2. **User configuration**: `~/.config/fastsync/config.toml`
3. **Project configuration**: `./.fastsync.toml`
4. **Explicit path**: Specified via `--config` flag

### Platform-Specific Paths

| Platform | User Config Path |
|----------|-----------------|
| Linux | `~/.config/fastsync/config.toml` |
| macOS | `~/Library/Application Support/fastsync/config.toml` |
| Windows | `%APPDATA%\fastsync\config.toml` |

---

## Configuration File Format

fastsync uses TOML format for configuration files.

### Basic Structure

```toml
# fastsync configuration file
# Comments start with #

[section]
key = "value"
number = 42
boolean = true
list = ["item1", "item2"]
```

### Configuration Sections

```toml
# Core settings
[defaults]
# Protocol-specific settings
[ssh]
[s3]
[azure]
[gcs]
[http]
# Feature settings
[compression]
[encryption]
[logging]
# Profile definitions
[profiles.lan]
[profiles.wan]
```

---

## All Configuration Options

### `[defaults]` Section

Core default settings for all transfers.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `profile` | string | `"auto"` | Default profile to use |
| `workers` | integer | `auto` | Number of worker threads (0 = auto) |
| `chunk_size` | string | `"auto"` | Default chunk size (e.g., "4M", "1G") |
| `streams` | integer | `auto` | Number of parallel streams |
| `buffer_size` | string | `"1M"` | I/O buffer size |
| `progress` | boolean | `true` | Show progress bar |
| `verbose` | boolean | `false` | Verbose output |
| `dry_run` | boolean | `false` | Preview without executing |
| `checksum` | boolean | `false` | Verify with checksums |
| `resume` | boolean | `true` | Enable resume capability |
| `follow_symlinks` | boolean | `false` | Follow symbolic links |
| `preserve_permissions` | boolean | `true` | Preserve file permissions |
| `preserve_times` | boolean | `true` | Preserve timestamps |
| `preserve_owner` | boolean | `false` | Preserve file ownership |
| `delete` | boolean | `false` | Delete extraneous files |
| `bwlimit` | string | `"0"` | Bandwidth limit (0 = unlimited) |
| `timeout` | string | `"30s"` | Connection timeout |
| `retries` | integer | `3` | Number of retries |
| `retry_delay` | string | `"1s"` | Delay between retries |

```toml
[defaults]
profile = "auto"
workers = 8
chunk_size = "4M"
streams = 4
buffer_size = "1M"
progress = true
verbose = false
dry_run = false
checksum = false
resume = true
follow_symlinks = false
preserve_permissions = true
preserve_times = true
preserve_owner = false
delete = false
bwlimit = "0"
timeout = "30s"
retries = 3
retry_delay = "1s"
```

### `[ssh]` Section

SSH/SFTP protocol configuration.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `identity_file` | string | `"~/.ssh/id_rsa"` | Default SSH private key |
| `port` | integer | `22` | Default SSH port |
| `strict_host_key_checking` | boolean | `true` | Verify host keys |
| `user_known_hosts_file` | string | `"~/.ssh/known_hosts"` | Known hosts file |
| `connection_timeout` | string | `"30s"` | Connection timeout |
| `keepalive_interval` | string | `"30s"` | Keepalive interval |
| `compression` | boolean | `false` | Enable SSH compression |
| `cipher` | string | `"aes256-gcm@openssh.com"` | Preferred cipher |
| `forward_agent` | boolean | `false` | Enable agent forwarding |
| `proxy_command` | string | `none` | Proxy command for connections |

```toml
[ssh]
identity_file = "~/.ssh/id_rsa"
port = 22
strict_host_key_checking = true
user_known_hosts_file = "~/.ssh/known_hosts"
connection_timeout = "30s"
keepalive_interval = "30s"
compression = false
cipher = "aes256-gcm@openssh.com"
forward_agent = false
# proxy_command = "nc -X connect -x proxy.example.com:8080 %h %p"
```

### `[s3]` Section

Amazon S3 configuration.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `region` | string | `"us-east-1"` | AWS region |
| `endpoint` | string | `"https://s3.amazonaws.com"` | S3 endpoint URL |
| `access_key_id` | string | `env` | AWS access key ID |
| `secret_access_key` | string | `env` | AWS secret access key |
| `session_token` | string | `none` | AWS session token |
| `bucket` | string | `none` | Default bucket |
| `storage_class` | string | `"STANDARD"` | Default storage class |
| `acl` | string | `"private"` | Default ACL |
| `multipart_threshold` | string | `"100M"` | Multipart upload threshold |
| `multipart_chunksize` | string | `"8M"` | Multipart chunk size |
| `max_concurrent_requests` | integer | `10` | Max concurrent requests |
| `use_accelerate_endpoint` | boolean | `false` | Use S3 Transfer Acceleration |
| `addressing_style` | string | `"auto"` | URL addressing style |

```toml
[s3]
region = "us-east-1"
endpoint = "https://s3.amazonaws.com"
# access_key_id = "AKIA..."
# secret_access_key = "..."
storage_class = "STANDARD"
acl = "private"
multipart_threshold = "100M"
multipart_chunksize = "8M"
max_concurrent_requests = 10
use_accelerate_endpoint = false
addressing_style = "auto"
```

### `[azure]` Section

Azure Blob Storage configuration.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `storage_account` | string | `env` | Storage account name |
| `storage_key` | string | `env` | Storage account key |
| `sas_token` | string | `none` | SAS token |
| `container` | string | `none` | Default container |
| `endpoint` | string | `auto` | Blob endpoint URL |
| `block_size` | string | `"4M"` | Block blob block size |
| `max_block_size` | string | "100M" | Maximum block size |
| `max_single_put_size` | string | "64M" | Max size for single put |
| `max_concurrency` | integer | `8` | Max concurrent transfers |

```toml
[azure]
# storage_account = "myaccount"
# storage_key = "..."
# sas_token = "?sv=..."
block_size = "4M"
max_block_size = "100M"
max_single_put_size = "64M"
max_concurrency = 8
```

### `[gcs]` Section

Google Cloud Storage configuration.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `project_id` | string | `env` | GCP project ID |
| `credentials_file` | string | `env` | Path to service account JSON |
| `access_token` | string | `none` | OAuth2 access token |
| `bucket` | string | `none` | Default bucket |
| `chunk_size` | string | `"8M"` | Upload chunk size |
| `max_retries` | integer | `3` | Max retry attempts |

```toml
[gcs]
# project_id = "my-project"
# credentials_file = "/path/to/key.json"
chunk_size = "8M"
max_retries = 3
```

### `[http]` Section

HTTP/HTTPS protocol configuration.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `user_agent` | string | `"fastsync/{version}"` | User-Agent header |
| `timeout` | string | `"30s"` | Request timeout |
| `connect_timeout` | string | `"10s"` | Connection timeout |
| `pool_idle_timeout` | string | `"90s"` | Pool idle timeout |
| `pool_max_idle_per_host` | integer | `10` | Max idle connections |
| `http2_prior_knowledge` | boolean | `false` | Force HTTP/2 |
| `http3_enabled` | boolean | `false` | Enable HTTP/3 |
| `redirect_policy` | string | `"limited"` | Redirect handling |
| `redirect_limit` | integer | `10` | Max redirects |
| `danger_accept_invalid_certs` | boolean | `false` | Skip cert validation |
| `danger_accept_invalid_hostnames` | boolean | `false` | Skip hostname validation |

```toml
[http]
user_agent = "fastsync/0.1.0"
timeout = "30s"
connect_timeout = "10s"
pool_idle_timeout = "90s"
pool_max_idle_per_host = 10
http2_prior_knowledge = false
http3_enabled = false
redirect_policy = "limited"
redirect_limit = 10
danger_accept_invalid_certs = false
danger_accept_invalid_hostnames = false
```

### `[compression]` Section

Compression settings.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | boolean | `true` | Enable compression |
| `algorithm` | string | `"zstd"` | Compression algorithm |
| `level` | integer | `3` | Compression level (1-22) |
| `threshold` | string | `"1K"` | Minimum size to compress |
| `compressible_types` | list | `["text/*", "application/json"]` | MIME types to compress |
| `incompressible_types` | list | `["image/*", "video/*", "audio/*", "application/zip"]` | Types to skip |

```toml
[compression]
enabled = true
algorithm = "zstd"
level = 3
threshold = "1K"
compressible_types = [
    "text/*",
    "application/json",
    "application/xml",
    "application/javascript"
]
incompressible_types = [
    "image/*",
    "video/*",
    "audio/*",
    "application/zip",
    "application/gzip",
    "application/x-7z-compressed"
]
```

**Supported Algorithms:**
- `zstd` - Best compression ratio and speed (recommended)
- `lz4` - Fastest, lower compression
- `gzip` - Widely compatible
- `brotli` - Best compression, slower
- `none` - Disable compression

### `[encryption]` Section

Encryption settings.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | boolean | `false` | Enable file encryption |
| `algorithm` | string | `"aes-256-gcm"` | Encryption algorithm |
| `key_file` | string | `none` | Path to encryption key |
| `key_id` | string | `none` | Key identifier |

```toml
[encryption]
enabled = false
algorithm = "aes-256-gcm"
# key_file = "/path/to/key"
# key_id = "my-key"
```

### `[logging]` Section

Logging configuration.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `level` | string | `"info"` | Log level |
| `format` | string | `"pretty"` | Log format |
| `output` | string | `"stderr"` | Log output destination |
| `file` | string | `none` | Log file path |
| `max_file_size` | string | `"10M"` | Max log file size |
| `max_files` | integer | `5` | Max log files to keep |

```toml
[logging]
level = "info"
format = "pretty"
output = "stderr"
# file = "/var/log/fastsync.log"
max_file_size = "10M"
max_files = 5
```

**Log Levels:** `trace`, `debug`, `info`, `warn`, `error`

**Log Formats:** `pretty`, `compact`, `json`

### `[include]` and `[exclude]` Sections

File filtering patterns.

```toml
[include]
patterns = [
    "*.txt",
    "*.md",
    "src/**"
]

[exclude]
patterns = [
    ".git/",
    "node_modules/",
    "*.tmp",
    "*.log",
    ".env"
]
```

**Pattern Syntax:**
- `*` - Match any characters except `/`
- `**` - Match any characters including `/`
- `?` - Match single character
- `[abc]` - Match any character in set
- `!pattern` - Negate pattern

---

## Profile Presets

Profiles are pre-configured sets of options optimized for specific scenarios.

### Built-in Profiles

#### `lan` - Local Area Network

Optimized for high-speed, low-latency local networks.

```toml
[profiles.lan]
workers = 16
chunk_size = "16M"
streams = 8
buffer_size = "4M"
compression = false
checksum = false
resume = true
```

**Characteristics:**
- Large chunks for efficiency
- Many parallel streams
- Compression disabled (network is fast)
- Checksums disabled (reliable network)

#### `wan` - Wide Area Network

Optimized for internet transfers with higher latency.

```toml
[profiles.wan]
workers = 8
chunk_size = "4M"
streams = 16
buffer_size = "1M"
compression = true
compression_level = 6
checksum = true
resume = true
retries = 5
retry_delay = "5s"
```

**Characteristics:**
- Smaller chunks for better pipelining
- More parallel streams to fill pipe
- Compression enabled
- Higher retry count

#### `small-files` - Many Small Files

Optimized for directories with many small files.

```toml
[profiles.small-files]
workers = 32
chunk_size = "256K"
streams = 4
buffer_size = "256K"
compression = false
checksum = false
batch_size = 100
metadata_cache = true
```

**Characteristics:**
- Many workers for parallel metadata ops
- Small chunks
- Batch file operations
- Metadata caching enabled

#### `low-memory` - Memory-Constrained

Optimized for systems with limited memory.

```toml
[profiles.low-memory]
workers = 2
chunk_size = "1M"
streams = 2
buffer_size = "256K"
compression = false
memory_limit = "256M"
file_buffer_pool = 4
```

**Characteristics:**
- Minimal workers
- Small buffers
- Memory limits enforced
- Compression disabled

#### `max-throughput` - Maximum Performance

Optimized for maximum throughput on high-end hardware.

```toml
[profiles.max-throughput]
workers = 64
chunk_size = "64M"
streams = 32
buffer_size = "16M"
compression = true
compression_level = 1
checksum = true
zero_copy = true
io_uring = true
memory_limit = "8G"
```

**Characteristics:**
- Maximum parallelism
- Large chunks and buffers
- Zero-copy enabled
- io_uring on Linux

### Custom Profiles

Define your own profiles:

```toml
[profiles.my-profile]
workers = 12
chunk_size = "8M"
streams = 8
compression = true
compression_level = 3

[profiles.my-profile.compression]
algorithm = "lz4"
threshold = "4K"
```

### Using Profiles

```bash
# Use a profile
fastsync copy --profile lan ./file server:/path/

# Override profile settings
fastsync copy --profile wan --workers 16 ./file server:/path/

# List available profiles
fastsync profiles list

# Show profile details
fastsync profiles show wan
```

---

## Environment Variables

All configuration options can be overridden via environment variables.

### Variable Naming Convention

Environment variables use `FASTSYNC_` prefix with uppercase section and key:

```
FASTSYNC_<SECTION>_<KEY>
```

Examples:
- `FASTSYNC_DEFAULTS_WORKERS`
- `FASTSYNC_SSH_PORT`
- `FASTSYNC_S3_REGION`

### Core Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `FASTSYNC_CONFIG` | Path to config file | `/etc/fastsync/config.toml` |
| `FASTSYNC_PROFILE` | Default profile | `wan` |
| `FASTSYNC_LOG_LEVEL` | Log level | `debug` |
| `FASTSYNC_WORKERS` | Number of workers | `8` |

### Protocol Environment Variables

| Variable | Description |
|----------|-------------|
| `FASTSYNC_SSH_KEY` | Default SSH private key |
| `AWS_ACCESS_KEY_ID` | AWS access key |
| `AWS_SECRET_ACCESS_KEY` | AWS secret key |
| `AWS_DEFAULT_REGION` | AWS region |
| `AZURE_STORAGE_ACCOUNT` | Azure account name |
| `AZURE_STORAGE_KEY` | Azure account key |
| `GOOGLE_APPLICATION_CREDENTIALS` | GCS credentials file |

### Security Environment Variables

| Variable | Description |
|----------|-------------|
| `FASTSYNC_ENCRYPTION_KEY` | Encryption key (base64) |
| `FASTSYNC_ENCRYPTION_KEY_FILE` | Path to key file |

### Priority Order

Configuration is loaded in this priority (highest first):

1. Command-line flags
2. Environment variables
3. Project config (`.fastsync.toml`)
4. User config (`~/.config/fastsync/config.toml`)
5. System config (`/etc/fastsync/config.toml`)
6. Built-in defaults

---

## Example Configurations

### Basic Home User

```toml
# ~/.config/fastsync/config.toml

[defaults]
profile = "wan"
progress = true
verbose = false

[ssh]
identity_file = "~/.ssh/id_ed25519"

[exclude]
patterns = [
    ".DS_Store",
    "Thumbs.db",
    "*.tmp",
    ".Trash/"
]
```

### Developer Workstation

```toml
# ~/.config/fastsync/config.toml

[defaults]
workers = 16
progress = true
checksum = true

[ssh]
identity_file = "~/.ssh/id_rsa"
forward_agent = true

[compression]
enabled = true
algorithm = "zstd"
level = 3

[exclude]
patterns = [
    ".git/",
    "node_modules/",
    "target/",
    "__pycache__/",
    "*.pyc",
    ".env",
    ".venv/",
    "*.log"
]

[profiles.lan]
workers = 32
compression = false

[profiles.ci]
workers = 8
progress = false
verbose = false
```

### Production Server

```toml
# /etc/fastsync/config.toml

[defaults]
profile = "max-throughput"
workers = 64
progress = false
log_file = "/var/log/fastsync.log"

[ssh]
strict_host_key_checking = true
identity_file = "/etc/fastsync/ssh_key"

[s3]
region = "us-east-1"
storage_class = "STANDARD_IA"

[logging]
level = "warn"
format = "json"
output = "file"
file = "/var/log/fastsync.log"
max_file_size = "100M"
max_files = 10

[profiles.backup]
workers = 16
chunk_size = "8M"
compression = true
checksum = true
retries = 10
```

### CI/CD Pipeline

```toml
# .fastsync.toml (in repository)

[defaults]
workers = 8
progress = false
verbose = false
dry_run = false

[compression]
enabled = true
algorithm = "lz4"
level = 1

[exclude]
patterns = [
    ".git/",
    ".github/",
    "tests/",
    "*.test",
    "coverage/"
]

[profiles.deploy]
workers = 16
streams = 8
compression = false
checksum = true
```

### Multi-Cloud Setup

```toml
# ~/.config/fastsync/config.toml

[defaults]
workers = 16
progress = true

# AWS Production
[profiles.aws-prod]
[s3]
region = "us-west-2"
storage_class = "INTELLIGENT_TIERING"

# AWS Development
[profiles.aws-dev]
[s3]
region = "us-east-1"
storage_class = "STANDARD"

# Azure Backup
[profiles.azure-backup]
[azure]
storage_account = "backupaccount"
block_size = "8M"

# GCS Archive
[profiles.gcs-archive]
[gcs]
project_id = "my-project"
chunk_size = "16M"
```

### Low-Bandwidth Satellite

```toml
# ~/.config/fastsync/config.toml

[defaults]
profile = "wan"
workers = 4
chunk_size = "1M"
streams = 32
bwlimit = "1M"
compression = true
compression_level = 19
retries = 10
retry_delay = "30s"
timeout = "5m"

[compression]
algorithm = "zstd"
threshold = "100B"
```

---

## Configuration Validation

Validate your configuration:

```bash
# Check config file syntax
fastsync config validate

# Show effective configuration
fastsync config show

# Show configuration with a profile
fastsync config show --profile wan

# Debug configuration loading
fastsync config show --verbose
```

---

For more information, see:
- [Architecture Overview](ARCHITECTURE.md)
- [Performance Tuning](PERFORMANCE.md)
