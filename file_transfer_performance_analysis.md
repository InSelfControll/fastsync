
# PERFORMANCE ANALYSIS OF FILE TRANSFER TOOLS: rsync, rclone, and scp

## Executive Summary

This report analyzes the performance limitations of three major file transfer tools: rsync, rclone, and scp. Each tool has specific bottlenecks that limit their ability to saturate high-bandwidth network connections, particularly over wide-area networks (WANs) with high latency.

---

## 1. RSYNC LIMITATIONS

### 1.1 Delta Transfer Algorithm Performance Issues

**What is the Delta Transfer Algorithm?**
- Rsync uses a rolling checksum algorithm to identify changed blocks within files
- Only transfers the differences (deltas) between source and destination files
- Uses weak 32-bit rolling checksums + strong 128-bit MD4 checksums for verification

**When Delta Transfer Hurts Performance:**

| Scenario | Impact |
|----------|--------|
| First-time transfers | No benefit - must transfer entire file anyway |
| Large files with small changes | High CPU overhead for checksum computation |
| Incompressible data (VM images, encrypted files) | Checksum computation is wasted effort |
| High-bandwidth, low-latency networks | CPU becomes bottleneck instead of network |
| Files with random data patterns | Poor block matching efficiency |

**Specific Performance Problems:**

1. **CPU-Intensive Checksum Computation**
   - For a 500GB VM image, rsync must compute checksums for every block
   - Rolling checksum algorithm requires reading entire file from both sides
   - Can consume 100% CPU on one core during delta calculation

2. **Memory Usage with Large File Counts**
   - Pre-rsync 3.0: ~100 bytes per file stored in memory
   - 800,000 files = ~80MB RAM usage
   - With `-H` (hard links) and `--delete`: even higher memory usage
   - Can trigger OOM killer on large transfers

3. **I/O Bottleneck on Destination**
   - Destination must read entire file to compute block checksums
   - For VM images on BTRFS with compression: 10+ hours vs 3 hours with `--whole-file`
   - Source: "Backing up VM images with --whole-file takes ~3 hours. Without it, killed rsync after 10 hours"

### 1.2 Single-Threaded Architecture

**Core Limitations:**
- Rsync is fundamentally single-threaded
- Cannot parallelize checksum computation across multiple CPU cores
- File transfers are sequential by default
- No built-in support for parallel stream processing

**Impact on Performance:**
- Cannot saturate multi-gigabit links with single rsync instance
- CPU-bound operations (checksums, compression) block I/O operations
- Network waits while CPU computes checksums

### 1.3 Connection Overhead Issues

**SSH Transport Overhead:**
- Default rsync over SSH uses single TCP connection
- SSH flow control buffers are limited (historically 64KB, now 2MB in OpenSSH)
- Each file transfer incurs SSH protocol overhead
- Connection setup/teardown per file in some configurations

**File System Scanning Overhead:**
- Must scan entire source and destination file systems
- Compare file lists to detect changes
- Time complexity grows linearly with file count
- No persistent indexing - full scan required each run

### 1.4 Network Protocol Limitations

**TCP/IP Throughput Issues:**
- Single TCP connection cannot utilize full bandwidth on high-BDP paths
- Formula: Max throughput = Buffer Size / RTT
- Example: With 2MB buffer and 100ms RTT: Max = 2MB/0.1s = 20 MB/s = 160 Mbps
- On a 1 Gbps link with 100ms latency: utilizing only ~16% of available bandwidth

---

## 2. RCLONE LIMITATIONS

### 2.1 Memory Usage Patterns

**Buffer-Related Memory Consumption:**
```
Default buffer sizes:
- --buffer-size: 16MB per transfer (default)
- --drive-chunk-size: 8MB (default), can increase to 128MB+
- --vfs-read-ahead: varies by configuration
- VFS cache: can grow to multiple GB with --vfs-cache-mode full
```

**Memory Scaling Issues:**
- Each concurrent transfer (`--transfers`) allocates separate buffers
- Default 4 transfers × 16MB buffer = 64MB minimum
- With `--drive-chunk-size 128M` and 4 transfers: 512MB+ RAM usage
- VFS cache can consume unlimited RAM if not constrained

