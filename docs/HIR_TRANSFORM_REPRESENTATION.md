# HIR Transform Representation - Implementation Plan

## Executive Summary

This document describes the architectural improvement to move transform-related semantics from the solver phase into the High-Level Intermediate Representation (HIR). Transform application (container variable creation, transform inlining) now happens during semantic analysis, properly separating concerns. Transforms are represented directly in the HIR with structural variable identifiers.

**Transform semantics work in two directions**:
1. **Internal declarations** (dot-prefix): `with sketch { let .p: Point2D; }` creates a container variable (`sketch.entities.p: Point3D`) and a temporary view (`.p: Point2D`)
2. **External variable access**: `let p: Point3D; with sketch { p.x == 10.0; }` automatically transforms accesses to `p` inside the with-block, wrapping them with transform calls

**Status**: ✅ Complete (Implemented and Tested)

**Actual Effort**: ~4-5 days (completed phases 1-5)

**Priority**: High (Architectural improvement - COMPLETED)

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
5. **Missing Semantic Information**: HIR doesn't show that variable `x` is actually defined as `transform(container_var)`
6. **Impediment to Other Analyses**: Dataflow analysis, optimizations, and alternative backends cannot see transform relationships

### Example Problems

Transforms work in **two directions**: variables declared inside with-blocks, and variables declared outside that are accessed inside.

#### Case 1: Internal Declaration (Dot-Prefix Variables)

Given this code:
```cad
with sketch {
    let .p: Point2D;
    .p.x == 10.0;
}
```

The dot-prefix syntax `.p` in a transform context should create **two variables** with a shadowing relationship:

**What HIR should represent**:
1. **Container variable**: `sketch.entities.p: Point3D` — the real, persistent entity (accessible outside the with-block)
2. **Temporary view**: `.p: Point2D` — defined as `sketch.__transform__(sketch.entities.p)`, only visible inside the with-block

**Scoping semantics**:
- **Outside the with-block**: Only `sketch.entities.p` exists, with type `Point3D`
- **Inside the with-block**: The temporary view `.p` (type `Point2D`) shadows `sketch.entities.p`, making the container variable inaccessible by that name

**Current problem**: HIR only stores metadata about the transform method but doesn't represent the variable definition relationship or the shadowing semantics.

**What's missing**:
- The container variable `sketch.entities.p` is not created during semantic analysis
- The view variable's definition (as a transform of the container variable) is not stored in HIR
- The shadowing relationship is not explicitly represented

#### Case 2: External Variable Access (Automatic Transformation)

Given this code:
```cad
let p: Point3D;
p.x == 100.0;
p.y == 200.0;
p.z == 0.0;

with sketch {
    // p is automatically transformed to Point2D here!
    p.x == 10.0;  // This refers to sketch.__transform__(&p).x
}
```

When an external variable with a transformable type is accessed inside a with-block, it should be **automatically transformed**.

**What HIR should represent**:
1. **External variable**: `p: Point3D` — declared outside, remains Point3D
2. **Automatic view**: Inside the with-block, all accesses to `p` should reference a transformed view `sketch.__transform__(&p)`
3. **Type changes**: Inside the block, `p` appears to have type `Point2D`, not `Point3D`

**This also applies to nested fields**:
```cad
struct Line {
    start: Point3D,
    end: Point3D,
}

let line: Line;
with sketch {
    // line.start is automatically transformed!
    line.start.x == 5.0;  // Refers to sketch.__transform__(&line.start).x
}
```

**Current problem**: The HIR doesn't represent that variable accesses inside with-blocks should be transformed.

**What's missing**:
- No mechanism to transform external variable accesses
- No way to represent that `p` inside the with-block refers to a transformed view
- No handling of nested field accesses like `line.start.x` being transformed

---

## Current Architecture

### Where Transform Logic Lives Now

| Phase | Component | Responsibility |
|-------|-----------|----------------|
| **Semantic Analysis** | `src/semantic_analyzer/pass2.rs` | Collects transform metadata (which methods exist, their types) |
| **HIR** | `src/hir/context.rs` | Stores `WithContext` with `Vec<TransformMethod>` |
| **Solver** | `src/solver/impls/stmt.rs` | **Applies transforms**: creates container variables, inlines methods, generates constraints |

### Transform Application Pipeline (Current)

Located in `src/solver/impls/stmt.rs::apply_transform_to_variable()`:

1. **Check context**: Is variable in transform context?
2. **Select transform**: Find matching `__transform__` method
3. **Create container variable**: Generate qualified name (e.g., `sketch.entities.p`)
4. **Create reference**: Make HIR expression referencing container variable
5. **Inline transform**: Substitute parameters in transform method body
6. **Add constraint**: `view_var == transform_result`

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
| **Container Variables** | Created during solving | Created during semantic analysis |
| **Transform Expressions** | Inlined during solving | Constructed in HIR |
| **Variable Definition** | HIR: "var exists" | HIR: "view = transform(container)" or "expr wrapped with transform" |
| **Solver Role** | Semantic + Code Gen | Pure Code Gen (HIR → Z3) |
| **Error Detection** | During solving | During semantic analysis |

---

## Detailed Design

### `VarDefinitionKind` Enum - Two-Variable Design

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

    /// Temporary transformed view: `with sketch { let .p: Point2D; }`
    /// Creates TWO variables:
    /// 1. Container variable: `sketch.entities.p: Point3D` (persistent, accessible outside)
    /// 2. View variable: `.p: Point2D` (temporary, only in with-block, shadows container)
    ///
    /// The view's value is the transform of the container variable.
    /// Supports nested transforms via transform_chain.
    TransformedView {
        /// The persistent container variable (e.g., `sketch.entities.p: Point3D`)
        /// This is the real entity that exists in the container's namespace.
        /// Outside the with-block, this variable is accessible as Point3D.
        /// Inside the with-block, it's shadowed by the view.
        container_var: &'arena VarDefinition<'src, 'arena>,

        /// The complete transform chain from outermost to innermost
        /// For single transform: vec has one element
        /// For nested transforms: `with outer { with inner { ... } }`
        /// Chain is [outer_transform, inner_transform] applied in order
        transform_chain: Vec<TransformStep<'src, 'arena>>,

        /// The transform expression defining the view
        /// Represents: view_var == innermost_transform(...(outermost_transform(&container_var)))
        /// Example: `.p == inner.__transform__(outer.__transform__(&sketch.entities.p))`
        transform_expr: &'arena ResolvedExpr<'src, 'arena>,
    },
}

