# Network-Level Optimizations for Maximum File Transfer Speed

## Executive Summary

This guide provides actionable recommendations for maximizing file transfer speeds through TCP optimization, alternative transport protocols, multi-connection strategies, and kernel-level tuning.

---

## 1. TCP Optimization

### 1.1 Socket Options

#### Essential Socket Options for High-Speed Transfers

```c
// C/C++ Socket Configuration for Maximum Throughput
#include <sys/socket.h>
#include <netinet/tcp.h>
#include <netinet/in.h>
#include <fcntl.h>

int configure_high_speed_socket(int sockfd) {
    int yes = 1;
    int buffer_size = 134217728;  // 128MB buffer

    // TCP_NODELAY - Disable Nagle's algorithm for low latency
    // Critical for: Interactive applications, small frequent writes
    if (setsockopt(sockfd, IPPROTO_TCP, TCP_NODELAY, &yes, sizeof(yes)) < 0) {
        perror("TCP_NODELAY failed");
        return -1;
    }

    // SO_SNDBUF - Send buffer size
    // Formula: 2-3x Bandwidth Delay Product (BDP)
    // For 10Gbps @ 100ms RTT: ~125MB
    if (setsockopt(sockfd, SOL_SOCKET, SO_SNDBUF, &buffer_size, sizeof(buffer_size)) < 0) {
        perror("SO_SNDBUF failed");
        return -1;
    }

    // SO_RCVBUF - Receive buffer size
    if (setsockopt(sockfd, SOL_SOCKET, SO_RCVBUF, &buffer_size, sizeof(buffer_size)) < 0) {
        perror("SO_RCVBUF failed");
        return -1;
    }

    // TCP_QUICKACK - Disable delayed ACKs
    // Reduces latency but increases ACK overhead
    if (setsockopt(sockfd, IPPROTO_TCP, TCP_QUICKACK, &yes, sizeof(yes)) < 0) {
        perror("TCP_QUICKACK failed");
        return -1;
    }

    // TCP_CORK - Cork data for fewer packets (opposite of NODELAY)
    // Use for bulk transfers - sends full segments
    // int cork = 1;
    // setsockopt(sockfd, IPPROTO_TCP, TCP_CORK, &cork, sizeof(cork));

    // SO_KEEPALIVE - Detect dead connections
    if (setsockopt(sockfd, SOL_SOCKET, SO_KEEPALIVE, &yes, sizeof(yes)) < 0) {
        perror("SO_KEEPALIVE failed");
        return -1;
    }

    return 0;
}
```

#### Python Socket Configuration

```python
import socket

def configure_high_speed_socket(sock: socket.socket, 
                                 sndbuf: int = 134217728,
                                 rcvbuf: int = 134217728) -> None:
    """
    Configure socket for maximum file transfer throughput.

    Args:
        sock: Socket to configure
        sndbuf: Send buffer size (default 128MB)
        rcvbuf: Receive buffer size (default 128MB)
    """
    # Disable Nagle's algorithm - critical for low latency
    sock.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)

    # Set buffer sizes
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_SNDBUF, sndbuf)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_RCVBUF, rcvbuf)

    # Enable keepalive
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_KEEPALIVE, 1)

    # Get actual buffer sizes (may be limited by system max)
    actual_sndbuf = sock.getsockopt(socket.SOL_SOCKET, socket.SO_SNDBUF)
    actual_rcvbuf = sock.getsockopt(socket.SOL_SOCKET, socket.SO_RCVBUF)

    print(f"Send buffer: requested={sndbuf}, actual={actual_sndbuf}")
    print(f"Recv buffer: requested={rcvbuf}, actual={actual_rcvbuf}")

# Usage
sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
configure_high_speed_socket(sock)
sock.connect(("remote_host", 8080))
```

### 1.2 TCP Window Scaling

Window scaling is **essential** for high BDP networks:

```python
# Window scaling is enabled by default in modern Linux kernels
# Verify with: sysctl net.ipv4.tcp_window_scaling

# The window scale factor allows TCP windows up to 1GB
# Without scaling: max window = 64KB
# With scaling (factor 7): max window = 64KB * 2^7 = 8MB
# With scaling (factor 14): max window = 64KB * 2^14 = 1GB

# To verify window scaling is working:
# ss -ti | grep wscale
```