**Real-World Impact:**
- User report: "rclone is constantly communicating and using big amount of bandwidth"
- High memory usage during sync operations
- Can cause system slowdowns on memory-constrained systems

### 2.2 API Rate Limiting Issues

**Cloud Provider Limitations:**

| Provider | Rate Limit | Impact |
|----------|------------|--------|
| Google Drive | ~2-3 files/second | Small file transfers severely limited |
| Google Drive | ~40-45 MB/s per upload stream | Single file uploads capped |
| Dropbox | 60M API calls/day across all users | Shared quota limitations |
| OneDrive | Undocumented throttling | Unpredictable slowdowns |
| pCloud | ~100 transactions/second | Requires --tpslimit tuning |

**Specific Rate Limiting Problems:**

1. **Google Drive Single-File Upload Cap**
   - Maximum ~40-45 MB/s per upload stream
   - Cannot saturate 1 Gbps connection with single file
   - Multiple concurrent files needed for full bandwidth utilization
   - User report: "Consistently hitting max upload speed of ~328 Mbps (40-41 MB/s)"

2. **Small File Penalty**
   - API calls per file create overhead
   - 10KB files: 2-3 files/second = 20-30 KB/s effective throughput
   - 10,000 small files can take hours vs minutes for equivalent data size

3. **Pacer Mechanism**
   - Rclone implements internal rate limiting to avoid API bans
   - `--drive-pacer-min-sleep` defaults to 100ms between API calls
   - Aggressive rate limiting from providers can reduce transfers to single-digit GBs/day

### 2.3 Bandwidth Saturation Issues

**Why Rclone Doesn't Always Saturate Bandwidth:**

1. **Single-Stream Limitations**
   - Single file transfer uses one HTTP connection
   - Subject to TCP slow-start on each new connection
   - HTTP overhead (headers, TLS handshake) reduces effective throughput

2. **Chunked Upload Overhead**
   - Large files split into chunks for resumable uploads
   - Each chunk requires separate API call and acknowledgment
   - Chunk boundaries create artificial transfer limits

3. **Checksum Computation**
   - Rclone computes checksums for integrity verification
   - SHA-1/MD5 computation is CPU-intensive
   - For 2,566 files at 200-400MB each: 3+ hours just for validation
   - User report: "Process results in increased CPU & memory utilization"

### 2.4 VFS Layer Performance

**VFS Cache Overhead:**
- `--vfs-cache-mode full` can consume excessive disk space and RAM
- Cache invalidation requires re-downloading data
- Double-write penalty: cache + destination

**Mount Performance Issues:**
- FUSE overhead adds latency to every operation
- Directory listings require API calls (slow on cloud storage)
- File seeks may require re-downloading data

---

## 3. SCP LIMITATIONS

### 3.1 Why SCP is Deprecated

**Official Deprecation (OpenSSH 8.8+ / April 2022):**
- SCP protocol has fundamental security vulnerabilities
- No integrity verification of transferred data
- Cannot resume interrupted transfers
- Protocol design flaws cannot be fixed without breaking compatibility

**OpenSSH Migration:**
- Modern OpenSSH uses SFTP protocol internally for `scp` command
- Maintains backward compatibility but inherits SFTP limitations
- No performance improvement from the change

### 3.2 Buffer Size Limitations

**Critical Performance Bottleneck:**

| SSH Implementation | Default Buffer | Maximum Effective |
|-------------------|----------------|-------------------|
| Standard OpenSSH | 2 MB | 2 MB |
| Pre-OpenSSH 4.x | 64 KB | 64 KB |
| HPN-SSH | 2 MB | 64-128 MB |

**Impact on Throughput:**
```
Formula: Max Throughput = Buffer Size / RTT

Examples with 2MB buffer:
- RTT 10ms: 2MB/0.01s = 200 MB/s = 1.6 Gbps
- RTT 50ms: 2MB/0.05s = 40 MB/s = 320 Mbps
- RTT 100ms: 2MB/0.1s = 20 MB/s = 160 Mbps
- RTT 200ms: 2MB/0.2s = 10 MB/s = 80 Mbps
```

**Real-World Performance (ESNet Data):**
- RTT = 88ms, network capacity = 100Gbps
- scp/sftp/rsync: 32 Mbps (0.032% utilization)
- HTTP (curl/wget): 5.2 Gbps (100x faster)
- Globus (4 streams): 9.5 Gbps (300x faster)