/// A single step in a transform chain
#[derive(Debug, Clone, PartialEq)]
pub struct TransformStep<'src, 'arena> {
    /// The transform method being applied in this step
    pub transform_method: &'arena FunctionDefinition<'src, 'arena>,

    /// The with-context that provides this transform
    pub with_context: &'arena WithContext<'src, 'arena>,

    /// Input type for this transform step
    pub input_type: &'arena ResolvedType<'src, 'arena>,

    /// Output type for this transform step
    pub output_type: &'arena ResolvedType<'src, 'arena>,
}
}
```

### Why This Design?

**Type Safety**: The three definition kinds are mutually exclusive - a variable is either uninitialized, initialized, or transformed view. The type system enforces handling all cases.

**Semantic Clarity**: Reading the HIR immediately reveals:
- Which variables are persistent container entities
- Which variables are temporary transformed views
- The shadowing relationships between them

**True Shadowing**: The view variable actually shadows the container variable, matching standard scoping semantics.

**Extensibility**: Future definition kinds can be added easily (e.g., `ConstraintDefined` for implicit definitions, `LoopInductionVariable` for loop counters).

### Container Variable Representation

Container variables are regular `VarDefinition` instances in the container's namespace:

```rust
// For: `with sketch { let .p: Point2D; }`
// Creates container variable:

VarDefinition {
    name: "sketch.entities.p",  // Full qualified name in container
    var_type: Some(Point3D),  // Container type (input to transform)
    definition_kind: VarDefinitionKind::Uninitialized,  // Free variable for solver
    scope_level: with_scope_level,
    ...
}
```

**Key properties**:
- Container variables are **persistent entities** stored in the container's namespace
- They are **free variables** (uninitialized) that the solver will assign values to
- They represent the **real coordinate space** of the container (e.g., Point3D in world space)
- They are **accessible outside the with-block** via their qualified name (`sketch.entities.p`)
- Inside the with-block, they are **shadowed** by the view variable

### View Variable Representation

View variables are temporary scoped variables with TransformedView kind:

```rust
// For: `with sketch { let .p: Point2D; }`
// Creates view variable:

VarDefinition {
    name: "p",  // Short name (without dot prefix)
    var_type: Some(Point2D),  // View type (output of transform)
    definition_kind: VarDefinitionKind::TransformedView {
        container_var: &container_var_def,  // Points to sketch.entities.p
        transform_chain: vec![...],
        transform_expr: &transform_expr,  // p == sketch.__transform__(&sketch.entities.p)
    },
    scope_level: with_scope_level,
    ...
}
```

**Key properties**:
- View variables are **temporary** and only exist inside the with-block
- They are **derived variables** defined as transforms of container variables
- They **shadow** the container variable by name inside the with-block
- They represent the **transformed coordinate space** (e.g., Point2D in sketch's local space)
- They are **not accessible outside the with-block**

### Transform Expression Structure

The `transform_expr` field contains a fully-resolved HIR expression representing the transform call:

```rust
// Example: .p == sketch.__transform__(&sketch.entities.p)
// View variable (.p: Point2D) defined as transform of container variable (sketch.entities.p: Point3D)

