//! Solver Context with Tree-Based Variable Management
//!
//! This module implements the core solver context that manages:
//! - Tree-structured variable storage
//! - Z3 integration
//! - Scope management with RAII guards
//! - With-statement context handling
//!
//! # Architecture
//!
//! Variables are stored as a tree structure that mirrors the type hierarchy:
//! - Primitive types (i32, f64, bool) are leaf nodes with Z3 variables
//! - Struct types are branch nodes with named children
//! - Array types are branch nodes with indexed children
//!
//! This structure enables zero-copy navigation using `&'src str` references,
//! with string allocation only when creating Z3 variables.

use super::PartialReason;
use super::{
    DeferredConstraint, PathComponent, Solution, SolveResult, SolverError, Value, VariablePath,
};
use crate::hir::types::ResolvedType;
use std::collections::HashMap;

// ============================================================================
// Z3 Primitive Types
// ============================================================================

/// Z3 primitive types (leaves in the variable tree)
#[derive(Debug, Clone)]
pub enum Z3Primitive {
    /// Z3 integer variable
    Int(z3::ast::Int),

    /// Z3 real (floating-point) variable
    Real(z3::ast::Real),

    /// Z3 boolean variable
    Bool(z3::ast::Bool),
}

// ============================================================================
// Variable Tree Structure
// ============================================================================

/// Node in the variable tree
///
/// Variables are organized in a tree that mirrors the type structure.
/// Primitive types are leaves with Z3 variables, while composite types
/// (structs and arrays) are branches with children.
#[derive(Debug)]
pub enum VariableNode<'src, 'arena> {
    /// Primitive variable (leaf node with Z3 variable)
    Primitive {
        /// The resolved type of this variable
        #[allow(dead_code)]
        typ: &'arena ResolvedType<'src, 'arena>,

        /// The Z3 variable
        z3_var: Z3Primitive,

        /// Scope level where this variable was declared
        #[allow(dead_code)]
        scope_level: usize,
    },

    /// Struct variable (branch node with named fields)
    Struct {
        /// The resolved type of this variable
        #[allow(dead_code)]
        typ: &'arena ResolvedType<'src, 'arena>,

        /// Child nodes (struct fields)
        children: HashMap<&'src str, VariableNode<'src, 'arena>>,

        /// Scope level where this variable was declared
        #[allow(dead_code)]
        scope_level: usize,
    },

    /// Array variable (branch node with indexed elements)
    Array {
        /// The resolved type of this variable
        #[allow(dead_code)]
        typ: &'arena ResolvedType<'src, 'arena>,

        /// Child nodes (array elements)
        children: Vec<VariableNode<'src, 'arena>>,

        /// Scope level where this variable was declared
        #[allow(dead_code)]
        scope_level: usize,
    },
}

impl<'src, 'arena> VariableNode<'src, 'arena> {

    /// Navigate to descendant node by path
    ///
    /// Returns `None` if the path doesn't exist or is invalid for this node type.
    pub fn get_at_path(&self, path: &[PathComponent<'src>]) -> Option<&Self> {
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
            _ => None, // Invalid path component for this node type
        }
    }

