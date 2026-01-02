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

### Alternative: Direct Installation with curl + dpkg

If proxy configuration doesn't work or you prefer a simpler approach, you can install packages directly using `curl` + `dpkg`:

**Installing mold:**
```bash
curl -L -o /tmp/mold.deb http://archive.ubuntu.com/ubuntu/pool/universe/m/mold/mold_2.30.0+dfsg-1build1_amd64.deb
sudo dpkg -i /tmp/mold.deb
```

**Installing Z3:**
```bash
# Download Z3 packages (library, binary, and development headers)
curl -L -o /tmp/libz3-4.deb http://archive.ubuntu.com/ubuntu/pool/universe/z/z3/libz3-4_4.8.12-3.1build1_amd64.deb
curl -L -o /tmp/z3.deb http://archive.ubuntu.com/ubuntu/pool/universe/z/z3/z3_4.8.12-3.1build1_amd64.deb
curl -L -o /tmp/libz3-dev.deb http://archive.ubuntu.com/ubuntu/pool/universe/z/z3/libz3-dev_4.8.12-3.1build1_amd64.deb

# Install in dependency order (libz3-4 → z3 → libz3-dev)
sudo dpkg -i /tmp/libz3-4.deb /tmp/z3.deb /tmp/libz3-dev.deb
```

**Note:** This manual installation method works when apt cannot use the proxy or when you prefer not to configure it. The proxy configuration method above is the preferred solution for environments with proxy settings.

### Z3 Missing

If Z3 is missing and `apt-get` works normally:
```bash
sudo apt-get update && sudo apt-get install --reinstall -y z3 libz3-dev
```
