# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

CAD-DSL is a constraint-based domain-specific language for 2D geometric design. The project implements a complete frontend pipeline (lexer, parser, semantic analyzer, type checker) plus a constraint solver for a declarative CAD language using Rust.

**Language Specification:** `docs/TEXTCAD_LANGUAGE_SPEC.md`
**Solver Architecture:** `docs/SOLVER_ARCHITECTURE.md`

## Development Environment

**CRITICAL - AUTO-RUN ON EVERY SESSION:** Before running ANY commands, Claude Code MUST execute:

```bash
source ./.claude_env.sh
```

This script:
- Installs mold (linker) and z3 (constraint solver) to ~/.local without sudo
- Sets up all required environment variables (PATH, LD_LIBRARY_PATH, PKG_CONFIG_PATH, etc.)
- Creates a nix wrapper for compatibility
- Works silently on success, only outputs errors

After sourcing the script, all commands work identically with or without Nix installed.

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

### Error Handling Best Practices

**IMPORTANT**: When implementing features, always fail explicitly rather than silently when encountering unsupported cases.

#### Prefer `todo!()` Over Silent Failures

- **Never silently ignore or clone unhandled cases**: Using fallback patterns like `_ => value.clone()` hides missing functionality and can produce incorrect results
- **Use `todo!()` for unsupported features**: This makes it immediately clear when unimplemented code paths are reached
- **Include descriptive messages**: Explain what feature is missing and why it matters
- **Report to user**: The `todo!()` panic will inform users they've hit an unsupported feature

**Example - BAD (Silent failure):**
```rust
match expr {
    SupportedCase => handle_it(),
    _ => expr.clone()  // Silently ignores unsupported cases!
}
```

**Example - GOOD (Explicit failure):**
```rust
match expr {
    SupportedCase => handle_it(),
    _ => todo!(
        "Feature X not implemented for this expression type: {:?}. \
         This will cause incorrect behavior. Please report this case.",
        expr
    )
}
```

#### When to Use Different Error Strategies

- **`todo!()`**: For features you know should be implemented but aren't yet
  - Missing expression types in pattern matching
  - Unimplemented optimization passes
  - Placeholder functions

- **`Result<T, E>`**: For recoverable errors that should be propagated
  - Parse errors
  - Type errors
  - Constraint solver failures

- **`unreachable!()`**: For cases that are impossible by construction
  - After exhaustive matches that are proven complete
  - Type system guarantees

- **`panic!()`**: For unrecoverable errors indicating bugs
  - Invariant violations
  - Internal consistency failures

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
  - Arithmetic operators: `+`, `-`, `*`, `/`, `%` (modulo), `^` (power)
  - Comparison operators: `==`, `!=`, `<`, `>`, `<=`, `>=`
  - Logical operators: `and`, `or`
  - Unary operators: `-` (negation), `&` (reference)
  - **If-statements**: Conditional constraints with Z3 ITE, nested if-statements, assignments in branches
  - **For loops**: Loop unrolling with constant ranges, deferred constraint solving for variable-dependent ranges
  - **Function calls**: Function inlining with parameter substitution
  - **Method calls**: Method inlining with receiver binding
  - Nested structs with qualified names
  - Recursive struct detection
  - **Container with-statements** (dot-prefix syntax for namespacing)
  - **Transform with-statements**: Automatic coordinate transformations via `__transform__` methods with container and view variables
  - **Struct literals**: Full support in variable initialization and transform return values

### ❌ Not Yet Implemented

- **Rune Blocks**: Imperative code blocks for complex calculations (see `docs/RUNE_BLOCKS_IMPLEMENTATION.md`)
- **Standard Library**: `point()`, `distance()`, math functions
- **Reference Types**: Full entity vs. reference semantics
- **Functional Operations**: `map`, `reduce`
- **Field Assignment**: Direct field assignment syntax (`p.x = 5` not supported, use constraint syntax `p.x == 5`)

### 🔮 Future Features (Low Priority)

- **Units System**: Length, Angle, Area with mm/cm/m/deg/rad suffixes
  - Language spec defines units extensively, but implementation deferred
  - Current solver works with dimensionless numeric types
  - Adds complexity across lexer, parser, type system, and solver
  - Recommended as later enhancement after core functionality is stable

