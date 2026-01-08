# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

CAD-DSL is a constraint-based domain-specific language for 2D geometric design. The project implements a complete frontend pipeline (lexer, parser, semantic analyzer, type checker) plus a constraint solver for a declarative CAD language using Rust.

**Language Specification:** `docs/TEXTCAD_LANGUAGE_SPEC.md`

## Development Environment

This project uses Nix for development environment management. Use `nix develop` or `nix-shell` to enter the development environment, which provides:

- Rust toolchain with rust-analyzer and LLVM tools
- Z3 constraint solver (system dependency)
- Code coverage tools (cargo-llvm-cov)
- mold linker (Linux only, for fast memory-efficient builds)

**Note:** If Nix is not available in your environment (e.g., CI/CD, Docker, or systems where Nix cannot be installed), see [CLAUDE-NO-NIX.md](CLAUDE-NO-NIX.md) for instructions on setting up dependencies manually using `apt-get` and running commands without the `nix shell -c` wrapper.

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

### Pipeline Overview

```
Source Code → Lexer → Parser → AST → Semantic Analyzer → HIR → Type Checker → Constraint Solver → Solution
```

### Core Components

**Lexer (`src/lexer.rs`)**
- Uses Logos for token generation
- Position tracking for error reporting
- Handles single-line (`//`) and multi-line (`/* */`) comments

**Parser (`src/parser.rs`)**
- Chumsky-based recursive descent parser
- Proper left-associative operators with precedence
- Rich error reporting with Ariadne integration
- Modular parser combinators in `src/parser/` submodules

**AST (`src/ast.rs`)**
- Type-safe expression AST with operator precedence hierarchy
- Uses subenum crate to enforce precedence at the type level
- Prevents invalid parse trees through the type system

**Semantic Analyzer (`src/semantic_analyzer/`)**
- Transforms AST to High-level Intermediate Representation (HIR)
- **Two-pass analysis** to support forward references:
  - Pass 1: Declaration collection (structs, functions, variables)
  - Pass 2: Name resolution and HIR construction
- Arena-based allocation for cross-references
- Output: Complete HIR with resolved names and types

**HIR (`src/hir/`)**
- High-level Intermediate Representation with complete semantic resolution
- Arena-allocated nodes with cross-references between definitions
- Key types: `ResolvedExpr`, `ResolvedStmt`, `ResolvedType`
- 30+ expression kinds, 11 statement kinds
- Submodules: `types.rs`, `definitions.rs`, `expr.rs`, `scope.rs`, `context.rs`

**Type Checker (`src/type_checker/`)**
- Performs type inference and validation on HIR
- Hindley-Milner inspired type inference algorithm
- Numeric type promotion (i32 → f64, bool → i32, etc.)
- Submodules: `inference.rs`, `validation.rs`, `context.rs`, `errors.rs`

**Solver Pipeline (`src/solver/`)**
- Orchestrates: semantic analysis → type checking → constraint extraction → Z3 solving
- **Struct and Array Flattening**: Nested structs and arrays become primitive fields (e.g., `line.start.x`, `points[0].x`)
- **Z3 Bridge**: Translates HIR expressions to Z3 constraints
- Returns SAT (with solution) or UNSAT (no solution exists)

**Solver Submodules:**
- `constraint_extractor.rs` - Extracts variables and constraints from typed HIR
- `struct_flattener.rs` - Flattens nested structs and arrays to primitive fields
- `recursive_struct_detector.rs` - Detects cycles in struct definitions
- `z3_bridge.rs` - Translates to Z3 solver format
- `solution_formatter.rs` - Formats Z3 solutions for display

### Key Design Patterns

**Type-Safe Precedence**: The AST uses Rust's type system to enforce operator precedence, making it impossible to construct invalid expression trees.

**Two-Pass Semantic Analysis**:
- Pass 1 collects all declarations (struct, function, variable names)
- Pass 2 resolves all references and constructs HIR
- Enables forward references (use before declaration)

**Arena Allocation**:
- All HIR nodes allocated in a single arena with lifetime `'arena`
- Cross-references use `&'arena T` pointers (no Rc/Arc needed)
- String slices use `&'src str` directly from source code
- Clean separation between source lifetime and arena lifetime

**Struct and Array Flattening for Constraints**:
- Structs and arrays are flattened to primitive fields for Z3 solving
- Struct example: `Point { x: i32, y: i32 }` becomes two variables `p.x` and `p.y`
- Array example: `[i32; 3]` becomes three variables `arr[0]`, `arr[1]`, `arr[2]`
- Array of structs: `[Point; 2]` becomes four variables `points[0].x`, `points[0].y`, `points[1].x`, `points[1].y`
- Supports arbitrary nesting depth with qualified names
- Array indexing supports only constant integer literals (not variable indices)

## Testing

The project has comprehensive test suites for each component. All major components have unit tests, integration tests, and error case coverage. Tests use the `assert_matches!` macro for clear error messages and include timeout mechanisms to prevent infinite loops during development.

**Test Organization:**
- Unit tests in component modules (`src/*/mod.rs`, `src/*/*.rs`)
- Integration tests in `tests/` directory
- Parser tests in `src/parser/tests/` submodules
- End-to-end tests in semantic analyzer and solver modules

## Language Implementation Status

### ✅ Fully Implemented