**System Configuration:**
```bash
# Enable TCP window scaling (usually enabled by default)
sysctl -w net.ipv4.tcp_window_scaling=1

# Add to /etc/sysctl.conf for persistence
echo "net.ipv4.tcp_window_scaling = 1" >> /etc/sysctl.conf
```

### 1.3 Congestion Control Algorithms

#### BBR (Bottleneck Bandwidth and RTT) - RECOMMENDED

BBR is the modern choice for high-throughput transfers:

```bash
# Check available congestion control algorithms
sysctl net.ipv4.tcp_available_congestion_control

# Enable BBR
modprobe tcp_bbr
echo "tcp_bbr" >> /etc/modules-load.d/bbr.conf

# Configure BBR
cat >> /etc/sysctl.d/99-bbr.conf << 'EOF'
net.core.default_qdisc = fq
net.ipv4.tcp_congestion_control = bbr
EOF

sysctl -p /etc/sysctl.d/99-bbr.conf
```

**BBR Benefits:**
- Model-based (not loss-based) - doesn't require packet loss to detect congestion
- Achieves higher throughput on high BDP networks
- Lower latency by keeping queues short
- Better fairness between flows

#### Algorithm Comparison

| Algorithm | Best For | Throughput | Latency | Fairness |
|-----------|----------|------------|---------|----------|
| **BBR** | High BDP, modern networks | Excellent | Low | Good |
| **CUBIC** | General purpose, default | Good | Medium | Good |
| **HTCP** | Very high-speed networks | Very Good | Medium | Fair |
| **Westwood** | Wireless/lossy networks | Good | Medium | Good |
| **Reno** | Legacy compatibility | Poor | High | Good |

### 1.4 High BDP Network Configuration

**Bandwidth Delay Product (BDP) Formula:**
```
BDP (bytes) = Bandwidth (bits/sec) / 8 * RTT (seconds)

Example: 10 Gbps * 100ms RTT
= (10,000,000,000 / 8) * 0.1
= 125,000,000 bytes = ~119 MB
```

**Complete High BDP Configuration:**

```bash
# /etc/sysctl.d/99-high-bdp.conf

# ============================================
# TCP Buffer Sizes for High BDP Networks
# ============================================
# Format: min default max (bytes)
# For 10Gbps @ 100ms: BDP = 125MB, set max to 256-512MB

net.ipv4.tcp_rmem = 4096 87380 536870912    # 512MB max
net.ipv4.tcp_wmem = 4096 65536 536870912    # 512MB max

net.core.rmem_default = 16777216             # 16MB default
net.core.rmem_max = 536870912                # 512MB max
net.core.wmem_default = 16777216             # 16MB default
net.core.wmem_max = 536870912                # 512MB max

# Enable auto-tuning with large limits
net.ipv4.tcp_moderate_rcvbuf = 1

# ============================================
# TCP Window and Performance Options
# ============================================
net.ipv4.tcp_window_scaling = 1
net.ipv4.tcp_workaround_signed_windows = 1
net.ipv4.tcp_timestamps = 1
net.ipv4.tcp_sack = 1
net.ipv4.tcp_fack = 1

# ============================================
# Congestion Control
# ============================================
net.core.default_qdisc = fq
net.ipv4.tcp_congestion_control = bbr

# ============================================
# Connection Handling
# ============================================
net.core.netdev_max_backlog = 65535
net.ipv4.tcp_max_syn_backlog = 65535
net.core.somaxconn = 65535
net.ipv4.tcp_max_orphans = 262144

# ============================================
# Memory Limits
# ============================================
# Increase TCP memory pool (pages)
net.ipv4.tcp_mem = 786432 1048576 26777216
```

---

## 2. Alternative Transport Protocols

### 2.1 QUIC Protocol