ResolvedExpr {
    kind: ResolvedExprKind::BinaryOp {
        op: BinaryOperator::Eq,
        left: ResolvedExpr {
            kind: Var { name: "p", definition: &view_var_def },  // View variable
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
                                    name: "sketch.entities.p",
                                    definition: &container_var_def  // Container variable
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

**Scoping note**: The container variable reference (`sketch.entities.p`) is the persistent entity that exists in the container's namespace. The view variable (`p`) is a temporary that only exists in the with-block scope.

### External Variable Transformation

When a variable declared **outside** a with-block is accessed **inside** the block, and its type can be transformed, the access should automatically reference a transformed view.

#### Approach: Transform at Variable Access

Unlike dot-prefix variables (which create container+view pairs at declaration), external variables are transformed **when accessed**:

```rust
// External variable transformation happens during expression resolution

// When resolving: p.x == 10.0 inside with-block

// 1. Resolve variable reference `p`
//    - Found: p: Point3D (declared outside)
//    - Current context: inside `with sketch` block
//    - Check: Can Point3D be transformed? Yes, sketch has __transform__(Point3D) -> Point2D

// 2. Create implicit transformed view expression
//    - Instead of: Var { name: "p", definition: &p_def }
//    - Generate: MethodCall { receiver: sketch, method: "__transform__", args: [&p] }
//    - Type changes from Point3D to Point2D

// 3. Continue resolving field access
//    - Access .x on the Point2D result (not the Point3D variable)
```

#### HIR Representation for External Variable Access

**Key insight**: External variables are **not** given a new `VarDefinition`. Instead, variable access expressions are **wrapped** with transform calls:

```rust
// Source code:
// let p: Point3D;
// with sketch {
//     p.x == 10.0;
// }

// HIR for `p.x` inside with-block:
ResolvedExpr {
    kind: FieldAccess {
        base: ResolvedExpr {
            // Base is NOT Var{p}, it's the transformed version:
            kind: MethodCall {
                receiver: sketch_var,
                method: "__transform__",
                args: [
                    ResolvedExpr {
                        kind: UnaryOp {
                            op: Ref,
                            operand: ResolvedExpr {
                                kind: Var { name: "p", definition: &p_def },
                                ty: Point3D,
                            }
                        },
                        ty: &Point3D,
                    }
                ],
            },
            ty: Point2D,  // Transformed type!
        },
        field: "x",
    },
    ty: f64,
}
```

#### Nested Field Transformation

This automatically handles nested structs:

```rust
// Source: line.start.x inside with-block
// Where line: Line { start: Point3D, end: Point3D }

// HIR:
FieldAccess {
    base: MethodCall {
        receiver: sketch,
        method: "__transform__",
        args: [
            &FieldAccess {
                base: Var { name: "line", ... },
                field: "start",  // Type: Point3D
            }
        ]
    },  // Result type: Point2D
    field: "x",  // Access x on Point2D
}
```

The key is: **Any expression** whose type is transformable gets wrapped when accessed in a transform context.

#### When to Apply External Variable Transform

During expression resolution in `resolve_expr()`, check:

1. Are we inside a with-block context?
2. Is the expression's type transformable by the current with-context?
3. If yes, wrap the expression with a transform call

This applies to:
- Variable references (`p`)
- Field accesses (`line.start`)
- Array elements (`points[0]`)
- Any expression returning a transformable type

### Transform Kind Selection

CAD-DSL supports two kinds of transform methods to handle different use cases:

#### Standard Transform (`__transform__`)

Used for:
- External variables referenced from outside a with-block
- Regular struct field access
- Coordinates passed into the transform context from external code

#### Container Transform (`__transform_container__`)

Used for:
- Variables declared with dot-prefix syntax (`.varname`) inside with-blocks
- Entities that exist within the container's namespace
- Allows different transformation semantics for "internal" vs. "external" coordinates

#### Selection Logic

When a variable is declared in a transform context:

1. **Determine variable kind**:
   - If `name_path` starts with `.` → Container variable
   - Otherwise → External variable

2. **Select transform**:
   - **Container variable**:
     - First, look for `__transform_container__` with matching output type
     - If not found, fall back to `__transform__` with matching output type
   - **External variable**:
     - Only use `__transform__` with matching output type
     - Never use `__transform_container__` for external variables

3. **Error if ambiguous**:
   - Multiple transforms of **same kind** with **same output type** → Error
   - Multiple transforms of **different kinds** with same output type → OK

#### Example: Different Behavior for Internal vs. External

```cad
struct Sketch {
    container entities,
    scale: f64,

    // External coordinates are scaled
    fn __transform__(p: &Point3D) -> Point2D {
        return Point2D { x: p.x * self.scale, y: p.y * self.scale };
    }

    // Internal entities are not scaled (already in sketch space)
    fn __transform_container__(p: &Point3D) -> Point2D {
        return Point2D { x: p.x, y: p.y };  // No scaling
    }
}

let s: Sketch;
s.scale == 2.0;

with s {
    let .internal: Point2D;  // Uses __transform_container__ (no scaling)
    internal.x == 10.0;      // Internal coordinate

    // External reference would use __transform__ (with scaling)
}
```

This allows sketch-internal entities to use local coordinates while external references are automatically scaled.

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
   - `VarDefinition::is_transformed_view() -> bool`
   - `VarDefinition::get_container_var() -> Option<&VarDefinition>`
   - `VarDefinition::get_init_expr() -> Option<&ResolvedExpr>`

**Testing**:
- Ensure all existing tests still pass with refactored structure
- Add unit tests for new enum variants

### Phase 2: Implement Container+View Variable Generation and External Variable Transformation in Semantic Analyzer (3-3.5 days)

**Files to modify**:
- `src/semantic_analyzer/pass2.rs`
- `src/semantic_analyzer/context.rs` (for error types)

**Tasks**:

#### 2.1 Add Ambiguity Detection to Transform Collection

**CRITICAL**: The existing `collect_transform_methods()` does not check for ambiguous transforms. We must add this check during semantic analysis, not wait until solving.

```rust
// In src/semantic_analyzer/pass2.rs

fn collect_transform_methods<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    definition: &'arena StructDefinition<'src, 'arena>,
) -> Vec<TransformMethod<'src, 'arena>> {
    let mut transforms = Vec::new();

    // Track output types by kind to detect ambiguity
    let mut standard_outputs = Vec::new();
    let mut container_outputs = Vec::new();

    for method in &definition.methods {
        let kind = match method.name {
            "__transform__" => TransformMethodKind::Standard,
            "__transform_container__" => TransformMethodKind::Container,
            _ => continue,
        };

        if method.params.is_empty() {
            ctx.report_error(SemanticError::InvalidTransformSignature {
                method_name: method.name,
                reason: "Transform methods must have at least one parameter",
                span: method.span,
            });
            continue;
        }

        let input_type = ctx.arena.alloc(method.params[0].param_type);
        let output_type = ctx.arena.alloc(method.return_type);

        // Check for ambiguous output types within the same kind
        let outputs = match kind {
            TransformMethodKind::Standard => &mut standard_outputs,
            TransformMethodKind::Container => &mut container_outputs,
        };

        // Check if another transform of this kind has the same output type
        if let Some((existing_method, existing_output)) = outputs.iter()
            .find(|(_, out_ty)| types_match(out_ty, output_type))
        {
            ctx.report_error(SemanticError::AmbiguousTransform {
                method_name: method.name,
                existing_method: *existing_method,
                output_type: format!("{:?}", output_type),
                kind_name: match kind {
                    TransformMethodKind::Standard => "__transform__",
                    TransformMethodKind::Container => "__transform_container__",
                },
                span: method.span,
            });
            continue;
        }

        outputs.push((method.name, output_type));
        transforms.push(TransformMethod::new(method, input_type, output_type, kind));
    }

    transforms
}
```

#### 2.2 Container Variable Naming

No counter needed! Container variables use the natural qualified name from the container namespace:

```rust
// For: `with sketch { let .p: Point2D; }`
// Container variable name: "sketch.entities.p"
// View variable name: "p"

// The container namespace (e.g., "entities") comes from the struct's container field
// The variable name comes from the dot-prefix declaration (.p becomes "p" in container)
```

Container variables are named using the existing `resolve_variable_path()` logic, which already handles dot-prefix variables.

#### 2.3 Add ScopeStack Method for Transform Chains

First, add a method to `ScopeStack` to get all active with-contexts:

```rust
// In src/hir/scope.rs

impl<'src, 'arena> ScopeStack<'src, 'arena> {
    /// Returns all active with-contexts from outermost to innermost
    ///
    /// This is needed for nested transform chains. When transforms are nested,
    /// they should be applied in order from outermost to innermost.
    ///
    /// # Example
    ///
    /// ```cad
    /// with outer {          // Context 1
    ///     with inner {      // Context 2
    ///         let .p: T;    // Needs both transforms: outer then inner
    ///     }
    /// }
    /// ```
    pub fn all_with_contexts(&self) -> Vec<&'arena WithContext<'src, 'arena>> {
        self.scopes
            .iter()
            .filter_map(|scope| scope.with_context)
            .collect()
    }
}
```

Then add transform chain detection logic in semantic analyzer:

```rust
// In src/semantic_analyzer/pass2.rs

/// Determine if a variable is a container variable based on its name path
/// Container variables are declared with dot-prefix syntax inside with-blocks
fn is_container_variable(name_path: &[(&str, Span)]) -> bool {
    name_path.first().map(|(name, _)| name.starts_with('.')).unwrap_or(false)
}

/// Check if a variable type requires transform in current with-context(s)
/// Returns the complete transform chain if transforms are needed
///
/// Handles both Standard and Container transform kinds:
/// - Container variables (dot-prefix): prefer __transform_container__, fallback to __transform__
/// - External variables: only use __transform__
fn should_apply_transform<'src, 'arena>(
    ctx: &AnalyzerContext<'src, 'arena>,
    var_type: &ResolvedType<'src, 'arena>,
    name_path: &[(&str, Span)],
) -> Option<Vec<TransformStep<'src, 'arena>>> {
    let with_contexts = ctx.scope_stack.all_with_contexts();
    if with_contexts.is_empty() {
        return None;
    }

    let is_container_var = is_container_variable(name_path);

    // Build transform chain from outermost to innermost
    let mut transform_chain = Vec::new();
    let mut current_type = var_type;

    // Work backwards from innermost to outermost to find matching transforms
    for with_ctx in with_contexts.iter().rev() {
        // Select appropriate transform based on variable kind
        let transform = if is_container_var {
            // Container variables: prefer __transform_container__, fallback to __transform__
            with_ctx.transforms.iter()
                .filter(|tm| matches!(tm.kind, TransformMethodKind::Container))
                .find(|tm| types_match(tm.output_type, current_type))
                .or_else(|| {
                    with_ctx.transforms.iter()
                        .filter(|tm| matches!(tm.kind, TransformMethodKind::Standard))
                        .find(|tm| types_match(tm.output_type, current_type))
                })
        } else {
            // External variables: only use __transform__ (Standard)
            with_ctx.transforms.iter()
                .filter(|tm| matches!(tm.kind, TransformMethodKind::Standard))
                .find(|tm| types_match(tm.output_type, current_type))
        };

        if let Some(transform) = transform {
            transform_chain.insert(0, TransformStep {
                transform_method: transform.function,
                with_context: with_ctx,
                input_type: transform.input_type,
                output_type: transform.output_type,
            });
            current_type = transform.input_type;
        }
    }

    if transform_chain.is_empty() {
        None
    } else {
        Some(transform_chain)
    }
}
```

#### 2.5 Implement Container+View Variable Creation Without String Allocation

**Key Principle**: Use existing `&'src str` slices from source code; don't allocate concatenated strings.

##### 2.5.1 Modify Variable Declaration Logic

Extend `resolve_let_statement()` to detect and apply transform chains:

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

    // Check if this variable should be transformed (returns chain)
    // Passes name_path to determine if it's a container variable
    if let Some(transform_chain) = should_apply_transform(ctx, &var_type, name_path) {
        return resolve_transformed_variable(
            ctx,
            name_path,
            var_type,
            transform_chain,  // Now a Vec, not a single transform
            span
        );
    }

    // Normal variable (existing logic)
    // ...
}
```

##### 2.5.2 Implement Variable Creation Using Source String Slices

```rust
fn resolve_transformed_variable<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    name_path: &[(&'src str, Span)],
    view_type: &'arena ResolvedType<'src, 'arena>,  // The declared type (e.g., Point2D)
    transform_chain: Vec<TransformStep<'src, 'arena>>,
    span: Span,
) -> Option<&'arena ResolvedStmt<'src, 'arena>> {
    // STEP 1: Create container variable (the real, persistent entity)
    // Input type of first transform is the container type (e.g., Point3D)
    let container_type = transform_chain.first()
        .expect("Transform chain should not be empty")
        .input_type;

    // Get the container variable name from the existing path resolution
    // This already returns a qualified path as &'src str (no allocation needed)
    // For `.p` in `with sketch`, resolve_variable_path returns "sketch.entities.p"
    let container_name = resolve_variable_path(ctx, name_path);

    let container_var_def = ctx.arena.alloc(VarDefinition {
        name: container_name,  // Already &'src str, no allocation!
        name_span: span,
        var_type: Some(container_type),
        definition_kind: VarDefinitionKind::Uninitialized,  // Free variable for solver
        scope_level: ctx.scope_stack.current_level(),
        span,
    });

    // Register container variable in scope (in container namespace)
    ctx.scope_stack.add_variable(container_name, container_var_def);

    // STEP 2: Build transform expression
    // This defines the view as transform of container: view == transform(container)
    let transform_expr = build_chained_transform_expression(
        ctx,
        &transform_chain,
        container_var_def,
        span,
    )?;

    // STEP 3: Create view variable (temporary, shadows container in this scope)
    // Extract short name without dot-prefix from source
    let (view_name_with_dot, _) = name_path.last().unwrap();
    let view_name = view_name_with_dot.trim_start_matches('.');  // Still &'src str, no allocation!

    let view_var_def = ctx.arena.alloc(VarDefinition {
        name: view_name,  // Already &'src str, no allocation!
        name_span: span,
        var_type: Some(view_type),
        definition_kind: VarDefinitionKind::TransformedView {
            container_var: container_var_def,
            transform_chain: transform_chain.clone(),
            transform_expr,
        },
        scope_level: ctx.scope_stack.current_level(),
        span,
    });

    // Register view variable in local scope (shadows container variable by short name)
    ctx.scope_stack.add_variable(view_name, view_var_def);

    // STEP 4: Create Let statement
    // The statement represents both variables, but primarily references the view
    Some(ctx.arena.alloc(ResolvedStmt {
        span,
        kind: ResolvedStmtKind::Let {
            dot_prefix: true,  // Indicates this created container+view pair
            name_path: name_path.to_vec(),
            var_def: view_var_def,  // Primary reference is to view variable
            init: None,
            span,
        },
    }))
}
```

**Critical Notes**:
1. **No `alloc_str` calls**: All variable names use existing `&'src str` slices
2. **View names**: Come directly from source via `trim_start_matches('.')`
3. **Container names**: Returned by `resolve_variable_path()` as existing `&'src str`
4. **Success criteria**: Zero string allocations for variable names

