# HIR Transform Representation - Implementation Plan

## Executive Summary

This document describes a planned architectural improvement to move transform-related semantics from the solver phase into the High-Level Intermediate Representation (HIR). Currently, transform application (shadow variable creation, transform inlining) happens during constraint solving, which violates separation of concerns. This plan proposes representing transforms directly in the HIR during semantic analysis.

**Status**: Planning (Not Yet Implemented)

**Estimated Effort**: 5-7 days

**Priority**: High (Architectural improvement)

---

## Table of Contents

1. [Problem Statement](#problem-statement)
2. [Current Architecture](#current-architecture)
3. [Proposed Solution](#proposed-solution)
4. [Detailed Design](#detailed-design)
5. [Implementation Steps](#implementation-steps)
6. [Migration Strategy](#migration-strategy)
7. [Testing Strategy](#testing-strategy)
8. [Benefits and Trade-offs](#benefits-and-trade-offs)
9. [Future Extensions](#future-extensions)

---

## Problem Statement

### The Core Issue

Transform application is a **semantic transformation**, not a **code generation detail**. However, the current implementation performs transform logic in the solver phase:

```
Current Architecture:
AST → HIR (Name Resolution, Type Checking)
HIR → Solver (Semantic Transform + Z3 Code Gen)  ← Mixed responsibilities!

Desired Architecture:
AST → HIR (Name Resolution, Type Checking, Transform Lowering)
HIR → Solver (Pure Z3 Code Gen)
```

### Specific Problems

1. **Late Error Detection**: Transform-related errors are only caught during solving, not during semantic analysis
2. **Scattered Logic**: Transform metadata in HIR, transform application in solver
3. **Hard to Test**: Must run entire solver to test transform behavior
4. **Unclear Responsibility**: Is transform semantics or code generation?
5. **Missing Semantic Information**: HIR doesn't show that variable `x` is actually defined as `transform(shadow_var)`
6. **Impediment to Other Analyses**: Dataflow analysis, optimizations, and alternative backends cannot see transform relationships

### Example Problem

Given this code:
```cad
with sketch {
    let .p: Point2D;
    .p.x == 10.0;
}
```

**Current HIR** says: "Variable `p` has type `Point2D`"

**What HIR should say**: "Variable `p` is defined as `sketch.__transform__(__shadow_0)` where `__shadow_0: Point3D`"

The **definition** of the variable is missing from the HIR!

---

## Current Architecture

### Where Transform Logic Lives Now

| Phase | Component | Responsibility |
|-------|-----------|----------------|
| **Semantic Analysis** | `src/semantic_analyzer/pass2.rs` | Collects transform metadata (which methods exist, their types) |
| **HIR** | `src/hir/context.rs` | Stores `WithContext` with `Vec<TransformMethod>` |
| **Solver** | `src/solver/impls/stmt.rs` | **Applies transforms**: creates shadow variables, inlines methods, generates constraints |

### Transform Application Pipeline (Current)

Located in `src/solver/impls/stmt.rs::apply_transform_to_variable()`:

1. **Check context**: Is variable in transform context?
2. **Select transform**: Find matching `__transform__` method
3. **Create shadow variable**: Generate `__shadow_0`, `__shadow_1`, etc.
4. **Create reference**: Make HIR expression referencing shadow
5. **Inline transform**: Substitute parameters in transform method body
6. **Add constraint**: `declared_var == transform_result`

### The Problem with This Approach

The solver is doing **semantic work** (defining what a variable means), not just **code generation** (translating to Z3).

---

## Proposed Solution

### High-Level Approach

**Move transform application from solver to semantic analyzer**, making the HIR fully represent the program's semantics including transform relationships.

### Key Principle

> **The HIR should contain complete semantic information about how every variable is defined.**

If a variable is defined through a transform, that should be explicit in the HIR, not implicitly applied later.

### What Changes

| Aspect | Before | After |
|--------|--------|-------|
| **Shadow Variables** | Created during solving | Created during semantic analysis |
| **Transform Expressions** | Inlined during solving | Constructed in HIR |
| **Variable Definition** | HIR: "var exists" | HIR: "var = transform(shadow)" |
| **Solver Role** | Semantic + Code Gen | Pure Code Gen (HIR → Z3) |
| **Error Detection** | During solving | During semantic analysis |

---

## Detailed Design

### Option 1: `VarDefinitionKind` Enum (Recommended)

Extend `VarDefinition` to explicitly represent how a variable is defined:

```rust
// File: src/hir/definitions.rs

pub struct VarDefinition<'src, 'arena> {
    /// Variable name as it appears in source
    pub name: &'src str,

    /// Span of the variable name for error reporting
    pub name_span: Span,

    /// Type of the variable
    pub var_type: Option<HirType<'src, 'arena>>,

    /// How this variable is defined (NEW!)
    pub definition_kind: VarDefinitionKind<'src, 'arena>,

    /// Scope level where this variable was defined
    pub scope_level: ScopeLevel,

    /// Full span of the variable definition
    pub span: Span,
}

/// How a variable is defined
#[derive(Debug, Clone, PartialEq)]
pub enum VarDefinitionKind<'src, 'arena> {
    /// Uninitialized variable: `let x: i32;`
    /// The solver will find a value satisfying all constraints
    Uninitialized,

    /// Initialized with explicit expression: `let x = 5;`
    Initialized {
        /// The initialization expression
        init: &'arena ResolvedExpr<'src, 'arena>,
    },

    /// Defined through coordinate transform: `with sketch { let .p: Point2D; }`
    /// The variable's value is computed by transforming a shadow variable
    Transformed {
        /// The internal shadow variable (e.g., `__shadow_0: Point3D`)
        /// This variable is not declared by the user but created automatically
        /// to represent the pre-transform coordinate space
        shadow_var: &'arena VarDefinition<'src, 'arena>,

        /// The transform constraint expression
        /// Represents: self_var == transform_method(context, &shadow_var)
        /// Example: `.p == sketch.__transform__(&__shadow_0)`
        transform_expr: &'arena ResolvedExpr<'src, 'arena>,

        /// The transform method being applied
        transform_method: &'arena FunctionDefinition<'src, 'arena>,

        /// The with-context that provides the transform
        with_context: &'arena WithContext<'src, 'arena>,
    },
}
```

### Why This Design?

**Type Safety**: The three definition kinds are mutually exclusive - a variable is either uninitialized, initialized, or transformed. The type system enforces handling all cases.

**Semantic Clarity**: Reading the HIR immediately reveals how each variable is defined.

**Extensibility**: Future definition kinds can be added easily (e.g., `ConstraintDefined` for implicit definitions, `LoopInductionVariable` for loop counters).

### Shadow Variable Representation

Shadow variables are regular `VarDefinition` instances with special naming:

```rust
VarDefinition {
    name: "__shadow_0",  // Generated name
    var_type: Some(Point3D),  // Input type of transform
    definition_kind: VarDefinitionKind::Uninitialized,  // Shadow is free variable
    scope_level: with_scope_level,
    ...
}
```

**Key properties**:
- Shadow variables are **free variables** (uninitialized) that the solver will assign values to
- They represent the **pre-transform coordinate space**
- They are **filtered from output** (users don't see `__shadow_0 = ...`)
- Their names start with `__shadow_` prefix for easy identification

### Transform Expression Structure

The `transform_expr` field contains a fully-resolved HIR expression representing the transform call:

```rust
// Example: .p == sketch.__transform__(&__shadow_0)

ResolvedExpr {
    kind: ResolvedExprKind::BinaryOp {
        op: BinaryOperator::Eq,
        left: ResolvedExpr {
            kind: Var { name: "p", definition: &p_var_def },
            ty: Point2D,
        },
        right: ResolvedExpr {
            kind: MethodCall {
                receiver: ResolvedExpr {
                    kind: Var { name: "sketch", definition: &sketch_var_def },
                    ty: Sketch,
                },
                method: "__transform__",
                method_def: &transform_method_def,
                args: [
                    ResolvedExpr {
                        kind: UnaryOp {
                            op: UnaryOperator::Ref,
                            operand: ResolvedExpr {
                                kind: Var {
                                    name: "__shadow_0",
                                    definition: &shadow_var_def
                                },
                                ty: Point3D,
                            }
                        },
                        ty: &Point3D,
                    }
                ],
            },
            ty: Point2D,
        },
    },
    ty: bool,  // Constraint expression
}
```

This is a **complete, type-checked HIR expression** that can be:
- Analyzed by dataflow algorithms
- Optimized (e.g., constant folding)
- Translated to different backends (Z3, C++, etc.)

---

## Implementation Steps

### Phase 1: Extend HIR Data Structures (1-2 days)

**Files to modify**:
- `src/hir/definitions.rs`
- `src/hir/expr.rs` (if needed for transform expressions)

**Tasks**:
1. Add `VarDefinitionKind` enum to `definitions.rs`
2. Replace `init: Option<HirExpr>` with `definition_kind: VarDefinitionKind` in `VarDefinition`
3. Update all existing `VarDefinition` construction sites to use `VarDefinitionKind::Initialized` or `VarDefinitionKind::Uninitialized`
4. Add helper methods:
   - `VarDefinition::is_transformed() -> bool`
   - `VarDefinition::get_shadow_var() -> Option<&VarDefinition>`
   - `VarDefinition::get_init_expr() -> Option<&ResolvedExpr>`

**Testing**:
- Ensure all existing tests still pass with refactored structure
- Add unit tests for new enum variants

### Phase 2: Implement Shadow Variable Generation in Semantic Analyzer (2 days)

**Files to modify**:
- `src/semantic_analyzer/pass2.rs`

**Tasks**:

#### 2.1 Add Shadow Variable Counter

```rust
pub struct AnalyzerContext<'src, 'arena> {
    // ... existing fields

    /// Counter for generating unique shadow variable names
    shadow_var_counter: usize,
}

impl<'src, 'arena> AnalyzerContext<'src, 'arena> {
    fn generate_shadow_name(&mut self) -> String {
        let name = format!("__shadow_{}", self.shadow_var_counter);
        self.shadow_var_counter += 1;
        name
    }
}
```

#### 2.2 Add Transform Detection Logic

```rust
/// Check if a variable type requires transform in current with-context
fn should_apply_transform<'src, 'arena>(
    ctx: &AnalyzerContext<'src, 'arena>,
    var_type: &ResolvedType<'src, 'arena>,
) -> Option<&'arena TransformMethod<'src, 'arena>> {
    let with_ctx = ctx.scope_stack.current_with_context()?;

    // Find transform method with matching output type
    with_ctx.transforms.iter()
        .find(|tm| types_match(tm.output_type, var_type))
}
```

#### 2.3 Modify Variable Declaration Logic

Extend `resolve_let_statement()` to detect and apply transforms:

```rust
fn resolve_let_statement<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    name_path: &[(&'src str, Span)],
    ty: Option<&Type<'src>>,
    init: Option<&Expr<'src>>,
    span: Span,
) -> Option<&'arena ResolvedStmt<'src, 'arena>> {
    // Resolve type
    let var_type = resolve_type(ctx, ty)?;

    // Check if this variable should be transformed
    if let Some(transform_method) = should_apply_transform(ctx, &var_type) {
        return resolve_transformed_variable(
            ctx,
            name_path,
            var_type,
            transform_method,
            span
        );
    }

    // Normal variable (existing logic)
    // ...
}
```

#### 2.4 Implement Transform Variable Creation

```rust
fn resolve_transformed_variable<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    name_path: &[(&'src str, Span)],
    output_type: &'arena ResolvedType<'src, 'arena>,
    transform_method: &'arena TransformMethod<'src, 'arena>,
    span: Span,
) -> Option<&'arena ResolvedStmt<'src, 'arena>> {
    let with_ctx = ctx.scope_stack.current_with_context()
        .expect("Transform without with-context");

    // 1. Generate shadow variable name
    let shadow_name = ctx.generate_shadow_name();
    let shadow_name_arena = ctx.arena.alloc_str(&shadow_name);

    // 2. Create shadow variable with input type
    let shadow_var_def = ctx.arena.alloc(VarDefinition {
        name: shadow_name_arena,
        name_span: span,
        var_type: Some(transform_method.input_type),
        definition_kind: VarDefinitionKind::Uninitialized,
        scope_level: ctx.scope_stack.current_level(),
        span,
    });

    // 3. Register shadow variable in scope
    ctx.scope_stack.add_variable(shadow_name_arena, shadow_var_def);

    // 4. Build transform expression: var == context.__transform__(&shadow)
    let transform_expr = build_transform_expression(
        ctx,
        with_ctx,
        transform_method,
        shadow_var_def,
        span,
    )?;

    // 5. Create main variable with Transformed kind
    let (var_name, _) = name_path.last().unwrap();
    let var_def = ctx.arena.alloc(VarDefinition {
        name: var_name,
        name_span: span,
        var_type: Some(output_type),
        definition_kind: VarDefinitionKind::Transformed {
            shadow_var: shadow_var_def,
            transform_expr,
            transform_method: transform_method.function,
            with_context: with_ctx,
        },
        scope_level: ctx.scope_stack.current_level(),
        span,
    });

    // 6. Register variable in scope
    let full_path = resolve_variable_path(ctx, name_path);
    ctx.scope_stack.add_variable(&full_path, var_def);

    // 7. Create Let statement
    Some(ctx.arena.alloc(ResolvedStmt {
        span,
        kind: ResolvedStmtKind::Let {
            dot_prefix: name_path[0].0.starts_with('.'),
            name_path: name_path.to_vec(),
            var_def,
            init: None,  // Transform variables don't have init expr
            span,
        },
    }))
}
```

#### 2.5 Build Transform Expression

```rust
fn build_transform_expression<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    with_ctx: &'arena WithContext<'src, 'arena>,
    transform_method: &'arena TransformMethod<'src, 'arena>,
    shadow_var: &'arena VarDefinition<'src, 'arena>,
    span: Span,
) -> Option<&'arena ResolvedExpr<'src, 'arena>> {
    // Build reference to shadow variable: &shadow
    let shadow_ref = ctx.arena.alloc(ResolvedExpr {
        span,
        kind: ResolvedExprKind::UnaryOp {
            op: UnaryOperator::Ref,
            operand: ctx.arena.alloc(ResolvedExpr {
                span,
                kind: ResolvedExprKind::Var {
                    name: shadow_var.name,
                    definition: shadow_var,
                },
                ty: shadow_var.var_type.unwrap(),
            }),
        },
        ty: ctx.arena.alloc(ResolvedType::Reference {
            inner: shadow_var.var_type.unwrap(),
        }),
    });

    // Build method call: context.__transform__(&shadow)
    let method_call = ctx.arena.alloc(ResolvedExpr {
        span,
        kind: ResolvedExprKind::MethodCall {
            receiver: with_ctx.context_expr,
            method: transform_method.function.name,
            method_def: transform_method.function,
            args: vec![shadow_ref],
        },
        ty: transform_method.output_type,
    });

    // Return the method call expression
    // (The equality constraint is implicit in the definition)
    Some(method_call)
}
```

**Testing**:
- Unit tests for shadow name generation
- Integration tests for transformed variable creation
- Verify HIR contains shadow variables and transform expressions

### Phase 3: Simplify Solver to Use HIR Transform Info (2 days)

**Files to modify**:
- `src/solver/impls/stmt.rs`
- `src/solver/impls/expr.rs`
- `src/solver/context.rs`

**Tasks**:

#### 3.1 Simplify Variable Solving

Replace complex transform application logic with simple HIR traversal:

```rust
// OLD CODE (remove):
fn apply_transform_to_variable(...) {
    // Complex logic: detect transform, create shadow, inline, etc.
}

// NEW CODE:
fn solve_let_statement<'src, 'arena>(
    &mut self,
    ctx: &mut SolverContext<'src, 'arena>,
    var_def: &'arena VarDefinition<'src, 'arena>,
    init: Option<&'arena ResolvedExpr<'src, 'arena>>,
) -> Result<(), SolverError> {
    match &var_def.definition_kind {
        VarDefinitionKind::Uninitialized => {
            // Free variable - just register in Z3
            self.register_free_variable(ctx, var_def)?;
        }

        VarDefinitionKind::Initialized { init } => {
            // Constraint: var == init
            let var_path = self.get_var_path(var_def);
            let init_z3 = init.solve(ctx)?;
            self.add_equality_constraint(ctx, &var_path, &init_z3)?;
        }

        VarDefinitionKind::Transformed {
            shadow_var,
            transform_expr,
            ..
        } => {
            // First, solve the shadow variable (it's free)
            self.solve_let_statement(ctx, shadow_var, None)?;

            // Then, add constraint: var == transform_expr
            let var_path = self.get_var_path(var_def);
            let transform_z3 = transform_expr.solve(ctx)?;
            self.add_equality_constraint(ctx, &var_path, &transform_z3)?;
        }
    }

    Ok(())
}
```

#### 3.2 Remove Transform Application Code

Delete or deprecate:
- `apply_transform_to_variable()` function
- `select_transform_method()` function
- `create_shadow_variable()` function
- `inline_transform_method()` function (if only used for transforms)

Keep only if used for user-written function calls.

#### 3.3 Update Solution Filtering

Shadow variables should still be filtered from output:

```rust
// In solution_formatter.rs (already exists)
if root_name.starts_with("__shadow_") {
    continue;  // Don't show shadow variables to user
}
```

**Testing**:
- Ensure all existing solver tests pass
- Verify same solutions are produced
- Check that shadow variables are hidden from output

### Phase 4: Update Documentation and Examples (0.5 days)

**Files to modify**:
- `CLAUDE.md`
- `docs/SOLVER_ARCHITECTURE.md`
- `docs/HIR_TRANSFORM_REPRESENTATION.md` (this document)

**Tasks**:
1. Update architecture diagrams
2. Update code examples
3. Mark implementation status as "Complete"
4. Add migration notes for external tools

### Phase 5: Comprehensive Testing (1 day)

**Test categories**:

#### 5.1 Unit Tests
- Shadow variable name generation
- Transform detection logic
- Transform expression building
- VarDefinitionKind pattern matching

#### 5.2 Integration Tests
- Simple transform: `with sketch { let .p: Point2D; }`
- Nested transforms (if supported)
- Multiple transform contexts
- Container + transform combination

#### 5.3 End-to-End Tests
- Full examples from `examples/` directory
- Verify solutions match expected values
- Check error messages for transform-related failures

#### 5.4 Regression Tests
- Ensure all existing tests still pass
- Verify no behavior changes from user perspective

**Test files**:
- `tests/hir_transform_tests.rs` (new)
- `tests/semantic_analyzer_tests.rs` (extend)
- `tests/solver_tests.rs` (verify no regressions)

---

## Migration Strategy

### Backward Compatibility

**Breaking Changes**:
- `VarDefinition` structure changes (field renamed/replaced)
- HIR structure changes (affects tools that parse HIR)

**Non-Breaking Changes**:
- Solver API remains same
- User-facing CAD language unchanged
- Solution output format unchanged

### Migration Path for Tools

External tools that read HIR will need updates:

#### Before (Old HIR):
```rust
match var_def.init {
    Some(init_expr) => { /* handle initialized */ }
    None => { /* handle uninitialized */ }
}
```

#### After (New HIR):
```rust
match &var_def.definition_kind {
    VarDefinitionKind::Initialized { init } => { /* handle initialized */ }
    VarDefinitionKind::Uninitialized => { /* handle uninitialized */ }
    VarDefinitionKind::Transformed { shadow_var, transform_expr, .. } => {
        /* handle transformed */
    }
}
```

### Phased Rollout

1. **Phase 1**: Internal refactoring (no external API changes)
2. **Phase 2**: Update HIR structure with compatibility shims
3. **Phase 3**: Deprecate old fields/methods
4. **Phase 4**: Remove deprecated code after migration period

---

## Testing Strategy

### Test Coverage Goals

- **Unit tests**: 90%+ coverage of new code
- **Integration tests**: All transform scenarios
- **Regression tests**: 100% of existing tests pass

### Test Scenarios

#### Basic Transform
```cad
struct Point2D { x: f64, y: f64 }
struct Point3D { x: f64, y: f64, z: f64 }
struct Sketch {
    container entities,
    origin: Point3D,
    fn __transform__(p: &Point3D) -> Point2D {
        return Point2D { x: p.x - self.origin.x, y: p.y - self.origin.y };
    }
}

let s: Sketch;
s.origin.x == 0.0;
s.origin.y == 0.0;
s.origin.z == 0.0;

with s {
    let .p: Point2D;
    .p.x == 10.0;
    .p.y == 20.0;
}
```

**Expected HIR**:
- Variable `s.entities.p` with `VarDefinitionKind::Transformed`
- Shadow variable `__shadow_0` with type `Point3D`
- Transform expression: `s.entities.p == s.__transform__(&__shadow_0)`

**Expected Solution**:
```
s.entities.p.x = 10
s.entities.p.y = 20
s.origin.x = 0
s.origin.y = 0
s.origin.z = 0
```

#### Error Cases

**Invalid Transform Type**:
```cad
with sketch {
    let .invalid: InvalidType;  // No transform to InvalidType
}
```
**Expected**: Semantic analysis error (not solver error!)

**Ambiguous Transform**:
```cad
struct Sketch {
    fn __transform__(p: &Point3D) -> Point2D { ... }
    fn __transform__(p: &OtherType) -> Point2D { ... }  // Same output type!
}
```
**Expected**: Semantic analysis error for ambiguous transform

### Performance Testing

- Measure HIR construction time increase (should be negligible)
- Verify solver time unchanged or improved
- Memory usage comparison (shadow variables in HIR vs solver)

---

## Benefits and Trade-offs

### Benefits

#### 1. Architectural Clarity
- **Clear separation**: Semantic analysis does semantics, solver does code gen
- **Single Responsibility**: Each phase has one job
- **Easier to reason about**: HIR fully represents program semantics

#### 2. Earlier Error Detection
- Transform errors caught during semantic analysis
- Better error messages with source context
- No need to wait for solver to fail

#### 3. Improved Testability
- Can test transform logic without running solver
- HIR can be inspected and validated independently
- Unit tests for semantic analysis more meaningful

#### 4. Better Dataflow Analysis
- Dependencies explicit: `.p` depends on `__shadow_0`
- Enables optimization passes
- Foundation for advanced analyses

#### 5. Simpler Solver
- Solver becomes pure translator (HIR → Z3)
- Less complex logic, fewer bugs
- Easier to maintain and extend

#### 6. Support for Alternative Backends
- Other solvers (CVC5, Yices) can use same HIR
- Code generation to C++, Rust, etc.
- HIR is backend-agnostic

### Trade-offs

#### 1. Slightly Larger HIR
- Shadow variables now in HIR (previously solver-only)
- Transform expressions stored in HIR
- **Impact**: Minimal (shadow vars are small, transforms are reused)

#### 2. More Complex Semantic Analyzer
- Additional logic for transform detection and application
- More code in `pass2.rs`
- **Mitigation**: Well-structured, testable code

#### 3. Migration Effort
- Need to update all VarDefinition construction sites
- External tools need updates
- **Mitigation**: Clear migration guide, compatibility period

#### 4. Slightly Longer Compilation
- Transform application moved earlier in pipeline
- **Impact**: Negligible (same work, just earlier)

### Overall Assessment

**Strongly recommended**. The architectural benefits far outweigh the costs. This aligns the codebase with compiler best practices and sets a solid foundation for future features.

---

## Future Extensions

### 1. Multiple Transform Chaining

Once transforms are in HIR, we can support chained transforms:

```cad
with sketch {
    with rotated_view {
        let .p: Point2D;  // Point3D → Rotated3D → Point2D
    }
}
```

**HIR representation**:
```rust
VarDefinitionKind::Transformed {
    shadow_var: __shadow_0,  // Point3D
    transform_expr: rotated_view.__transform__(
        sketch.__transform__(&__shadow_0)
    ),
    ...
}
```

### 2. Conditional Transforms

Transforms that depend on runtime conditions:

```cad
with sketch {
    if perspective_mode {
        let .p: Point2D;  // Apply perspective transform
    } else {
        let .p: Point2D;  // Apply orthographic transform
    }
}
```

**HIR representation**:
```rust
VarDefinitionKind::ConditionallyTransformed {
    condition: condition_expr,
    then_transform: perspective_transform,
    else_transform: orthographic_transform,
    ...
}
```

### 3. Transform Optimization

With transforms in HIR, we can optimize:

**Constant folding**:
```cad
let origin = Point3D { x: 0, y: 0, z: 0 };
// Transform: Point2D { x: p.x - origin.x, y: p.y - origin.y }
// Optimized: Point2D { x: p.x, y: p.y }
```

**Transform fusion**:
```cad
// Two transforms: translate then rotate
// Optimized: single combined transform matrix
```

### 4. Inverse Transforms

Allow specifying inverse transforms for bi-directional constraints:

```cad
struct Sketch {
    fn __transform__(p3d: &Point3D) -> Point2D { ... }
    fn __inverse_transform__(p2d: &Point2D) -> Point3D { ... }
}

with sketch {
    let external_3d: Point3D;
    let .p: Point2D;

    // Solver can choose to solve in either direction!
    .p.x == 10.0;  // Use __inverse_transform__ to find external_3d
}
```

### 5. Transform Verification

Add verification that transforms are actually transformations (invertible, etc.):

```rust
fn verify_transform_properties(
    transform: &TransformMethod,
    inverse: Option<&TransformMethod>,
) -> Result<(), TransformError> {
    // Check dimensionality
    // Verify inverse relationship
    // Ensure bijectivity (if claimed)
}
```

---

## Appendix: Code Examples

### Example 1: Simple Transformed Variable

#### Input CAD Code
```cad
struct Point2D { x: f64, y: f64 }
struct Point3D { x: f64, y: f64, z: f64 }

struct Sketch {
    container entities,
    origin: Point3D,

    fn __transform__(p: &Point3D) -> Point2D {
        return Point2D {
            x: p.x - self.origin.x,
            y: p.y - self.origin.y,
        };
    }
}

let s: Sketch;
s.origin.x == 0.0;
s.origin.y == 0.0;
s.origin.z == 0.0;

with s {
    let .p: Point2D;
    .p.x == 5.0;
}
```

#### Generated HIR (Pseudocode)

```rust
// Variable: s
VarDefinition {
    name: "s",
    var_type: Sketch,
    definition_kind: Uninitialized,
}

// Constraint: s.origin.x == 0.0
Constraint(Eq(
    FieldAccess(Var(s), "origin.x"),
    Literal(0.0)
))

// Constraint: s.origin.y == 0.0
Constraint(Eq(
    FieldAccess(Var(s), "origin.y"),
    Literal(0.0)
))

// Constraint: s.origin.z == 0.0
Constraint(Eq(
    FieldAccess(Var(s), "origin.z"),
    Literal(0.0)
))

// Shadow variable (auto-generated)
VarDefinition {
    name: "__shadow_0",
    var_type: Point3D,
    definition_kind: Uninitialized,
}

// Variable: s.entities.p (transformed)
VarDefinition {
    name: "p",
    var_type: Point2D,
    definition_kind: Transformed {
        shadow_var: &__shadow_0_def,
        transform_expr: MethodCall(
            receiver: Var(s),
            method: "__transform__",
            args: [Ref(Var(__shadow_0))]
        ),
        transform_method: &__transform___method_def,
        with_context: &s_with_context,
    },
}

// User constraint: .p.x == 5.0
Constraint(Eq(
    FieldAccess(Var(p), "x"),
    Literal(5.0)
))
```

#### Solver Behavior

```rust
// Register free variables
register_var("s.origin.x", f64);
register_var("s.origin.y", f64);
register_var("s.origin.z", f64);
register_var("__shadow_0.x", f64);
register_var("__shadow_0.y", f64);
register_var("__shadow_0.z", f64);
register_var("s.entities.p.x", f64);
register_var("s.entities.p.y", f64);

// Add constraints from HIR
add_constraint("s.origin.x == 0.0");
add_constraint("s.origin.y == 0.0");
add_constraint("s.origin.z == 0.0");

// Add transform constraint (from VarDefinition.transform_expr)
// s.entities.p == s.__transform__(&__shadow_0)
// Inlined:
add_constraint("s.entities.p.x == __shadow_0.x - s.origin.x");
add_constraint("s.entities.p.y == __shadow_0.y - s.origin.y");

// Add user constraint
add_constraint("s.entities.p.x == 5.0");

// Solve with Z3
let solution = z3_solve();

// Filter output (hide shadow variables)
output:
  s.entities.p.x = 5.0
  s.origin.x = 0.0
  s.origin.y = 0.0
  s.origin.z = 0.0
  // __shadow_0.* hidden
```

### Example 2: Error Detection (Type Mismatch)

#### Input CAD Code
```cad
struct Sketch {
    fn __transform__(p: &Point3D) -> Point2D { ... }
}

with sketch {
    let .invalid: SomeOtherType;  // No transform produces SomeOtherType!
}
```

#### Semantic Analysis Error

**When**: During `resolve_let_statement()` in pass 2

**Check**:
```rust
if let Some(transform_method) = should_apply_transform(ctx, &var_type) {
    // Found matching transform
} else if ctx.scope_stack.current_with_context().is_some() {
    // In with-context but no matching transform
    return Err(SemanticError::NoMatchingTransform {
        var_type: var_type.to_string(),
        available_transforms: list_available_transforms(ctx),
        span,
    });
}
```

**Error Message**:
```
Error: No transform available for type 'SomeOtherType'
  ┌─ example.cad:5:9
  │
5 │     let .invalid: SomeOtherType;
  │         ^^^^^^^^ cannot create variable of this type in transform context
  │
  = note: Available transforms in this context:
    - __transform__(Point3D) -> Point2D
```

---

## Glossary

- **HIR**: High-Level Intermediate Representation - typed, name-resolved representation of the program
- **Shadow Variable**: Auto-generated internal variable representing pre-transform coordinate space (e.g., `__shadow_0`)
- **Transform Method**: Special method named `__transform__` that converts between coordinate spaces
- **Transform Expression**: HIR expression representing the transform function call and constraint
- **VarDefinitionKind**: Enum describing how a variable is defined (uninitialized, initialized, or transformed)
- **With-Context**: Context created by `with` statement, providing transform methods and container fields
- **Solver**: Backend phase that translates HIR to Z3 constraints and finds solutions

---

## References

- **Language Specification**: `docs/TEXTCAD_LANGUAGE_SPEC.md`
- **Solver Architecture**: `docs/SOLVER_ARCHITECTURE.md`
- **Rune Blocks Implementation**: `docs/RUNE_BLOCKS_IMPLEMENTATION.md`
- **Partial Solve Design**: `docs/PARTIAL_SOLVE_DESIGN.md`

---

## Status Tracking

| Phase | Status | Estimated Days | Actual Days | Notes |
|-------|--------|----------------|-------------|-------|
| Phase 1: HIR Data Structures | Not Started | 1-2 | - | |
| Phase 2: Shadow Variable Generation | Not Started | 2 | - | |
| Phase 3: Simplify Solver | Not Started | 2 | - | |
| Phase 4: Documentation | Not Started | 0.5 | - | |
| Phase 5: Testing | Not Started | 1 | - | |
| **Total** | **Not Started** | **6.5-7.5** | **-** | |

---

## Revision History

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2026-01-24 | 1.0 | Claude Code | Initial implementation plan |

---

## Approval Status

- [ ] Technical Review
- [ ] Architecture Review
- [ ] Implementation Approved
- [ ] Testing Plan Approved

---

*This document is part of the CAD-DSL project documentation. For questions or suggestions, please file an issue on GitHub.*
