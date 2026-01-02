# Running CAD-DSL Without Nix

This guide provides instructions for working with CAD-DSL when Nix is not available in your environment (e.g., CI/CD pipelines, Docker containers, or systems where Nix cannot be installed).

## System Requirements

- **Operating System**: Linux (Ubuntu 20.04+ or similar)
- **Rust**: 1.70.0 or later
- **Build Tools**: Standard C/C++ compiler toolchain

## Installation

### 1. Install System Dependencies

CAD-DSL requires the following system dependencies:

```bash
# Install all required dependencies
sudo apt-get update && sudo apt-get install -y \
    mold \
    z3 \
    libz3-dev \
    build-essential \
    pkg-config
```

### 2. Install Rust

If Rust is not already installed, install it via rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

Verify installation:

```bash
rustc --version
cargo --version
```

## Building and Testing

Once dependencies are installed, you can use standard Cargo commands directly (no `nix shell -c` wrapper needed):

### Build the Project

```bash
cargo build
```

### Run Tests

```bash
cargo test
```

### Run Specific Tests

```bash
cargo test <test_name>
```

### Run Tests with Output Visible

```bash
cargo test -- --nocapture
```

### Format Code

```bash
cargo fmt
```

### Run Linter

```bash
cargo clippy
```

### Run Linter with No Warnings Allowed

```bash
cargo clippy -- -D warnings
```

## Running the CLI

### Tokenize a CAD File

```bash
cargo run -- lex <file.cad>
```

### Parse a CAD File

```bash
cargo run -- parse <file.cad>
```

### Solve Constraints

```bash
cargo run -- solve <file.cad>
```

## Quality Checks Before Committing

Always run these checks before committing code:

```bash
# 1. Format code
cargo fmt

# 2. Check for warnings (must pass with zero warnings)
cargo clippy -- -D warnings

# 3. Run all tests (must pass)
cargo test
```

If any check fails:
1. Fix the issues in the code
2. Re-run ALL quality checks in sequence
3. Repeat until all checks pass

## Troubleshooting

### mold Linker Issues

If you encounter linker errors related to mold:

1. **Check if mold is installed:**
   ```bash
   which mold
   ```

2. **Verify mold version:**
   ```bash
   mold --version
   ```

3. **If mold is not available**, you can temporarily disable it by renaming the Cargo config:
   ```bash
   mv .cargo/config.toml .cargo/config.toml.bak
   ```

   Then restore it after building:
   ```bash
   mv .cargo/config.toml.bak .cargo/config.toml
   ```

### Z3 Library Issues

If you encounter Z3-related build errors:

1. **Verify Z3 is installed:**
   ```bash
   z3 --version
   pkg-config --exists z3 && echo "Z3 found" || echo "Z3 not found"
   ```

2. **Check Z3 development headers:**
   ```bash
   dpkg -L libz3-dev | grep z3.h
   ```

3. **If Z3 is missing**, reinstall:
   ```bash
   sudo apt-get update && sudo apt-get install --reinstall -y z3 libz3-dev
   ```

### Build Performance

The project is configured with the mold linker for optimal build performance on systems with limited memory (e.g., CI runners with ~7GB RAM). The configuration in `.cargo/config.toml` includes:

- **Serial linking** (`jobs = 1`): Prevents OOM issues on memory-constrained systems
- **Minimal debug info** (`debug = 1`): Provides backtraces without excessive binary size
- **Single codegen unit** (`codegen-units = 1`): Reduces linker memory pressure
- **No incremental compilation**: Simplifies the build process

These settings prioritize build reliability over speed on constrained systems. On more powerful development machines, you can override these locally:

```bash
# Override in your environment
export CARGO_BUILD_JOBS=4
export CARGO_PROFILE_DEV_DEBUG=2
export CARGO_PROFILE_DEV_CODEGEN_UNITS=16
```

## Alternative: Using Docker

If you prefer containerization, you can create a Docker environment:

```dockerfile
FROM ubuntu:22.04

# Install dependencies
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    pkg-config \
    mold \
    z3 \
    libz3-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Set working directory
WORKDIR /workspace

# Copy project files
COPY . .

# Build the project
RUN cargo build

# Default command
CMD ["cargo", "test"]
```

Build and run:

```bash
docker build -t cad-dsl .
docker run -v $(pwd):/workspace cad-dsl cargo test
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install system dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y mold z3 libz3-dev

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true

      - name: Run tests
        run: |
          cargo fmt -- --check
          cargo clippy -- -D warnings
          cargo test
```

## Notes

- This guide assumes a Debian/Ubuntu-based Linux distribution. For other distributions, adjust the package manager commands accordingly (e.g., `yum`, `dnf`, `pacman`).
- The mold linker is Linux-specific and not available on macOS or Windows. On those platforms, the default system linker will be used.
- Z3 version 4.8.12 or later is recommended for full compatibility.

## See Also

- Main documentation: [CLAUDE.md](CLAUDE.md)
- Language specification: [docs/TEXTCAD_LANGUAGE_SPEC.md](docs/TEXTCAD_LANGUAGE_SPEC.md)
