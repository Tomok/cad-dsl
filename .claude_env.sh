#!/bin/bash
# .claude_env.sh - Automated environment setup for CAD-DSL
# Handles both Nix and non-Nix environments transparently

set -e  # Exit on error

# 1. If nix exists, nothing to do - exit successfully
if command -v nix >/dev/null 2>&1; then
    exit 0
fi

# 2. Check for proxy configuration and configure apt if needed
if [ -n "$http_proxy" ] || [ -n "$https_proxy" ]; then
    # Only configure if not already configured
    if [ ! -f /etc/apt/apt.conf.d/95proxy ]; then
        sudo tee /etc/apt/apt.conf.d/95proxy > /dev/null <<EOF
Acquire::http::Proxy "$http_proxy";
Acquire::https::Proxy "$https_proxy";
EOF
    fi
fi

# 3. Install dependencies if not already installed
NEED_INSTALL=false
for pkg in mold z3 libz3-dev; do
    if ! dpkg -s "$pkg" >/dev/null 2>&1; then
        NEED_INSTALL=true
        break
    fi
done

if [ "$NEED_INSTALL" = true ]; then
    sudo apt-get update >/dev/null || {
        echo "Error: apt-get update failed" >&2
        exit 1
    }
    sudo apt-get install -y mold z3 libz3-dev >/dev/null || {
        echo "Error: Failed to install dependencies" >&2
        exit 1
    }
fi

# 4. Create nix wrapper script in PATH
NIX_WRAPPER="/usr/local/bin/nix"
if [ ! -f "$NIX_WRAPPER" ]; then
    sudo tee "$NIX_WRAPPER" > /dev/null <<'EOF'
#!/bin/bash
# Nix wrapper - forwards commands to actual applications
# Usage: nix shell -c <command>

if [ "$1" = "shell" ] && [ "$2" = "-c" ]; then
    shift 2
    exec "$@"
else
    echo "Error: Unsupported nix command: $*" >&2
    exit 1
fi
EOF
    sudo chmod +x "$NIX_WRAPPER" || {
        echo "Error: Failed to make nix wrapper executable" >&2
        exit 1
    }
fi

exit 0