##### 2.5.3 Verify resolve_variable_path Returns &'src str

Ensure that `resolve_variable_path()` returns `&'src str` without allocation:
- Container paths like "sketch.entities.p" must come from existing source slices
- If current implementation allocates, refactor to use source slices or component-based representation
- Document how qualified names are constructed from path components

**Success Criteria**:
- No `Box::leak` calls in semantic analyzer (outside tests)
- No `arena.alloc_str()` calls for variable names
- All variable names are `&'src str` referencing source code
- All tests pass

#### 2.6 Build Chained Transform Expression

```rust
fn build_chained_transform_expression<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    transform_chain: &[TransformStep<'src, 'arena>],
    container_var: &'arena VarDefinition<'src, 'arena>,
    span: Span,
) -> Option<&'arena ResolvedExpr<'src, 'arena>> {
    // Start with reference to container variable: &container_var
    let mut current_expr = ctx.arena.alloc(ResolvedExpr {
        span,
        kind: ResolvedExprKind::UnaryOp {
            op: UnaryOperator::Ref,
            operand: ctx.arena.alloc(ResolvedExpr {
                span,
                kind: ResolvedExprKind::Var {
                    name: container_var.name,
                    definition: container_var,
                },
                ty: container_var.var_type.unwrap(),
            }),
        },
        ty: ctx.arena.alloc(ResolvedType::Reference {
            inner: container_var.var_type.unwrap(),
        }),
    });

    // Apply each transform in order (outermost to innermost)
    for step in transform_chain {
        // Build method call: context.__transform__(current_expr)
        current_expr = ctx.arena.alloc(ResolvedExpr {
            span,
            kind: ResolvedExprKind::MethodCall {
                receiver: step.with_context.context_expr,
                method: step.transform_method.name,
                method_def: step.transform_method,
                args: vec![current_expr],
            },
            ty: step.output_type,
        });
    }

    // Return the fully chained expression
    // For single transform: context.__transform__(&container_var)
    // For nested: inner.__transform__(outer.__transform__(&container_var))
    Some(current_expr)
}
```

