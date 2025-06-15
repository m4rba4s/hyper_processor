#!/bin/bash

# HyperProcessor RASP Installation Script

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "=========================================="
echo "HyperProcessor RASP Installation"
echo "=========================================="
echo

# Check if running as root (optional, for system-wide install)
if [ "$EUID" -eq 0 ]; then 
   INSTALL_DIR="/usr/local/lib"
   CONFIG_DIR="/etc/hyper_processor"
   echo -e "${YELLOW}Installing system-wide (running as root)${NC}"
else
   INSTALL_DIR="$HOME/.local/lib"
   CONFIG_DIR="$HOME/.config/hyper_processor"
   echo -e "${YELLOW}Installing for current user only${NC}"
   mkdir -p "$INSTALL_DIR"
fi

# Build the library
echo -e "\n${GREEN}Building RASP library...${NC}"
cargo build --release --lib

# Create directories
echo -e "\n${GREEN}Creating directories...${NC}"
mkdir -p "$CONFIG_DIR"

# Install library
echo -e "\n${GREEN}Installing library...${NC}"
cp target/release/libhyper_processor.so "$INSTALL_DIR/"
echo "Library installed to: $INSTALL_DIR/libhyper_processor.so"

# Install default config if not exists
if [ ! -f "$CONFIG_DIR/rasp_config.yaml" ]; then
    echo -e "\n${GREEN}Installing default configuration...${NC}"
    cat > "$CONFIG_DIR/rasp_config.yaml" << 'EOF'
# HyperProcessor RASP Configuration
# This file configures the Runtime Application Self-Protection behavior

# Audit mode: If true, only logs unauthorized libraries without terminating the process
# If false (default), terminates the process when unauthorized libraries are detected
audit_mode: false

# Additional whitelisted library filenames (basenames only, not full paths)
# The system already includes common system libraries in its default whitelist
# Add application-specific libraries here
whitelisted_filenames:
  # Example entries:
  # - "libcustom.so"
  # - "libapp-specific.so.1"
EOF
    echo "Configuration installed to: $CONFIG_DIR/rasp_config.yaml"
else
    echo -e "\n${YELLOW}Configuration already exists, skipping...${NC}"
fi

# Create wrapper script
echo -e "\n${GREEN}Creating wrapper script...${NC}"
WRAPPER_SCRIPT="$HOME/.local/bin/with-rasp"
mkdir -p "$(dirname "$WRAPPER_SCRIPT")"

cat > "$WRAPPER_SCRIPT" << EOF
#!/bin/bash
# Wrapper script to run commands with HyperProcessor RASP protection

export LD_PRELOAD="$INSTALL_DIR/libhyper_processor.so:\$LD_PRELOAD"
export HYPER_RASP_CONFIG="$CONFIG_DIR/rasp_config.yaml"

# Run the command with RASP protection
exec "\$@"
EOF

chmod +x "$WRAPPER_SCRIPT"
echo "Wrapper script installed to: $WRAPPER_SCRIPT"

# Installation complete
echo -e "\n${GREEN}=========================================="
echo "Installation Complete!"
echo "==========================================${NC}"
echo
echo "Usage examples:"
echo "  1. Run a command with RASP protection:"
echo "     $WRAPPER_SCRIPT <command>"
echo
echo "  2. Set LD_PRELOAD manually:"
echo "     LD_PRELOAD=$INSTALL_DIR/libhyper_processor.so <command>"
echo
echo "  3. Enable audit mode (detection only):"
echo "     HYPER_RASP_AUDIT_MODE=true $WRAPPER_SCRIPT <command>"
echo
echo "  4. Add custom whitelist via environment:"
echo "     HYPER_RASP_WHITELIST=\"lib1.so,lib2.so\" $WRAPPER_SCRIPT <command>"
echo
echo "Configuration file: $CONFIG_DIR/rasp_config.yaml"
echo
echo -e "${YELLOW}Note: Add $HOME/.local/bin to your PATH if not already present${NC}" 