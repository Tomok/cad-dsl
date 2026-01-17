# Solver Architecture

This document describes the trait-based constraint solver architecture for CAD-DSL, including the tree-based variable management, scope handling with RAII guards, transform mechanics for coordinate system transformations, and iterative solving with deferred constraints.

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
- [Transform Mechanics](#transform-mechanics)
- [Module Structure](#module-structure)
- [Usage Guide](#usage-guide)

## Overview

The solver uses a **trait-based design** where HIR (High-level Intermediate Representation) nodes implement a `Solvable` trait that translates them into Z3 constraints. This architecture provides:

- **Modular constraint generation**: Each HIR node type implements its own solving logic
- **Type-safe variable management**: Tree structure enforces correct variable access patterns
- **Automatic scope cleanup**: RAII guards prevent scope leaks
- **Iterative solving**: Handles deferred constraints (e.g., for-loops with computed ranges)
- **Zero-copy efficiency**: String allocation only when creating Z3 variables

**Key Innovation**: Variables are stored in a **tree structure** that mirrors the type hierarchy. Instead of flattening everything to strings upfront (e.g., `"p.x"`, `"p.y"`, `"points[0].x"`), we maintain structural hierarchy and only generate flattened Z3 variable names when creating Z3 primitives.

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

### Tree-Based Variable Storage

Variables are stored as a tree that mirrors the type structure.

```rust
/// Z3 primitive types (leaves in the tree)
#[derive(Debug, Clone)]
enum Z3Primitive<'ctx> {
    Int(z3::ast::Int<'ctx>),
    Real(z3::ast::Real<'ctx>),
    Bool(z3::ast::Bool<'ctx>),
}

/// Node in the variable tree
#[derive(Debug)]
enum VariableNode<'src, 'arena, 'ctx> {
    /// Primitive variable (leaf node)
    Primitive {
        typ: ResolvedType<'src, 'arena>,
        z3_var: Z3Primitive<'ctx>,
        scope_level: usize,
        transform_link: Option<Box<TransformLink<'src, 'arena, 'ctx>>>,
    },

    /// Struct variable (branch node)
    Struct {
        typ: ResolvedType<'src, 'arena>,
        children: HashMap<&'src str, VariableNode<'src, 'arena, 'ctx>>,
        scope_level: usize,
        transform_link: Option<Box<TransformLink<'src, 'arena, 'ctx>>>,
    },

    /// Array variable (branch node)
    Array {
        typ: ResolvedType<'src, 'arena>,
        children: Vec<VariableNode<'src, 'arena, 'ctx>>,
        scope_level: usize,
        transform_link: Option<Box<TransformLink<'src, 'arena, 'ctx>>>,
    },
}

impl<'src, 'arena, 'ctx> VariableNode<'src, 'arena, 'ctx> {
    /// Get scope level of this node
    fn scope_level(&self) -> usize {
        match self {
            Self::Primitive { scope_level, .. } => *scope_level,
            Self::Struct { scope_level, .. } => *scope_level,
            Self::Array { scope_level, .. } => *scope_level,
        }
    }

    /// Navigate to descendant node by path
    fn get_at_path(&self, path: &[PathComponent<'src>]) -> Option<&Self> {
        if path.is_empty() {
            return Some(self);
        }

        match (self, &path[0]) {
            (Self::Struct { children, .. }, PathComponent::Field(field)) => {
                children.get(field)?.get_at_path(&path[1..])
            }
            (Self::Array { children, .. }, PathComponent::Index(idx)) => {
                children.get(*idx)?.get_at_path(&path[1..])
            }
            _ => None,
        }
    }

    /// Mutable navigation
    fn get_at_path_mut(&mut self, path: &[PathComponent<'src>]) -> Option<&mut Self> {
        if path.is_empty() {
            return Some(self);
        }

        match (self, &path[0]) {
            (Self::Struct { children, .. }, PathComponent::Field(field)) => {
                children.get_mut(field)?.get_at_path_mut(&path[1..])
            }
            (Self::Array { children, .. }, PathComponent::Index(idx)) => {
                children.get_mut(*idx)?.get_at_path_mut(&path[1..])
            }
            _ => None,
        }
    }

    /// Extract primitive Z3 variable (only valid for Primitive nodes)
    fn as_primitive(&self) -> Option<&Z3Primitive<'ctx>> {
        match self {
            Self::Primitive { z3_var, .. } => Some(z3_var),
            _ => None,
        }
    }

    /// Recursively collect all primitive leaves under this node
    fn collect_primitives(&self, base_path: &VariablePath<'src>)
        -> Vec<(VariablePath<'src>, &Z3Primitive<'ctx>)>
    {
        match self {
            Self::Primitive { z3_var, .. } => {
                vec![(base_path.clone(), z3_var)]
            }
            Self::Struct { children, .. } => {
                children.iter()
                    .flat_map(|(field_name, child)| {
                        child.collect_primitives(&base_path.with_field(field_name))
                    })
                    .collect()
            }
            Self::Array { children, .. } => {
                children.iter()
                    .enumerate()
                    .flat_map(|(idx, child)| {
                        child.collect_primitives(&base_path.with_index(idx))
                    })
                    .collect()
            }
        }
    }
}
```

**Tree Structure Example**:

Given this CAD-DSL code:
```
struct Point { x: i32, y: i32 }
let points: [Point; 2];
```

The tree looks like:
```
variables["points"] = Array {
    children: [
        Struct {                          // points[0]
            children: {
                "x": Primitive(Z3Int),    // points[0].x
                "y": Primitive(Z3Int),    // points[0].y
            }
        },
        Struct {                          // points[1]
            children: {
                "x": Primitive(Z3Int),    // points[1].x
                "y": Primitive(Z3Int),    // points[1].y
            }
        }
    ]
}
```

### Solver Context

The `SolverContext` manages the variable tree, scopes, and Z3 integration.

```rust
/// Context information for with-statements
#[derive(Debug, Clone)]
enum WithContextInfo<'src, 'arena> {
    /// Container with-statement: `with container { .field }`
    Container {
        container_path: VariablePath<'src>,
        container_field: &'arena FieldDefinition<'src, 'arena>,
    },

    /// Transform with-statement: coordinate transformations
    Transform {
        source_path: VariablePath<'src>,
        transform_fn: &'arena FunctionDefinition<'src, 'arena>,
        source_scope: usize,
    },
}

/// Main solver context
struct SolverContext<'src, 'arena, 'ctx> {
    /// Z3 context (persistent across scopes)
    z3_ctx: &'ctx z3::Context,

    /// Z3 solver (persistent, constraints accumulate)
    z3_solver: &'ctx z3::Solver<'ctx>,

    /// Root variable tree
    variables: HashMap<&'src str, VariableNode<'src, 'arena, 'ctx>>,

    /// Current scope depth (incremented on scope entry)
    scope_level: usize,

    /// Stack of active with-statement contexts
    with_stack: Vec<WithContextInfo<'src, 'arena>>,
}

impl<'src, 'arena, 'ctx> SolverContext<'src, 'arena, 'ctx> {
    /// Declare a new variable (builds entire tree for composite types)
    fn declare_variable(
        &mut self,
        name: &'src str,
        typ: ResolvedType<'src, 'arena>,
    ) -> Result<(), SolverError> {
        let base_path = VariablePath::from_name(name);
        let node = self.build_variable_tree(&base_path, typ)?;

        // Handle transform contexts (create shadow variables)
        if let Some(with_ctx) = self.with_stack.last() {
            if let WithContextInfo::Transform { .. } = with_ctx {
                self.create_transform_shadow(&base_path, &typ, with_ctx)?;
            }
        }

        self.variables.insert(name, node);
        Ok(())
    }

    /// Recursively build variable tree from type
    fn build_variable_tree(
        &self,
        path: &VariablePath<'src>,
        typ: ResolvedType<'src, 'arena>,
    ) -> Result<VariableNode<'src, 'arena, 'ctx>, SolverError> {
        match typ {
            ResolvedType::I32 | ResolvedType::F64 | ResolvedType::Bool => {
                // Leaf node: create Z3 primitive
                let z3_var = self.create_z3_primitive(path, &typ)?;
                Ok(VariableNode::Primitive {
                    typ,
                    z3_var,
                    scope_level: self.scope_level,
                    transform_link: None,
                })
            }

            ResolvedType::Struct { def, .. } => {
                // Branch node: recursively create children
                let mut children = HashMap::new();
                for field in &def.fields {
                    let child_path = path.with_field(field.name);
                    let child_node = self.build_variable_tree(&child_path, field.field_type)?;
                    children.insert(field.name, child_node);
                }
                Ok(VariableNode::Struct {
                    typ,
                    children,
                    scope_level: self.scope_level,
                    transform_link: None,
                })
            }

            ResolvedType::Array { element_type, size, .. } => {
                // Branch node: create indexed children
                let mut children = Vec::with_capacity(size);
                for i in 0..size {
                    let child_path = path.with_index(i);
                    let child_node = self.build_variable_tree(&child_path, *element_type)?;
                    children.push(child_node);
                }
                Ok(VariableNode::Array {
                    typ,
                    children,
                    scope_level: self.scope_level,
                    transform_link: None,
                })
            }

            _ => Err(SolverError::UnsupportedType(typ)),
        }
    }

    /// Create Z3 primitive variable (STRING ALLOCATION HAPPENS HERE)
    fn create_z3_primitive(
        &self,
        path: &VariablePath<'src>,
        typ: &ResolvedType<'src, 'arena>,
    ) -> Result<Z3Primitive<'ctx>, SolverError> {
        let name = path.to_z3_name(); // Only string allocation!
        Ok(match typ {
            ResolvedType::I32 => Z3Primitive::Int(z3::ast::Int::new_const(self.z3_ctx, name)),
            ResolvedType::F64 => Z3Primitive::Real(z3::ast::Real::new_const(self.z3_ctx, name)),
            ResolvedType::Bool => Z3Primitive::Bool(z3::ast::Bool::new_const(self.z3_ctx, name)),
            _ => return Err(SolverError::NotAPrimitiveType),
        })
    }

    /// Lookup variable by path
    fn get_variable(&self, path: &VariablePath<'src>)
        -> Option<&VariableNode<'src, 'arena, 'ctx>>
    {
        if path.components.is_empty() {
            return None;
        }

        // Extract root name
        let root_name = match &path.components[0] {
            PathComponent::Field(name) => name,
            _ => return None,
        };

        // Navigate from root
        let root = self.variables.get(root_name)?;
        root.get_at_path(&path.components[1..])
    }

    /// Mutable lookup
    fn get_variable_mut(&mut self, path: &VariablePath<'src>)
        -> Option<&mut VariableNode<'src, 'arena, 'ctx>>
    {
        if path.components.is_empty() {
            return None;
        }

        let root_name = match &path.components[0] {
            PathComponent::Field(name) => name,
            _ => return None,
        };

        let root = self.variables.get_mut(root_name)?;
        root.get_at_path_mut(&path.components[1..])
    }

    /// Remove all variables from current scope level
    fn pop_scope(&mut self) {
        self.variables.retain(|_, node| node.scope_level() < self.scope_level);
        self.scope_level -= 1;
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

## Transform Mechanics

Transform with-statements automatically invoke `__transform__` methods to create **shadow variables** linked by constraints.

### Key Insight: __transform__ is Just a Method

**`__transform__` is a regular method** - it uses the same inlining mechanism as any other method. The only special behavior is:

1. **Lookup**: When entering a with-statement, collect **all** `__transform__` methods (a struct can have multiple overloads)
2. **Selection**: When a variable is declared, select the appropriate `__transform__` based on declared type
3. **Auto-call**: Automatically call the selected `__transform__` method
4. **Shadow creation**: Create a shadow variable for that method's parameter type

The **inlining mechanism is identical** to regular method calls!

**Important**: The language spec allows **multiple `__transform__` overloads** for different types (e.g., one for `Point`, one for `Length`). The correct method must be selected based on the declared variable's type.

### Example: Multiple Transform Overloads

The language spec allows multiple `__transform__` methods for different types:

```rust
struct Scale {
    factor: f64,
    center: Point,

    // Transform for Point type
    fn __transform__(p: &Point) -> Point {
        Point {
            x: self.center.x + (p.x - self.center.x) * self.factor,
            y: self.center.y + (p.y - self.center.y) * self.factor
        }
    }

    // Transform for f64 type (lengths scale linearly)
    fn __transform__(len: &f64) -> f64 {
        len * self.factor
    }
}

with scale_2x {
    let .scaled_point: Point;   // Uses __transform__(&Point) -> Point
    let .scaled_length: f64;     // Uses __transform__(&f64) -> f64
}
```

**At declaration time**, the solver must:
1. Look at the declared type (`Point` or `f64`)
2. Find the matching `__transform__` method (by return type)
3. Use that method's parameter type for the shadow variable
4. Call that specific method

### Concept: Single Transform

When you write:
```
struct Point2D { x: f64, y: f64 }
struct Point3D { x: f64, y: f64, z: f64 }

struct Sketch2D {
    container entities,
    origin: Point3D,

    // Transform method: Point3D -> Point2D (2D projection)
    fn __transform__(p3d: &Point3D) -> Point2D {
        Point2D {
            x: p3d.x - self.origin.x,
            y: p3d.y - self.origin.y
        }
    }
}

let sketch: Sketch2D = Sketch2D {
    origin: Point3D { x: 0.0, y: 0.0, z: 0.0 }
};

with sketch {
    let .p: Point2D;  // Declared in local 2D scope
    .p.x == 10.0;
    .p.y == 20.0;
}

// After with-statement, 'sketch.entities' contains a 2D point 'p'
// linked to a 3D shadow variable via the __transform__ projection
```

**What happens**:
1. **With-statement enters**: Collect **all** `__transform__` methods from `Sketch2D` → push transform context
   - In this case: one method `fn __transform__(&Point3D) -> Point2D`
2. **Variable declaration** `.p: Point2D`:
   - Create variable `sketch.entities.p` (type: Point2D)
   - Detect transform context is active
   - **Select** the `__transform__` method where return type matches declared type (Point2D)
   - Found: `fn __transform__(p3d: &Point3D) -> Point2D`
   - Extract parameter type from selected method: `Point3D`
   - Create shadow variable of type `Point3D`
3. **Auto-invoke the selected `__transform__`**:
   - This is a **normal method call**: `sketch.__transform__(&shadow)`
   - Uses standard method inlining mechanism
   - Method body evaluated, returns Point2D expression
   - Constraint created: `sketch.entities.p == <method result>`
4. **User constraints** `.p.x == 10.0` and `.p.y == 20.0` are added normally
5. **Z3 solving**: Finds values for both the 2D variable and its 3D shadow, maintaining the transformation relationship

### Implementation

The implementation has two parts:

**Part 1: General method/function inlining** (used for ALL methods):

```rust
impl<'src, 'arena, 'ctx> SolverContext<'src, 'arena, 'ctx> {
    /// Inline any method call - works for ALL methods including __transform__
    fn inline_method(
        &mut self,
        self_path: VariablePath<'src>,
        method: &'arena MethodDefinition<'src, 'arena>,
        args: &[&'arena ResolvedExpr<'src, 'arena>],
    ) -> Result<Z3Ast<'ctx>, SolverError> {
        // 1. Create new scope for method body
        let _guard = ScopeGuard::new(self);

        // 2. Bind self
        self.self_binding = Some(self_path);

        // 3. Bind parameters to arguments
        for (param, arg) in method.params.iter().zip(args.iter()) {
            // For reference parameters: resolve to path
            // For value parameters: evaluate expression
            self.bind_parameter(param, arg)?;
        }

        // 4. Process method body statements
        for stmt in &method.body {
            stmt.solve(self)?;
        }

        // 5. Evaluate return expression
        method.return_expr.solve(self)
    }

    /// Inline any function call
    fn inline_function(
        &mut self,
        func: &'arena FunctionDefinition<'src, 'arena>,
        args: &[&'arena ResolvedExpr<'src, 'arena>],
    ) -> Result<Z3Ast<'ctx>, SolverError> {
        // Similar to inline_method, but without self binding
        // ...
    }
}
```

**Part 2: Transform auto-call logic** (only special handling for `__transform__`):

```rust
impl<'src, 'arena, 'ctx> SolverContext<'src, 'arena, 'ctx> {
    /// Handle variable declaration in transform context
    /// This is the ONLY transform-specific code!
    fn declare_variable_in_transform_context(
        &mut self,
        var_name: &'src str,
        declared_type: &ResolvedType<'src, 'arena>,
    ) -> Result<(), SolverError> {
        let transform_ctx = self.current_transform_context()?;

        // 1. Create the declared variable (e.g., Point2D)
        let local_path = self.declare_variable(var_name, declared_type)?;

        // 2. Select the appropriate __transform__ method
        //    Find method where return type matches declared_type
        let selected_method = transform_ctx.methods
            .iter()
            .find(|m| m.return_type == declared_type)
            .ok_or(SolverError::NoMatchingTransform(declared_type))?;

        // 3. Get source type from selected method's parameter
        let source_type = selected_method.params[0].ty.as_reference()?;

        // 4. Create shadow variable with source type
        let shadow_path = self.create_shadow_variable(source_type)?;

        // 5. Call the selected __transform__ using normal method inlining
        let result = self.inline_method(
            transform_ctx.struct_path,           // self
            selected_method,                      // the selected __transform__
            &[&ResolvedExpr::Variable(shadow_path)],  // args
        )?;

        // 6. Constrain: local == result
        self.add_constraint(local_path == result)?;

        Ok(())
    }
}
```

**Critical Point**: The transform context stores **all** `__transform__` methods, not just one. At declaration time, we select the method whose return type matches the declared variable type.

**Key Point**: The `inline_method()` function is used for **both** regular method calls (like `circle.area()`) **and** automatic `__transform__` calls. The only difference is **when** it gets invoked, not **how** it works.

### Transform Example Walkthrough

**Input**:
```
struct Point2D { x: f64, y: f64 }
struct Point3D { x: f64, y: f64, z: f64 }

struct Sketch2D {
    container entities,
    origin: Point3D,

    // Transform method for Point3D -> Point2D (2D projection)
    fn __transform__(p3d: &Point3D) -> Point2D {
        Point2D {
            x: p3d.x - self.origin.x,
            y: p3d.y - self.origin.y
        }
    }
}

let sketch: Sketch2D = Sketch2D {
    origin: Point3D { x: 0.0, y: 0.0, z: 0.0 }
};

with sketch {
    let .p: Point2D;
    .p.x == 10.0;
    .p.y == 20.0;
}
```

**Step-by-step**:

1. **Enter with-statement**: `WithGuard` collects **all** `__transform__` methods → push transform context
   - Found methods: `[fn __transform__(&Point3D) -> Point2D]`
   - Store all methods in transform context
2. **Declare `.p: Point2D`**:
   - Create variable: `sketch.entities.p` (type: Point2D)
   - Check: Are we in transform context? **Yes**
   - **Select** method where return type matches Point2D
   - Found: `fn __transform__(p3d: &Point3D) -> Point2D`
   - Extract parameter type: `Point3D`
   - Create shadow variable of type `Point3D` in higher scope
3. **Auto-invoke the selected `__transform__`** (this is a **normal method call**!):
   - Call: `sketch.__transform__(&shadow)` using `inline_method()`
   - Create scope for method body
   - Bind `self` to `sketch`
   - Bind parameter `p3d` to `shadow` path
   - Evaluate return expression:
     ```rust
     Point2D {
         x: p3d.x - self.origin.x,  // = shadow.x - sketch.origin.x
         y: p3d.y - self.origin.y   // = shadow.y - sketch.origin.y
     }
     ```
   - Returns Z3 struct expression
4. **Create constraint**: `sketch.entities.p == <method result>`
   - Expands to field-wise constraints:
     - `sketch.entities.p.x == shadow.x - sketch.origin.x`
     - `sketch.entities.p.y == shadow.y - sketch.origin.y`
5. **Add user constraints**: `.p.x == 10.0` and `.p.y == 20.0`
6. **Z3 solving**: Finds values for all variables including shadow

**Note**: In this example there's only one `__transform__` method, but the selection mechanism works the same way when multiple overloads exist.

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