## Next Implementation Steps

### Recommended Priority Order

1. **Rune Blocks** (High Priority - In Progress)
   - Imperative code blocks for complex calculations
   - Enables algorithms difficult to express as constraints (iterative methods, accumulation, complex conditionals)
   - Syntax: `let result = rune(x, y) { /* Rune code */ }`
   - Parameter assignments: `rune(x=p.x, y, z=100) { ... }`
   - Executes after constraint solving for parameters
   - Results can constrain other variables (one-way data flow)
   - **Status**: Planning complete, implementation not started
   - **Plan**: See `docs/RUNE_BLOCKS_IMPLEMENTATION.md` for detailed implementation plan
   - **Timeline**: 12-16 days estimated

2. **Standard Library** (High Priority - Game Changer)
   - Basic constructors: `point(x, y)` for Point creation
   - Geometric functions: `distance(p1, p2)`, `midpoint(p1, p2)`
   - Math functions: `sqrt()`, `sin()`, `cos()`, `tan()`, `abs()`
   - Makes language practically usable for CAD workflows
   - Builds on existing function call infrastructure

3. **Reference Types** (Medium Priority)
   - Entity vs. reference distinction
   - Reference type validation
   - Important for correct semantics
   - Requires type system enhancements

4. **Field Assignment Syntax** (Low Priority)
   - Direct field assignment: `p.x = 5` instead of constraint syntax `p.x == 5`
   - Currently workaround exists (use constraints)
   - Nice-to-have for ergonomics but not critical

### Extension Guidelines

When adding new features to the constraint solver:

1. **Check Parser/HIR Support**: Many features already parsed, just need solver support
2. **Follow Struct Flattening Pattern**: Arrays are now implemented using this pattern
3. **Add Tests First**: Write integration tests before implementation
4. **Update This File**: Keep implementation status current

## Constraint Solver - Current Capabilities

### Supported

- **Types**: `i32`, `f64`, `bool`, structs (auto-flattened), arrays (fixed-size, auto-flattened)
- **Variable Declarations**: Let statements with/without initializers, dot-prefix variables in containers
- **Operators**:
  - Arithmetic: `+`, `-`, `*`, `/`, `%` (modulo), `^` (power), unary `-`
  - Comparisons: `==`, `!=`, `<`, `>`, `<=`, `>=`
  - Logical: `and`, `or`
- **Array Access**: Constant integer indices (e.g., `arr[0]`, `points[1].x`)
- **Control Flow**:
  - If-statements with conditional constraints (Z3 ITE), nested if-statements, assignments in branches
  - For loops with constant or variable-dependent ranges (automatic loop unrolling and deferred solving)
- **Functions**: Function calls (inlining with parameter substitution), method calls (with receiver binding)
- **Struct Features**: Nested structs with qualified names (e.g., `line.start.x`), struct literal initialization (e.g., `Point { x: 5, y: 10 }`)
- **With-Statements**:
  - Container contexts with dot-prefix syntax for namespacing
  - Transform contexts with automatic coordinate transformations via `__transform__` methods
  - See `docs/HIR_TRANSFORM_REPRESENTATION.md` for detailed transform semantics

### Limitations

**Type System:**
- Array indexing only supports constant integer indices (not variable indices)
- No generic types or type parameters

**Control Flow:**
- Variable declarations inside if-statement branches are not supported (only constraints and assignments)
- Assignments inside if-statements create conditional constraints (not mutations)
- For-loop bodies only support constraint expressions (e.g., `arr[i] == value`), not assignment statements or variable declarations

**Functions:**
- No standard library yet (no built-in `point()`, `distance()`, math functions)
- No recursion support

**Struct Features:**
- Field assignment syntax not supported: `p.x = 5` must be written as constraint `p.x == 5`
- Struct literal initialization is fully supported: `let p: Point = Point { x: 5, y: 10 };`

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

#### Container With-Statement Example

**Input file (with_statement_example.cad):**
```
struct Point {
    x: i32,
    y: i32,
}

struct Sketch {
    container entities,
}

let sketch: Sketch;

with sketch {
    let .p1: Point;
    let .p2: Point;
    .p1.x == 0;
    .p1.y == 0;
    .p2.x == .p1.x + 10;
    .p2.y == .p1.y + 10;
}
```

