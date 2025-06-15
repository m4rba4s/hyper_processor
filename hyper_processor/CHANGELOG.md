# Changelog

All notable changes to HyperProcessor RASP will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.2.0] - 2025-06-03

### Added
- **CLI Tool** (`hyper-processor`) with subcommands:
  - `learn` - Automatic library discovery mode
  - `monitor` - Prometheus metrics exporter  
  - `verify` - SHA256 hash verification for libraries
  - `protect` - Run applications with RASP protection
  - `generate` - Generate whitelists from audit logs
- **Learning Mode** - Automatically discover required libraries
- **Prometheus Metrics** integration:
  - `hyper_processor_blocks_total` - Total blocked attempts
  - `hyper_processor_audits_total` - Total audited attempts
  - `hyper_processor_library_loads` - Per-library statistics
  - `hyper_processor_unauthorized_loads` - Unauthorized attempts by library
- **Grafana Dashboard** template for monitoring
- Support for parallel whitelist sources (file + env var)

### Changed
- Improved logging with structured fields
- Better error messages for configuration issues
- Optimized performance of library verification

### Fixed
- Thread Local Storage panic on process exit
- Config file permission check now uses eprintln for early errors
- Whitelist parsing from environment variables

## [0.2.0] - 2024-12-17

### Added
- SHA256 hash calculation for detected libraries
- File size reporting for unauthorized libraries
- Process command line logging via `/proc/self/cmdline`
- Version logging using `env!("CARGO_PKG_VERSION")`
- Config file permission warnings
- Enhanced process context in logs (PID, PPID, command line)

### Changed
- Upgraded logging from `log` to `tracing` for structured JSON output
- Improved error handling and logging consistency
- Updated default system whitelist for Fedora compatibility

### Fixed
- Parsing bug in `/proc/self/maps` (incorrect index for permissions)
- Environment variable handling for `HYPER_RASP_WHITELIST`
- Test isolation issues with environment variables

## [0.1.0] - 2024-04-01

### Added
- Initial release of HyperProcessor RASP
- Core LD_PRELOAD protection mechanism
- Whitelist-based library verification
- Audit mode for testing
- Configuration via YAML file
- Basic logging support
- Integration tests

### Security
- Protection against unauthorized library injection
- Process termination on detection (configurable)
- Minimal performance overhead (~6ms) 