**Notes**:
- No string allocation occurs; all names are `&'src str` from source
- Container variable reference uses the qualified name from existing path resolution

**Example with nested transforms:**

```cad
struct Outer {
    container entities,
    fn __transform__(p: &Point3D) -> Point2D { ... }
}

struct Inner {
    container entities,
    fn __transform__(p: &Point2D) -> Point1D { ... }
}

with outer {
    with inner {
        let .p: Point1D;  // Transform chain: outer then inner
    }
}
```

**Generated HIR:**
- **Container variable**: `outer.entities.inner.entities.p: Point3D` (type Point3D - input to first transform)
- **View variable**: `p: Point1D` (type Point1D - output of last transform)
- **Transform expression**: `inner.__transform__(outer.__transform__(&outer.entities.inner.entities.p))`

**Scoping**:
- Outside both with-blocks: Can access `outer.entities.inner.entities.p` as Point3D
- Inside inner with-block: Name `p` refers to Point1D view, container is shadowed

**Testing**:
- Unit tests for container variable naming
- Integration tests for transformed variable creation
- Verify HIR contains both container and view variables with correct types
- Verify no Box::leak calls exist in semantic analyzer code (all allocations use arena)
- Verify no string allocations for variable names (grep for `alloc_str` in variable name code paths)

#### 2.7 Implement External Variable Access Transformation

This is a **critical addition** to handle variables declared outside with-blocks that are accessed inside.

**Approach**: Modify expression resolution to automatically wrap transformable expressions with transform calls when in a transform context.

```rust
// In src/semantic_analyzer/pass2.rs

/// Checks if an expression's type can be transformed in the current context
/// Returns the transform chain if applicable
fn get_transform_for_type<'src, 'arena>(
    ctx: &AnalyzerContext<'src, 'arena>,
    expr_type: &ResolvedType<'src, 'arena>,
) -> Option<Vec<TransformStep<'src, 'arena>>> {
    let with_contexts = ctx.scope_stack.all_with_contexts();
    if with_contexts.is_empty() {
        return None;
    }

    // Build transform chain for this type
    let mut transform_chain = Vec::new();
    let mut current_type = expr_type;

    for with_ctx in with_contexts.iter() {
        // Only use Standard transforms for external variables
        let transform = with_ctx.transforms.iter()
            .filter(|tm| matches!(tm.kind, TransformMethodKind::Standard))
            .find(|tm| types_match(tm.input_type, current_type))?;

        transform_chain.push(TransformStep {
            transform_method: transform.function,
            with_context: with_ctx,
            input_type: transform.input_type,
            output_type: transform.output_type,
        });
        current_type = transform.output_type;
    }

    if transform_chain.is_empty() {
        None
    } else {
        Some(transform_chain)
    }
}

/// Wraps an expression with transform calls if in transform context
/// This is called after resolving any expression
fn maybe_apply_transform<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    expr: &'arena ResolvedExpr<'src, 'arena>,
    span: Span,
) -> &'arena ResolvedExpr<'src, 'arena> {
    // Check if expression's type is transformable
    if let Some(transform_chain) = get_transform_for_type(ctx, &expr.ty) {
        // Wrap expression with transform chain
        wrap_with_transforms(ctx, expr, &transform_chain, span)
    } else {
        // No transform needed
        expr
    }
}

/// Wraps an expression with a chain of transform calls
fn wrap_with_transforms<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    expr: &'arena ResolvedExpr<'src, 'arena>,
    transform_chain: &[TransformStep<'src, 'arena>],
    span: Span,
) -> &'arena ResolvedExpr<'src, 'arena> {
    // Start with reference to the original expression
    let mut current_expr = ctx.arena.alloc(ResolvedExpr {
        span,
        kind: ResolvedExprKind::UnaryOp {
            op: UnaryOperator::Ref,
            operand: expr,
        },
        ty: ctx.arena.alloc(ResolvedType::Reference {
            inner: expr.ty,
        }),
    });

    // Apply each transform in order
    for step in transform_chain {
        current_expr = ctx.arena.alloc(ResolvedExpr {
            span,
            kind: ResolvedExprKind::MethodCall {
                receiver: step.with_context.context_expr,
                method: step.transform_method.name,
                method_def: step.transform_method,
                args: vec![current_expr],
            },
            ty: step.output_type,
        });
    }

    current_expr
}
```

