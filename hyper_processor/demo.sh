#!/bin/bash

# HyperProcessor RASP Demo Script
# This script demonstrates the key features of the RASP library

set -e

echo "=========================================="
echo "HyperProcessor RASP Demonstration"
echo "=========================================="
echo

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Ensure we're in the right directory
cd "$(dirname "$0")"

# Build the library if needed
if [ ! -f "target/release/libhyper_processor.so" ]; then
    echo "Building RASP library..."
    cargo build --release --lib
fi

# Build test app if needed
if [ ! -f "test_app" ]; then
    echo "Building test application..."
    gcc -o test_app test_app.c
fi

# Build evil library if needed
if [ ! -f "libevil.so" ]; then
    echo "Building evil library..."
    gcc -shared -fPIC -o libevil.so ../evil.c
fi

echo
echo "=========================================="
echo "Test 1: Normal execution (no LD_PRELOAD)"
echo "=========================================="
echo -e "${GREEN}Expected: Application runs normally${NC}"
./test_app
echo

echo "=========================================="
echo "Test 2: With RASP protection only"
echo "=========================================="
echo -e "${GREEN}Expected: Application runs with RASP protection${NC}"
LD_PRELOAD=./target/release/libhyper_processor.so ./test_app 2>&1 | grep -E "(HyperProcessor|Test application)"
echo

echo "=========================================="
echo "Test 3: Unauthorized library (blocking mode)"
echo "=========================================="
echo -e "${RED}Expected: Process terminated due to unauthorized library${NC}"
echo "Command: LD_PRELOAD=\"./target/release/libhyper_processor.so:./libevil.so\" ./test_app"
if LD_PRELOAD="./target/release/libhyper_processor.so:./libevil.so" ./test_app 2>&1 | grep -E "(EVIL|SECURITY|Terminating)"; then
    echo -e "${RED}Process was terminated as expected${NC}"
fi
echo

echo "=========================================="
echo "Test 4: Audit mode (detection without blocking)"
echo "=========================================="
echo -e "${YELLOW}Expected: Unauthorized library detected but process continues${NC}"
echo "Command: HYPER_RASP_AUDIT_MODE=true LD_PRELOAD=\"./target/release/libhyper_processor.so:./libevil.so\" ./test_app"
HYPER_RASP_AUDIT_MODE=true LD_PRELOAD="./target/release/libhyper_processor.so:./libevil.so" ./test_app 2>&1 | grep -E "(EVIL|AUDIT|Test application|Evil library unloaded)"
echo

echo "=========================================="
echo "Test 5: Custom whitelist configuration"
echo "=========================================="
echo -e "${GREEN}Expected: Evil library allowed when whitelisted${NC}"

# Create temporary config with evil library whitelisted
cat > temp_rasp_config.yaml << EOF
audit_mode: false
whitelisted_filenames:
  - "libevil.so"
EOF

echo "Using config with libevil.so whitelisted..."
cp temp_rasp_config.yaml rasp_config.yaml
LD_PRELOAD="./target/release/libhyper_processor.so:./libevil.so" ./test_app 2>&1 | grep -E "(EVIL|Test application|Evil library unloaded)"
rm temp_rasp_config.yaml

# Restore original config
cat > rasp_config.yaml << EOF
# HyperProcessor RASP Configuration
audit_mode: false
whitelisted_filenames: []
EOF

echo
echo "=========================================="
echo "Test 6: Environment variable whitelist"
echo "=========================================="
echo -e "${GREEN}Expected: Evil library allowed via environment variable${NC}"
echo "Command: HYPER_RASP_WHITELIST=\"libevil.so\" LD_PRELOAD=\"./target/release/libhyper_processor.so:./libevil.so\" ./test_app"
HYPER_RASP_WHITELIST="libevil.so" LD_PRELOAD="./target/release/libhyper_processor.so:./libevil.so" ./test_app 2>&1 | grep -E "(EVIL|Test application|Evil library unloaded)"

echo
echo "=========================================="
echo "Demo completed!"
echo "=========================================="
echo
echo "Key features demonstrated:"
echo "✓ Automatic detection of preloaded libraries"
echo "✓ Blocking mode (default) - terminates process on unauthorized libraries"
echo "✓ Audit mode - logs but allows execution"
echo "✓ Configuration file support (rasp_config.yaml)"
echo "✓ Environment variable overrides"
echo "✓ Whitelisting mechanism"
echo "✓ Detailed security logging with file hashes" 