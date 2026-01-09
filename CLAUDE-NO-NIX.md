# Running CAD-DSL Without Nix

**NOTE:** Environment setup is now automated. Simply run:

```bash
./.claude_env.sh
```

This script automatically handles:
- Detection of Nix (exits if already available)
- Proxy configuration for apt-get
- Installation of system dependencies (mold, z3, libz3-dev)
- Creation of transparent command wrappers

After running the setup script, all `nix shell -c` commands in CLAUDE.md work identically whether Nix is installed or not.

## Manual Setup (Not Recommended)

If you need to set up the environment manually without the automated script:

### Install System Dependencies

```bash
sudo apt-get update && sudo apt-get install -y mold z3 libz3-dev
```

### Commands

All commands in CLAUDE.md use `nix shell -c` prefix. Without the automated wrapper, use standard Cargo commands directly:

```bash
# Instead of: nix shell -c cargo build
cargo build

# Instead of: nix shell -c cargo test
cargo test

# Instead of: nix shell -c cargo fmt
cargo fmt
```
