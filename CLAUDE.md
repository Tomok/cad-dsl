# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

TextCAD is a domain-specific language for constraint-based 2D geometric design implemented in Rust. The project implements a lexer for the TextCAD language using the Logos crate for tokenization and string-interner for efficient identifier handling.

## Development Commands

### Building and Running
- `cargo build` - Build the project
- `cargo run` - Run the main binary (demonstrates lexer functionality)
- `nix develop` - Enter Nix development shell with all dependencies

### Testing and Quality
- `cargo test` - Run all tests
- `cargo fmt` - Format code
- `cargo fmt --check` - Check code formatting
- `nix develop --command cargo test` - Run tests in Nix environment (required for Z3 dependency)

### Environment
The project uses Nix for dependency management, particularly for the Z3 theorem prover. Use `nix develop` to enter the development shell with all required dependencies including Z3.

## Architecture

### Current Implementation Status
The project is in early development with only the lexer phase implemented. The complete architecture design is documented in `docs/SW_ARCHITECTURE.md`, which outlines a multi-phase compilation pipeline:

1. **Lexical Analysis** (✅ Implemented) - Transform source text into tokens
2. **Syntax Analysis** (Planned) - Build AST using Chumsky parser combinators  
3. **Name Resolution** (Planned) - Resolve identifiers and build symbol tables
4. **Type Checking** (Planned) - Validate types and annotate expressions
5. **Constraint Collection** (Future) - Extract constraint equations for solver

### Core Components

#### Lexer (`src/lexer/`)
- **scanner.rs** - Main lexer implementation using Logos
- **token.rs** - Token definitions and types  
- **mod.rs** - Module exports

#### Supporting Modules
- **ident.rs** - Identifier interning using string-interner crate
- **span.rs** - Source location tracking
- **error.rs** - Error type definitions using thiserror
- **lib.rs** - Main library interface
- **main.rs** - CLI demonstration of lexer functionality

### Key Dependencies
- `logos` - Efficient DFA-based lexing
- `string-interner` - Identifier string interning for memory efficiency and O(1) comparison
- `thiserror` - Error handling

## Development Guidelines

### Code Organization
The project follows the modular architecture outlined in `docs/SW_ARCHITECTURE.md`. Each compilation phase (lexer, parser, name resolution, type checking) will have its own module with clear interfaces.

### Identifier Handling
All identifiers are interned using `IdentArena` from the string-interner crate. This provides:
- Memory efficiency (each unique string stored once)
- O(1) identifier comparison using IdentId
- Consistent handling across all compilation phases

### Testing
The pre-commit hook enforces code formatting and test passing. Tests may require the Nix environment for Z3 dependency access.

### Git Hooks
Pre-commit hooks are configured to:
- Check code formatting with `cargo fmt --check`
- Run tests (can be skipped with `SKIP_TESTS=1`)
- Use Nix development environment when available

## Documentation

- **`docs/SW_ARCHITECTURE.md`** - Complete software architecture design including all compilation phases, data structures, error handling strategy, and implementation technologies
- **`docs/TEXTCAD_LANGUAGE_SPEC.md`** - Complete language specification including syntax, semantics, type system, and examples

## File Structure

```
src/
├── main.rs           # CLI entry point
├── lib.rs            # Library exports  
├── ident.rs          # Identifier interning
├── span.rs           # Source location tracking
├── error.rs          # Error definitions
└── lexer/
    ├── mod.rs        # Module exports
    ├── scanner.rs    # Lexer implementation
    └── token.rs      # Token types
```

## Language Features

Key language features as specified in `docs/TEXTCAD_LANGUAGE_SPEC.md`:
- Declarative constraint-based design
- Geometric entities (Point, Length, Angle, etc.)
- User-defined structs
- Views and coordinate systems
- Unit-aware numeric literals
- Functional operations (map, reduce)
- before commiting, make sure to format the code and fix any errors and warnings from cargo and clippy
- Make sure to use chumskys Recursive type when handling potentially recursive components of the AST.