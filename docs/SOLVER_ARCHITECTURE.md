# Solver Architecture

This document describes the trait-based constraint solver architecture for CAD-DSL, including the tree-based variable management, scope handling with RAII guards, and iterative solving with deferred constraints.

**For transform mechanics (coordinate system transformations)**, see [HIR_TRANSFORM_REPRESENTATION.md](HIR_TRANSFORM_REPRESENTATION.md), which describes how transforms are applied during semantic analysis before reaching the solver.

## Table of Contents

- [Overview](#overview)
- [Core Design Principles](#core-design-principles)
- [Architecture Components](#architecture-components)
  - [Variable Path System](#variable-path-system)
  - [Tree-Based Variable Storage](#tree-based-variable-storage)
  - [Solver Context](#solver-context)
  - [RAII Scope Guards](#raii-scope-guards)
- [Solvable Trait](#solvable-trait)
- [Iterative Solving](#iterative-solving)
- [Transform Handling](#transform-handling)
- [Module Structure](#module-structure)
- [Usage Guide](#usage-guide)

## Overview

The solver is a **pure translation layer** that converts fully-resolved HIR (High-level Intermediate Representation) into Z3 constraints and extracts solutions. All semantic transformations (including coordinate transforms) are completed during semantic analysis before the solver runs.

The solver uses a **trait-based design** where HIR nodes implement a `Solvable` trait:

- **Modular constraint generation**: Each HIR node type implements its own solving logic
- **Type-safe variable management**: Structural variable identifiers eliminate string manipulation
- **Automatic scope cleanup**: RAII guards prevent scope leaks
- **Iterative solving**: Handles deferred constraints (e.g., for-loops with computed ranges)
- **Pure translation**: Solver only translates HIR to Z3, no semantic transformations

**Key Principle**: The solver receives **complete semantic information** from the HIR. It does not perform semantic analysis, type checking, or transform application - those phases are complete before solving begins.

## Core Design Principles

### 1. Tree Structure Over Flat Strings

**Traditional Approach** (what we're avoiding):
```rust
// Flatten everything upfront
variables: HashMap<String, Z3Variable>
// "p.x" -> Z3Int
// "p.y" -> Z3Int
// "points[0].x" -> Z3Int
// "points[1].x" -> Z3Int
```

**Our Approach**:
```rust
// Maintain structural hierarchy
variables: HashMap<&'src str, VariableNode>
// "p" -> Struct {
//   children: { "x" -> Primitive(Z3Int), "y" -> Primitive(Z3Int) }
// }
// "points" -> Array {
//   children: [
//     Struct { children: { "x" -> Primitive, "y" -> Primitive } },
//     Struct { children: { "x" -> Primitive, "y" -> Primitive } }
//   ]
// }
```

**Benefits**:
- Zero-copy navigation with `&'src str` references
- String allocation **only** when creating Z3 variables
- Natural representation of type structure
- Efficient transform link management

### 2. Zero-Copy String Management

All variable and field names use `&'src str` references:
- Variable names come from source code → `&'src str`
- Struct field names come from struct definitions (arena-allocated) → `&'src str`
- Array indices are `usize`, no string needed
- **Only allocation**: `path.to_z3_name()` when creating Z3 variables

### 3. RAII Scope Management

Use Rust's RAII pattern for automatic scope cleanup:
- `ScopeGuard` for general scopes
- `WithGuard` for with-statement contexts (both container and transform)
- Impossible to forget cleanup thanks to `Drop` implementation

### 4. Function and Method Inlining

User-defined functions and methods are **inlined** rather than called:
- Function/method body is analyzed and substituted
- Parameters bound to arguments in the function scope
- Return expression evaluated to produce result
- Works for all functions/methods, including `__transform__`
- Generates direct constraints using existing expression-to-Z3 infrastructure

**Special case - `__transform__` methods:**
- `__transform__` is a regular method with a special name
- The **only** difference: automatically invoked by with-statements
- Inlining mechanism is identical to any other method

## Architecture Components

### Variable Path System

Paths represent navigation routes through the variable tree.

```rust
/// Component of a variable path
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum PathComponent<'src> {
    /// Struct field access: `.field`
    Field(&'src str),
    /// Array index access: `[0]`
    Index(usize),
}

/// Complete path to a variable or sub-variable
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VariablePath<'src> {
    components: Vec<PathComponent<'src>>,
}

impl<'src> VariablePath<'src> {
    /// Create path from root variable name
    fn from_name(name: &'src str) -> Self {
        Self {
            components: vec![PathComponent::Field(name)],
        }
    }

    /// Extend path with field access
    fn with_field(&self, field: &'src str) -> Self {
        let mut new_path = self.clone();
        new_path.components.push(PathComponent::Field(field));
        new_path
    }

    /// Extend path with array index
    fn with_index(&self, idx: usize) -> Self {
        let mut new_path = self.clone();
        new_path.components.push(PathComponent::Index(idx));
        new_path
    }

    /// Generate Z3 variable name (ONLY place where String allocation happens!)
    fn to_z3_name(&self) -> String {
        let mut result = String::new();
        for (i, comp) in self.components.iter().enumerate() {
            match comp {
                PathComponent::Field(name) => {
                    if i > 0 { result.push('.'); }
                    result.push_str(name);
                }
                PathComponent::Index(idx) => {
                    write!(&mut result, "[{}]", idx).unwrap();
                }
            }
        }
        result
    }
}
```

**Example Paths**:
- `p.x` → `[Field("p"), Field("x")]`
- `points[0].y` → `[Field("points"), Index(0), Field("y")]`
- `sketch.entities.line` → `[Field("sketch"), Field("entities"), Field("line")]`

### Structural Variable Identifiers

Variables are identified structurally using `VariableIdentifier` rather than flattened strings.

```rust
/// Identifies a variable structurally without string concatenation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VariableIdentifier<'src, 'arena> {
    /// Simple variable: `x`
    Simple(&'src str),

    /// Field access: `base.field`
    FieldAccess {
        base: &'arena VariableIdentifier<'src, 'arena>,
        field: &'src str,
    },

    /// Container field access: `container.field` (dot-prefix variables)
    ContainerFieldAccess {
        container_var: &'arena VariableIdentifier<'src, 'arena>,
        container_field: &'src str,
    },

    /// Array index: `array[index]`
    ArrayIndex {
        array: &'arena VariableIdentifier<'src, 'arena>,
        index: usize,
    },
}

**Key features**:
- **Structural representation**: Variables are identified by their structure, not string names
- **Arena-allocated**: All components use `&'arena` references, avoiding heap allocations
- **Type-safe navigation**: Compile-time guarantees about valid access patterns
- **Lazy string generation**: Strings only created when needed for Z3 variable names

**Example identifiers**:
```rust
// Simple variable: x
VariableIdentifier::Simple("x")

// Field access: p.x
VariableIdentifier::FieldAccess {
    base: &VariableIdentifier::Simple("p"),
    field: "x",
}

// Array element: points[0].y
VariableIdentifier::ArrayIndex {
    array: &VariableIdentifier::Simple("points"),
    index: 0,
}.with_field("y")

// Container variable: sketch.entities.p
VariableIdentifier::ContainerFieldAccess {
    container_var: &VariableIdentifier::Simple("sketch"),
    container_field: "entities",
}.with_field("p")
```

### Solver Context

The `SolverContext` manages variables, scopes, and Z3 integration.

```rust
/// Main solver context
struct SolverContext<'src, 'arena, 'ctx> {
    /// Z3 context (persistent across scopes)
    z3_ctx: &'ctx z3::Context,

    /// Z3 solver (persistent, constraints accumulate)
    z3_solver: &'ctx z3::Solver<'ctx>,

    /// Variable storage: maps identifiers to Z3 variables
    /// Variables are flattened to primitive fields for Z3
    variables: HashMap<VariablePath<'src>, Z3Expr<'ctx>>,

    /// Current scope depth (incremented on scope entry)
    scope_level: usize,

    /// HIR arena for temporary allocations during solving
    arena: &'arena bumpalo::Bump,
}

impl<'src, 'arena, 'ctx> SolverContext<'src, 'arena, 'ctx> {
    /// Declare a new variable based on its definition kind
    fn declare_variable(
        &mut self,
        var_def: &'arena VarDefinition<'src, 'arena>,
    ) -> Result<(), SolverError> {
        match &var_def.definition_kind {
            VarDefinitionKind::Uninitialized => {
                // Free variable - create Z3 variables for all primitive fields
                self.declare_variable_at_path(&var_def.identifier, &var_def.var_type)?;
            }

            VarDefinitionKind::Initialized { init } => {
                // Declare variable and add initialization constraint
                self.declare_variable_at_path(&var_def.identifier, &var_def.var_type)?;
                let init_z3 = init.solve(self)?;
                self.add_equality_constraint(&var_def.identifier, &init_z3)?;
            }

            VarDefinitionKind::TransformedView { container_var, transform_expr, .. } => {
                // Container variable should already be declared (it's Uninitialized)
                // Declare the view variable
                self.declare_variable_at_path(&var_def.identifier, &var_def.var_type)?;

                // Add transform constraint: view == transform_expr
                let transform_z3 = transform_expr.solve(self)?;
                self.add_equality_constraint(&var_def.identifier, &transform_z3)?;
            }
        }
        Ok(())
    }

    /// Declare variable at a specific path (flattens structs/arrays to primitives)
    fn declare_variable_at_path(
        &mut self,
        identifier: &VariableIdentifier<'src, 'arena>,
        typ: &ResolvedType<'src, 'arena>,
    ) -> Result<(), SolverError> {
        match typ {
            ResolvedType::I32 | ResolvedType::F64 | ResolvedType::Bool => {
                // Primitive type: create Z3 variable
                let path = self.identifier_to_path(identifier);
                let z3_name = path.to_string(); // Only string allocation!
                let z3_var = match typ {
                    ResolvedType::I32 => Z3Expr::Int(Int::new_const(self.z3_ctx, z3_name)),
                    ResolvedType::F64 => Z3Expr::Real(Real::new_const(self.z3_ctx, z3_name)),
                    ResolvedType::Bool => Z3Expr::Bool(Bool::new_const(self.z3_ctx, z3_name)),
                    _ => unreachable!(),
                };
                self.variables.insert(path, z3_var);
            }

            ResolvedType::Struct { def, .. } => {
                // Recursively declare fields
                for field in &def.fields {
                    let field_id = self.arena.alloc(VariableIdentifier::FieldAccess {
                        base: identifier,
                        field: field.name,
                    });
                    self.declare_variable_at_path(field_id, &field.field_type)?;
                }
            }

            ResolvedType::Array { element_type, size, .. } => {
                // Recursively declare elements
                for i in 0..*size {
                    let elem_id = self.arena.alloc(VariableIdentifier::ArrayIndex {
                        array: identifier,
                        index: i,
                    });
                    self.declare_variable_at_path(elem_id, element_type)?;
                }
            }

            _ => return Err(SolverError::UnsupportedType(typ.clone())),
        }
        Ok(())
    }
}
```

### RAII Scope Guards

Automatic scope management using RAII pattern prevents forgetting to clean up scopes.

```rust
/// General scope guard
struct ScopeGuard<'a, 'src, 'arena, 'ctx> {
    ctx: &'a mut SolverContext<'src, 'arena, 'ctx>,
    active: bool,
}

impl<'a, 'src, 'arena, 'ctx> ScopeGuard<'a, 'src, 'arena, 'ctx> {
    fn new(ctx: &'a mut SolverContext<'src, 'arena, 'ctx>) -> Self {
        ctx.scope_level += 1;
        ScopeGuard { ctx, active: true }
    }

    fn context(&mut self) -> &mut SolverContext<'src, 'arena, 'ctx> {
        self.ctx
    }

    /// Manually disable guard if needed
    fn disarm(mut self) {
        self.active = false;
    }
}

impl Drop for ScopeGuard<'_, '_, '_, '_> {
    fn drop(&mut self) {
        if self.active {
            self.ctx.pop_scope();
        }
    }
}

/// With-statement guard (handles both container and transform contexts)
struct WithGuard<'a, 'src, 'arena, 'ctx> {
    ctx: &'a mut SolverContext<'src, 'arena, 'ctx>,
    active: bool,
}

impl<'a, 'src, 'arena, 'ctx> WithGuard<'a, 'src, 'arena, 'ctx> {
    fn new(
        ctx: &'a mut SolverContext<'src, 'arena, 'ctx>,
        with_info: WithContextInfo<'src, 'arena>,
    ) -> Self {
        ctx.with_stack.push(with_info);
        ctx.scope_level += 1;
        WithGuard { ctx, active: true }
    }

    fn context(&mut self) -> &mut SolverContext<'src, 'arena, 'ctx> {
        self.ctx
    }
}

impl Drop for WithGuard<'_, '_, '_, '_> {
    fn drop(&mut self) {
        if self.active {
            self.ctx.pop_scope();
            self.ctx.with_stack.pop();
        }
    }
}
```

**Usage Example**:
```rust
// Scopes are cleaned up automatically when guards drop
fn solve_function_call(&mut self, ctx: &mut SolverContext) -> Result<(), SolverError> {
    let mut scope_guard = ScopeGuard::new(ctx);

    // Declare local variables
    scope_guard.context().declare_variable("local", ResolvedType::I32)?;

    // Solve body
    self.body.solve(scope_guard.context())?;

    // Scope automatically cleaned up when scope_guard drops here
    Ok(())
}
```

## Solvable Trait

The `Solvable` trait allows HIR nodes to translate themselves into Z3 constraints.

```rust
/// Trait for HIR nodes that can be solved as constraints
trait Solvable<'src, 'arena, 'ctx> {
    fn solve(
        &self,
        ctx: &mut SolverContext<'src, 'arena, 'ctx>,
    ) -> Result<(), SolverError>;
}

impl<'src, 'arena, 'ctx> Solvable<'src, 'arena, 'ctx> for ResolvedStmt<'src, 'arena> {
    fn solve(&self, ctx: &mut SolverContext<'src, 'arena, 'ctx>) -> Result<(), SolverError> {
        match &self.kind {
            ResolvedStmtKind::Let { name, var_type, initializer, .. } => {
                // Declare variable (builds tree)
                ctx.declare_variable(name, *var_type)?;

                // Add constraint if initialized
                if let Some(init) = initializer {
                    let path = VariablePath::from_name(name);
                    let target = ctx.get_variable(&path)
                        .ok_or_else(|| SolverError::UndefinedVariable(path.clone()))?;

                    let constraint = self.generate_assignment_constraint(target, init, ctx)?;
                    ctx.z3_solver.assert(&constraint);
                }
                Ok(())
            }

            ResolvedStmtKind::Expression { expr, .. } => {
                // Expression statement must be a constraint
                let constraint = expr.to_z3_constraint(ctx)?;
                ctx.z3_solver.assert(&constraint);
                Ok(())
            }

            ResolvedStmtKind::If { condition, then_body, .. } => {
                // Conditional constraints
                let cond_z3 = condition.to_z3_bool(ctx)?;

                for stmt in then_body {
                    // Each constraint in then_body becomes: condition => constraint
                    let inner_constraint = stmt.to_z3_constraint(ctx)?;
                    let implication = cond_z3.implies(&inner_constraint);
                    ctx.z3_solver.assert(&implication);
                }
                Ok(())
            }

            ResolvedStmtKind::With { context: with_ctx, body, .. } => {
                // Unified handling for both container and transform contexts
                let with_info = match with_ctx {
                    WithContext::Container { target_expr, container_field } => {
                        let container_path = target_expr.to_path(ctx)?;
                        WithContextInfo::Container {
                            container_path,
                            container_field: *container_field,
                        }
                    }
                    WithContext::Transform { target_expr, transform_fn } => {
                        let source_path = target_expr.to_path(ctx)?;
                        WithContextInfo::Transform {
                            source_path,
                            transform_fn: *transform_fn,
                            source_scope: ctx.scope_level,
                        }
                    }
                };

                // Create guard - automatic cleanup on drop
                let mut guard = WithGuard::new(ctx, with_info);

                // Solve body
                for stmt in body {
                    stmt.solve(guard.context())?;
                }

                // Guard drops here, cleaning up scope and with-context
                Ok(())
            }

            _ => Err(SolverError::UnsupportedStatement(self.span)),
        }
    }
}
```

## Transform Handling

**Transform semantics are implemented in the semantic analyzer**, not the solver. By the time the HIR reaches the solver, all transform-related variable creation and constraint generation is complete.

For detailed information on how transforms work, see [HIR_TRANSFORM_REPRESENTATION.md](HIR_TRANSFORM_REPRESENTATION.md).

### Solver's Role in Transforms

The solver receives HIR that already contains:

1. **Container variables**: Persistent entities in container namespaces (e.g., `sketch.entities.p: Point3D`)
2. **View variables**: Temporary transformed views with `VarDefinitionKind::TransformedView`
3. **Transform constraints**: Method call expressions linking container and view variables

The solver simply:
- Declares variables based on their `VarDefinitionKind`
- Evaluates transform method calls like any other method
- Adds the resulting constraints to Z3

### Example: What the Solver Sees

**Source code**:
```
with sketch {
    let .p: Point2D;
    .p.x == 10.0;
}
```

**HIR received by solver** (simplified):
```rust
// Container variable (created during semantic analysis)
VarDefinition {
    name: "sketch.entities.p",
    var_type: Point3D,
    definition_kind: Uninitialized,  // Free variable
}

// View variable (created during semantic analysis)
VarDefinition {
    name: "p",
    var_type: Point2D,
    definition_kind: TransformedView {
        container_var: &container_var,
        transform_expr: sketch.__transform__(&sketch.entities.p),
    }
}

// User constraint
Constraint: p.x == 10.0
```

**Solver processing**:
1. Declare `sketch.entities.p: Point3D` as free variable (uninitialized)
2. Declare `p: Point2D` as free variable
3. Add transform constraint: `p == sketch.__transform__(&sketch.entities.p)` (method inlining)
4. Add user constraint: `p.x == 10.0`
5. Solve with Z3

The solver doesn't need to know about transform semantics - it just follows the HIR structure.

### Function and Method Inlining

User-defined functions and methods (including `__transform__`) are **inlined** rather than called:

- Function/method body is analyzed and substituted
- Parameters bound to arguments in the function scope
- Return expression evaluated to produce result
- Generates direct constraints using existing expression-to-Z3 infrastructure

**There is no special handling for `__transform__`** in the solver - it's just a method that gets inlined like any other.

## Module Structure

The solver implementation follows a clean separation between trait definitions and implementations, with the `impls/` submodule structure mirroring the HIR structure.

### Directory Layout

```
src/
  solver.rs                 # Solvable trait definition, core types, submodule declarations
  solver/
    context.rs              # SolverContext with variable management
    constraint_extractor.rs # High-level constraint extraction pipeline
    struct_flattener.rs     # Struct/array flattening for Z3
    z3_bridge.rs            # Z3 interface and expression conversion
    solution_formatter.rs   # Format Z3 solutions for display
    function_inliner.rs     # Function/method inlining logic
    impls.rs                # Submodule declarations for trait implementations
    impls/
      expr.rs               # impl Solvable for ResolvedExpr
      stmt.rs               # impl Solvable for ResolvedStmt
      definitions.rs        # impl Solvable for FunctionDefinition, etc.
```

**Note**: Following modern Rust conventions (since 2018 edition), we use `solver.rs` + `solver/` directory instead of `solver/mod.rs`. Similarly, `impls.rs` instead of `impls/mod.rs`.

### Design Rationale

**Why separate impls from the trait?**
- **Modularity**: Each HIR type's solver logic is isolated
- **Parallel structure**: `impls/` mirrors `hir/` for easy navigation
- **Clarity**: Trait definition stays clean and focused
- **Testing**: Each impl module can have its own test suite

**Comparison with HIR structure:**

```
src/hir/                    src/solver/impls/
  expr.rs                     expr.rs        (impl Solvable for ResolvedExpr)
  definitions.rs              definitions.rs (impl Solvable for Function/Struct/etc.)
  types.rs                    types.rs       (impl helpers for ResolvedType if needed)
  scope.rs                    (not needed in solver impls)
  context.rs                  (not needed in solver impls)
```

### Example: expr.rs

```rust
// src/solver/impls/expr.rs
use crate::hir::expr::ResolvedExpr;
use crate::solver::{Solvable, SolverContext, SolverError};

impl<'src, 'arena, 'ctx> Solvable<'src, 'arena, 'ctx> for ResolvedExpr<'src, 'arena> {
    fn solve(
        &self,
        ctx: &mut SolverContext<'src, 'arena, 'ctx>,
    ) -> Result<Z3Ast<'ctx>, SolverError> {
        match self {
            ResolvedExpr::Literal(lit) => {
                // Convert literal to Z3
            }
            ResolvedExpr::Variable(var) => {
                // Lookup variable in context
            }
            ResolvedExpr::BinaryOp { op, left, right } => {
                // Recursively solve operands
                let left_z3 = left.solve(ctx)?;
                let right_z3 = right.solve(ctx)?;
                // Apply operator
            }
            ResolvedExpr::FunctionCall { func, args } => {
                // Inline function
                ctx.inline_function(func, args)
            }
            ResolvedExpr::MethodCall { receiver, method, args } => {
                // Inline method
                let self_path = ctx.resolve_expr_to_path(receiver)?;
                ctx.inline_method(self_path, method, args)
            }
            // ... other expression types
        }
    }
}

#[cfg(test)]
mod tests {
    // Tests specific to expression solving
}
```

### Example: stmt.rs

```rust
// src/solver/impls/stmt.rs
use crate::hir::ResolvedStmt;
use crate::solver::{Solvable, SolverContext, SolverError};

impl<'src, 'arena, 'ctx> Solvable<'src, 'arena, 'ctx> for ResolvedStmt<'src, 'arena> {
    fn solve(
        &self,
        ctx: &mut SolverContext<'src, 'arena, 'ctx>,
    ) -> Result<(), SolverError> {
        match self {
            ResolvedStmt::Let { name, ty, init, .. } => {
                // Declare variable
                ctx.declare_variable(name, ty)?;

                // Handle initializer
                if let Some(init_expr) = init {
                    let value = init_expr.solve(ctx)?;
                    ctx.add_initialization_constraint(name, value)?;
                }

                Ok(())
            }
            ResolvedStmt::Expression(expr) => {
                // Expression statement (constraint)
                let z3_expr = expr.solve(ctx)?;
                ctx.assert_constraint(z3_expr)
            }
            ResolvedStmt::With { target, body, .. } => {
                // Handle with-statement
                let _guard = ctx.enter_with_context(target)?;
                for stmt in body {
                    stmt.solve(ctx)?;
                }
                Ok(())
            }
            // ... other statement types
        }
    }
}

#[cfg(test)]
mod tests {
    // Tests specific to statement solving
}
```

### Module Declarations

```rust
// src/solver/impls.rs
// Declare submodules for trait implementations
mod expr;
mod stmt;
mod definitions;

// All impls are automatically available when the module is imported
// No need to explicitly re-export since traits are implemented on foreign types
```

```rust
// src/solver.rs
// Declare submodules
mod context;
mod constraint_extractor;
mod struct_flattener;
mod z3_bridge;
mod solution_formatter;
mod function_inliner;
mod impls;  // This pulls in all trait implementations

// Public exports
pub use context::SolverContext;

// Trait definition
pub trait Solvable<'src, 'arena, 'ctx> {
    type Output;

    fn solve(
        &self,
        ctx: &mut SolverContext<'src, 'arena, 'ctx>,
    ) -> Result<Self::Output, SolverError>;
}
```

## Iterative Solving

The solver supports **iterative solving** to handle constraints that depend on values computed in earlier iterations. This is essential for features like for-loops with computed ranges.

### How It Works

1. **First Iteration**: Solve all constraints that can be immediately satisfied
2. **Extract Values**: Get concrete values for resolved variables from the Z3 model
3. **Check Deferred**: Try to resolve deferred constraints (e.g., for-loops) using the new values
4. **Repeat**: If progress was made, run another iteration
5. **Terminate**: Stop when either all constraints are resolved or no progress is made

### Result Types

The solver returns a `SolveResult` which can be either:

- **`Complete`**: All constraints fully resolved
  ```rust
  SolveResult::Complete {
      solution: Solution,     // Variable assignments
      iterations: usize,      // Number of iterations performed
  }
  ```

- **`Partial`**: Some constraints could not be resolved (still a valid outcome, not an error)
  ```rust
  SolveResult::Partial {
      solution: Solution,            // Partial variable assignments
      deferred: Vec<DeferredConstraint>,  // Unresolved constraints
      reason: PartialReason,         // Why solving stopped
      iterations: usize,             // Number of iterations performed
  }
  ```

### Example: For-Loop with Computed Range

```rust
let n: i32;
n == 5;

for i in 0..n {
    // Loop body uses i
}
```

**Iteration 1**: The for-loop is deferred because `n` is unknown
**After iteration 1**: Z3 solves `n = 5`
**Iteration 2**: For-loop can now be unrolled with range `0..5`
**Result**: Complete solution

## Usage Guide

### Basic Usage

The primary entry point is the `solve()` function in `src/solver.rs`:

```rust
use cad_dsl::solver;
use bumpalo::Bump;

let arena = Bump::new();
let statements = /* HIR statements */;

match solver::solve(&statements, &arena) {
    Ok(solution_string) => println!("{}", solution_string),
    Err(e) => eprintln!("Solver error: {}", e),
}
```

### Implementing `Solvable` for New HIR Nodes

To add solver support for a new HIR node type:

1. **Import the trait**:
   ```rust
   use crate::solver::{Solvable, SolverContext, SolverError};
   ```

2. **Implement the trait**:
   ```rust
   impl<'src, 'arena> Solvable<'src, 'arena> for MyHirNode<'src, 'arena> {
       type Output = (); // or Z3 AST type for expressions

       fn solve(&self, ctx: &mut SolverContext<'src, 'arena>)
           -> Result<Self::Output, SolverError>
       {
           // Your constraint generation logic here
       }
   }
   ```

3. **Add implementation to** `src/solver/impls/`:
   - Statement nodes → `impls/stmt.rs`
   - Expression nodes → `impls/expr.rs`

### Working with Variable Paths

The `VariablePath` type represents navigation through the variable tree:

```rust
// Create path from variable name
let path = VariablePath::from_name("p");

// Extend with field access
let x_path = path.with_field("x");  // p.x

// Extend with array index
let elem_path = path.with_index(0);  // p[0]

// Chain operations
let nested = VariablePath::from_name("points")
    .with_index(0)      // points[0]
    .with_field("x");   // points[0].x

// Generate Z3 variable name (only allocation point!)
let z3_name = nested.to_z3_name();  // "points[0].x"
```

### RAII Scope Management

Use scope guards to ensure automatic cleanup:

```rust
fn solve_with_scope(ctx: &mut SolverContext) -> Result<(), SolverError> {
    // Create scope guard - increments scope level
    let mut guard = ScopeGuard::new(ctx);

    // Declare variables in this scope
    guard.context().declare_variable("local", type_ref)?;

    // Solve statements
    for stmt in statements {
        stmt.solve(guard.context())?;
    }

    // Scope automatically cleaned up when guard drops here
    Ok(())
}
```

### Key Components

- **`VariablePath`**: Zero-copy navigation through variable tree
- **`VariableNode`**: Tree structure (Primitive, Struct, or Array)
- **`SolverContext`**: Manages variables, scopes, Z3 integration
- **`ScopeGuard`/`WithGuard`**: RAII automatic scope cleanup
- **`Solvable` trait**: HIR nodes implement constraint generation
- **`Solution`**: Maps variable paths to concrete values
- **`SolveResult`**: Complete or Partial solving outcome

### Testing

The solver has comprehensive test coverage:

```bash
# Run all solver tests
cargo test solver

# Run specific integration tests
cargo test solver_integration

# Run with output visible
cargo test solver -- --nocapture
```

**Test organization**:
- Unit tests in module files (`src/solver.rs`, `src/solver/context.rs`)
- Integration tests in `tests/solver_integration_test.rs`
- Performance tests in `tests/solver_performance_test.rs`

## Future Optimizations

Potential improvements for performance and functionality:

- **Lazy Z3 variable creation**: Only create Z3 variables when referenced in constraints
- **Constraint simplification**: Detect trivial constraints (e.g., `x == x`) before sending to Z3
- **Incremental solving**: Use Z3's push/pop for faster repeated solving
- **Parallel constraint generation**: Generate constraints for independent statements in parallel

## Summary

The trait-based solver architecture provides:

- ✅ **Modularity**: HIR nodes implement their own constraint logic
- ✅ **Efficiency**: Zero-copy navigation, string allocation only for Z3 variables
- ✅ **Safety**: RAII guards prevent scope leaks, type system enforces correct access
- ✅ **Extensibility**: Add features by implementing `Solvable` trait
- ✅ **Robustness**: Iterative solving handles complex dependency chains
- ✅ **Clarity**: Clean separation between HIR semantics and solver mechanics

Adding new language features is straightforward: implement `Solvable` for the new HIR node type, and the solver infrastructure handles variable management, scoping, and Z3 integration automatically.
