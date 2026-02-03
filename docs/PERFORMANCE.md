# fastsync Performance Tuning Guide

This guide provides comprehensive instructions for optimizing fastsync performance across different network conditions and hardware configurations.

---

## Table of Contents

- [Understanding Network Performance](#understanding-network-performance)
- [Network Optimization](#network-optimization)
- [TCP Tuning Parameters](#tcp-tuning-parameters)
- [Chunk Size Selection](#chunk-size-selection)
- [Parallel Streams](#parallel-streams)
- [Benchmarking](#benchmarking)
- [Common Bottlenecks](#common-bottlenecks)
- [Scenario-Specific Tuning](#scenario-specific-tuning)

---

## Understanding Network Performance

### Key Metrics

| Metric | Description | Target |
|--------|-------------|--------|
| **Bandwidth** | Maximum data rate (bps) | Match link capacity |
| **Latency (RTT)** | Round-trip time | Minimize for responsiveness |
| **Jitter** | Latency variation | Minimize for stability |
| **Packet Loss** | Dropped packets | < 0.1% for TCP |
| **BDP** | Bandwidth-Delay Product | Critical for tuning |

### Bandwidth-Delay Product (BDP)

BDP determines how much data must be "in flight" to maximize throughput:

```
BDP = Bandwidth (bits/sec) × RTT (seconds) / 8 = Bytes
```

**Example:**
- 1 Gbps link with 100ms RTT
- BDP = 1,000,000,000 × 0.1 / 8 = 12.5 MB

Your system must be able to buffer at least the BDP to achieve full throughput.

### Network Profiles

| Profile | Bandwidth | Latency | Use Case |
|---------|-----------|---------|----------|
| LAN | 1-100 Gbps | < 1ms | Data center, local network |
| MAN | 1-10 Gbps | 1-10ms | Metro area |
| WAN | 100 Mbps - 1 Gbps | 10-100ms | Cross-country |
| Internet | 10-100 Mbps | 50-200ms | General internet |
| Satellite | 1-10 Mbps | 500-700ms | Satellite links |
| Mobile | 1-100 Mbps | 30-100ms | Cellular networks |

---

## Network Optimization

### Linux Network Stack Tuning

#### TCP Buffer Sizes

```bash
# Check current values
sysctl net.ipv4.tcp_rmem
sysctl net.ipv4.tcp_wmem

# Set optimal values (for 10 Gbps, 100ms RTT)
# Format: min default max (in bytes)
sudo sysctl -w net.ipv4.tcp_rmem="4096 87380 12582912"
sudo sysctl -w net.ipv4.tcp_wmem="4096 65536 12582912"

# Make persistent
echo "net.ipv4.tcp_rmem = 4096 87380 12582912" | sudo tee -a /etc/sysctl.conf
echo "net.ipv4.tcp_wmem = 4096 65536 12582912" | sudo tee -a /etc/sysctl.conf
```

#### TCP Congestion Control

```bash
# List available algorithms
sysctl net.ipv4.tcp_available_congestion_control

# Check current
sysctl net.ipv4.tcp_congestion_control

# Use BBR (recommended for high-BDP links)
sudo sysctl -w net.ipv4.tcp_congestion_control=bbr

# Alternative: CUBIC (default on most systems)
sudo sysctl -w net.ipv4.tcp_congestion_control=cubic
```

**Congestion Control Comparison:**

| Algorithm | Best For | Characteristics |
|-----------|----------|-----------------|
| **BBR** | High BDP, variable loss | Probes bandwidth, low latency |
| **CUBIC** | General purpose | Balanced, widely supported |
| **Reno** | Low BDP | Conservative, stable |
| **Westwood** | Wireless | Adapts to packet loss |

#### Additional TCP Settings

```bash
# Enable TCP Fast Open
sudo sysctl -w net.ipv4.tcp_fastopen=3

# Increase maximum backlog
sudo sysctl -w net.core.netdev_max_backlog=65536

# Increase socket listen backlog
sudo sysctl -w net.core.somaxconn=65535

# Enable TCP window scaling
sudo sysctl -w net.ipv4.tcp_window_scaling=1

# Reduce TCP slow start after idle
sudo sysctl -w net.ipv4.tcp_slow_start_after_idle=0

# Increase local port range
sudo sysctl -w net.ipv4.ip_local_port_range="1024 65535"

# Apply all changes
sudo sysctl -p
```

### Interface Configuration

#### NIC Offloading

```bash
# Check current settings
ethtool -k eth0

# Enable offloading features
sudo ethtool -K eth0 tso on  # TCP Segmentation Offload
sudo ethtool -K eth0 gso on  # Generic Segmentation Offload
sudo ethtool -K eth0 gro on  # Generic Receive Offload
sudo ethtool -K eth0 lro off # Large Receive Offload (often buggy)
```

#### IRQ Affinity

```bash
# Distribute interrupts across CPUs
sudo apt install irqbalance
sudo systemctl enable irqbalance
sudo systemctl start irqbalance

# Or manually for specific NIC
sudo ethool -C eth0 rx-usecs 50 tx-usecs 50
```

#### Ring Buffer Sizes

```bash
# Check current ring buffer
ethtool -g eth0

# Increase ring buffer (if supported)
sudo ethtool -G eth0 rx 4096 tx 4096
```

### macOS Network Tuning

```bash
# Increase TCP buffer sizes
sudo sysctl -w net.inet.tcp.sendspace=12582912
sudo sysctl -w net.inet.tcp.recvspace=12582912

# Enable TCP window scaling
sudo sysctl -w net.inet.tcp.win_scale_factor=8

# Make persistent
echo "net.inet.tcp.sendspace=12582912" | sudo tee -a /etc/sysctl.conf
echo "net.inet.tcp.recvspace=12582912" | sudo tee -a /etc/sysctl.conf
```

### Windows Network Tuning

```powershell
# Increase TCP window size
netsh interface tcp set global autotuninglevel=experimental

# Enable TCP Fast Open
netsh interface tcp set global fastopen=enabled

# Set congestion provider to CTCP (Compound TCP)
netsh interface tcp set global congestionprovider=ctcp
```

---

## TCP Tuning Parameters

### Parameter Reference Table

| Parameter | Default | Recommended | Description |
|-----------|---------|-------------|-------------|
| `tcp_rmem` | 4K/87K/6M | 4K/87K/12M+ | Receive buffer sizes |
| `tcp_wmem` | 4K/64K/6M | 4K/64K/12M+ | Send buffer sizes |
| `tcp_congestion_control` | cubic | bbr/cubic | Congestion algorithm |
| `tcp_notsent_lowat` | 4294967295 | 16384 | Reduce buffer bloat |
| `tcp_fastopen` | 0 | 3 | Enable TFO client+server |
| `tcp_tw_reuse` | 0 | 1 | Reuse TIME_WAIT sockets |
| `tcp_fin_timeout` | 60 | 30 | Reduce FIN_WAIT time |
| `tcp_keepalive_time` | 7200 | 600 | Keepalive interval |

### Quick Tuning Script

```bash
#!/bin/bash
# tune-network.sh - Network optimization for fastsync

# Detect link speed (simplified)
LINK_SPEED=$(ethtool eth0 2>/dev/null | grep Speed | awk '{print $2}')
echo "Detected link speed: $LINK_SPEED"

# Calculate BDP for 100ms RTT
case $LINK_SPEED in
    "1000Mb/s"|"1Gb/s")
        BDP=12500000  # ~12MB
        ;;
    "10000Mb/s"|"10Gb/s")
        BDP=125000000  # ~125MB
        ;;
    "25000Mb/s"|"25Gb/s")
        BDP=312500000  # ~312MB
        ;;
    "40000Mb/s"|"40Gb/s")
        BDP=500000000  # ~500MB
        ;;
    "100000Mb/s"|"100Gb/s")
        BDP=1250000000  # ~1.25GB
        ;;
    *)
        BDP=12500000  # Default to 1Gbps
        ;;
esac

echo "Setting TCP buffers for BDP: $BDP bytes"

# Apply settings
sysctl -w net.ipv4.tcp_rmem="4096 87380 $BDP"
sysctl -w net.ipv4.tcp_wmem="4096 65536 $BDP"
sysctl -w net.core.rmem_max=$BDP
sysctl -w net.core.wmem_max=$BDP
sysctl -w net.ipv4.tcp_congestion_control=bbr
sysctl -w net.ipv4.tcp_notsent_lowat=16384
sysctl -w net.ipv4.tcp_fastopen=3

echo "Network tuning complete"
```

---

## Chunk Size Selection

### Chunk Size Guidelines

| Scenario | Recommended | Rationale |
|----------|-------------|-----------|
| Small files (< 1 MB) | File size | Single chunk |
| Medium files (1-100 MB) | 4-8 MB | Balance overhead/parallelism |
| Large files (100 MB - 1 GB) | 8-16 MB | Good parallelism |
| Very large files (> 1 GB) | 16-64 MB | Maximize throughput |
| High latency (> 100ms) | 4-8 MB | More parallel chunks |
| Low latency (< 10ms) | 16-64 MB | Fewer, larger chunks |

### Dynamic Chunk Sizing

fastsync can automatically adjust chunk sizes:

```bash
# Enable dynamic chunk sizing (default)
fastsync copy --chunk-size auto ./file server:/path/

# Set range for dynamic sizing
fastsync copy --chunk-size-range 1M,64M ./file server:/path/
```

**Dynamic Sizing Algorithm:**

```
chunk_size = max(min_chunk, min(max_chunk, BDP / streams / 2))
```

### Chunk Size Benchmark

```bash
#!/bin/bash
# benchmark-chunk-size.sh

FILE="test-1gb.bin"
DEST="server:/tmp/"
SIZES=("1M" "2M" "4M" "8M" "16M" "32M" "64M")

echo "Chunk Size Benchmark"
echo "===================="

for size in "${SIZES[@]}"; do
    echo -n "Testing chunk size $size... "
    result=$(fastsync copy --chunk-size "$size" --stats "$FILE" "$DEST" 2>&1 | grep "Throughput")
    echo "$result"
done
```

---

## Parallel Streams

### Stream Count Guidelines

| Latency | Bandwidth | Recommended Streams |
|---------|-----------|---------------------|
| < 1ms | 1-10 Gbps | 4-8 |
| 1-10ms | 1-10 Gbps | 8-16 |
| 10-50ms | 100 Mbps - 1 Gbps | 16-32 |
| 50-100ms | 100 Mbps - 1 Gbps | 32-64 |
| > 100ms | < 100 Mbps | 64-128 |

### Stream Scaling Formula

```
streams = max(MIN_STREAMS, min(MAX_STREAMS, BDP / chunk_size))
```

Where:
- `MIN_STREAMS` = 4
- `MAX_STREAMS` = 128

### Configuring Streams

```bash
# Fixed number of streams
fastsync copy --streams 16 ./file server:/path/

# Auto-scaling (default)
fastsync copy --streams auto ./file server:/path/

# Set min/max for auto-scaling
fastsync copy --streams-min 8 --streams-max 32 ./file server:/path/
```

### Stream vs Chunk Trade-off

| Configuration | Best For | Characteristics |
|---------------|----------|-----------------|
| Few streams, large chunks | Low latency | Lower overhead |
| Many streams, small chunks | High latency | Better pipelining |
| Balanced | General purpose | Good default |

---

## Benchmarking

### Built-in Benchmark

```bash
# Quick benchmark
fastsync benchmark

# Detailed benchmark
fastsync benchmark --duration 60 --output benchmark.json

# Benchmark specific path
fastsync benchmark --target server:/tmp/

# Compare profiles
fastsync benchmark --profile lan --output lan.json
fastsync benchmark --profile wan --output wan.json
```

### Manual Benchmarking

```bash
#!/bin/bash
# comprehensive-benchmark.sh

SERVER="server.example.com"
TEST_FILE="benchmark-10gb.bin"
RESULTS_DIR="./benchmark-results"

mkdir -p "$RESULTS_DIR"

# Generate test file if needed
if [ ! -f "$TEST_FILE" ]; then
    dd if=/dev/urandom of="$TEST_FILE" bs=1M count=10240
fi

echo "=== fastsync Benchmark ==="
echo "File: $TEST_FILE ($(du -h "$TEST_FILE" | cut -f1))"
echo "Destination: $SERVER"
echo ""

# Test different configurations
CONFIGS=(
    "default:"
    "lan:--profile lan"
    "wan:--profile wan"
    "max-throughput:--profile max-throughput"
    "small-chunks:--chunk-size 1M"
    "large-chunks:--chunk-size 64M"
    "few-streams:--streams 4"
    "many-streams:--streams 32"
)

for config in "${CONFIGS[@]}"; do
    name="${config%%:*}"
    args="${config#*:}"
    
    echo -n "Testing $name... "
    
    start=$(date +%s.%N)
    fastsync copy $args --progress=false "$TEST_FILE" "$SERVER:/tmp/" > /dev/null 2>&1
    end=$(date +%s.%N)
    
    duration=$(echo "$end - $start" | bc)
    throughput=$(echo "scale=2; 10240 / $duration" | bc)
    
    echo "${duration}s - ${throughput} MB/s"
    echo "$name,$duration,$throughput" >> "$RESULTS_DIR/results.csv"
done

echo ""
echo "Results saved to $RESULTS_DIR/results.csv"
```

### Network Path Analysis

```bash
# Measure RTT
ping -c 10 server.example.com

# Measure bandwidth with iperf3
iperf3 -c server.example.com -t 30

# Trace network path
mtr --report --report-cycles 100 server.example.com

# Check for packet loss
netstat -s | grep -i "packet\|error\|drop"
```

### Interpreting Results

| Metric | Good | Poor | Action |
|--------|------|------|--------|
| Throughput | > 80% of link | < 50% of link | Tune buffers/streams |
| CPU Usage | < 70% | > 90% | Reduce compression, add workers |
| Memory | Stable | Growing | Enable backpressure |
| Retries | < 1% | > 5% | Check network, increase timeout |

---

## Common Bottlenecks

### 1. TCP Buffer Undersizing

**Symptoms:**
- Throughput far below link capacity
- High RTT but low bandwidth utilization

**Diagnosis:**
```bash
# Check current buffer usage
ss -tin | grep -E "rcv_space|snd_wnd"

# Monitor during transfer
watch -n 1 'ss -tin | grep -E "rcv_space|snd_wnd"'
```

**Solution:**
```bash
# Increase TCP buffers
sudo sysctl -w net.ipv4.tcp_rmem="4096 87380 12582912"
sudo sysctl -w net.ipv4.tcp_wmem="4096 65536 12582912"
```

### 2. Insufficient Parallelism

**Symptoms:**
- Throughput increases with more streams
- CPU usage low during transfer

**Solution:**
```bash
# Increase streams
fastsync copy --streams 32 ./file server:/path/

# Or use high-throughput profile
fastsync copy --profile max-throughput ./file server:/path/
```

### 3. Disk I/O Bottleneck

**Symptoms:**
- High iowait in top
- Throughput limited by disk speed

**Diagnosis:**
```bash
# Monitor disk I/O
iostat -x 1

# Check disk utilization
iotop -o
```

**Solutions:**
- Use SSD instead of HDD
- Enable read-ahead: `blockdev --setra 8192 /dev/sda`
- Use memory-mapped I/O: `--mmap-enabled`

### 4. CPU Bottleneck

**Symptoms:**
- CPU usage at 100%
- Throughput limited by processing

**Diagnosis:**
```bash
# Monitor CPU usage
htop

# Profile fastsync
perf record -g fastsync copy ./file server:/path/
perf report
```

**Solutions:**
- Disable compression: `--no-compress`
- Use faster compression: `--compression-algorithm lz4`
- Reduce checksum frequency: `--checksum-interval 100M`

### 5. Network Congestion

**Symptoms:**
- Variable throughput
- Packet loss observed
- High retransmission rate

**Diagnosis:**
```bash
# Check for retransmissions
netstat -s | grep -i retrans

# Monitor with ss
ss -tin | grep retrans
```

**Solutions:**
- Use BBR congestion control
- Enable bandwidth limiting: `--bwlimit 800M`
- Reduce streams during congestion

### 6. Small File Overhead

**Symptoms:**
- Low throughput with many small files
- High metadata operation time

**Solutions:**
```bash
# Use small-files profile
fastsync sync --profile small-files ./dir/ server:/path/

# Increase batch size
fastsync sync --batch-size 1000 ./dir/ server:/path/

# Enable metadata caching
fastsync sync --cache-metadata ./dir/ server:/path/
```

---

## Scenario-Specific Tuning

### Data Center Transfer (10 Gbps, < 1ms)

```toml
# /etc/fastsync/config.d/datacenter.toml

[defaults]
profile = "lan"
workers = 64
chunk_size = "64M"
streams = 8
buffer_size = "16M"
compression = false
checksum = false

[http]
http2_prior_knowledge = true
pool_max_idle_per_host = 100
```

**System Tuning:**
```bash
# Maximum performance
sudo sysctl -w net.ipv4.tcp_rmem="4096 87380 536870912"
sudo sysctl -w net.ipv4.tcp_wmem="4096 65536 536870912"
sudo ethtool -G eth0 rx 8192 tx 8192
```

### Cross-Country WAN (1 Gbps, 50ms)

```toml
# ~/.config/fastsync/wan.toml

[defaults]
profile = "wan"
workers = 16
chunk_size = "8M"
streams = 32
buffer_size = "4M"
compression = true
compression_level = 6
checksum = true
retries = 5
retry_delay = "5s"
```

**System Tuning:**
```bash
# Optimize for high BDP
BDP=6250000  # 1Gbps * 50ms
sudo sysctl -w net.ipv4.tcp_rmem="4096 87380 $BDP"
sudo sysctl -w net.ipv4.tcp_wmem="4096 65536 $BDP"
sudo sysctl -w net.ipv4.tcp_congestion_control=bbr
```

### Satellite Link (10 Mbps, 600ms)

```toml
# ~/.config/fastsync/satellite.toml

[defaults]
workers = 4
chunk_size = "256K"
streams = 128
buffer_size = "256K"
compression = true
compression_level = 19
bwlimit = "9M"
retries = 10
retry_delay = "30s"
timeout = "5m"
```

**System Tuning:**
```bash
# Optimize for very high BDP
BDP=750000  # 10Mbps * 600ms
sudo sysctl -w net.ipv4.tcp_rmem="4096 87380 $BDP"
sudo sysctl -w net.ipv4.tcp_wmem="4096 65536 $BDP"
sudo sysctl -w net.ipv4.tcp_congestion_control=bbr
sudo sysctl -w net.ipv4.tcp_slow_start_after_idle=0
```

### Cloud Storage (S3, Variable)

```toml
# ~/.config/fastsync/s3.toml

[defaults]
workers = 32
chunk_size = "8M"
streams = 16
compression = true

[s3]
max_concurrent_requests = 50
multipart_threshold = "100M"
multipart_chunksize = "8M"
use_accelerate_endpoint = true
```

**Best Practices:**
- Use S3 Transfer Acceleration for distant regions
- Enable multipart for files > 100 MB
- Use instance profiles instead of credentials

### Many Small Files (100K+ files)

```toml
# ~/.config/fastsync/small-files.toml

[defaults]
profile = "small-files"
workers = 64
chunk_size = "256K"
streams = 4
batch_size = 500
metadata_cache = true
progress_interval = "10s"
```

**Command Options:**
```bash
# Additional optimizations
fastsync sync \
    --profile small-files \
    --exclude "*.tmp" \
    --exclude ".git/" \
    --no-checksum \
    ./source/ s3://bucket/dest/
```

---

## Performance Checklist

Before running large transfers:

- [ ] Measure network RTT: `ping target`
- [ ] Measure bandwidth: `iperf3 -c target`
- [ ] Calculate BDP and set TCP buffers
- [ ] Enable appropriate congestion control (BBR for high BDP)
- [ ] Verify NIC offloading is enabled
- [ ] Choose appropriate profile or configure manually
- [ ] Test with a small subset first
- [ ] Monitor with `--stats` during transfer
- [ ] Verify checksums for critical data

---

For more information, see:
- [Architecture Overview](ARCHITECTURE.md)
- [Configuration Reference](CONFIGURATION.md)
