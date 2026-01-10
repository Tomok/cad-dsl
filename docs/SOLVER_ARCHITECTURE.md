# Solver Architecture

This document describes the trait-based constraint solver architecture for CAD-DSL, including the tree-based variable management, scope handling with RAII guards, and transform mechanics for coordinate system transformations.

## Table of Contents

- [Overview](#overview)
- [Core Design Principles](#core-design-principles)
- [Architecture Components](#architecture-components)
  - [Variable Path System](#variable-path-system)
  - [Tree-Based Variable Storage](#tree-based-variable-storage)
  - [Solver Context](#solver-context)
  - [RAII Scope Guards](#raii-scope-guards)
- [Solvable Trait](#solvable-trait)
- [Transform Mechanics](#transform-mechanics)
- [Implementation Guide](#implementation-guide)

## Overview

The solver architecture uses a **trait-based design** where HIR (High-level Intermediate Representation) nodes implement a `Solvable` trait that allows them to translate themselves into Z3 constraints. This approach:

- Decouples constraint extraction from specific HIR node types
- Enables incremental feature development (implement `Solvable` for new nodes)
- Maintains clean separation between HIR semantics and solver mechanics

**Key Innovation**: Instead of flattening all variables to strings upfront, we maintain a **tree structure** that mirrors the type hierarchy and only generate flattened Z3 variable names when creating Z3 primitives.

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

### 4. Function Inlining for Transforms

Transform functions are **inlined** rather than called:
- Transform function body is analyzed and substituted
- Generates direct constraints between source and target variables
- Enables complex transforms with type changes
- Reuses existing expression-to-Z3 infrastructure

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

Transform with-statements create **shadow variables** in multiple scopes linked by constraints derived from the transform function.

### Concept

When you write:
```
struct Point2D { x: f64, y: f64 }
struct Point3D { x: f64, y: f64, z: f64 }

fn project(p: Point3D) -> Point2D {
    return Point2D { x: p.x, y: p.y };
}

let sketch: Sketch;  // Has Point3D coordinate system

with sketch.transform(project) {
    let p: Point2D;  // Declared in local 2D scope
    p.x == 10.0;
    p.y == 20.0;
}

// After with-statement, 'sketch' has a new 3D point named 'p'
// linked to the local 2D point via the projection
```

**What happens**:
1. Local variable `p: Point2D` is declared in the with-statement scope
2. Shadow variable `p: Point3D` is **automatically created** in the higher scope
3. Constraints link them: `p_2d.x == p_3d.x` and `p_2d.y == p_3d.y`
4. When solving: solver finds values for both, maintaining the relationship

### Implementation

```rust
/// Link between local and shadow variables
struct TransformLink<'src, 'arena, 'ctx> {
    /// Shadow variable in higher scope (e.g., Point3D)
    source_path: VariablePath<'src>,

    /// Transform function defining the relationship
    transform_fn: &'arena FunctionDefinition<'src, 'arena>,
}

impl<'src, 'arena, 'ctx> SolverContext<'src, 'arena, 'ctx> {
    /// Create shadow variable with transform link
    fn create_transform_shadow(
        &mut self,
        local_path: &VariablePath<'src>,
        local_type: &ResolvedType<'src, 'arena>,
        with_info: &WithContextInfo<'src, 'arena>,
    ) -> Result<(), SolverError> {
        let (source_path, transform_fn, source_scope) = match with_info {
            WithContextInfo::Transform { source_path, transform_fn, source_scope } => {
                (source_path, transform_fn, *source_scope)
            }
            _ => return Ok(()), // Not a transform context
        };

        // 1. Get source type from transform return type
        let source_type = transform_fn.return_type
            .ok_or(SolverError::TransformNoReturnType)?;

        // 2. Build source variable in higher scope
        let temp_scope = self.scope_level;
        self.scope_level = source_scope;

        let source_node = self.build_variable_tree(source_path, source_type)?;

        if let Some(PathComponent::Field(root_name)) = source_path.components.first() {
            self.variables.insert(root_name, source_node);
        }

        self.scope_level = temp_scope;

        // 3. Inline transform function to generate constraints
        let constraints = self.inline_function_as_constraint(
            local_path,
            transform_fn,
            source_path,
        )?;

        // 4. Add constraints to solver
        for constraint in constraints {
            self.z3_solver.assert(&constraint);
        }

        // 5. Store transform link
        if let Some(local_node) = self.get_variable_mut(local_path) {
            let link = TransformLink {
                source_path: source_path.clone(),
                transform_fn,
            };

            match local_node {
                VariableNode::Primitive { transform_link, .. } |
                VariableNode::Struct { transform_link, .. } |
                VariableNode::Array { transform_link, .. } => {
                    *transform_link = Some(Box::new(link));
                }
            }
        }

        Ok(())
    }

    /// Inline function to generate constraints
    /// Returns constraints of form: local = transform_fn(source)
    fn inline_function_as_constraint(
        &mut self,
        target_path: &VariablePath<'src>,
        func: &FunctionDefinition<'src, 'arena>,
        source_path: &VariablePath<'src>,
    ) -> Result<Vec<z3::ast::Bool<'ctx>>, SolverError> {
        // 1. Extract return expression from function body
        let body = func.body.as_ref()
            .ok_or(SolverError::TransformNoBody)?;
        let return_expr = self.find_return_expression(body)?;

        // 2. Build parameter substitution map
        let mut substitutions = HashMap::new();
        if let Some(param) = func.parameters.first() {
            substitutions.insert(param.name, source_path.clone());
        }

        // 3. Evaluate return expression with substitutions
        //    This replaces parameter references with source_path references
        let result_expr = self.eval_expr_with_substitution(return_expr, &substitutions)?;

        // 4. Generate equality constraints between target and result
        self.generate_equality_constraints(target_path, result_expr)
    }

    /// Evaluate expression with variable substitutions
    fn eval_expr_with_substitution(
        &mut self,
        expr: &ResolvedExpr<'src, 'arena>,
        substitutions: &HashMap<&'src str, VariablePath<'src>>,
    ) -> Result<Z3Expression<'ctx>, SolverError> {
        match &expr.kind {
            ResolvedExprKind::Variable { name, .. } => {
                // Check if this is a parameter reference that should be substituted
                if let Some(substitute_path) = substitutions.get(name) {
                    return self.path_to_z3(substitute_path);
                }
                // Normal variable
                let path = VariablePath::from_name(name);
                self.path_to_z3(&path)
            }

            ResolvedExprKind::FieldAccess { base, field, .. } => {
                // Recursively evaluate base with substitutions
                let base_expr = self.eval_expr_with_substitution(base, substitutions)?;
                // Extend path with field
                base_expr.with_field(field)
            }

            ResolvedExprKind::StructLiteral { fields, .. } => {
                // Build struct from field expressions
                let mut field_map = HashMap::new();
                for (field_name, field_expr) in fields {
                    let z3_expr = self.eval_expr_with_substitution(field_expr, substitutions)?;
                    field_map.insert(*field_name, z3_expr);
                }
                Ok(Z3Expression::Struct { fields: field_map })
            }

            // Arithmetic, comparisons, etc.
            _ => {
                // Convert to Z3 with substitutions
                self.expr_to_z3_with_substitutions(expr, substitutions)
            }
        }
    }

    /// Generate constraints equating two composite expressions
    fn generate_equality_constraints(
        &mut self,
        target_path: &VariablePath<'src>,
        source_expr: Z3Expression<'ctx>,
    ) -> Result<Vec<z3::ast::Bool<'ctx>>, SolverError> {
        let mut constraints = Vec::new();

        match source_expr {
            Z3Expression::Primitive(source_z3) => {
                // Simple case: target primitive == source primitive
                let target_node = self.get_variable(target_path)
                    .ok_or_else(|| SolverError::UndefinedVariable(target_path.clone()))?;
                let target_z3 = target_node.as_primitive()
                    .ok_or_else(|| SolverError::NotAPrimitive(target_path.clone()))?;

                let constraint = self.generate_primitive_equality(target_z3, &source_z3)?;
                constraints.push(constraint);
            }

            Z3Expression::Struct { fields } => {
                // Recursive case: equate each field
                for (field_name, field_expr) in fields {
                    let field_path = target_path.with_field(field_name);
                    let field_constraints = self.generate_equality_constraints(&field_path, field_expr)?;
                    constraints.extend(field_constraints);
                }
            }
        }

        Ok(constraints)
    }

    fn generate_primitive_equality(
        &self,
        target: &Z3Primitive<'ctx>,
        source: &Z3Primitive<'ctx>,
    ) -> Result<z3::ast::Bool<'ctx>, SolverError> {
        match (target, source) {
            (Z3Primitive::Int(t), Z3Primitive::Int(s)) => Ok(t._eq(s).into()),
            (Z3Primitive::Real(t), Z3Primitive::Real(s)) => Ok(t._eq(s).into()),
            (Z3Primitive::Bool(t), Z3Primitive::Bool(s)) => Ok(t._eq(s)),
            _ => Err(SolverError::TypeMismatch),
        }
    }
}

/// Intermediate Z3 expression representation
enum Z3Expression<'ctx> {
    Primitive(Z3Primitive<'ctx>),
    Struct {
        fields: HashMap<&'static str, Z3Expression<'ctx>>,
    },
}
```

### Transform Example Walkthrough

**Input**:
```
struct Point2D { x: f64, y: f64 }
struct Point3D { x: f64, y: f64, z: f64 }

fn project(p: Point3D) -> Point2D {
    return Point2D { x: p.x, y: p.y };
}

let world: World;

with world.transform(project) {
    let p: Point2D;
    p.x == 10.0;
}
```

**Step-by-step**:

1. **Enter with-statement**: `WithGuard` pushes `Transform` context
2. **Declare `p: Point2D`** in local scope (scope_level = 1)
   - Creates tree: `p -> Struct { x: Primitive(Real), y: Primitive(Real) }`
3. **Detect transform context**, call `create_transform_shadow`
4. **Create shadow variable** `p: Point3D` in higher scope (scope_level = 0)
   - Creates tree: `p -> Struct { x: Primitive(Real), y: Primitive(Real), z: Primitive(Real) }`
5. **Inline `project` function**:
   - Return expression: `Point2D { x: p.x, y: p.y }`
   - Substitute parameter `p` with source path (3D point)
   - Generate constraints:
     - `p_2d.x == p_3d.x`
     - `p_2d.y == p_3d.y`
6. **Add constraint** `p.x == 10.0` (refers to 2D point)
7. **Solve**: Z3 finds `p_3d.x = 10.0`, `p_3d.y = <any>`, `p_3d.z = <any>`, `p_2d.x = 10.0`, `p_2d.y = <any>`

## Implementation Guide

### Phase 1: Core Infrastructure
1. Implement `VariablePath` and `PathComponent`
2. Implement `VariableNode` with tree operations
3. Implement `SolverContext` with basic variable management
4. Write tests for tree navigation and lookup

### Phase 2: Guards and Scopes
1. Implement `ScopeGuard`
2. Implement `WithGuard`
3. Add scope management to `SolverContext`
4. Write tests for scope push/pop

### Phase 3: Basic Solving
1. Implement `Solvable` for simple statements (`Let`, `Expression`)
2. Implement expression-to-Z3 conversion
3. Write end-to-end tests for simple constraint problems

### Phase 4: Container With-Statements
1. Implement container context handling in `WithGuard`
2. Add dot-prefix variable name resolution
3. Write tests for container namespacing

### Phase 5: Transform With-Statements (Most Complex)
1. Implement `create_transform_shadow`
2. Implement `inline_function_as_constraint`
3. Implement `eval_expr_with_substitution`
4. Write comprehensive tests for transforms

### Testing Strategy

**Unit Tests**:
- Path operations (construction, extension, to_z3_name)
- Tree navigation (get_at_path, get_at_path_mut)
- Scope management (push, pop, shadowing)

**Integration Tests**:
- Simple constraints (primitives, arithmetic)
- Struct constraints (field access, nested structs)
- Array constraints (indexing, array of structs)
- Container with-statements (dot-prefix syntax)
- Transform with-statements (coordinate transformations)

**End-to-End Tests**:
- Complete CAD-DSL programs with transforms
- Multi-level scope nesting
- Complex transform chains

## Future Extensions

### Function Calls
Function calls will use similar inlining mechanics:
- Inline function body with parameter substitution
- Generate constraints for return value
- Handle recursive calls with depth limits

### For Loops
For loops will be unrolled:
- Expand loop body N times
- Each iteration gets unique variables (scoped)
- Constraints from all iterations accumulated

### Optimization
Potential optimizations:
- **Lazy Z3 variable creation**: Only create Z3 variables when referenced in constraints
- **Constraint simplification**: Detect trivial constraints before sending to Z3
- **Incremental solving**: Use Z3's push/pop for faster repeated solving

## Summary

This architecture provides:
- ✅ **Zero-copy efficiency**: String allocation only when creating Z3 variables
- ✅ **Type-safe navigation**: Rust compiler enforces valid tree operations
- ✅ **Automatic cleanup**: RAII guards prevent scope leaks
- ✅ **Extensibility**: New HIR nodes implement `Solvable` trait
- ✅ **Transform support**: Automatic shadow variable creation with constraints
- ✅ **Clear separation**: HIR semantics vs. solver mechanics

The trait-based design makes adding new language features straightforward: implement `Solvable` for the new HIR node type, and the solver infrastructure handles the rest.