**Benefits over TCP:**
- 0-RTT connection establishment (vs 1-RTT for TCP+TLS)
- Stream multiplexing without head-of-line blocking
- Built-in encryption (TLS 1.3)
- Connection migration (IP address changes don't break connection)
- User-space implementation (faster evolution)

**When to use QUIC:**
- Web applications (HTTP/3)
- Mobile applications with network switching
- High-latency or lossy networks
- When you control both client and server

**QUIC Implementation (Python with aioquic):**

```python
# pip install aioquic

from aioquic.asyncio import QuicConnectionProtocol, serve
from aioquic.quic.configuration import QuicConfiguration

async def run_quic_server(host: str, port: int):
    configuration = QuicConfiguration(
        is_client=False,
        alpn_protocols=["h3"],
        max_datagram_frame_size=65536,
    )

    # Load TLS certificates
    configuration.load_cert_chain("cert.pem", "key.pem")

    # QUIC-specific optimizations
    configuration.initial_max_data = 10485760  # 10MB
    configuration.initial_max_stream_data_bidi_local = 1048576  # 1MB
    configuration.initial_max_stream_data_bidi_remote = 1048576  # 1MB

    await serve(
        host,
        port,
        configuration=configuration,
        create_protocol=FileTransferProtocol,
    )
```

### 2.2 UDT (UDP-based Data Transfer)

UDT is designed specifically for high-speed WAN transfers:

**Key Features:**
- Built on UDP but provides reliable delivery
- Designed for high BDP networks (10Gbps+)
- Can achieve 90%+ bandwidth utilization
- Fair bandwidth sharing between flows

**UDT Installation and Usage:**

```bash
# Install UDT library
# Ubuntu/Debian
sudo apt-get install libudt-dev

# Or build from source
git clone https://github.com/esnet/udt.git
cd udt
make
```

**UDT C++ Example:**

```cpp
#include <udt.h>
#include <iostream>

// Initialize UDT
UDT::startup();

// Create UDT socket
UDTSOCKET serv = UDT::socket(AF_INET, SOCK_STREAM, 0);

// Configure for high throughput
int snd_buf = 134217728;  // 128MB
int rcv_buf = 134217728;  // 128MB
UDT::setsockopt(serv, 0, UDT_SNDBUF, &snd_buf, sizeof(int));
UDT::setsockopt(serv, 0, UDT_RCVBUF, &rcv_buf, sizeof(int));

// Set maximum bandwidth (0 = unlimited)
int64_t max_bw = 0;
UDT::setsockopt(serv, 0, UDT_MAXBW, &max_bw, sizeof(int64_t));

// Bind and listen
sockaddr_in serv_addr;
serv_addr.sin_family = AF_INET;
serv_addr.sin_port = htons(9000);
serv_addr.sin_addr.s_addr = INADDR_ANY;
UDT::bind(serv, (sockaddr*)&serv_addr, sizeof(serv_addr));
UDT::listen(serv, 10);

// Accept connections
UDTSOCKET client;
sockaddr_in client_addr;
int addrlen = sizeof(client_addr);
client = UDT::accept(serv, (sockaddr*)&client_addr, &addrlen);

// Send file
const char* filename = "large_file.bin";
UDT::sendfile(client, filename, 0, file_size, 64000);  // 64KB block size

// Cleanup
UDT::close(client);
UDT::close(serv);
UDT::cleanup();
```

**Performance Comparison (UDT vs TCP):**
- 10Gbps WAN with 100ms RTT: UDT achieves 9+ Gbps, TCP (CUBIC) achieves 2-4 Gbps
- 40Gbps networks: UDT scales well, TCP struggles

### 2.3 When to Use UDP vs TCP

| Scenario | Recommended Protocol | Reason |
|----------|---------------------|--------|
| Bulk file transfer over WAN | **UDT** | Designed for high BDP |
| Web/mobile apps | **QUIC** | Fast setup, mobility |
| Real-time streaming | **UDP** | Low latency, tolerates loss |
| General internet | **TCP/BBR** | Compatibility, reliability |
| Data center internal | **TCP/BBR** or **RDMA** | Low latency, high throughput |
| Lossy networks | **QUIC** or **BBR** | Better loss recovery |

---

## 3. Multi-Connection Strategies

### 3.1 Why Multiple Connections Beat Single Connection

**The Problem with Single TCP Connection:**
1. TCP slow start limits initial throughput
2. Single packet loss stalls entire transfer
3. Congestion control affects all data
4. Can't fully utilize high BDP links

**Benefits of Multiple Connections:**
- Aggregate throughput from multiple slow-start windows
- Parallel error recovery
- Better bandwidth utilization on high BDP links
- Fair sharing with other traffic

**Optimal Number of Connections:**
```
For 10Gbps @ 100ms RTT:
- Single connection with 128MB buffer: ~8-9 Gbps
- 4 connections with 32MB each: ~9.5+ Gbps
- 8 connections with 16MB each: ~9.8+ Gbps

General rule: 4-16 parallel streams depending on BDP
```

### 3.2 Multi-Connection Implementation

```python
import socket
import threading
from concurrent.futures import ThreadPoolExecutor, as_completed
import os

class ParallelFileTransfer:
    def __init__(self, host: str, port: int, num_connections: int = 8):
        self.host = host
        self.port = port
        self.num_connections = num_connections
        self.chunk_size = 65536  # 64KB

    def _create_optimized_socket(self) -> socket.socket:
        """Create socket with optimal settings for file transfer."""
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)

        # Disable Nagle's algorithm
        sock.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)

        # Set large buffers (system max may limit this)
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_SNDBUF, 33554432)  # 32MB
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_RCVBUF, 33554432)  # 32MB

        return sock

    def _transfer_chunk(self, chunk_id: int, offset: int, size: int, 
                        filename: str) -> tuple:
        """Transfer a single chunk of the file."""
        sock = self._create_optimized_socket()

        try:
            sock.connect((self.host, self.port))

            # Send chunk request: "CHUNK <id> <offset> <size>\n"
            request = f"CHUNK {chunk_id} {offset} {size}\n"
            sock.sendall(request.encode())

            # Receive data
            received = 0
            with open(filename, 'r+b') as f:
                f.seek(offset)
                while received < size:
                    data = sock.recv(min(self.chunk_size, size - received))
                    if not data:
                        break
                    f.write(data)
                    received += len(data)

            return (chunk_id, received, None)

        except Exception as e:
            return (chunk_id, 0, e)
        finally:
            sock.close()

    def download_file(self, remote_file: str, local_file: str, 
                      file_size: int) -> bool:
        """
        Download file using multiple parallel connections.

        Args:
            remote_file: Path to remote file
            local_file: Path to save locally
            file_size: Total file size in bytes
        """
        # Create local file
        with open(local_file, 'wb') as f:
            f.truncate(file_size)

        # Calculate chunk sizes
        chunk_size = file_size // self.num_connections
        chunks = []

        for i in range(self.num_connections):
            offset = i * chunk_size
            size = chunk_size if i < self.num_connections - 1 else                    file_size - offset
            chunks.append((i, offset, size))

        # Transfer chunks in parallel
        completed = 0
        failed = []

        with ThreadPoolExecutor(max_workers=self.num_connections) as executor:
            futures = {
                executor.submit(self._transfer_chunk, cid, off, sz, local_file): cid
                for cid, off, sz in chunks
            }

            for future in as_completed(futures):
                chunk_id, bytes_recv, error = future.result()
                if error:
                    failed.append((chunk_id, error))
                else:
                    completed += 1
                    print(f"Chunk {chunk_id}: {bytes_recv} bytes received")

        if failed:
            print(f"Failed chunks: {failed}")
            return False

        return True
```

### 3.3 Avoiding Connection Synchronization

**Problem:** Multiple TCP connections can synchronize their congestion windows, causing throughput collapse.

**Solutions:**

```python
import random
import time

class DesynchronizedTransfer:
    def __init__(self):
        self.connections = []
        self.base_delay = 0.001  # 1ms base

    def _stagger_connection_starts(self, num_connections: int):
        """Stagger connection starts to avoid synchronization."""
        for i in range(num_connections):
            # Random delay between 0-50ms
            delay = random.uniform(0, 0.05)
            time.sleep(delay)
            self._start_connection(i)

    def _apply_random_buffer_sizes(self):
        """Use slightly different buffer sizes per connection."""
        base_size = 33554432  # 32MB
        for conn in self.connections:
            # Vary by ±10%
            variation = random.uniform(0.9, 1.1)
            buf_size = int(base_size * variation)
            conn.socket.setsockopt(
                socket.SOL_SOCKET, socket.SO_SNDBUF, buf_size
            )
```

### 3.4 Fair Bandwidth Sharing

```python
class FairBandwidthManager:
    """Ensure fair sharing between parallel connections."""

    def __init__(self, total_bandwidth: float, num_connections: int):
        """
        Args:
            total_bandwidth: Total bandwidth in bytes/sec
            num_connections: Number of parallel connections
        """
        self.total_bandwidth = total_bandwidth
        self.num_connections = num_connections
        self.fair_share = total_bandwidth / num_connections

    def get_rate_limit(self, connection_id: int) -> float:
        """Get rate limit for a specific connection."""
        # Allow burst up to 1.5x fair share
        return self.fair_share * 1.5

    def adapt_to_network(self, measured_rtt: float, loss_rate: float):
        """Adapt bandwidth allocation based on network conditions."""
        if loss_rate > 0.01:  # >1% loss
            # Reduce total bandwidth
            self.total_bandwidth *= 0.9
        elif measured_rtt < 0.01:  # <10ms RTT
            # Can increase
            self.total_bandwidth *= 1.05
```

---

## 4. Kernel/OS Level Optimizations

### 4.1 Sysctl Tuning Parameters

**Complete sysctl Configuration for File Transfer Hosts:**

```bash
# /etc/sysctl.d/99-file-transfer.conf

# ============================================
# Core Network Settings
# ============================================

# Increase system file descriptor limits
fs.file-max = 2097152

# Increase inotify limits for file monitoring
fs.inotify.max_user_instances = 8192
fs.inotify.max_user_watches = 524288

# ============================================
# TCP Buffer Optimization
# ============================================

# TCP read buffer: min default max (bytes)
# For 10Gbps @ 100ms: BDP=125MB, set max to 256-512MB
net.ipv4.tcp_rmem = 4096 87380 536870912

# TCP write buffer: min default max (bytes)
net.ipv4.tcp_wmem = 4096 65536 536870912

# Socket buffer maximums
net.core.rmem_max = 536870912
net.core.wmem_max = 536870512
net.core.rmem_default = 16777216
net.core.wmem_default = 16777216

# Enable TCP buffer auto-tuning
net.ipv4.tcp_moderate_rcvbuf = 1

# ============================================
# TCP Performance Features
# ============================================

# Enable window scaling (essential for high BDP)
net.ipv4.tcp_window_scaling = 1

# Enable timestamps (needed for RTT measurements)
net.ipv4.tcp_timestamps = 1

# Enable selective acknowledgments (faster recovery)
net.ipv4.tcp_sack = 1

# Enable forward acknowledgment
net.ipv4.tcp_fack = 1

# Enable TCP Fast Open (0-RTT for repeated connections)
# 1 = client, 2 = server, 3 = both
net.ipv4.tcp_fastopen = 3

# ============================================
# Congestion Control
# ============================================

# Use fq (fair queuing) qdisc for BBR
net.core.default_qdisc = fq

# Use BBR congestion control
net.ipv4.tcp_congestion_control = bbr

# ============================================
# Connection Handling
# ============================================

# Maximum connections in listen backlog
net.core.somaxconn = 65535
net.core.netdev_max_backlog = 65535
net.ipv4.tcp_max_syn_backlog = 65535

# Allow more orphaned sockets
net.ipv4.tcp_max_orphans = 262144

# Local port range
net.ipv4.ip_local_port_range = 1024 65535

# Reuse TIME_WAIT sockets
net.ipv4.tcp_tw_reuse = 1

# Reduce TIME_WAIT duration
net.ipv4.tcp_fin_timeout = 15

# ============================================
# Memory Management
# ============================================

# TCP memory pool (pages): min pressure max
net.ipv4.tcp_mem = 786432 1048576 26777216

# Virtual memory settings
vm.swappiness = 10
vm.dirty_ratio = 15
vm.dirty_background_ratio = 5

# ============================================
# MTU/MSS Optimization
# ============================================

# Enable MTU probing
net.ipv4.tcp_mtu_probing = 1
net.ipv4.tcp_base_mss = 1024

# ============================================
# Security (SYN flood protection)
# ============================================

net.ipv4.tcp_syncookies = 1
net.ipv4.tcp_syn_retries = 2
net.ipv4.tcp_synack_retries = 2
```

**Apply Configuration:**
```bash
# Apply settings immediately
sysctl -p /etc/sysctl.d/99-file-transfer.conf

# Verify settings
sysctl -a | grep tcp_rmem
sysctl -a | grep tcp_wmem
sysctl net.ipv4.tcp_congestion_control
```

### 4.2 NIC Offloading

**Enable Hardware Offloading:**

```bash
# Check current offload settings
ethtool -k eth0

# Enable transmit offloads
ethtool -K eth0 tso on      # TCP Segmentation Offload
ethtool -K eth0 gso on      # Generic Segmentation Offload
ethtool -K eth0 gro on      # Generic Receive Offload

# Enable checksum offloading
ethtool -K eth0 tx-checksum-ip-generic on
ethtool -K eth0 rx-checksum on

# Enable Large Receive Offload (use with caution)
# ethtool -K eth0 lro on

# Verify settings
ethtool -k eth0 | grep -E "tcp-segmentation|generic-segmentation|generic-receive|large-receive"
```

**Offload Types Explained:**

| Offload | Direction | Benefit | Use Case |
|---------|-----------|---------|----------|
| **TSO** | TX | NIC segments large TCP packets | High throughput |
| **GSO** | TX | Software fallback for TSO | Compatibility |
| **GRO** | RX | Merge small packets into larger | Reduce overhead |
| **LRO** | RX | Aggressive packet merging | Throughput (not for forwarding) |
| **CSO** | TX/RX | NIC handles checksums | Reduce CPU load |

### 4.3 Memory Mapping for File I/O

**Zero-Copy File Transfer with sendfile():**

```c
#include <sys/sendfile.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/stat.h>

ssize_t send_file_zero_copy(int out_fd, const char* filename) {
    int in_fd = open(filename, O_RDONLY);
    if (in_fd < 0) {
        perror("open");
        return -1;
    }

    // Get file size
    struct stat stat_buf;
    fstat(in_fd, &stat_buf);
    off_t offset = 0;

    // sendfile() - kernel-space copy from file to socket
    // No data copies to user space!
    ssize_t sent = sendfile(out_fd, in_fd, &offset, stat_buf.st_size);

    close(in_fd);
    return sent;
}
```

**Python Zero-Copy with sendfile:**

```python
import os
import socket

def sendfile_zero_copy(sock: socket.socket, filename: str) -> int:
    """
    Send file using zero-copy sendfile.

    Returns number of bytes sent.
    """
    with open(filename, 'rb') as f:
        # Get file descriptor
        fd = f.fileno()

        # Get file size
        file_size = os.fstat(fd).st_size

        # Use sendfile for zero-copy transfer
        # Data goes directly from page cache to socket
        offset = 0
        total_sent = 0

        while offset < file_size:
            try:
                sent = os.sendfile(sock.fileno(), fd, offset, 
                                   file_size - offset)
                if sent == 0:
                    break
                offset += sent
                total_sent += sent
            except BlockingIOError:
                # Socket buffer full, wait and retry
                import select
                select.select([], [sock], [], 1)
                continue

    return total_sent
```

**Memory-Mapped Files:**

```python
import mmap
import os

def process_file_mmap(filename: str, callback) -> None:
    """
    Process file using memory mapping.
    Useful for parsing/processing without explicit reads.
    """
    with open(filename, 'rb') as f:
        # Memory map the file
        with mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ) as mm:
            # Access file as if it's in memory
            # No explicit read() calls needed
            callback(mm)

# Example: Fast file copy with mmap
def fast_copy_mmap(src: str, dst: str) -> None:
    with open(src, 'rb') as fsrc:
        with open(dst, 'wb') as fdst:
            # Get file size
            size = os.fstat(fsrc.fileno()).st_size
            fdst.truncate(size)

            # Memory map both files
            with mmap.mmap(fsrc.fileno(), 0, access=mmap.ACCESS_READ) as src_mm:
                with mmap.mmap(fdst.fileno(), 0, access=mmap.ACCESS_WRITE) as dst_mm:
                    # Copy via memory (kernel handles efficiently)
                    dst_mm[:] = src_mm[:]
```

**splice() for Pipe-to-Socket Zero-Copy:**

```c
#include <fcntl.h>
#include <unistd.h>

// splice() moves data between file descriptors without copying to user space
// Requires one fd to be a pipe

int pipefd[2];
pipe(pipefd);

// Read from file into pipe (kernel space only)
ssize_t n = splice(file_fd, NULL, pipefd[1], NULL, len, SPLICE_F_MOVE);

// Write from pipe to socket (kernel space only)
splice(pipefd[0], NULL, socket_fd, NULL, n, SPLICE_F_MOVE);
```

### 4.4 Kernel Bypass Technologies

**DPDK (Data Plane Development Kit):**

```bash
# Install DPDK
# Ubuntu/Debian
sudo apt-get install dpdk dpdk-dev

# Or build from source
git clone https://github.com/DPDK/dpdk.git
cd dpdk
meson setup build
ninja -C build
sudo ninja -C build install
```

**Configure Hugepages (required for DPDK):**

```bash
# Reserve 2MB hugepages
echo 1024 > /sys/kernel/mm/hugepages/hugepages-2048kB/nr_hugepages

# Or reserve 1GB hugepages
echo 4 > /sys/kernel/mm/hugepages/hugepages-1048576kB/nr_hugepages

# Mount hugepages
mkdir -p /mnt/huge
mount -t hugetlbfs nodev /mnt/huge

# Make persistent
echo "vm.nr_hugepages = 1024" >> /etc/sysctl.conf
```

**RDMA (Remote Direct Memory Access):**

```bash
# Install RDMA libraries
sudo apt-get install rdma-core libibverbs-dev librdmacm-dev

# Check RDMA devices
ibv_devices

# Check device capabilities
ibv_devinfo

# Test RDMA connection
ib_write_bw  # Server
ib_write_bw <server_ip>  # Client
```

**RDMA Performance Characteristics:**
- Latency: 1-2 microseconds (vs 10-50µs for kernel TCP)
- Throughput: Can saturate 100Gbps+ with single core
- CPU usage: Near zero during transfer

---

## 5. Quick Reference: Performance Checklist

### Pre-Transfer Checklist

```bash
# 1. Verify congestion control
sysctl net.ipv4.tcp_congestion_control
# Should show: bbr

# 2. Check buffer sizes
sysctl net.ipv4.tcp_rmem
sysctl net.ipv4.tcp_wmem
# Max should be 2-3x your BDP

# 3. Verify window scaling
sysctl net.ipv4.tcp_window_scaling
# Should be: 1

# 4. Check NIC offloads
ethtool -k eth0 | grep -E "tcp-segmentation|generic-segmentation"
# Should show: on

# 5. Verify qdisc
sysctl net.core.default_qdisc
# Should show: fq (for BBR)

# 6. Check CPU frequency scaling
cpufreq-info | grep "current policy"
# Should be: performance

# 7. Verify IRQ affinity
cat /proc/interrupts | grep eth0
# Should be distributed across cores
```

### Benchmarking Commands

```bash
# Test TCP throughput with iperf3
# Server
iperf3 -s

# Client (single stream)
iperf3 -c server_ip -t 60

# Client (multiple streams)
iperf3 -c server_ip -t 60 -P 8

# Client (with specific window size)
iperf3 -c server_ip -t 60 -w 32M

# Monitor network in real-time
iftop -i eth0
nload eth0

# Check TCP connection details
ss -ti
ss -ti | grep -E "cwnd|bytes_acked|rcv_space"

# Monitor packet drops
netstat -s | grep -i drop
watch -n 1 'netstat -s | grep -i drop'
```

---

## 6. Summary of Recommendations

### For Maximum Throughput (10Gbps+ WAN):

1. **Use BBR congestion control** - Significantly outperforms CUBIC on high BDP
2. **Set buffers to 2-3x BDP** - 256-512MB for 10Gbps @ 100ms
3. **Use 4-16 parallel connections** - Better utilization than single connection
4. **Enable all NIC offloads** - TSO, GSO, GRO reduce CPU overhead
5. **Consider UDT for dedicated WAN links** - Designed for high-speed transfers
6. **Use zero-copy APIs** - sendfile(), splice(), memory-mapped files
7. **Enable TCP Fast Open** - Reduces connection setup time

### For Minimum Latency:

1. **Use kernel bypass (DPDK/RDMA)** - Sub-microsecond latency
2. **Disable delayed ACKs** - TCP_QUICKACK
3. **Use busy polling** - Instead of interrupt-driven
4. **CPU isolation** - Dedicate cores to networking
5. **Consider QUIC** - 0-RTT connection establishment

### Configuration Priority:

| Priority | Setting | Expected Gain |
|----------|---------|---------------|
| 1 | BBR congestion control | 2-4x throughput |
| 2 | Increase buffer sizes | 2-3x throughput |
| 3 | Multiple connections | 1.5-2x throughput |
| 4 | NIC offloads | 20-50% CPU reduction |
| 5 | Zero-copy I/O | 10-30% throughput |
| 6 | Kernel bypass | 10x latency reduction |
