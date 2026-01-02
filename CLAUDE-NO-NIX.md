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

### DNS Issues with apt-get

If `apt-get` fails with "Temporary failure resolving" errors, you can install packages directly using `curl` + `dpkg`:

**Installing mold:**
```bash
curl -L -o /tmp/mold.deb http://archive.ubuntu.com/ubuntu/pool/universe/m/mold/mold_2.30.0+dfsg-1build1_amd64.deb
sudo dpkg -i /tmp/mold.deb
```

**Installing Z3:**
```bash
# Download Z3 and its development library
curl -L -o /tmp/z3.deb http://archive.ubuntu.com/ubuntu/pool/universe/z/z3/z3_4.8.12-3.1_amd64.deb
curl -L -o /tmp/libz3-dev.deb http://archive.ubuntu.com/ubuntu/pool/universe/z/z3/libz3-dev_4.8.12-3.1_amd64.deb

# Install in order (z3 first, then libz3-dev)
sudo dpkg -i /tmp/z3.deb
sudo dpkg -i /tmp/libz3-dev.deb
```

**Note:** This workaround is necessary when `/etc/resolv.conf` is empty or DNS resolution is not working for `apt-get`, but `curl` can still resolve domains (e.g., via DNS over HTTPS).

### Z3 Missing

If Z3 is missing and `apt-get` works normally:
```bash
sudo apt-get update && sudo apt-get install --reinstall -y z3 libz3-dev
```
