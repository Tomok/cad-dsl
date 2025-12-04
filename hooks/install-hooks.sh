#!/usr/bin/env bash
# Installation script for TextCAD git hooks
# Run this script from the repository root to install git hooks

set -e

HOOKS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GIT_HOOKS_DIR=".git/hooks"

echo "Installing git hooks..."

# Check if we're in a git repository
if [ ! -d ".git" ]; then
    echo "Error: This script must be run from the repository root"
    exit 1
fi

# Install pre-commit hook
if [ -f "${HOOKS_DIR}/pre-commit" ]; then
    echo "Installing pre-commit hook..."
    cp "${HOOKS_DIR}/pre-commit" "${GIT_HOOKS_DIR}/pre-commit"
    chmod +x "${GIT_HOOKS_DIR}/pre-commit"
    echo "✓ pre-commit hook installed"
else
    echo "Warning: pre-commit hook not found in ${HOOKS_DIR}"
fi

echo ""
echo "✅ Git hooks installed successfully!"
echo ""
echo "The pre-commit hook will REJECT commits if:"
echo "  • Code is not formatted (cargo fmt --check fails)"
echo "  • Tests fail (cargo test fails)"
echo "  • Z3 environment is not available (unless SKIP_TESTS=1 is set)"
echo ""
echo "To skip tests during commit, use: SKIP_TESTS=1 git commit ..."
echo "To bypass hooks entirely (not recommended), use: git commit --no-verify ..."