**Command:**
```bash
cargo run -- solve with_statement_example.cad
```

**Output:**
```
sketch.entities.p1.x = 0
sketch.entities.p1.y = 0
sketch.entities.p2.x = 10
sketch.entities.p2.y = 10
```

**How it works:** The `with sketch { ... }` block creates a container context. Inside the block, the dot-prefix syntax (`.p1`, `.p2`) creates variables in the container's namespace (`sketch.entities.p1`, `sketch.entities.p2`). These variables are automatically flattened and solved like regular struct variables.

#### If-Statement Example

**Input file (if_statement_example.cad):**
```
let x: i32;
let y: i32;

x > 10;

if x > 20 {
    y = x * 2;
} else {
    y = x + 5;
}
```

**Command:**
```bash
cargo run -- solve if_statement_example.cad
```

**Output (example solution):**
```
x = 21
y = 42
```

**How it works:** The if-statement is translated to a Z3 ITE (if-then-else) constraint. The solver finds values satisfying: `x > 10` AND `(x > 20 → y = x*2) OR (x ≤ 20 → y = x+5)`. Since `x > 20` is possible, the solver chooses a value like `x = 21`, which triggers the then-branch (`y = 42`).

#### For Loop Example

**Input file (for_loop_example.cad):**
```
let arr: [i32; 3];

for i in 0..3 {
    arr[i] == i * 10;
}
```

**Command:**
```bash
cargo run -- solve for_loop_example.cad
```

**Output:**
```
arr[0] = 0
arr[1] = 10
arr[2] = 20
```

**How it works:** The for loop is automatically unrolled into separate constraints for each iteration. The loop body is executed with `i` substituted for each value in the range `0..3`, creating three equality constraints (`arr[0] == 0`, `arr[1] == 10`, `arr[2] == 20`) that the solver resolves.

#### Function Call Example

**Input file (function_call_example.cad):**
```
fn double(x: i32) -> i32 {
    return x * 2;
}

let a: i32;
let b: i32;

a == 5;
b = double(a);
```

**Command:**
```bash
cargo run -- solve function_call_example.cad
```

**Output:**
```
a = 5
b = 10
```

**How it works:** The function call `double(a)` is inlined by substituting the parameter `x` with the argument `a` in the function's return expression. This creates the constraint `b = a * 2`, which the solver resolves using the constraint `a == 5`.

#### Transform With-Statement Example

**Input file (transform_example.cad):**
```
struct Point2D {
    x: f64,
    y: f64,
}

struct Point3D {
    x: f64,
    y: f64,
    z: f64,
}

struct Sketch2D {
    container entities,
    origin: Point3D,

    fn __transform__(p3d: &Point3D) -> Point2D {
        return Point2D {
            x: p3d.x - self.origin.x,
            y: p3d.y - self.origin.y,
        };
    }
}

let sketch: Sketch2D;
sketch.origin.x == 0.0;
sketch.origin.y == 0.0;
sketch.origin.z == 0.0;

with sketch {
    let .p: Point2D;
    .p.x == 10.0;
    .p.y == 20.0;
}
```

**Command:**
```bash
cargo run -- solve transform_example.cad
```

**Output:**
```
sketch.entities.p.x = 10
sketch.entities.p.y = 20
sketch.origin.x = 0
sketch.origin.y = 0
sketch.origin.z = 0
```

**How it works:** When a variable is declared with a 2D type in a transform context (e.g., `let .p: Point2D`), the semantic analyzer creates two variables:
1. **Container variable**: `sketch.entities.p: Point3D` - the real entity in 3D world space
2. **View variable**: `p: Point2D` - temporary transformed view, only visible inside the with-block

The `__transform__` method links them via constraints. View variables are filtered from output; only container variables (with full 3D coordinates) are shown. See `docs/HIR_TRANSFORM_REPRESENTATION.md` for complete details.

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

Planned dependencies (for Rune blocks implementation):
- `rune` - Embedded scripting language for imperative code blocks (not yet added)
- `rune-alloc` - Memory allocation support for Rune (not yet added)

Z3 constraint solver is provided as both a system dependency (via Nix) and a Rust crate dependency for constraint solving implementation.
- if you find cases that are not handled correctly, create unit or integration tests for them