- **Lexer**: All tokens, comments, position tracking
- **Parser**: Expressions, statements, declarations (struct, function, let, if, for, with)
- **Semantic Analyzer**: Two-pass analysis, forward references, scope management, complete HIR generation
- **Type Checker**: Type inference (including struct literals), validation, numeric promotion
- **HIR**: Complete type-safe IR with arena allocation
- **Constraint Solver**:
  - Basic types: `i32`, `f64`, `bool`
  - Struct types (flattened to primitive fields)
  - Array types (fixed-size, flattened to indexed primitive fields)
  - Array indexing with constant integer indices
  - Arithmetic operators: `+`, `-`, `*`, `/`
  - Comparison operators: `==`, `!=`, `<`, `>`, `<=`, `>=`
  - If-statements with conditional constraints (Z3 ITE)
  - Nested structs with qualified names
  - Recursive struct detection

### 🚧 Partially Implemented (Parser/HIR Only)

These features have parser and HIR support but **not yet in constraint solver**:

- **For Loops**: Parsed and in HIR, needs loop unrolling in constraint extractor
- **Functions**: Definitions parsed and in HIR, function calls need solver support
- **With Statements**: Parsed and in HIR, transform semantics not in solver

### ❌ Not Yet Implemented

- **Standard Library**: `point()`, `distance()`, math functions
- **Reference Types**: Full entity vs. reference semantics
- **Container Structs**: Dynamic entity namespacing
- **Transform Pattern**: `__transform__` methods
- **Functional Operations**: `map`, `reduce`

### 🔮 Future Features (Low Priority)

- **Units System**: Length, Angle, Area with mm/cm/m/deg/rad suffixes
  - Language spec defines units extensively, but implementation deferred
  - Current solver works with dimensionless numeric types
  - Adds complexity across lexer, parser, type system, and solver
  - Recommended as later enhancement after core functionality is stable

## Next Implementation Steps

### Recommended Priority Order

1. **For Loops** (High Priority)
   - Requires arrays to be useful (arrays are now implemented)
   - Implement loop unrolling in constraint extractor
   - Generate constraints for each iteration

2. **Function Calls** (High Priority - Game Changer)
   - Function call inlining/expansion in HIR
   - Standard library basics: `point()`, `distance()`
   - Makes language practically usable for CAD workflows

3. **Reference Types** (Medium Priority)
   - Entity vs. reference distinction
   - Reference type validation
   - Important for correct semantics

4. **With Statements + Transforms** (Low Priority)
   - Coordinate transformations
   - Requires container structs
   - Complex feature, defer until core is stable

### Extension Guidelines

When adding new features to the constraint solver:

1. **Check Parser/HIR Support**: Many features already parsed, just need solver support
2. **Follow Struct Flattening Pattern**: Arrays are now implemented using this pattern
3. **Add Tests First**: Write integration tests before implementation
4. **Update This File**: Keep implementation status current

## Constraint Solver - Current Capabilities

### Supported

- Basic types: `i32`, `f64`, `bool`
- Struct types (automatically flattened to primitive fields)
- Array types (fixed-size, automatically flattened to indexed primitive fields)
- Array indexing with constant integer indices (e.g., `arr[0]`, `points[1].x`)
- Let statements with/without initializers
- Arithmetic: `+`, `-`, `*`, `/`
- Comparisons: `==`, `!=`, `<`, `>`, `<=`, `>=`
- If-statements with conditional constraints
- Nested structs with qualified names (e.g., `line.start.x`)
- Struct literal type inference

### Limitations

- No variable declarations inside if-statement branches
- No assignments inside if-statement branches
- Only constraint expressions in if-statement branches
- No nested if-statements
- Array indexing only supports constant integer indices (not variable indices)
- No for loops, functions, or with statements (yet)

### Examples

#### Struct Example

**Input file (struct_example.cad):**
```
struct Point {
    x: i32,
    y: i32,
}

let p: Point;
p.x + p.y == 15;
p.x == 10;
```

**Command:**
```bash
nix shell -c cargo run -- solve struct_example.cad
```

**Output:**
```
p.x = 10
p.y = 5
```

**How it works:** The struct variable `p` is automatically flattened into two primitive variables `p.x` and `p.y`. The Z3 solver finds values that satisfy both constraints.

#### Array Example

**Input file (array_example.cad):**
```
struct Point {
    x: i32,
    y: i32,
}

let points: [Point; 2];
points[0].x == 1;
points[0].y == 2;
points[1].x == 3;
points[1].y == 4;
```

**Command:**
```bash
nix shell -c cargo run -- solve array_example.cad
```

**Output:**
```
points[0].x = 1
points[0].y = 2
points[1].x = 3
points[1].y = 4
```

**How it works:** The array variable `points` is automatically flattened into four primitive variables: `points[0].x`, `points[0].y`, `points[1].x`, and `points[1].y`. The Z3 solver finds values that satisfy all constraints.

## Dependencies

Key dependencies:
- `ariadne` - Error reporting with source code highlighting
- `assert_matches` - Pattern matching assertions in tests
- `bumpalo` - Arena allocator for HIR nodes
- `chumsky` - Parser combinators
- `clap` - CLI interface
- `logos` - Lexical analysis
- `subenum` - Type-safe enum subsets
- `z3` - Constraint solver integration

Z3 constraint solver is provided as both a system dependency (via Nix) and a Rust crate dependency for constraint solving implementation.