### 3.3 OpenSSH Performance Issues

**Encryption Overhead:**
- Single-threaded encryption/decryption
- AES cipher can bottleneck on CPU at high speeds
- No hardware acceleration for all cipher modes

**Flow Control Problems:**
- SSH implements application-layer flow control
- Additional layer on top of TCP flow control
- Buffer size limits are hardcoded in many implementations

**Connection Per File:**
- Each file transfer requires separate SSH channel
- Channel setup/teardown overhead
- No pipelining of file operations

### 3.4 Lack of Modern Features

**Missing Capabilities:**
- No delta transfer (always copies full files)
- No compression option
- No parallel transfer support
- No resume capability
- No integrity verification

---

## 4. COMMON BOTTLENECKS ACROSS ALL TOOLS

### 4.1 TCP Slow-Start Impact

**The Slow-Start Problem:**
- TCP congestion window starts small (typically 10 segments = ~14KB)
- Exponential growth until congestion threshold
- On high-BDP paths, slow-start dominates transfer time for files < 5-10MB

**Mathematical Analysis:**
```
Time to reach congestion window of size N:
- Traditional slow-start: log2(N/initial_cwnd) RTTs
- For 1300 packets: ~7 RTTs to reach full speed
- At 100ms RTT: 700ms just to ramp up

Impact on small files:
- 64KB file, 56ms RTT: 264ms (new connection) vs 96ms (existing)
- 275% improvement possible by avoiding slow-start
```

**Tool-Specific Impact:**
- rsync: Each new connection to daemon incurs slow-start
- rclone: Each API connection has TLS + TCP slow-start
- scp: Each file transfer = new SSH channel = slow-start penalty

### 4.2 Small Buffer Sizes

**Buffer Size Comparison:**

| Tool | Default Buffer | Configurable | Notes |
|------|----------------|--------------|-------|
| rsync (SSH) | 2 MB (via SSH) | Limited | Depends on SSH implementation |
| rclone | 16 MB | Yes (--buffer-size) | Per transfer buffer |
| scp | 2 MB | No | Hardcoded in OpenSSH |

**Bandwidth-Delay Product (BDP) Requirements:**
```
BDP = Bandwidth (bytes/sec) × RTT (seconds)

Examples:
- 1 Gbps × 50ms = 62.5 MB minimum buffer needed
- 10 Gbps × 100ms = 125 MB minimum buffer needed
- 100 Gbps × 200ms = 2.5 GB minimum buffer needed

All three tools fall far short of these requirements!
```

### 4.3 Lack of Parallel Connections

**Single-Connection Limitations:**
- Single TCP connection cannot exceed BDP/BufferSize throughput
- Even with optimal tuning, one connection is insufficient for modern networks

**Parallel Transfer Benefits:**
```
Aggregate Throughput = N × (Single Connection Throughput)

Where N = number of parallel connections

Example (ESNet data):
- Single connection: 32 Mbps
- 4 parallel streams (Globus): 9.5 Gbps
- Improvement: ~300x
```

**Tool Parallelism Support:**

| Tool | Parallel Files | Parallel Streams per File | Notes |
|------|---------------|---------------------------|-------|
| rsync | No (native) | No | Can run multiple instances |
| rclone | Yes (--transfers) | Limited | Cloud-dependent |
| scp | No | No | One file at a time |

### 4.4 Encryption Overhead

**Encryption Performance Impact:**

| Cipher | Single-Core Throughput | Hardware Accelerated |
|--------|----------------------|---------------------|
| AES-256-GCM | ~1-2 Gbps | Yes (AES-NI) |
| AES-256-CTR | ~1-2 Gbps | Yes (AES-NI) |
| ChaCha20-Poly1305 | ~1-2 Gbps | No (but efficient) |
| 3DES | ~100 Mbps | No |

**Real-World Observations:**
- HPN-SSH parallel AES-CTR: 100%+ improvement over single-threaded
- Encryption is NOT the primary bottleneck on high-latency paths
- Buffer size is typically 10-100x more important than cipher choice

