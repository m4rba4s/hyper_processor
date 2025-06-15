#!/bin/bash

# HyperProcessor RASP Daemon
# Monitors and protects system processes in real-time

# Configuration
RASP_LIB="${RASP_LIB:-/opt/hyper_processor/libhyper_processor.so}"
CONFIG_FILE="${CONFIG_FILE:-/etc/hyper_processor/rasp_config.yaml}"
LOG_DIR="${LOG_DIR:-/var/log/hyper_rasp}"
AUDIT_LOG="$LOG_DIR/audit.log"
SECURITY_LOG="$LOG_DIR/security.log"

# Create log directory if needed
mkdir -p "$LOG_DIR"

# Function to log messages
log_message() {
    local level=$1
    local message=$2
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[$timestamp] [$level] $message" | tee -a "$LOG_DIR/daemon.log"
}

# Function to monitor system-wide
monitor_system() {
    log_message "INFO" "Starting system-wide RASP monitoring"
    
    # Monitor new process creation using auditd if available
    if command -v auditctl &> /dev/null; then
        log_message "INFO" "Setting up auditd rules for process monitoring"
        
        # Add audit rule for execve
        auditctl -a exit,always -F arch=b64 -S execve -k rasp_monitor 2>/dev/null || true
    fi
    
    # Monitor using /proc polling as fallback
    while true; do
        # Check for new processes every second
        for pid in $(ls /proc 2>/dev/null | grep -E '^[0-9]+$'); do
            # Skip if we can't read the process info
            [ ! -r "/proc/$pid/exe" ] && continue
            
            # Check if process needs protection
            check_process "$pid"
        done
        
        sleep 1
    done
}

# Function to check if a process needs RASP protection
check_process() {
    local pid=$1
    
    # Skip if already protected
    if grep -q "libhyper_processor.so" "/proc/$pid/maps" 2>/dev/null; then
        return 0
    fi
    
    # Get process info
    local exe=$(readlink "/proc/$pid/exe" 2>/dev/null || echo "unknown")
    local cmd=$(cat "/proc/$pid/comm" 2>/dev/null || echo "unknown")
    
    # Skip kernel threads and system processes
    case "$cmd" in
        systemd*|kernel*|init|kworker*|ksoftirqd*)
            return 0
            ;;
    esac
    
    # Log unprotected process
    log_message "WARN" "Unprotected process detected: PID=$pid CMD=$cmd EXE=$exe"
    
    # In a real implementation, you could:
    # 1. Use LD_PRELOAD injection via ptrace
    # 2. Restart the process with RASP protection
    # 3. Alert administrators
}

# Function to watch RASP logs
watch_rasp_logs() {
    # Use journalctl to watch for RASP events
    journalctl -f -o json | while read -r line; do
        # Parse JSON log entries
        if echo "$line" | grep -q "hyper_processor"; then
            # Extract relevant fields
            local timestamp=$(echo "$line" | jq -r '.__REALTIME_TIMESTAMP // empty' 2>/dev/null)
            local message=$(echo "$line" | jq -r '.MESSAGE // empty' 2>/dev/null)
            local level=$(echo "$line" | jq -r '.PRIORITY // empty' 2>/dev/null)
            
            # Check for security alerts
            if echo "$message" | grep -q "SECURITY\|Unauthorized"; then
                echo "[$timestamp] SECURITY ALERT: $message" >> "$SECURITY_LOG"
                
                # Send alert (email, webhook, etc.)
                send_alert "SECURITY" "$message"
            elif echo "$message" | grep -q "AUDIT"; then
                echo "[$timestamp] AUDIT: $message" >> "$AUDIT_LOG"
            fi
        fi
    done
}

# Function to send alerts
send_alert() {
    local alert_type=$1
    local message=$2
    
    # Example: Send to syslog
    logger -t "hyper_rasp" -p security.alert "$alert_type: $message"
    
    # Example: Send webhook (uncomment and configure)
    # curl -X POST https://your-webhook-url \
    #      -H "Content-Type: application/json" \
    #      -d "{\"alert_type\":\"$alert_type\",\"message\":\"$message\"}"
}

# Main daemon logic
main() {
    log_message "INFO" "HyperProcessor RASP Daemon starting"
    log_message "INFO" "RASP Library: $RASP_LIB"
    log_message "INFO" "Config File: $CONFIG_FILE"
    
    # Export environment variables for child processes
    export LD_PRELOAD="$RASP_LIB"
    export HYPER_RASP_CONFIG="$CONFIG_FILE"
    
    # Start log watcher in background
    watch_rasp_logs &
    LOG_WATCHER_PID=$!
    
    # Start system monitor
    monitor_system &
    MONITOR_PID=$!
    
    # Handle signals
    trap 'log_message "INFO" "Shutting down"; kill $LOG_WATCHER_PID $MONITOR_PID 2>/dev/null; exit 0' TERM INT
    
    # Wait for background processes
    wait
}

# Run main function
main "$@" 