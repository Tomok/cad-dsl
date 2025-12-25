# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

CAD-DSL is a constraint-based domain-specific language for 2D geometric design. The project implements a lexer and parser for a declarative CAD language using Rust. The language specification is documented in `docs/TEXTCAD_LANGUAGE_SPEC.md`.

## Development Environment

This project uses Nix for development environment management. Use `nix develop` or `nix-shell` to enter the development environment, which provides:

- Rust toolchain with rust-analyzer and LLVM tools
- Z3 constraint solver (system dependency)
- Code coverage tools (cargo-llvm-cov)

## Common Commands

### Building and Testing
- `nix shell -c cargo build` - Build the project
- `nix shell -c cargo test` - Run all tests
- `nix shell -c cargo test <test_name>` - Run specific test
- `nix shell -c cargo test -- --nocapture` - Run tests with output visible

### Running the CLI
- `nix shell -c cargo run -- lex <file.cad>` - Tokenize a CAD file and display tokens
- `nix shell -c cargo run -- parse <file.cad>` - Parse a CAD file and display AST

### Code Quality
- `nix shell -c cargo fmt` - Format code
- `nix shell -c cargo clippy` - Run linter

### Dependencies
- `nix shell -c cargo add <crate>` - Add new dependency (per user's CLAUDE.local.md instructions)

### Git Hooks
- `./hooks/install-hooks.sh` - Install pre-commit hooks for code quality enforcement
- Pre-commit hook runs: `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`

## Code Architecture

### Core Components

**Lexer (`src/lexer.rs`)**
- Uses Logos for token generation
- Comprehensive token definitions with position tracking
- Supports all TextCAD language constructs (keywords, operators, literals, identifiers)
- Handles single-line (`//`) and multi-line (`/* */`) comments
- Line/column position tracking for error reporting

**AST (`src/ast.rs`)**
- Type-safe expression AST with operator precedence hierarchy
- Uses subenum crate to enforce precedence at the type level
- Separate types for different precedence levels (AddLhs, AddRhs, MulLhs, MulRhs, Atom)
- Prevents invalid parse trees through the type system

**Parser (`src/parser.rs`)**
- Chumsky-based recursive descent parser
- Implements proper left-associative operators
- Rich error reporting with Ariadne integration
- Handles parentheses and operator precedence correctly

**CLI (`src/main.rs`)**
- Simple CLI with `lex` and `parse` subcommands
- File input handling and error reporting

### Key Design Patterns

**Type-Safe Precedence**: The AST uses Rust's type system to enforce operator precedence, making it impossible to construct invalid expression trees.

**Rich Error Reporting**: Parser errors include position information and expected vs. found tokens, formatted with Ariadne for user-friendly output.

**Separation of Concerns**: Clear separation between lexical analysis, syntactic analysis, and CLI interface.

## Testing

The project has comprehensive test suites for each component:

- **Lexer tests**: Token recognition, position tracking, comment handling
- **Parser tests**: Expression parsing, precedence, error cases with timeout protection
- **AST tests**: Type conversions and display formatting

Tests use timeout mechanisms to prevent infinite loops during development.

## Language Implementation Status

Currently implements:
- Complete lexical analysis for TextCAD syntax
- Expression parsing with proper operator precedence (arithmetic only)
- Error reporting infrastructure

The language specification in `docs/TEXTCAD_LANGUAGE_SPEC.md` defines the full TextCAD language, including constraints, structs, transforms, and the standard library. The current implementation focuses on the foundational parsing infrastructure.

See `docs/AST_PARSING_TODO.md` for a detailed checklist of parsing features aligned with the language specification.

## Dependencies

Key dependencies:
- `logos` - Lexical analysis
- `chumsky` - Parser combinators
- `ariadne` - Error reporting
- `clap` - CLI interface
- `subenum` - Type-safe enum subsets

Z3 constraint solver is included as a system dependency for future constraint solving implementation.
