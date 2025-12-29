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

## Coding Standards

### Testing Best Practices

- **Use `assert_matches!` macro**: Always use `assert_matches!(value, Pattern { .. })` instead of `assert!(matches!(value, Pattern { .. }))` in tests. The `assert_matches!` macro provides clearer error messages when assertions fail.

## Commit Guidelines

When creating commits, follow these quality standards:

### Quality Check Process

**MANDATORY CHECKS** - All must pass before committing:
1. `nix shell -c cargo fmt` - Format all code
2. `nix shell -c cargo clippy -- -D warnings` - No warnings allowed
3. `nix shell -c cargo test` - All tests must pass

### Quality Check Loop

**IMPORTANT**: If any check fails:
1. Fix the issues in the code
2. Re-run ALL quality checks (fmt → clippy → tests) in sequence
3. Repeat until all checks pass

**NEVER commit code with:**
- Failing tests
- Clippy warnings
- Unformatted code

### Commit Message Format

**Subject Line** (max 50 characters):
- Use imperative mood: "Add feature" not "Added feature"
- Start with a verb: Add, Fix, Refactor, Update, Remove, etc.
- Be specific and concise
- No period at the end

**Body** (wrap at 72 characters):
- Explain WHY the change was made (the diff shows WHAT)
- Include context and motivation
- Reference related issues if applicable

**Example:**
```
Add commit skill with quality checks

Implements a custom Claude skill that ensures code quality before
committing by running tests, clippy, and formatting in a loop until
all checks pass. This prevents broken code from entering the repo.
```

### Pre-Commit Verification

Before committing, always:
1. Run `git status` and `git diff` to review changes
2. Verify all quality checks have passed
3. Ensure commit message follows the format above

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
