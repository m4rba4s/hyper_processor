#!/bin/bash
# HyperProcessor RASP Real-time Monitor
# Shows security alerts and statistics

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
NC='\033[0m'

# Default log source
LOG_SOURCE="journalctl"
LOG_FILE=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -f|--file)
            LOG_SOURCE="file"
            LOG_FILE="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 [options]"
            echo "Options:"
            echo "  -f, --file <path>    Monitor specific log file"
            echo "  -h, --help           Show this help"
            echo
            echo "Default: Monitor system journal"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Statistics
declare -A blocked_libs
declare -A blocked_processes
total_blocks=0
total_audits=0

# Clear screen and show header
clear
echo -e "${GREEN}═══════════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}║           HyperProcessor RASP Security Monitor v0.2.0           ║${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════════════════${NC}"
echo

# Function to update statistics display
update_display() {
    # Move cursor to line 5
    tput cup 4 0
    
    # Clear from cursor to end of screen
    tput ed
    
    echo -e "${BLUE}┌─ Statistics ─────────────────────────────────────────────────────┐${NC}"
    echo -e "${BLUE}│${NC} Total Blocked: ${RED}$total_blocks${NC}  │  Total Audited: ${YELLOW}$total_audits${NC}"
    echo -e "${BLUE}├─ Top Blocked Libraries ──────────────────────────────────────────┤${NC}"
    
    # Sort and display top 5 blocked libraries
    local count=0
    for lib in $(printf '%s\n' "${!blocked_libs[@]}" | sort -t: -k2 -nr | head -5); do
        if [ ! -z "$lib" ]; then
            printf "${BLUE}│${NC} %-40s ${RED}%5d${NC} attempts\n" "$lib" "${blocked_libs[$lib]}"
            ((count++))
        fi
    done
    
    # Fill empty lines
    while [ $count -lt 5 ]; do
        echo -e "${BLUE}│${NC}"
        ((count++))
    done
    
    echo -e "${BLUE}├─ Recent Alerts ──────────────────────────────────────────────────┤${NC}"
}

# Function to process log line
process_log_line() {
    local line="$1"
    
    # Check if it's a RASP alert
    if echo "$line" | grep -q "unauthorized_library_filename"; then
        # Parse JSON
        local alert_type=$(echo "$line" | jq -r '.fields.alert_type' 2>/dev/null || echo "UNKNOWN")
        local lib_name=$(echo "$line" | jq -r '.fields.unauthorized_library_filename' 2>/dev/null || echo "unknown")
        local lib_path=$(echo "$line" | jq -r '.fields.unauthorized_library_path' 2>/dev/null || echo "unknown")
        local process=$(echo "$line" | jq -r '.fields.process_name' 2>/dev/null || echo "unknown")
        local timestamp=$(echo "$line" | jq -r '.timestamp' 2>/dev/null || date -Iseconds)
        
        # Update statistics
        if [ "$alert_type" = "SECURITY" ]; then
            ((total_blocks++))
            ((blocked_libs[$lib_name]++))
            ((blocked_processes[$process]++))
            
            # Show alert
            echo -e "${BLUE}│${NC} ${RED}[BLOCKED]${NC} $(date '+%H:%M:%S') - ${PURPLE}$process${NC} tried to load ${RED}$lib_name${NC}"
        elif [ "$alert_type" = "AUDIT" ]; then
            ((total_audits++))
            
            # Show alert
            echo -e "${BLUE}│${NC} ${YELLOW}[AUDIT]${NC}   $(date '+%H:%M:%S') - ${PURPLE}$process${NC} loaded ${YELLOW}$lib_name${NC}"
        fi
    fi
}

# Main monitoring loop
update_display

echo -e "${BLUE}└──────────────────────────────────────────────────────────────────┘${NC}"
echo
echo -e "${GREEN}Monitoring for RASP alerts... Press Ctrl+C to stop.${NC}"

# Start monitoring
if [ "$LOG_SOURCE" = "journalctl" ]; then
    # Monitor system journal
    journalctl -f -n 0 --output=json | while read -r line; do
        process_log_line "$line"
    done
else
    # Monitor file
    if [ ! -f "$LOG_FILE" ]; then
        echo -e "${RED}Error: Log file not found: $LOG_FILE${NC}"
        exit 1
    fi
    
    tail -f "$LOG_FILE" | while read -r line; do
        process_log_line "$line"
    done
fi 