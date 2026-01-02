# Running CAD-DSL Without Nix

Quick reference for environments where Nix is not available.

## Install System Dependencies

```bash
sudo apt-get update && sudo apt-get install -y mold z3 libz3-dev
```

## Commands

Use standard Cargo commands directly (no `nix shell -c` wrapper):

```bash
# Build
cargo build

# Test
cargo test

# Format
cargo fmt

# Lint (no warnings)
cargo clippy -- -D warnings

# Run CLI
cargo run -- lex <file.cad>
cargo run -- parse <file.cad>
cargo run -- solve <file.cad>
```

## Troubleshooting

If Z3 is missing:
```bash
sudo apt-get update && sudo apt-get install --reinstall -y z3 libz3-dev
```