    /// Mutable navigation to descendant node by path
    pub fn get_at_path_mut(&mut self, path: &[PathComponent<'src>]) -> Option<&mut Self> {
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
    pub fn as_primitive(&self) -> Option<&Z3Primitive> {
        match self {
            Self::Primitive { z3_var, .. } => Some(z3_var),
            _ => None,
        }
    }

}

// ============================================================================
// With-Statement Context
// ============================================================================

/// Context information for with-statements
///
/// Tracks the type of with-statement and necessary information for
/// variable resolution within the with-block.
#[derive(Debug, Clone)]
pub enum WithContextInfo<'src, 'arena> {
    /// Container with-statement: `with container { .field }`
    ///
    /// Variables declared with dot-prefix are placed in the container field.
    Container {
        /// Path to the container variable
        container_path: VariablePath<'src>,

        /// The container field definition
        container_field: &'arena crate::hir::definitions::ContainerField<'src, 'arena>,
    },

    /// Transform with-statement: coordinate transformations
    ///
    /// Variables get shadow variables linked by transform constraints.
    /// Not yet implemented - planned feature for coordinate system transformations.
    #[allow(dead_code)] // Reserved for future transform implementation
    Transform {
        /// Path to the source variable
        source_path: VariablePath<'src>,

        /// Scope level of the source (for shadow variables)
        source_scope: usize,
    },
}

// ============================================================================
// Solver Context
// ============================================================================

/// Main solver context
///
/// Manages the variable tree, Z3 integration, scopes, and with-statement contexts.
pub struct SolverContext<'src, 'arena> {
    /// Z3 context (persistent across scopes)
    pub z3_ctx: z3::Context,

    /// Z3 solver (persistent, constraints accumulate)
    pub z3_solver: z3::Solver,

    /// Arena allocator for creating new HIR nodes during function inlining
    pub arena: &'arena bumpalo::Bump,

    /// Root variable tree (maps root variable names to their trees)
    variables: HashMap<&'src str, VariableNode<'src, 'arena>>,

    /// Current scope depth (incremented on scope entry)
    scope_level: usize,

    /// Stack of active with-statement contexts
    with_stack: Vec<WithContextInfo<'src, 'arena>>,

    // Function inlining support
    /// Map from function name to its return expression
    /// Populated during the first pass over statements for correct scoping
    function_return_exprs: HashMap<&'src str, &'arena crate::hir::expr::ResolvedExpr<'src, 'arena>>,

    // Iterative solving fields
    /// Constraints that have been deferred for later resolution
    deferred_constraints: Vec<DeferredConstraint<'src>>,

    /// Current iteration number (for diagnostics)
    iteration: usize,

    /// Solution from the last Z3 solve (if any)
    current_solution: Option<Solution<'src>>,

    /// Number of variables with determined values in previous iteration
    /// (used to detect progress)
    previous_solved_count: usize,
}

impl<'src, 'arena> SolverContext<'src, 'arena> {
    /// Create a new solver context
    pub fn new(z3_ctx: z3::Context, z3_solver: z3::Solver, arena: &'arena bumpalo::Bump) -> Self {
        Self {
            z3_ctx,
            z3_solver,
            arena,
            variables: HashMap::new(),
            scope_level: 0,
            with_stack: Vec::new(),
            function_return_exprs: HashMap::new(),
            deferred_constraints: Vec::new(),
            iteration: 0,
            current_solution: None,
            previous_solved_count: 0,
        }
    }

    /// Register a function's return expression for inlining
    pub fn register_function_return(
        &mut self,
        function_name: &'src str,
        return_expr: &'arena crate::hir::expr::ResolvedExpr<'src, 'arena>,
    ) {
        self.function_return_exprs
            .insert(function_name, return_expr);
    }

    /// Get a function's return expression
    pub fn get_function_return(
        &self,
        function_name: &str,
    ) -> Option<&'arena crate::hir::expr::ResolvedExpr<'src, 'arena>> {
        self.function_return_exprs.get(function_name).copied()
    }

    /// Get current with-statement context (if any)
    pub fn current_with_context(&self) -> Option<&WithContextInfo<'src, 'arena>> {
        self.with_stack.last()
    }

    /// Push a with-statement context onto the stack
    pub fn push_with_context(
        &mut self,
        with_context: &'arena crate::hir::WithContext<'src, 'arena>,
    ) {
        use crate::hir::expr::ResolvedExprKind;

        // Check if this is a container context
        if let Some(container_field) = with_context.container_field {
            // Extract the container variable from the context expression
            if let ResolvedExprKind::Var { definition, .. } = &with_context.context_expr.kind {
                let info = WithContextInfo::Container {
                    container_path: VariablePath::from_name(definition.name),
                    container_field,
                };
                self.with_stack.push(info);
                self.scope_level += 1;
            }
        }
        // Transform contexts are not yet implemented - just ignore them
    }

    /// Pop a with-statement context from the stack
    pub fn pop_with_context(&mut self) {
        self.with_stack.pop();
        if self.scope_level > 0 {
            self.scope_level -= 1;
        }
    }

    // ========================================================================
    // Variable Declaration and Management
    // ========================================================================

    /// Declare a new variable (builds entire tree for composite types)
    ///
    /// This is the main entry point for variable declarations.
    /// For composite types (structs, arrays), it recursively builds
    /// the entire tree structure.
    pub fn declare_variable(
        &mut self,
        name: &'src str,
        typ: &'arena ResolvedType<'src, 'arena>,
    ) -> Result<(), SolverError> {
        let base_path = VariablePath::from_name(name);
        let node = self.build_variable_tree(&base_path, typ)?;
        self.variables.insert(name, node);
        Ok(())
    }

    /// Declare a new variable at a specific path
    ///
    /// This is used for dot-prefix variables in with-statements,
    /// where variables are declared under a container path.
    /// The path should be the full path including the variable name.
    pub fn declare_variable_at_path(
        &mut self,
        path: &VariablePath<'src>,
        typ: &'arena ResolvedType<'src, 'arena>,
    ) -> Result<(), SolverError> {
        // Build the variable tree for this type
        let node = self.build_variable_tree(path, typ)?;

        // If path has only one component, it's a root variable
        if path.components().len() == 1 {
            let root_name = match path.components().first() {
                Some(PathComponent::Field(name)) => name,
                _ => {
                    return Err(SolverError::ContextError(
                        "Invalid path for variable declaration".to_string(),
                    ));
                }
            };
            self.variables.insert(root_name, node);
            return Ok(());
        }

        // Otherwise, we need to insert it as a child of its parent
        // Extract parent path and field name
        let components = path.components();
        let parent_components = &components[..components.len() - 1];
        let field_name = match components.last() {
            Some(PathComponent::Field(name)) => name,
            _ => {
                return Err(SolverError::ContextError(
                    "Last component must be a field name".to_string(),
                ));
            }
        };

        // Get root name for parent lookup
        let root_name = match parent_components.first() {
            Some(PathComponent::Field(name)) => name,
            _ => {
                return Err(SolverError::ContextError(
                    "Parent path must start with a field".to_string(),
                ));
            }
        };

        // Navigate to parent and insert child
        let root = self
            .variables
            .get_mut(root_name)
            .ok_or_else(|| SolverError::UndefinedVariable(root_name.to_string()))?;

        let parent_node = root
            .get_at_path_mut(&parent_components[1..])
            .ok_or_else(|| {
                SolverError::UndefinedVariable(
                    "Parent path not found for variable declaration".to_string(),
                )
            })?;

        // Insert as child
        match parent_node {
            VariableNode::Struct { children, .. } => {
                children.insert(field_name, node);
                Ok(())
            }
            _ => Err(SolverError::ContextError(
                "Cannot add field to non-struct variable".to_string(),
            )),
        }
    }

    /// Recursively build variable tree from type
    ///
    /// This is where the magic happens: we create a tree structure
    /// that mirrors the type hierarchy, creating Z3 variables only
    /// for primitive types (leaves).
    fn build_variable_tree(
        &self,
        path: &VariablePath<'src>,
        typ: &'arena ResolvedType<'src, 'arena>,
    ) -> Result<VariableNode<'src, 'arena>, SolverError> {
        match typ {
            ResolvedType::I32 { .. } | ResolvedType::F64 { .. } | ResolvedType::Bool { .. } => {
                // Leaf node: create Z3 primitive
                let z3_var = self.create_z3_primitive(path, typ)?;
                Ok(VariableNode::Primitive {
                    typ,
                    z3_var,
                    scope_level: self.scope_level,
                })
            }

            ResolvedType::UserDefined { definition, .. } => {
                // Branch node: recursively create children
                let mut children = HashMap::new();
                for field in &definition.fields {
                    let child_path = path.with_field(field.name);
                    let child_node = self.build_variable_tree(&child_path, &field.field_type)?;
                    children.insert(field.name, child_node);
                }
                Ok(VariableNode::Struct {
                    typ,
                    children,
                    scope_level: self.scope_level,
                })
            }

            ResolvedType::Array {
                element_type, size, ..
            } => {
                // Branch node: create indexed children
                let mut children = Vec::with_capacity(*size);
                for i in 0..*size {
                    let child_path = path.with_index(i);
                    let child_node = self.build_variable_tree(&child_path, element_type)?;
                    children.push(child_node);
                }
                Ok(VariableNode::Array {
                    typ,
                    children,
                    scope_level: self.scope_level,
                })
            }

            ResolvedType::Reference { inner, .. } => {
                // References are transparent for variable creation
                self.build_variable_tree(path, inner)
            }

            _ => Err(SolverError::UnsupportedType(format!("{:?}", typ))),
        }
    }

    /// Create Z3 primitive variable
    ///
    /// STRING ALLOCATION HAPPENS HERE! This is the only place where
    /// we convert paths to strings for Z3 variable names.
    fn create_z3_primitive(
        &self,
        path: &VariablePath<'src>,
        typ: &ResolvedType<'src, 'arena>,
    ) -> Result<Z3Primitive, SolverError> {
        let name = path.to_z3_name(); // Only string allocation!
        Ok(match typ {
            ResolvedType::I32 { .. } => Z3Primitive::Int(z3::ast::Int::new_const(name)),
            ResolvedType::F64 { .. } => Z3Primitive::Real(z3::ast::Real::new_const(name)),
            ResolvedType::Bool { .. } => Z3Primitive::Bool(z3::ast::Bool::new_const(name)),
            _ => return Err(SolverError::NotAPrimitiveType),
        })
    }

    // ========================================================================
    // Variable Lookup
    // ========================================================================

    /// Lookup variable by path
    pub fn get_variable(&self, path: &VariablePath<'src>) -> Option<&VariableNode<'src, 'arena>> {
        if path.is_empty() {
            return None;
        }

        // Extract root name
        let root_name = match path.components().first()? {
            PathComponent::Field(name) => name,
            _ => return None, // Root must be a field name
        };

        // Navigate from root
        let root = self.variables.get(root_name)?;
        root.get_at_path(&path.components()[1..])
    }

    // ========================================================================
    // Deferral Management
    // ========================================================================

    /// Defer a constraint for later resolution
    pub fn defer_constraint(&mut self, dependencies: Vec<&'src str>, description: String) {
        self.deferred_constraints.push(DeferredConstraint {
            dependencies,
            description,
        });
    }

    /// Check if a variable has a known value in the current solution
    #[cfg(test)]
    pub fn is_variable_known(&self, var: &str) -> bool {
        if let Some(solution) = &self.current_solution {
            let path = VariablePath::from_name(var);
            solution.assignments.contains_key(&path)
        } else {
            false
        }
    }

    /// Get the value of a variable from the current solution
    ///
    /// Note: This method is only usable for variables that match the 'src lifetime.
    /// Future extension: handle qualified paths (e.g., "p.x", "points[0].y").
    #[allow(dead_code)] // Reserved for future use
    pub fn get_variable_value(&self, var: &'src str) -> Option<&Value> {
        let path = VariablePath::from_name(var);
        self.current_solution
            .as_ref()
            .and_then(|sol| sol.assignments.get(&path))
    }

    /// Get a reference to the current solution
    ///
    /// Returns the solution from the last successful Z3 solve, if any.
    pub fn get_current_solution(&self) -> Option<&Solution<'src>> {
        self.current_solution.as_ref()
    }

