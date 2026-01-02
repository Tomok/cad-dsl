# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

CAD-DSL is a constraint-based domain-specific language for 2D geometric design. The project implements a complete frontend pipeline (lexer, parser, semantic analyzer, type checker) for a declarative CAD language using Rust. The language specification is documented in `docs/TEXTCAD_LANGUAGE_SPEC.md`.

## Development Environment

This project uses Nix for development environment management. Use `nix develop` or `nix-shell` to enter the development environment, which provides:

- Rust toolchain with rust-analyzer and LLVM tools
- Z3 constraint solver (system dependency)
- Code coverage tools (cargo-llvm-cov)
- mold linker (Linux only, for fast memory-efficient builds)

## Common Commands

### Building and Testing
- `nix shell -c cargo build` - Build the project
- `nix shell -c cargo test` - Run all tests
- `nix shell -c cargo test <test_name>` - Run specific test
- `nix shell -c cargo test -- --nocapture` - Run tests with output visible

### Running the CLI
- `nix shell -c cargo run -- lex <file.cad>` - Tokenize a CAD file and display tokens
- `nix shell -c cargo run -- parse <file.cad>` - Parse a CAD file and display AST
- `nix shell -c cargo run -- solve <file.cad>` - Solve constraints and display variable assignments

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

**Semantic Analyzer (`src/semantic_analyzer.rs`)**
- Transforms AST to High-level Intermediate Representation (HIR)
- Two-pass analysis to support forward references
- Pass 1: Declaration collection (structs, functions, top-level variables)
- Pass 2: Name resolution and HIR construction (produces ResolvedStmt and ResolvedExpr)
- Arena-based allocation for cross-references
- Comprehensive error reporting with span tracking
- Output: Complete HIR with no AST nodes

**Semantic Analyzer Submodules:**
- `semantic_analyzer_errors.rs` - Error types for semantic analysis (9 error variants)
- `semantic_analyzer_context.rs` - Analyzer context with symbol tables and scope management
- `semantic_analyzer_pass1.rs` - Declaration collection with two-phase type resolution
- `semantic_analyzer_pass2.rs` - AST to HIR transformation with name resolution

**Type Checker (`src/type_checker.rs`)**
- Performs type inference and validation on HIR
- Works with ResolvedStmt and ResolvedExpr (not AST)
- Ensures type safety across the program
- Hindley-Milner inspired type inference algorithm
- Type validation for assignments, function calls, and operators
- Numeric type promotion (i32 → f64, bool → i32, etc.)
- Comprehensive error reporting with span tracking

**Type Checker Submodules:**
- `type_checker_errors.rs` - Type-specific errors (8 error variants)
- `type_checker_context.rs` - Type checking context with constraint management
- `type_checker_inference.rs` - Type inference algorithm with unification
- `type_checker_validation.rs` - Type validation and compatibility checks

**Constraint Extractor (`src/constraint_extractor.rs`)**
- Extracts variables and constraints from typed HIR
- Identifies uninitialized variables that need solving
- Collects constraint equations from expression statements
- Validates that constraints are solvable (basic types only)
- Supports: let statements, assignments, comparison operators, arithmetic
- Out of scope: control flow, structs, functions, standard library

**Z3 Bridge (`src/z3_bridge.rs`)**
- Translates HIR expressions to Z3 solver format
- Maps TextCAD types to Z3 sorts (i32 → Int, f64 → Real, bool → Bool)
- Converts arithmetic and comparison operators to Z3 operations
- Creates Z3 variables and assertions from constraint equations
- Type-safe wrapper around Z3 API

**Solution Formatter (`src/solution_formatter.rs`)**
- Extracts solutions from Z3 models
- Formats variable assignments for display
- Handles type-specific value extraction (Int, Real, Bool)
- Provides user-friendly output of constraint solutions
- Error handling for unsatisfiable constraints

**Solver (`src/solver.rs`)**
- End-to-end constraint solving pipeline
- Orchestrates: semantic analysis → type checking → constraint extraction → Z3 solving → formatting
- Returns SAT (with solution) or UNSAT (no solution exists)
- Integrates all constraint solving components
- Main entry point for constraint solving from HIR

**HIR (High-level IR) Modules:**
- `hir_types.rs` - Resolved types with struct definition references
- `hir_definitions.rs` - Definitions for variables, functions, structs, and fields
- `hir_expr.rs` - Resolved expressions and statements with type information
  - ResolvedExpr: 30+ expression kinds with type annotations
  - ResolvedStmt: 11 statement kinds (Let, Assignment, If, For, FunctionDef, StructDef, Return, Expression, Block, With, FieldAssignment)
  - All HIR nodes use arena allocation with cross-references to definitions
- `hir_context.rs` - With-context support for container field resolution
- `hir_scope.rs` - Scope management with lexical scoping and shadowing

**CLI (`src/main.rs`)**
- CLI with `lex`, `parse`, and `solve` subcommands
- File input handling and error reporting
- Integration with lexer, parser, semantic analyzer, type checker, and solver

### Key Design Patterns

**Type-Safe Precedence**: The AST uses Rust's type system to enforce operator precedence, making it impossible to construct invalid expression trees.

**Rich Error Reporting**: Parser and semantic analyzer errors include position information and expected vs. found tokens, formatted with Ariadne for user-friendly output.