#### 2.8 Update Expression Resolution to Apply Transforms

Modify `resolve_expr()` to call `maybe_apply_transform()` on certain expression kinds:

```rust
fn resolve_expr<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    expr: &Expr<'src>,
) -> Option<&'arena ResolvedExpr<'src, 'arena>> {
    let resolved = match &expr.kind {
        ExprKind::Var { name } => {
            let var_ref = resolve_variable(ctx, name)?;
            ctx.arena.alloc(ResolvedExpr {
                span: expr.span,
                kind: ResolvedExprKind::Var { name, definition: var_ref },
                ty: var_ref.var_type?,
            })
        }

        ExprKind::FieldAccess { base, field } => {
            let base_resolved = resolve_expr(ctx, base)?;
            let field_type = get_field_type(base_resolved.ty, field)?;
            ctx.arena.alloc(ResolvedExpr {
                span: expr.span,
                kind: ResolvedExprKind::FieldAccess {
                    base: base_resolved,
                    field,
                },
                ty: field_type,
            })
        }

        // ... other cases ...
    };

    // CRITICAL: Apply automatic transformation if in transform context
    // This handles external variables like `p: Point3D` accessed in `with sketch { ... }`
    // Also handles nested fields like `line.start` where start: Point3D
    match &resolved.kind {
        // Transform these expression kinds:
        ResolvedExprKind::Var { .. } |
        ResolvedExprKind::FieldAccess { .. } |
        ResolvedExprKind::Index { .. } => {
            Some(maybe_apply_transform(ctx, resolved, expr.span))
        }

        // Don't transform these:
        _ => Some(resolved)
    }
}
```

**Key points**:
1. Variable references (`p`) are automatically wrapped with transforms
2. Field accesses (`line.start`) are automatically wrapped if the field type is transformable
3. Array elements (`points[0]`) are automatically wrapped if the element type is transformable
4. Other expressions (literals, operators, function calls) are NOT transformed
5. Transforms are applied **after** type resolution, so we know the expression's type

**Testing**:
- Unit tests for `get_transform_for_type()`
- Unit tests for `wrap_with_transforms()`
- Integration test: external variable `p: Point3D` accessed in with-block
- Integration test: nested field `line.start.x` accessed in with-block
- Integration test: array element `points[0]` accessed in with-block
- Verify wrapped expressions have correct types (Point2D, not Point3D)

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
            // Free variable - register in Z3
            // This includes container variables (e.g., sketch.entities.p: Point3D)
            self.register_free_variable(ctx, var_def)?;
        }

        VarDefinitionKind::Initialized { init } => {
            // Constraint: var == init
            let var_path = self.get_var_path(var_def);
            let init_z3 = init.solve(ctx)?;
            self.add_equality_constraint(ctx, &var_path, &init_z3)?;
        }

        VarDefinitionKind::TransformedView {
            container_var,
            transform_expr,
            ..
        } => {
            // First, ensure container variable is registered (it's free)
            self.solve_let_statement(ctx, container_var, None)?;

            // Then, add constraint: view == transform_expr
            // This constrains the view based on the container variable
            let view_path = self.get_var_path(var_def);
            let transform_z3 = transform_expr.solve(ctx)?;
            self.add_equality_constraint(ctx, &view_path, &transform_z3)?;
        }
    }

    Ok(())
}
```

#### 3.2 Remove Transform Application Code

Delete or deprecate:
- `apply_transform_to_variable()` function
- `select_transform_method()` function
- `create_container_variable()` function (if it exists as separate function)
- `inline_transform_method()` function (if only used for transforms)

Keep only if used for user-written function calls.

#### 3.3 Update Solution Filtering

View variables should be filtered from output (only show container variables):

```rust
// In solution_formatter.rs

