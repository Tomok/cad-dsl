#!/bin/bash
# .claude_env.sh - Automated environment setup for CAD-DSL
# Handles both Nix and non-Nix environments transparently
# Works WITHOUT sudo by installing to ~/.local

set -e  # Exit on error

# Detect if script is being sourced or executed
# When sourced: use 'return', when executed: use 'exit'
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    SCRIPT_EXIT="exit"
else
    SCRIPT_EXIT="return"
fi

LOCAL_PREFIX="$HOME/.local"
LOCAL_BIN="$LOCAL_PREFIX/usr/bin"
LOCAL_LIB="$LOCAL_PREFIX/usr/lib/x86_64-linux-gnu"
LOCAL_PKGCONFIG="$LOCAL_LIB/pkgconfig"

# 1. If nix exists, nothing to do - exit successfully
if command -v nix >/dev/null 2>&1; then
    $SCRIPT_EXIT 0
fi

# 2. Check if we need to install dependencies
NEED_INSTALL=false
if [ ! -f "$LOCAL_BIN/mold" ] || [ ! -f "$LOCAL_LIB/libz3.so.4" ]; then
    NEED_INSTALL=true
fi

# 3. Install dependencies to ~/.local without sudo
if [ "$NEED_INSTALL" = true ]; then
    # Create directories
    mkdir -p "$LOCAL_PREFIX" /tmp/cad-dsl-deps
    cd /tmp/cad-dsl-deps

    # Download packages
    apt-get download mold z3 libz3-dev libz3-4 >/dev/null 2>&1 || {
        echo "Error: Failed to download packages" >&2
        $SCRIPT_EXIT 1
    }

    # Extract to ~/.local
    for deb in *.deb; do
        dpkg-deb -x "$deb" "$LOCAL_PREFIX" >/dev/null 2>&1 || {
            echo "Error: Failed to extract $deb" >&2
            $SCRIPT_EXIT 1
        }
    done

    # Cleanup
    cd /
    rm -rf /tmp/cad-dsl-deps
fi

# 4. Fix z3.pc file to point to local installation
Z3_PC="$LOCAL_PKGCONFIG/z3.pc"
if [ -f "$Z3_PC" ]; then
    sed -i "s|^prefix=/usr|prefix=$LOCAL_PREFIX/usr|" "$Z3_PC"
    sed -i "s|^exec_prefix=/usr|exec_prefix=$LOCAL_PREFIX/usr|" "$Z3_PC"
fi

# 5. Add to current session environment variables
export PATH="$LOCAL_BIN:$PATH"
export LD_LIBRARY_PATH="$LOCAL_LIB:${LD_LIBRARY_PATH:-}"
export PKG_CONFIG_PATH="$LOCAL_PKGCONFIG:${PKG_CONFIG_PATH:-}"
export C_INCLUDE_PATH="$LOCAL_PREFIX/usr/include:${C_INCLUDE_PATH:-}"
export CPLUS_INCLUDE_PATH="$LOCAL_PREFIX/usr/include:${CPLUS_INCLUDE_PATH:-}"

# 6. Create nix wrapper script in ~/.local/bin
NIX_WRAPPER="$LOCAL_BIN/nix"
if [ ! -f "$NIX_WRAPPER" ]; then
    mkdir -p "$LOCAL_BIN"
    cat > "$NIX_WRAPPER" <<'EOF'
#!/bin/bash
# Nix wrapper - forwards commands to actual applications
# Usage: nix shell -c <command>

# Ensure environment variables are set
export PATH="$HOME/.local/usr/bin:$PATH"
export LD_LIBRARY_PATH="$HOME/.local/usr/lib/x86_64-linux-gnu:${LD_LIBRARY_PATH:-}"
export PKG_CONFIG_PATH="$HOME/.local/usr/lib/x86_64-linux-gnu/pkgconfig:${PKG_CONFIG_PATH:-}"
export C_INCLUDE_PATH="$HOME/.local/usr/include:${C_INCLUDE_PATH:-}"
export CPLUS_INCLUDE_PATH="$HOME/.local/usr/include:${CPLUS_INCLUDE_PATH:-}"

if [ "$1" = "shell" ] && [ "$2" = "-c" ]; then
    shift 2
    exec "$@"
else
    echo "Error: Unsupported nix command: $*" >&2
    exit 1
fi
EOF
    chmod +x "$NIX_WRAPPER" || {
        echo "Error: Failed to make nix wrapper executable" >&2
        $SCRIPT_EXIT 1
    }
fi

# 7. Update shell rc files to set environment variables automatically
for rcfile in "$HOME/.bashrc" "$HOME/.zshrc"; do
    if [ -f "$rcfile" ]; then
        if ! grep -q "CAD-DSL local environment" "$rcfile" 2>/dev/null; then
            cat >> "$rcfile" <<'EOF'

# CAD-DSL local environment (added by .claude_env.sh)
export PATH="$HOME/.local/usr/bin:$PATH"
export LD_LIBRARY_PATH="$HOME/.local/usr/lib/x86_64-linux-gnu:${LD_LIBRARY_PATH:-}"
export PKG_CONFIG_PATH="$HOME/.local/usr/lib/x86_64-linux-gnu/pkgconfig:${PKG_CONFIG_PATH:-}"
export C_INCLUDE_PATH="$HOME/.local/usr/include:${C_INCLUDE_PATH:-}"
export CPLUS_INCLUDE_PATH="$HOME/.local/usr/include:${CPLUS_INCLUDE_PATH:-}"
EOF
        fi
    fi
done

$SCRIPT_EXIT 0