**Two-Pass Semantic Analysis**: The semantic analyzer uses a two-pass approach to support forward references in CAD-DSL:
- Pass 1 collects all declarations (struct, function, variable names)
- Pass 2 resolves all references and constructs HIR (ResolvedStmt and ResolvedExpr)
- This allows variables to reference types or functions defined later in the source
- Output is pure HIR with complete type information and cross-references

**Arena Allocation**: The HIR uses arena allocation (bumpalo) for memory management:
- All HIR nodes are allocated in a single arena with lifetime `'arena`
- Cross-references use `&'arena T` pointers (no Rc/Arc needed)
- String slices use `&'src str` directly from source code
- Clean separation between source lifetime and arena lifetime

**Separation of Concerns**: Clear separation between lexical analysis, syntactic analysis, semantic analysis, and CLI interface.

## Testing

The project has comprehensive test suites for each component:

- **Lexer tests**: Token recognition, position tracking, comment handling
- **Parser tests**: Expression parsing, precedence, error cases with timeout protection
- **AST tests**: Type conversions and display formatting
- **HIR tests**:
  - ResolvedExpr: Construction and type annotation tests
  - ResolvedStmt: 16 tests for statement construction and behavior
  - Integration tests: 17 end-to-end tests covering complete source → HIR → type checker pipeline
- **Semantic Analyzer tests**:
  - Error type formatting (12 tests)
  - Context operations and symbol tables (11 tests)
  - Declaration collection with duplicates (11 tests)
  - Resolution and HIR construction (9 tests)
  - Full pipeline integration tests (17 tests)
- **Type Checker tests**:
  - Error type formatting (11 tests in type_checker_errors)
  - Context operations (17 tests in type_checker_context)
  - Type inference (25 tests in type_checker_inference)
  - Type validation (25 tests in type_checker_validation)
  - Integration tests (6 tests in type_checker)
- **Constraint Extractor tests**: Variable extraction, constraint identification, error handling
- **Z3 Bridge tests**: Expression translation, type mapping, Z3 assertion creation
- **Solution Formatter tests**: Model extraction, value formatting, error cases
- **Solver tests**: End-to-end pipeline, SAT/UNSAT cases, integration tests

Tests use timeout mechanisms to prevent infinite loops during development. The semantic analyzer has 60 comprehensive tests covering declaration collection, name resolution, scoping, error cases, and the complete analysis pipeline. The type checker has 84 comprehensive tests covering type inference, validation, numeric promotion, error cases, and the complete type checking pipeline. The constraint solver has comprehensive tests covering variable extraction, Z3 translation, solution formatting, and end-to-end solving.

## Language Implementation Status

Currently implements:
- **Complete lexical analysis** for TextCAD syntax (lexer)
- **Syntactic analysis** with proper operator precedence (parser)
- **Semantic analysis** with name resolution (semantic analyzer)
- **Type checking** with inference and validation (type checker)
- **HIR construction** for further compilation stages
- **Constraint solving** with Z3 integration (solver)
- **Comprehensive error reporting** infrastructure across all stages

### Completed Features:
- Lexer: All tokens, comments, position tracking
- Parser: Expressions, statements, declarations
- Semantic Analyzer: Two-pass analysis, forward references, scope management, complete HIR generation
- Type Checker: Type inference, validation, numeric promotion (works on HIR)
- HIR: Complete type-safe intermediate representation with arena allocation
  - ResolvedExpr: 30+ expression kinds with type annotations
  - ResolvedStmt: 11 statement kinds with cross-references
  - All AST nodes transformed to HIR in semantic analyzer
- Constraint Solver: Z3 integration for solving constraint equations
  - Extracts variables and constraints from typed HIR
  - Translates to Z3 format and solves for unknowns
  - Supports basic types (i32, f64, bool) and arithmetic/comparison operators
  - Returns variable assignments or UNSAT for unsolvable constraints

### Next Steps:
- Code generation or interpretation

The language specification in `docs/TEXTCAD_LANGUAGE_SPEC.md` defines the full TextCAD language, including constraints, structs, transforms, and the standard library. The current implementation covers the complete frontend pipeline from source code to typed HIR, plus constraint solving.

## Constraint Solving

The project integrates Z3 constraint solver to solve for unknown variables in constraint equations.

### Scope

**Supported:**
- Basic types: `i32`, `f64`, `bool`
- Let statements with and without initializers
- Expression statements containing constraints
- Comparison operators: `==`, `!=`, `<`, `>`, `<=`, `>=`
- Arithmetic operators: `+`, `-`, `*`, `/`

**Not Supported (Out of Scope):**
- Control flow: `if`, `for`, `with` statements
- Structs and struct fields
- Functions and function calls
- Standard library functions
- Transforms and geometric operations

### Usage Example

**Input file (example.cad):**
```
let x;
let y = 10;
x + y == 20;
```

**Command:**
```bash
nix shell -c cargo run -- solve example.cad
```

**Output:**
```
x = 10
y = 10
```

### Pipeline

Source Code → Lexer → Parser → Semantic Analyzer → Type Checker → Constraint Extractor → Z3 Bridge → Solution

## Dependencies

Key dependencies:
- `ariadne` - Error reporting
- `assert_matches` - Pattern matching assertions in tests
- `bumpalo` - Arena allocator for HIR nodes
- `chumsky` - Parser combinators
- `clap` - CLI interface
- `logos` - Lexical analysis
- `subenum` - Type-safe enum subsets
- `z3` - Constraint solver integration

Z3 constraint solver is provided as both a system dependency (via Nix) and a Rust crate dependency for constraint solving implementation.