fn should_show_variable(var_def: &VarDefinition) -> bool {
    match &var_def.definition_kind {
        // Show uninitialized variables (includes container variables)
        VarDefinitionKind::Uninitialized => true,

        // Show initialized variables
        VarDefinitionKind::Initialized { .. } => true,

        // DON'T show view variables (temporary, derived from container)
        // The container variable will be shown instead
        VarDefinitionKind::TransformedView { .. } => false,
    }
}
```

**Rationale**: Container variables are the real, persistent entities (e.g., `sketch.entities.p: Point3D`). View variables are temporary transformed views that only exist inside with-blocks. Users should see the container variable values in the solution.

**Testing**:
- Ensure all existing solver tests pass
- Verify solutions show container variables (Point3D), not view variables (Point2D)
- Check that view variables are hidden from output

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
- Container variable naming
- Transform detection logic for both internal and external variables
- Transform expression building
- VarDefinitionKind pattern matching
- `get_transform_for_type()` function
- `wrap_with_transforms()` function

#### 5.2 Integration Tests - Internal Declarations (Dot-Prefix)
- Simple transform: `with sketch { let .p: Point2D; }`
- Nested transforms: `with outer { with inner { let .p: T; } }`
- Transform chains with 3+ levels of nesting
- Multiple independent transform contexts
- Container + transform combination
- Transform type compatibility (input type of inner matches output type of outer)

#### 5.3 Integration Tests - External Variable Access
- External variable in transform context: `let p: Point3D; with sketch { p.x == 10.0; }`
- Nested field access: `let line: Line; with sketch { line.start.x == 5.0; }`
- Array element access: `let points: [Point3D; 2]; with sketch { points[0].x == 3.0; }`
- External variable with nested transforms: `let p: Point3D; with outer { with inner { p.x == 1.0; } }`
- Mixed internal and external: Both dot-prefix and external variables in same with-block
- Verify HIR wraps external variable accesses with transform calls
- Verify type changes from Point3D to Point2D in HIR

#### 5.4 End-to-End Tests
- Full examples from `examples/` directory
- Verify solutions match expected values
- Check error messages for transform-related failures

#### 5.5 Regression Tests
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
    VarDefinitionKind::TransformedView { container_var, transform_expr, .. } => {
        /* handle transformed view */
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
- **Container variable**: `s.entities.p` with type `Point3D`, `VarDefinitionKind::Uninitialized`
- **View variable**: `p` with type `Point2D`, `VarDefinitionKind::TransformedView`
- Transform expression: `p == s.__transform__(&s.entities.p)`

**Scoping**:
- Outside with-block: Can access `s.entities.p` as Point3D
- Inside with-block: Name `p` refers to Point2D view (shadows `s.entities.p` by short name)

**Expected Solution** (shows container variables only):
```
s.entities.p.x = 10
s.entities.p.y = 20
s.entities.p.z = 0
s.origin.x = 0
s.origin.y = 0
s.origin.z = 0
```

Note: The view variable `p` is not shown in output. The container variable `s.entities.p` has type Point3D, so we see x, y, and z coordinates.

#### Nested Transform

```cad
struct Point3D { x: f64, y: f64, z: f64 }
struct Point2D { x: f64, y: f64 }
struct Point1D { value: f64 }

struct Outer {
    container entities,
    fn __transform__(p: &Point3D) -> Point2D {
        return Point2D { x: p.x, y: p.y };  // Project to 2D
    }
}

struct Inner {
    container entities,
    fn __transform__(p: &Point2D) -> Point1D {
        return Point1D { value: p.x + p.y };  // Sum coordinates
    }
}

let outer: Outer;
with outer {
    let inner: Inner;
    with inner {
        let .p: Point1D;
        .p.value == 30.0;
    }
}
```

**Expected HIR**:
- **Container variable**: `outer.entities.inner.entities.p` with type `Point3D`, `VarDefinitionKind::Uninitialized`
- **View variable**: `p` with type `Point1D`, `VarDefinitionKind::TransformedView`
- Transform chain: `[outer.__transform__, inner.__transform__]`
- Transform expression: `p == inner.__transform__(outer.__transform__(&outer.entities.inner.entities.p))`

**Scoping**:
- Outside both with-blocks: Can access `outer.entities.inner.entities.p` as Point3D
- Inside inner with-block: Name `p` refers to Point1D view

**Expected Solution** (shows container variables only):
```
outer.entities.inner.entities.p.x = 15
outer.entities.inner.entities.p.y = 15
outer.entities.inner.entities.p.z = 0
```

Note: The solver finds Point3D values such that when transformed through both transforms, the final Point1D value is 30. Since the final transform sums x+y, one solution is x=15, y=15.

#### Transform Kinds (Standard vs Container)

```cad
struct Point2D { x: f64, y: f64 }
struct Point3D { x: f64, y: f64, z: f64 }

struct Sketch {
    container entities,

    // Standard transform for external variables
    fn __transform__(p: &Point3D) -> Point2D {
        return Point2D { x: p.x * 2.0, y: p.y * 2.0 };
    }

    // Container transform for dot-prefix variables (preferred)
    fn __transform_container__(p: &Point3D) -> Point2D {
        return Point2D { x: p.x, y: p.y };  // Different behavior!
    }
}

let s: Sketch;
with s {
    let .p: Point2D;  // Uses __transform_container__
    .p.x == 5.0;
}
```

**Expected HIR**:
- Transform selected: `__transform_container__` (preferred for container variables)
- If `__transform_container__` didn't exist, would fall back to `__transform__`

**Key Point**: Container variables (dot-prefix) prefer `__transform_container__` over `__transform__`, allowing different transformation behavior for internal vs. external coordinates.

#### External Variable Access (Automatic Transformation)

```cad
struct Point2D { x: f64, y: f64 }
struct Point3D { x: f64, y: f64, z: f64 }
struct Line {
    start: Point3D,
    end: Point3D,
}

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

let p: Point3D;  // External variable
let line: Line;  // External variable with nested Point3D fields

with s {
    // External variable p is automatically transformed
    p.x == 10.0;  // Refers to s.__transform__(&p).x (Point2D)
    p.y == 20.0;  // Refers to s.__transform__(&p).y (Point2D)

    // Nested field line.start is automatically transformed
    line.start.x == 5.0;  // Refers to s.__transform__(&line.start).x (Point2D)
    line.end.x == 15.0;   // Refers to s.__transform__(&line.end).x (Point2D)
}

// Outside with-block, p and line.start are Point3D again
p.z == 0.0;
line.start.z == 0.0;
line.end.z == 0.0;
```

**Expected HIR**:
- **No new VarDefinition created** for external variables
- Variable access expressions are **wrapped** with transform calls
- Inside with-block:
  - `p.x` resolves to `FieldAccess { base: MethodCall { s.__transform__(&p) }, field: "x" }`
  - Type of `p` inside block is Point2D (transformed)
  - Type of `p` outside block is Point3D (original)
- Nested field `line.start` is transformed:
  - `line.start.x` resolves to `FieldAccess { base: MethodCall { s.__transform__(&line.start) }, field: "x" }`

**Expected Solution** (shows all variables with their real types):
```
p.x = 10
p.y = 20
p.z = 0
line.start.x = 5
line.start.y = 0
line.start.z = 0
line.end.x = 15
line.end.y = 0
line.end.z = 0
s.origin.x = 0
s.origin.y = 0
s.origin.z = 0
```

**Key Points**:
- External variables (`p`, `line`) are shown with their real Point3D type
- The transform is applied during HIR construction (not at solving time)
- The solver sees constraints on `s.__transform__(&p).x == 10.0`
- This creates a constraint that links the Point3D variable `p` to the Point2D constraint
- No "shadow" or "view" variables are created - the original variables are constrained

#### Error Cases

**Invalid Transform Type**:
```cad
with sketch {
    let .invalid: InvalidType;  // No transform to InvalidType
}
```
**Expected**: Semantic analysis error (not solver error!)

**Ambiguous Transform (Same Kind, Same Output)**:
```cad
struct Sketch {
    fn __transform__(p: &Point3D) -> Point2D { ... }
    fn __transform__(p: &OtherType) -> Point2D { ... }  // ERROR: Same output type!
}
```
**Expected**: Semantic analysis error during `collect_transform_methods()`:
```
Error: Ambiguous __transform__ methods
  ┌─ example.cad:4:5
  │
