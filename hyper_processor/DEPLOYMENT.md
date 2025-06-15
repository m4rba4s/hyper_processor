# HyperProcessor RASP Deployment Guide

## Table of Contents
- [Quick Start](#quick-start)
- [Installation](#installation)
- [Configuration](#configuration)
- [Distribution-Specific Examples](#distribution-specific-examples)
- [Systemd Integration](#systemd-integration)
- [Security Best Practices](#security-best-practices)
- [Troubleshooting](#troubleshooting)

## Quick Start

```bash
# Build the library
cd hyper_processor
cargo build --release

# Test in audit mode first
HYPER_RASP_AUDIT_MODE=true \
LD_PRELOAD="$(pwd)/target/release/libhyper_processor.so" \
your_application

# Deploy to system location
sudo cp target/release/libhyper_processor.so /usr/local/lib/
sudo chmod 644 /usr/local/lib/libhyper_processor.so
```

## Installation

### From Source

```bash
# Clone and build
git clone https://github.com/your/hyper_processor.git
cd hyper_processor
cargo build --release

# Install system-wide
sudo install -m 644 target/release/libhyper_processor.so /usr/local/lib/
sudo install -m 644 examples/rasp_config.yaml /etc/hyper_processor/
sudo chmod 600 /etc/hyper_processor/rasp_config.yaml
```

### Package Installation (Future)

```bash
# Fedora/RHEL
sudo dnf install hyper_processor

# Debian/Ubuntu
sudo apt install hyper-processor

# Arch
sudo pacman -S hyper-processor
```

## Configuration

### 1. Generate Initial Whitelist

Run your application in audit mode to collect all required libraries:

```bash
# Set audit mode
export HYPER_RASP_AUDIT_MODE=true
export HYPER_RASP_CONFIG=/etc/hyper_processor/rasp_config.yaml
export LD_PRELOAD=/usr/local/lib/libhyper_processor.so
export RUST_LOG=warn

# Run your application (exercise all features)
your_application 2> audit.log

# Extract unique libraries
grep "unauthorized_library_filename" audit.log | \
  jq -r '.fields.unauthorized_library_filename' | \
  sort -u > additional_libs.txt
```

### 2. Configure rasp_config.yaml

```yaml
# /etc/hyper_processor/rasp_config.yaml
audit_mode: false  # Set to true for testing

whitelisted_filenames:
  # Add libraries from additional_libs.txt here
  - "libcustom.so.1"
  - "libapp_specific.so"
```

### 3. Environment Variables

```bash
# System-wide (/etc/environment)
HYPER_RASP_CONFIG="/etc/hyper_processor/rasp_config.yaml"

# Per-application (systemd service)
Environment="LD_PRELOAD=/usr/local/lib/libhyper_processor.so"
Environment="HYPER_RASP_CONFIG=/etc/hyper_processor/app_specific.yaml"
```

## Distribution-Specific Examples

### Fedora 41+ / RHEL 9+

Common additional libraries needed:

```yaml
whitelisted_filenames:
  # SELinux
  - "libselinux.so.1"
  
  # systemd integration
  - "libsystemd.so.0.39.0"
  - "libsystemd.so.0.38.0"  # RHEL 9
  
  # NetworkManager
  - "libnm.so.0.1.0"
  
  # SSSD (if using)
  - "libnss_sss.so.2"
  
  # Kerberos (enterprise)
  - "libkrb5.so.3.3"
  - "libgssapi_krb5.so.2.2"
```

### Ubuntu 22.04 / Debian 12

```yaml
whitelisted_filenames:
  # AppArmor
  - "libapparmor.so.1"
  
  # systemd
  - "libsystemd.so.0.32.0"
  
  # GNOME/GTK apps
  - "libgtk-3.so.0"
  - "libgdk-3.so.0"
  
  # Snap support
  - "libsquashfuse.so.0"
```

### Arch Linux

```yaml
whitelisted_filenames:
  # Latest versions (update regularly)
  - "libsystemd.so.0.40.0"
  - "libpcre2-8.so.0.14.0"
  
  # AUR helpers (if needed)
  - "libalpm.so.15"
```

## Systemd Integration

### Protecting a Single Service

Create a drop-in directory:

```bash
sudo mkdir -p /etc/systemd/system/nginx.service.d/
```

Create `/etc/systemd/system/nginx.service.d/rasp.conf`:

```ini
[Service]
Environment="LD_PRELOAD=/usr/local/lib/libhyper_processor.so"
Environment="HYPER_RASP_CONFIG=/etc/hyper_processor/nginx.yaml"
Environment="RUST_LOG=error"
```

Reload and restart:

```bash
sudo systemctl daemon-reload
sudo systemctl restart nginx
```

### Global Protection (Advanced)

Add to `/etc/ld.so.preload`:

```
/usr/local/lib/libhyper_processor.so
```

**Warning**: This affects ALL processes. Test thoroughly!

### Systemd Service for RASP Management

Create `/etc/systemd/system/hyper-processor-updater.service`:

```ini
[Unit]
Description=HyperProcessor RASP Configuration Updater
After=network.target

[Service]
Type=oneshot
ExecStart=/usr/local/bin/hyper-processor-update
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

## Security Best Practices

### 1. File Permissions

```bash
# RASP library - readable by all, writable by none
sudo chown root:root /usr/local/lib/libhyper_processor.so
sudo chmod 644 /usr/local/lib/libhyper_processor.so

# Config files - readable only by root
sudo chown root:root /etc/hyper_processor/
sudo chmod 700 /etc/hyper_processor/
sudo chmod 600 /etc/hyper_processor/*.yaml
```

### 2. SELinux Context (Fedora/RHEL)

```bash
# Set proper context
sudo semanage fcontext -a -t lib_t '/usr/local/lib/libhyper_processor\.so'
sudo restorecon -v /usr/local/lib/libhyper_processor.so

# Allow in policy if needed
sudo ausearch -c 'your_app' --raw | audit2allow -M hyper_processor
sudo semodule -i hyper_processor.pp
```

### 3. Monitoring

```bash
# Watch for RASP alerts
sudo journalctl -f -t hyper_processor --priority=err

# Set up log rotation
cat > /etc/logrotate.d/hyper_processor << EOF
/var/log/hyper_processor/*.log {
    daily
    rotate 30
    compress
    missingok
    notifempty
    create 0640 root root
}
EOF
```

## Troubleshooting

### Application Won't Start

1. Check audit mode first:
```bash
HYPER_RASP_AUDIT_MODE=true LD_PRELOAD=/usr/local/lib/libhyper_processor.so your_app
```

2. Verify library path:
```bash
ldd /usr/local/lib/libhyper_processor.so
```

3. Check SELinux denials:
```bash
sudo ausearch -m avc -ts recent
```

### Performance Impact

Measure overhead:
```bash
# Without RASP
time your_app

# With RASP
time LD_PRELOAD=/usr/local/lib/libhyper_processor.so your_app
```

Expected overhead: 5-10ms startup time

### Debug Logging

```bash
export RUST_LOG=debug
export LD_PRELOAD=/usr/local/lib/libhyper_processor.so
your_app 2> debug.log
```

### Common Issues

| Issue | Solution |
|-------|----------|
| "Unauthorized library detected" for system lib | Update DEFAULT_SYSTEM_WHITELIST in code |
| Config file not found | Check HYPER_RASP_CONFIG path |
| Permission denied | Check file permissions and SELinux |
| High CPU usage | Disable debug logging in production |

## Advanced Usage

### Multi-tenant Configuration

```bash
# Per-user configs
/etc/hyper_processor/users/
├── alice.yaml
├── bob.yaml
└── default.yaml

# In wrapper script
export HYPER_RASP_CONFIG="/etc/hyper_processor/users/${USER}.yaml"
```

### Container Integration

Dockerfile example:
```dockerfile
FROM fedora:41
RUN dnf install -y cargo gcc
COPY . /build
WORKDIR /build
RUN cargo build --release && \
    cp target/release/libhyper_processor.so /usr/local/lib/

# In your app container
FROM fedora:41
COPY --from=builder /usr/local/lib/libhyper_processor.so /usr/local/lib/
ENV LD_PRELOAD=/usr/local/lib/libhyper_processor.so
ENV HYPER_RASP_AUDIT_MODE=false
```

### Kubernetes DaemonSet

Deploy as DaemonSet to protect all pods on a node:
```yaml
apiVersion: apps/v1
kind: DaemonSet
metadata:
  name: hyper-processor-rasp
spec:
  selector:
    matchLabels:
      name: hyper-processor-rasp
  template:
    spec:
      hostPID: true
      hostNetwork: true
      containers:
      - name: installer
        image: hyper-processor:latest
        securityContext:
          privileged: true
        volumeMounts:
        - name: host-lib
          mountPath: /host/usr/local/lib
      volumes:
      - name: host-lib
        hostPath:
          path: /usr/local/lib
```

## Support

- GitHub Issues: https://github.com/your/hyper_processor/issues
- Documentation: https://hyper-processor.io/docs
- Security: security@hyper-processor.io 