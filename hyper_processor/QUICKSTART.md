# HyperProcessor RASP - Quick Start Guide

## What is HyperProcessor RASP?

HyperProcessor is a Runtime Application Self-Protection (RASP) tool that detects and prevents unauthorized dynamic library injection via `LD_PRELOAD`. It protects Linux applications from malicious libraries being loaded at runtime.

## Quick Installation

```bash
# Clone and enter directory
git clone <repository>
cd hyper_processor

# Run installation script
./install.sh
```

## Basic Usage

### 1. Protect a single command
```bash
# Using the wrapper script (after installation)
with-rasp ls -la

# Or manually with LD_PRELOAD
LD_PRELOAD=/path/to/libhyper_processor.so command
```

### 2. Audit Mode (detect but don't block)
```bash
HYPER_RASP_AUDIT_MODE=true with-rasp suspicious_app
```

### 3. Add libraries to whitelist
```bash
# Via environment variable
HYPER_RASP_WHITELIST="libcustom.so,libplugin.so" with-rasp myapp

# Or edit config file: ~/.config/hyper_processor/rasp_config.yaml
```

## Demo

Run the included demo to see all features:
```bash
./demo.sh
```

## Key Features

- ✅ **Automatic Detection**: Scans all loaded libraries on startup
- ✅ **Blocking Mode**: Terminates process if unauthorized library detected
- ✅ **Audit Mode**: Logs detections without blocking
- ✅ **Whitelisting**: Configure allowed libraries via config or environment
- ✅ **Security Logging**: Detailed logs with file hashes and metadata
- ✅ **Zero Dependencies**: Works with any Linux application

## Configuration

Default config location: `~/.config/hyper_processor/rasp_config.yaml`

```yaml
# Enable/disable audit mode
audit_mode: false

# Whitelist additional libraries
whitelisted_filenames:
  - "libcustom.so"
  - "libapp.so.1"
```

## Environment Variables

- `HYPER_RASP_AUDIT_MODE`: Set to `true` for audit mode
- `HYPER_RASP_WHITELIST`: Comma-separated list of allowed libraries
- `HYPER_RASP_CONFIG`: Path to custom config file
- `RUST_LOG`: Set to `hyper_processor=debug` for detailed logs

## Testing

```bash
# Run unit tests
cargo test

# Run integration tests
./run_tests.sh basic
```

## Troubleshooting

1. **Library not loading**: Check that the .so file has execute permissions
2. **False positives**: Add legitimate libraries to whitelist
3. **No logs**: Set `RUST_LOG=hyper_processor=info` or higher

## Security Notes

- Always test in audit mode first before enabling blocking
- Keep config file permissions restrictive (644 or 600)
- Review logs regularly for unauthorized attempts
- Update whitelist as needed for your applications 