    // ========================================================================
    // Solution Extraction
    // ========================================================================

    /// Extract solution from Z3 model
    ///
    /// Walks the variable tree and evaluates each primitive variable
    /// in the Z3 model to build a complete solution.
    pub fn extract_solution(&self) -> Result<Solution<'src>, SolverError> {
        // Get the Z3 model (only available after SAT result)
        let model = self
            .z3_solver
            .get_model()
            .ok_or_else(|| SolverError::ModelEvaluationError("No model available".to_string()))?;

        let mut solution = Solution::new();

        // Walk through all variables and collect primitive values
        for (root_name, root_node) in &self.variables {
            let root_path = VariablePath::from_name(root_name);
            self.extract_node_values(&root_path, root_node, &model, &mut solution)?;
        }

        Ok(solution)
    }

    /// Recursively extract values from a variable node
    fn extract_node_values(
        &self,
        path: &VariablePath<'src>,
        node: &VariableNode<'src, 'arena>,
        model: &z3::Model,
        solution: &mut Solution<'src>,
    ) -> Result<(), SolverError> {
        match node {
            VariableNode::Primitive { z3_var, .. } => {
                // Evaluate primitive variable in model
                let value = self.evaluate_z3_primitive(z3_var, model)?;
                solution.assignments.insert(path.clone(), value);
            }
            VariableNode::Struct { children, .. } => {
                // Recursively extract struct fields
                for (field_name, child) in children {
                    let child_path = path.with_field(field_name);
                    self.extract_node_values(&child_path, child, model, solution)?;
                }
            }
            VariableNode::Array { children, .. } => {
                // Recursively extract array elements
                for (idx, child) in children.iter().enumerate() {
                    let child_path = path.with_index(idx);
                    self.extract_node_values(&child_path, child, model, solution)?;
                }
            }
        }
        Ok(())
    }

    /// Evaluate a Z3 primitive variable in the model
    fn evaluate_z3_primitive(
        &self,
        z3_var: &Z3Primitive,
        model: &z3::Model,
    ) -> Result<Value, SolverError> {
        match z3_var {
            Z3Primitive::Int(z3_int) => {
                // Use false to avoid model completion - only get values that are actually constrained
                match model.eval(z3_int, false) {
                    Some(evaluated) => {
                        // Try to convert to concrete value
                        match evaluated.as_i64() {
                            Some(value) => Ok(Value::Int(value)),
                            None => {
                                // as_i64() failed - this could mean either:
                                // 1. It's a symbolic expression (under-constrained)
                                // 2. It's a concrete integer that doesn't fit in i64 (overflow)
                                // In either case, treat it as under-constrained
                                // This allows the solver to return partial results for unconstrained variables
                                Ok(Value::UnderConstrained)
                            }
                        }
                    }
                    None => {
                        // Variable is not constrained - return UnderConstrained
                        Ok(Value::UnderConstrained)
                    }
                }
            }
            Z3Primitive::Real(z3_real) => {
                // Use false to avoid model completion - only get values that are actually constrained
                match model.eval(z3_real, false) {
                    Some(evaluated) => {
                        // Z3 Real values are represented as rationals
                        // Convert to f64
                        match evaluated.as_rational() {
                            Some((num, den)) => {
                                if den == 0 {
                                    // Division by zero in rational representation
                                    Err(SolverError::ModelEvaluationError(format!(
                                        "Real value has invalid rational representation (division by zero): {}",
                                        evaluated
                                    )))
                                } else {
                                    // Convert to f64 (handles both positive and negative denominators)
                                    let value = num as f64 / den as f64;
                                    Ok(Value::Real(value))
                                }
                            }
                            None => {
                                // as_rational() failed - this means the value exists but
                                // cannot be represented as a rational or converted to f64
                                Err(SolverError::ModelEvaluationError(format!(
                                    "Real variable has value that cannot be converted to f64: {}",
                                    evaluated
                                )))
                            }
                        }
                    }
                    None => {
                        // Variable is not constrained - return UnderConstrained
                        Ok(Value::UnderConstrained)
                    }
                }
            }
            Z3Primitive::Bool(z3_bool) => {
                // Use false to avoid model completion - only get values that are actually constrained
                match model.eval(z3_bool, false) {
                    Some(evaluated) => {
                        match evaluated.as_bool() {
                            Some(value) => Ok(Value::Bool(value)),
                            None => {
                                // as_bool() failed - this means the value exists but
                                // cannot be converted to bool (shouldn't happen normally)
                                Err(SolverError::ModelEvaluationError(format!(
                                    "Boolean variable has unexpected value: {}",
                                    evaluated
                                )))
                            }
                        }
                    }
                    None => {
                        // Variable is not constrained - return UnderConstrained
                        Ok(Value::UnderConstrained)
                    }
                }
            }
        }
    }

    // ========================================================================
    // Iterative Solve Loop
    // ========================================================================

    /// Main solve function with iterative partial solving
    ///
    /// Takes a list of HIR statements and attempts to solve them iteratively.
    /// Returns either a Complete solution (all constraints resolved) or a Partial
    /// solution (some constraints deferred due to unknown dependencies).
    ///
    /// The solve loop continues until either:
    /// 1. All constraints are resolved (Complete)
    /// 2. No progress is made between iterations (Partial - NoProgress)
    /// 3. Z3 returns UNSAT (error - no solution exists)
    pub fn solve(
        &mut self,
        statements: &[&'arena crate::hir::expr::ResolvedStmt<'src, 'arena>],
    ) -> Result<SolveResult<'src>, SolverError> {
        use crate::hir::expr::ResolvedStmtKind;
        use crate::solver::Solvable;

        // Pre-pass to register all function return expressions for correct scoping
        for stmt in statements {
            match &stmt.kind {
                // Register standalone functions
                ResolvedStmtKind::FunctionDef {
                    func_def,
                    body,
                    return_expr,
                    ..
                } => {
                    // Try implicit return first
                    if let Some(ret_expr) = return_expr {
                        self.register_function_return(func_def.name, ret_expr);
                    } else if body.len() == 1 {
                        // Try extracting from explicit return statement
                        if let ResolvedStmtKind::Return {
                            value: Some(ret_expr),
                            ..
                        } = &body[0].kind
                        {
                            self.register_function_return(func_def.name, ret_expr);
                        }
                    }
                }

                // Register methods from struct definitions
                ResolvedStmtKind::StructDef { methods, .. } => {
                    for method_stmt in methods {
                        if let ResolvedStmtKind::FunctionDef {
                            func_def,
                            body,
                            return_expr,
                            ..
                        } = &method_stmt.kind
                        {
                            // For methods, use qualified name (StructName::method_name)
                            let qualified_name = if let Some(parent) = func_def.parent_struct {
                                format!("{}::{}", parent.name, func_def.name)
                            } else {
                                func_def.name.to_string()
                            };

                            // Leak the string to get a 'src lifetime
                            // This is safe because we're storing it for the lifetime of the solver context
                            let qualified_name_leaked: &'src str =
                                Box::leak(qualified_name.into_boxed_str());

                            // Try implicit return first
                            if let Some(ret_expr) = return_expr {
                                self.function_return_exprs
                                    .insert(qualified_name_leaked, ret_expr);
                            } else if body.len() == 1 {
                                // Try extracting from explicit return statement
                                if let ResolvedStmtKind::Return {
                                    value: Some(ret_expr),
                                    ..
                                } = &body[0].kind
                                {
                                    self.function_return_exprs
                                        .insert(qualified_name_leaked, ret_expr);
                                }
                            }
                        }
                    }
                }

                _ => {}
            }
        }

        const MAX_ITERATIONS: usize = 100; // Prevent infinite loops

        loop {
            self.iteration += 1;

            if self.iteration > MAX_ITERATIONS {
                return Err(SolverError::ContextError(format!(
                    "Maximum iterations ({}) exceeded",
                    MAX_ITERATIONS
                )));
            }

            // Clear deferred constraints for this iteration
            let deferred_before = self.deferred_constraints.len();
            self.deferred_constraints.clear();

            // Process all statements
            for stmt in statements {
                stmt.solve(self)?;
            }

            // Run Z3 solver
            match self.z3_solver.check() {
                z3::SatResult::Sat => {
                    let solution = self.extract_solution()?;
                    let current_solved_count = solution.resolved_count();

                    // Check progress
                    let made_progress = current_solved_count > self.previous_solved_count;
                    self.previous_solved_count = current_solved_count;
                    self.current_solution = Some(solution.clone());

                    // Return if complete (no deferred constraints)
                    if self.deferred_constraints.is_empty() {
                        return Ok(SolveResult::Complete {
                            solution,
                            iterations: self.iteration,
                        });
                    }

                    // If we have deferred constraints but made no progress, stop
                    if !made_progress && deferred_before > 0 {
                        return Ok(SolveResult::Partial {
                            solution,
                            deferred: self.deferred_constraints.clone(),
                            reason: PartialReason::NoProgress {
                                stuck_constraints: self
                                    .deferred_constraints
                                    .iter()
                                    .map(|dc| dc.description.clone())
                                    .collect(),
                            },
                            iterations: self.iteration,
                        });
                    }

                    // Made progress - continue to next iteration
                    continue;
                }
                z3::SatResult::Unsat => return Err(SolverError::Unsatisfiable),
                z3::SatResult::Unknown => return Err(SolverError::Unknown),
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::{LineColumn, Span};
    use assert_matches::assert_matches;

    fn test_span() -> Span {
        Span {
            start: LineColumn { line: 1, column: 1 },
            lines: 0,
            end_column: 10,
        }
    }

    #[test]
    fn test_path_from_name() {
        let path = VariablePath::from_name("x");
        assert_eq!(path.len(), 1);
        assert_eq!(path.to_z3_name(), "x");
    }

    #[test]
    fn test_path_with_field() {
        let path = VariablePath::from_name("p").with_field("x");
        assert_eq!(path.len(), 2);
        assert_eq!(path.to_z3_name(), "p.x");
    }

    #[test]
    fn test_path_with_index() {
        let path = VariablePath::from_name("arr").with_index(0);
        assert_eq!(path.len(), 2);
        assert_eq!(path.to_z3_name(), "arr[0]");
    }

    #[test]
    fn test_path_complex() {
        let path = VariablePath::from_name("points")
            .with_index(0)
            .with_field("x");
        assert_eq!(path.len(), 3);
        assert_eq!(path.to_z3_name(), "points[0].x");
    }

    // NOTE: Full integration tests with Z3 context creation will be in integration tests.
    // The z3 crate uses internal APIs that are not suitable for unit tests here.
    // These unit tests focus on the structure and logic that doesn't require Z3.

    #[test]
    fn test_variable_path_operations() {
        // Test path construction and manipulation
        let path = VariablePath::from_name("test");
        assert_eq!(path.to_z3_name(), "test");
        assert!(!path.is_empty());
        assert_eq!(path.len(), 1);

        let nested = path.with_field("field");
        assert_eq!(nested.to_z3_name(), "test.field");
        assert_eq!(nested.len(), 2);

        let indexed = nested.with_index(5);
        assert_eq!(indexed.to_z3_name(), "test.field[5]");
        assert_eq!(indexed.len(), 3);
    }

    #[test]
    fn test_path_components() {
        let path = VariablePath::from_name("x").with_field("y").with_index(0);
        let components = path.components();

        assert_eq!(components.len(), 3);
        assert_matches!(components[0], PathComponent::Field("x"));
        assert_matches!(components[1], PathComponent::Field("y"));
        assert_matches!(components[2], PathComponent::Index(0));
    }
}