3 │     fn __transform__(p: &Point3D) -> Point2D { ... }
  │        ------------- first definition here
4 │     fn __transform__(p: &OtherType) -> Point2D { ... }
  │        ^^^^^^^^^^^^^ ambiguous: multiple __transform__ methods with output type Point2D
  │
  = note: Transform methods of the same kind must have unique output types
```

**Ambiguous Transform (Mixed Kinds is OK)**:
```cad
struct Sketch {
    fn __transform__(p: &Point3D) -> Point2D { ... }
    fn __transform_container__(p: &OtherType) -> Point2D { ... }  // OK: Different kinds
}
```
**Expected**: No error - different transform kinds can have the same output type

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
- Dependencies explicit: view variable `.p` depends on container variable `sketch.entities.p`
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
- Container variables now in HIR (previously solver-only)
- Transform expressions stored in HIR
- **Impact**: Minimal (container vars are normal variables, transforms are reused)

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

### 1. Conditional Transforms

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

### 2. Transform Optimization

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

### 3. Inverse Transforms

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

### 4. Transform Verification

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

// Container variable (persistent entity in container namespace)
VarDefinition {
    name: "s.entities.p",  // Qualified name from container
    var_type: Point3D,
    definition_kind: Uninitialized,  // Free variable for solver
}

// View variable (temporary, shadows container in local scope)
VarDefinition {
    name: "p",  // Short name from source
    var_type: Point2D,
    definition_kind: TransformedView {
        container_var: &s_entities_p_def,
        transform_chain: vec![...],
        transform_expr: MethodCall(
            receiver: Var(s),
            method: "__transform__",
            args: [Ref(Var("s.entities.p"))]
        ),
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
register_var("s.entities.p.x", f64);  // Container variable (Point3D)
register_var("s.entities.p.y", f64);
register_var("s.entities.p.z", f64);
register_var("p.x", f64);  // View variable (Point2D)
register_var("p.y", f64);

// Add constraints from HIR
add_constraint("s.origin.x == 0.0");
add_constraint("s.origin.y == 0.0");
add_constraint("s.origin.z == 0.0");

// Add transform constraint (from VarDefinition.transform_expr)
// p == s.__transform__(&s.entities.p)
// Inlined:
add_constraint("p.x == s.entities.p.x - s.origin.x");
add_constraint("p.y == s.entities.p.y - s.origin.y");

// Add user constraint
add_constraint("p.x == 5.0");

// Solve with Z3
let solution = z3_solve();

// Filter output (hide view variables, show container variables)
output:
  s.entities.p.x = 5.0
  s.entities.p.y = 0.0  // Unconstrained, solver picks arbitrary value
  s.entities.p.z = 0.0  // Unconstrained, solver picks arbitrary value
  s.origin.x = 0.0
  s.origin.y = 0.0
  s.origin.z = 0.0
  // p.* hidden (view variable)
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
- **Container Variable**: Persistent variable in container namespace representing pre-transform coordinate space (e.g., `sketch.entities.p: Point3D`)
- **View Variable**: Temporary variable that shadows the container variable and represents post-transform coordinate space (e.g., `p: Point2D`)
- **Transform Method**: Special method named `__transform__` or `__transform_container__` that converts between coordinate spaces
- **Transform Expression**: HIR expression representing the transform function call and constraint
- **VarDefinitionKind**: Enum describing how a variable is defined (uninitialized, initialized, or transformed view)
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
| Phase 1: HIR Data Structures | ✅ Complete | 1-2 | - | VarDefinitionKind enum, VariableIdentifier structural representation |
| Phase 2: Container+View Variable Generation | ✅ Complete | 3-3.5 | - | Internal & external transforms, structural variable identifiers |
| Phase 3: Simplify Solver | ✅ Complete | 2 | - | Eliminated Box::leak(), uses structural variable paths |
| Phase 4: Documentation | ✅ Complete | 0.5 | - | Architecture documentation updated |
| Phase 5: Testing | ✅ Complete | 1 | 1 | Comprehensive test suite with 15 integration tests |
| **Total** | **✅ Complete** | **7.5-9** | **~4-5** | All phases implemented and tested |

---

## Revision History

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2026-01-24 | 1.0 | Claude Code | Initial implementation plan |
| 2026-01-31 | 2.0 | Claude Code | Phase 5 complete - added comprehensive test suite |

---

## Approval Status

- [x] Technical Review
- [x] Architecture Review
- [x] Implementation Approved
- [x] Testing Plan Approved
- [x] Implementation Complete
- [x] Testing Complete

---

## Implementation Summary

**Phase 5 (Testing) completed on 2026-01-31:**
- Created comprehensive test suite: `tests/hir_transform_tests.rs` with 15 integration tests
- Tests cover:
  - Internal declarations (dot-prefix variables)
  - External variable access in transform contexts
  - Nested transform contexts
  - Multiple independent transforms
  - Transform type compatibility
  - Regression tests for basic features
- All 794 tests in the project pass
- No regressions introduced

**Key achievements:**
- Structural variable identifiers eliminate memory leaks (no more `Box::leak()`)
- Transform semantics properly represented in HIR
- Clear separation between container variables (persistent) and view variables (temporary)
- Comprehensive test coverage ensures correctness

---

*This document is part of the CAD-DSL project documentation. For questions or suggestions, please file an issue on GitHub.*
