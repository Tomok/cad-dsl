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

### Alternative: Authenticated Package Download

If proxy configuration doesn't work, use `apt-get download` which maintains GPG signature verification:

```bash
# Configure proxy first (required for apt-get download to work)
sudo tee /etc/apt/apt.conf.d/95proxy > /dev/null <<EOF
Acquire::http::Proxy "$http_proxy";
Acquire::https::Proxy "$https_proxy";
EOF

sudo apt-get update

# Download with signature verification, then install
cd /tmp
apt-get download mold libz3-4 z3 libz3-dev
sudo dpkg -i libz3-4_*.deb z3_*.deb libz3-dev_*.deb mold_*.deb
```

**Security Note:** Never install .deb files downloaded via plain HTTP or curl without verification - this bypasses apt's GPG signature checks and is vulnerable to MITM attacks. Always use `apt-get download` or configure the proxy as shown above.

### Z3 Missing

If Z3 is missing and `apt-get` works normally:
```bash
sudo apt-get update && sudo apt-get install --reinstall -y z3 libz3-dev
```
