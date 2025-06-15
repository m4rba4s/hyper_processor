#!/bin/bash

# Script to protect specific applications with HyperProcessor RASP

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Default configuration
RASP_LIB="${RASP_LIB:-$HOME/.local/lib/libhyper_processor.so}"
CONFIG_FILE="${CONFIG_FILE:-$HOME/.config/hyper_processor/rasp_config.yaml}"
AUDIT_MODE="${AUDIT_MODE:-false}"

# Function to show usage
usage() {
    echo "Usage: $0 [OPTIONS] <command> [args...]"
    echo
    echo "Options:"
    echo "  -a, --audit       Enable audit mode (detect only, don't block)"
    echo "  -c, --config FILE Use custom config file"
    echo "  -w, --whitelist   Add comma-separated libraries to whitelist"
    echo "  -l, --log-level   Set log level (debug, info, warn, error)"
    echo "  -h, --help        Show this help message"
    echo
    echo "Examples:"
    echo "  $0 firefox                    # Protect Firefox"
    echo "  $0 -a chrome                  # Protect Chrome in audit mode"
    echo "  $0 -w libcustom.so myapp      # Run myapp with libcustom.so whitelisted"
    echo "  $0 -l debug /usr/bin/app      # Run app with debug logging"
    exit 1
}

# Parse command line arguments
WHITELIST=""
LOG_LEVEL="info"

while [[ $# -gt 0 ]]; do
    case $1 in
        -a|--audit)
            AUDIT_MODE="true"
            shift
            ;;
        -c|--config)
            CONFIG_FILE="$2"
            shift 2
            ;;
        -w|--whitelist)
            WHITELIST="$2"
            shift 2
            ;;
        -l|--log-level)
            LOG_LEVEL="$2"
            shift 2
            ;;
        -h|--help)
            usage
            ;;
        *)
            break
            ;;
    esac
done

# Check if command provided
if [ $# -eq 0 ]; then
    echo -e "${RED}Error: No command specified${NC}"
    usage
fi

# Check if RASP library exists
if [ ! -f "$RASP_LIB" ]; then
    echo -e "${RED}Error: RASP library not found at $RASP_LIB${NC}"
    echo "Please install HyperProcessor RASP first with: ./install.sh"
    exit 1
fi

# Get command info
COMMAND="$1"
shift
ARGS="$@"

# Resolve full path of command
COMMAND_PATH=$(which "$COMMAND" 2>/dev/null || echo "$COMMAND")

echo -e "${GREEN}=========================================="
echo "Protecting Application with RASP"
echo "==========================================${NC}"
echo
echo -e "${BLUE}Application:${NC} $COMMAND_PATH"
echo -e "${BLUE}Arguments:${NC} $ARGS"
echo -e "${BLUE}RASP Library:${NC} $RASP_LIB"
echo -e "${BLUE}Config File:${NC} $CONFIG_FILE"
echo -e "${BLUE}Audit Mode:${NC} $AUDIT_MODE"
echo -e "${BLUE}Log Level:${NC} $LOG_LEVEL"
if [ -n "$WHITELIST" ]; then
    echo -e "${BLUE}Additional Whitelist:${NC} $WHITELIST"
fi
echo

# Set up environment
export LD_PRELOAD="$RASP_LIB:$LD_PRELOAD"
export HYPER_RASP_CONFIG="$CONFIG_FILE"
export HYPER_RASP_AUDIT_MODE="$AUDIT_MODE"
export RUST_LOG="hyper_processor=$LOG_LEVEL"

if [ -n "$WHITELIST" ]; then
    export HYPER_RASP_WHITELIST="$WHITELIST"
fi

# Run the protected application
echo -e "${GREEN}Starting protected application...${NC}"
echo

exec "$COMMAND_PATH" $ARGS 