**None Cipher Option (HPN-SSH):**
- Authentication remains encrypted
- Bulk data transfer is unencrypted
- Useful for non-sensitive data on trusted networks
- Can provide 2x+ speedup in CPU-bound scenarios

---

## 5. PERFORMANCE COMPARISON SUMMARY

### 5.1 Maximum Achievable Throughput

| Tool | Typical Max (WAN) | Typical Max (LAN) | Bottleneck |
|------|-------------------|-------------------|------------|
| rsync | 20-100 Mbps | 1-10 Gbps | Single-threaded, SSH buffers |
| rclone | 40-400 Mbps | 1-10 Gbps | API rate limits, single streams |
| scp | 10-50 Mbps | 100 Mbps - 1 Gbps | 2MB buffer limit, no parallelism |

### 5.2 Use Case Suitability

| Use Case | Best Tool | Why |
|----------|-----------|-----|
| Large file, high latency | None of these | Use HPN-SSH, Globus, or UDT |
| Many small files | rclone (with tuning) | Parallel transfers |
| Incremental sync | rsync | Delta algorithm |
| Cloud storage | rclone | Native API support |
| One-time large transfer | rsync --whole-file | Minimal overhead |
| Secure, fast transfer | HPN-SSH | Large buffers, parallel ciphers |

---

## 6. RECOMMENDATIONS FOR HIGH-SPEED TRANSFERS

### 6.1 For Rsync:
```bash
# High-bandwidth, low-latency (skip delta algorithm)
rsync --whole-file -avz --progress source/ dest/

# Many files, parallelize with GNU Parallel
find source -type f | parallel -j 8 rsync -avz {} dest/

# Large files over WAN (use HPN-SSH)
rsync -avz -e "hpnssh -o TCPRcvBufPoll=yes" source/ dest/
```

### 6.2 For Rclone:
```bash
# Saturate bandwidth with parallel transfers
rclone copy source remote:dest \
    --transfers=16 \
    --checkers=16 \
    --drive-chunk-size=128M \
    --buffer-size=64M \
    --fast-list

# Handle small files efficiently
rclone copy source remote:dest \
    --transfers=32 \
    --tpslimit=10 \
    --fast-list
```

### 6.3 Alternative Tools for High Performance:

| Tool | Best For | Key Advantage |
|------|----------|---------------|
| HPN-SSH | Secure WAN transfers | 64-128MB buffers, parallel ciphers |
| Globus | Research/scientific | Parallel streams, automatic tuning |
| UDT (UDP-based) | Very high latency | UDP avoids TCP limitations |
| Aspera/FASP | Commercial high-speed | UDP-based, commercial support |
| bbcp | Bulk data copy | Parallel TCP streams |

---

## 7. CONCLUSIONS

### Key Findings:

1. **All three tools (rsync, rclone, scp) were designed for an era of slower networks and smaller files**

2. **The fundamental bottleneck is buffer sizing relative to bandwidth-delay product:**
   - Standard SSH: 2MB buffer
   - Required for 1Gbps/100ms: 12.5MB
   - Required for 10Gbps/100ms: 125MB
   - Gap: 6x to 60x undersized

3. **Single-threaded architecture limits CPU utilization and throughput:**
   - Cannot parallelize checksum computation
   - Cannot use multiple network streams
   - One slow operation blocks entire transfer

4. **Protocol overhead compounds the problem:**
   - TCP slow-start on every connection
   - Application-layer flow control (SSH)
   - API rate limiting (cloud storage)
   - Small default buffer sizes

5. **For modern high-speed networks, these tools require significant tuning or replacement:**
   - HPN-SSH provides 10-100x improvement for SSH-based transfers
   - Parallel transfer tools (Globus, bbcp) can saturate 10Gbps+ links
   - UDP-based protocols avoid TCP limitations entirely

### Performance Improvement Potential:

| Optimization | Potential Speedup |
|--------------|-------------------|
| HPN-SSH buffer tuning | 10-100x |
| Parallel transfers (rclone) | 2-10x |
| Skipping delta algorithm (--whole-file) | 2-5x |
| Using UDP-based tools | 10-1000x |
| Hardware-accelerated encryption | 1.5-2x |

---

*Report compiled from technical documentation, user reports, and academic research on TCP performance and file transfer protocols.*
