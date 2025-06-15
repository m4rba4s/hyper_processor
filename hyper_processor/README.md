# HyperProcessor: Runtime Application Self-Protection (RASP)

[![Version](https://img.shields.io/badge/version-2.3.0-blue.svg)](https://github.com/your/hyper_processor)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)

A lightweight, high-performance RASP library for Linux that protects applications against unauthorized library injection via `LD_PRELOAD`.

## 🎯 Features

### Core Protection
- **Real-time Detection**: Blocks unauthorized libraries at process startup
- **Whitelist-based**: Only explicitly allowed libraries can be loaded
- **Audit Mode**: Test configurations without blocking legitimate libraries
- **Zero Performance Impact**: ~6ms startup overhead, no runtime cost

### Enhanced Security (v0.2.0)
- **SHA256 Hashing**: Calculates and logs hashes of detected libraries
- **File Size Tracking**: Reports size of unauthorized libraries
- **Process Context**: Logs command line, PID, PPID for forensics
- **Structured Logging**: JSON format for SIEM integration
- **Config Validation**: Warns about insecure file permissions

### New in v2.2.0 🚀
- **Learning Mode**: Automatically discover required libraries
- **CLI Tool**: Comprehensive command-line interface
- **Prometheus Metrics**: Real-time monitoring and alerting
- **Library Verification**: SHA256 hash verification
- **Grafana Dashboard**: Pre-built monitoring dashboard

### Experimental: eBPF Integration 🔬
- **Kernel-level Protection**: Block unauthorized libraries at kernel level using eBPF
- **LSM Hooks**: Integrates with Linux Security Modules for deep system integration
- **Real-time Detection**: Monitor and block library loads before they happen
- **Zero Userspace Overhead**: All checks happen in kernel space
- **Requirements**: Linux kernel 5.7+ with BTF support and eBPF LSM enabled

## 🚀 Quick Start

```bash
# Build with CLI
cargo build --release --features cli

# Learn what libraries your app needs
./hyper-processor learn --duration 30s your_application

# Generate whitelist from learning
./hyper-processor generate --input learned_whitelist.yaml --output config.yaml

# Run with protection
./hyper-processor protect --config config.yaml your_application

# Monitor metrics
./hyper-processor monitor --bind 0.0.0.0:9100
```

## 🛠️ CLI Commands

### `learn` - Learning Mode
Automatically discover libraries used by an application:
```bash
hyper-processor learn --duration 5m --output whitelist.yaml ./myapp
```

### `monitor` - Metrics Server
Start Prometheus metrics exporter:
```bash
hyper-processor monitor --bind 0.0.0.0:9100
```

### `verify` - Library Verification
Verify library integrity:
```bash
hyper-processor verify --sha256 abc123... /path/to/lib.so
```

### `protect` - Protection Mode
Run application with RASP protection:
```bash
hyper-processor protect --audit --config rasp.yaml ./myapp
```

### `generate` - Whitelist Generator
Generate whitelist from audit logs:
```bash
hyper-processor generate --input audit.log --output whitelist.yaml
```

### `ebpf` - eBPF Kernel Protection (Experimental)
Use kernel-level eBPF protection (requires root and Linux 5.7+):
```bash
# Build eBPF programs first
./scripts/build-ebpf.sh

# Build with eBPF support
cargo build --release --features cli,ebpf

# Run eBPF monitor
sudo ./target/release/hyper-processor ebpf --audit

# List detected attempts
sudo ./target/release/hyper-processor ebpf --list

# Clear detection history
sudo ./target/release/hyper-processor ebpf --clear
```

## 📊 Metrics & Monitoring

### Prometheus Metrics
- `hyper_processor_blocks_total` - Total blocked library loads
- `hyper_processor_audits_total` - Total audited library loads
- `hyper_processor_library_loads{library,status}` - Library load attempts
- `hyper_processor_unauthorized_loads{library,action}` - Unauthorized attempts

### Grafana Dashboard
Import `