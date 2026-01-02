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

### Proxy Configuration for apt-get

If `apt-get` fails with "Temporary failure resolving" errors but `curl` works, the issue is likely that `curl` uses proxy environment variables (`http_proxy`, `https_proxy`) but `apt-get` doesn't automatically. Configure apt to use the proxy:

```bash
# Configure apt to use the same proxy as curl
sudo tee /etc/apt/apt.conf.d/95proxy > /dev/null <<EOF
Acquire::http::Proxy "$http_proxy";
Acquire::https::Proxy "$https_proxy";
EOF

# Now apt-get should work normally
sudo apt-get update
sudo apt-get install -y mold z3 libz3-dev
```

### Z3 Missing

If Z3 is missing and `apt-get` works normally:
```bash
sudo apt-get update && sudo apt-get install --reinstall -y z3 libz3-dev
```
