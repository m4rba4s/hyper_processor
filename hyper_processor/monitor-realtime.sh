#!/bin/bash

# HyperProcessor RASP Real-time Monitor
# This script monitors system processes and protects them with RASP

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuration
RASP_LIB="${RASP_LIB:-./target/release/libhyper_processor.so}"
LOG_FILE="${LOG_FILE:-/tmp/hyper_rasp_monitor.log}"
PID_FILE="/tmp/hyper_rasp_monitor.pid"
MONITOR_INTERVAL="${MONITOR_INTERVAL:-5}"  # seconds
AUDIT_MODE="${AUDIT_MODE:-false}"

# Ensure RASP library exists
if [ ! -f "$RASP_LIB" ]; then
    echo -e "${RED}Error: RASP library not found at $RASP_LIB${NC}"
    echo "Please build it first with: cargo build --release --lib"
    exit 1
fi

# Function to cleanup on exit
cleanup() {
    echo -e "\n${YELLOW}Stopping RASP monitor...${NC}"
    rm -f "$PID_FILE"
    exit 0
}

trap cleanup EXIT INT TERM

# Check if already running
if [ -f "$PID_FILE" ]; then
    OLD_PID=$(cat "$PID_FILE")
    if kill -0 "$OLD_PID" 2>/dev/null; then
        echo -e "${RED}Monitor already running with PID $OLD_PID${NC}"
        exit 1
    else
        rm -f "$PID_FILE"
    fi
fi

# Save our PID
echo $$ > "$PID_FILE"

echo -e "${GREEN}=========================================="
echo "HyperProcessor RASP Real-time Monitor"
echo "==========================================${NC}"
echo
echo -e "${BLUE}Configuration:${NC}"
echo "  RASP Library: $RASP_LIB"
echo "  Log File: $LOG_FILE"
echo "  Audit Mode: $AUDIT_MODE"
echo "  Check Interval: ${MONITOR_INTERVAL}s"
echo
echo -e "${YELLOW}Press Ctrl+C to stop monitoring${NC}"
echo

# Create or clear log file
> "$LOG_FILE"

# Function to monitor a specific process
monitor_process() {
    local pid=$1
    local cmd=$2
    
    # Skip if process is already being monitored (has our lib in maps)
    if grep -q "libhyper_processor.so" "/proc/$pid/maps" 2>/dev/null; then
        return 0
    fi
    
    echo -e "${BLUE}[$(date '+%Y-%m-%d %H:%M:%S')] Monitoring process: PID=$pid CMD=$cmd${NC}"
    
    # For demonstration, we'll show how to inject into running process
    # Note: This requires ptrace capabilities and is more complex
    # For now, we'll just log that we detected an unprotected process
    
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] UNPROTECTED: PID=$pid CMD=$cmd" >> "$LOG_FILE"
}

# Function to scan for new processes
scan_processes() {
    # Get list of all processes
    for pid in $(ls /proc | grep -E '^[0-9]+$'); do
        # Skip kernel threads
        [ ! -r "/proc/$pid/exe" ] && continue
        
        # Get command name
        cmd=$(cat "/proc/$pid/comm" 2>/dev/null || echo "unknown")
        
        # Skip our own monitor process
        [ "$pid" = "$$" ] && continue
        
        # Skip system processes (optional)
        case "$cmd" in
            systemd|kernel|init)
                continue
                ;;
        esac
        
        # Monitor this process
        monitor_process "$pid" "$cmd" 2>/dev/null || true
    done
}

# Main monitoring loop
echo -e "${GREEN}Starting real-time monitoring...${NC}"
echo

while true; do
    # Scan for processes
    scan_processes
    
    # Show log tail
    if [ -s "$LOG_FILE" ]; then
        echo -e "\n${YELLOW}Recent activity:${NC}"
        tail -n 5 "$LOG_FILE"
    fi
    
    # Wait before next scan
    sleep "$MONITOR_INTERVAL"
done 