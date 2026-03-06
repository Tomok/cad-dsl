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
use crate::hir::expr::ResolvedExprKind;
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
pub enum VariableNode<'src> {
    /// Primitive variable (leaf node with Z3 variable)
    Primitive {
        /// The Z3 variable
        z3_var: Z3Primitive,
    },

    /// Struct variable (branch node with named fields)
    Struct {
        /// Child nodes (struct fields)
        children: HashMap<&'src str, VariableNode<'src>>,
    },

    /// Array variable (branch node with indexed elements)
    Array {
        /// Child nodes (array elements)
        children: Vec<VariableNode<'src>>,
    },
}

impl<'src> VariableNode<'src> {
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
    /// May also have transforms if the container struct defines __transform__ methods.
    Container,

    /// Transform with-statement: coordinate transformations
    ///
    /// When variables are accessed in this context, they are automatically
    /// transformed using the appropriate __transform__ method.
    ///
    /// Note: Transform semantics are partially implemented. The context tracking
    /// infrastructure is in place, but the actual application of transforms to
    /// variable accesses is not yet complete.
    #[allow(dead_code)] // Infrastructure for future transform implementation
    Transform {
        /// Path to the transform context variable
        source_path: VariablePath<'src>,

        /// Scope level of the source
        source_scope: usize,

        /// Available transform methods for automatic type transformations
        transforms: Vec<crate::hir::TransformMethod<'src, 'arena>>,

        /// The context expression (for binding "self" in transform methods)
        context_expr: &'arena crate::hir::expr::ResolvedExpr<'src, 'arena>,
    },
}

// ============================================================================
// Rune Block Execution Tracking
// ============================================================================

/// Information about a rune block that needs to be executed after constraint solving
#[derive(Clone)]
pub struct RuneBlockExecution<'src, 'arena> {
    /// Path to the variable where the result should be stored
    pub result_path: VariablePath<'src>,

    /// The rune block parameters
    pub params: Vec<crate::hir::expr::ResolvedRuneParam<'src, 'arena>>,

    /// Pre-compiled Rune unit (compiled once, reused for execution)
    pub compiled_unit: std::sync::Arc<rune::Unit>,

    /// Return type of the rune block
    pub return_type: &'arena crate::hir::types::ResolvedType<'src, 'arena>,
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
    variables: HashMap<&'src str, VariableNode<'src>>,

    /// Alias tracking for reference types
    ///
    /// Maps reference variable paths to their target paths.
    /// When we have `let r = &x`, we record `alias_map[r] = x`.
    /// When looking up `r`, we follow the alias chain to get `x`'s Z3 variable.
    alias_map: HashMap<VariablePath<'src>, VariablePath<'src>>,

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

    /// Rune blocks that need to be executed after constraint solving
    rune_blocks: Vec<RuneBlockExecution<'src, 'arena>>,

    /// Current iteration number (for diagnostics)
    iteration: usize,

    /// Solution from the last Z3 solve (if any)
    current_solution: Option<Solution<'src>>,

    /// Number of variables with determined values in previous iteration
    /// (used to detect progress)
    previous_solved_count: usize,

    /// Storage for owned qualified name strings
    ///
    /// When we need to create a VariablePath from an identifier's qualified name,
    /// we need the string to outlive the function call. This Vec stores those strings
    /// and we return references to them.
    qualified_name_storage: Vec<String>,

    /// Global counter for uniquely naming scoped let variables
    ///
    /// Each `let` declaration inside a scoped block (for-loop body, if-branch, etc.)
    /// gets a unique suffix derived from this counter, preventing name collisions
    /// when the same variable name appears in multiple loops or branches.
    pub scoped_let_counter: usize,
}

impl<'src, 'arena> SolverContext<'src, 'arena> {
    /// Create a new solver context
    pub fn new(z3_ctx: z3::Context, z3_solver: z3::Solver, arena: &'arena bumpalo::Bump) -> Self {
        Self {
            z3_ctx,
            z3_solver,
            arena,
            variables: HashMap::new(),
            alias_map: HashMap::new(),
            scope_level: 0,
            with_stack: Vec::new(),
            function_return_exprs: HashMap::new(),
            deferred_constraints: Vec::new(),
            rune_blocks: Vec::new(),
            iteration: 0,
            current_solution: None,
            previous_solved_count: 0,
            qualified_name_storage: Vec::new(),
            scoped_let_counter: 0,
        }
    }

    /// Store a qualified name string and return a reference to it
    ///
    /// This is used when we need to create a VariablePath from an owned String
    /// (like from `identifier.to_qualified_name()`). The string is stored in the
    /// context and a reference with the appropriate lifetime is returned.
    fn store_qualified_name(&mut self, name: String) -> &'src str {
        self.qualified_name_storage.push(name);
        // SAFETY: We're converting the reference lifetime from the Vec's lifetime
        // to 'src. This is safe because:
        // 1. The Vec is never shrunk or reallocated during solving
        // 2. The SolverContext lives for the entire solving process
        // 3. Strings are only added, never removed
        // 4. The caller needs these strings to persist for solving
        unsafe {
            let last_idx = self.qualified_name_storage.len() - 1;
            let s: &str = &self.qualified_name_storage[last_idx];
            std::mem::transmute::<&str, &'src str>(s)
        }
    }

    /// Public version of store_qualified_name for use in stmt.rs
    ///
    /// This allows solver implementations to store qualified names from HIR identifiers.
    pub fn store_qualified_name_public(&mut self, name: String) -> &'src str {
        self.store_qualified_name(name)
    }

    /// Build a VariablePath from a VariableIdentifier by traversing its structure
    ///
    /// This correctly handles all identifier variants including TransformedView by
    /// building the path component-by-component instead of treating the qualified
    /// name as a single component.
    pub fn build_var_path_from_identifier(
        &mut self,
        identifier: &crate::hir::definitions::VariableIdentifier<'src, 'arena>,
    ) -> Result<VariablePath<'src>, crate::solver::SolverError> {
        use crate::hir::definitions::VariableIdentifier;
        use crate::solver::SolverError;

        match identifier {
            VariableIdentifier::Simple(name) => Ok(VariablePath::from_name(name)),

            VariableIdentifier::FieldAccess { base, field_name } => {
                let base_path = self.build_var_path_from_identifier(base)?;
                Ok(base_path.with_field(field_name))
            }

            VariableIdentifier::ContainerAccess {
                container_var,
                container_field,
                entity_name,
            } => {
                let container_path = self.build_var_path_from_identifier(container_var)?;
                Ok(container_path
                    .with_field(container_field.name)
                    .with_field(entity_name))
            }

            VariableIdentifier::ArrayIndex { array, index } => {
                let array_path = self.build_var_path_from_identifier(array)?;
                Ok(array_path.with_index(*index))
            }

            VariableIdentifier::TransformedView { container_var, .. } => {
                // For transformed views, build the container's path and add __view suffix
                let container_path = self.build_var_path_from_identifier(container_var)?;

                // The container path is like "t.entities.p", we need to replace the last
                // component "p" with "p__view"
                let components = container_path.components();
                if components.is_empty() {
                    return Err(SolverError::ContextError(
                        "TransformedView with empty container path".to_string(),
                    ));
                }

                // Extract the last component (entity name)
                let last_idx = components.len() - 1;
                let entity_name = match components[last_idx] {
                    crate::solver::PathComponent::Field(name) => name,
                    _ => {
                        return Err(SolverError::ContextError(
                            "TransformedView container path must end with field name".to_string(),
                        ));
                    }
                };

                // Build new path with all components except last, then add entity__view
                let mut view_path = VariablePath::from_name(match components[0] {
                    crate::solver::PathComponent::Field(name) => name,
                    _ => {
                        return Err(SolverError::ContextError(
                            "Path must start with field name".to_string(),
                        ));
                    }
                });

                for component in components.iter().take(last_idx).skip(1) {
                    match component {
                        crate::solver::PathComponent::Field(field) => {
                            view_path = view_path.with_field(field);
                        }
                        crate::solver::PathComponent::Index(idx) => {
                            view_path = view_path.with_index(*idx);
                        }
                    }
                }

                // Add the last component with __view suffix
                let view_entity_name = format!("{}__view", entity_name);
                let view_entity_name_ref = self.store_qualified_name_public(view_entity_name);
                Ok(view_path.with_field(view_entity_name_ref))
            }
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

    /// Find a matching transform method for the given type in the current context
    ///
    /// Returns the transform method if a matching one is found, or None if:
    /// - We're not in a transform context
    /// - No transform matches the given input type
    ///
    /// Note: This method is part of the transform infrastructure. It will be used
    /// when transform application is fully implemented.
    #[allow(dead_code)] // Infrastructure for future transform implementation
    pub fn find_transform_for_type(
        &self,
        input_type: &'arena crate::hir::types::ResolvedType<'src, 'arena>,
    ) -> Option<&crate::hir::TransformMethod<'src, 'arena>> {
        if let Some(WithContextInfo::Transform { transforms, .. }) = self.current_with_context() {
            // Find a transform whose input type matches the given type
            transforms.iter().find(|t| {
                // Type equality check - we need to match the input type
                t.input_type == input_type
            })
        } else {
            None
        }
    }

    /// Push a with-statement context onto the stack
    pub fn push_with_context(
        &mut self,
        with_context: &'arena crate::hir::WithContext<'src, 'arena>,
    ) {
        use crate::hir::expr::ResolvedExprKind;

        // Check if this is a container context
        if with_context.container_field.is_some() {
            // Extract the container variable from the context expression
            if let ResolvedExprKind::Var { .. } = &with_context.context_expr.kind {
                self.with_stack.push(WithContextInfo::Container);
                self.scope_level += 1;
            }
        } else if !with_context.transforms.is_empty() {
            // This is a transform-only context (no container)
            // Extract the source variable from the context expression
            if let ResolvedExprKind::Var { definition, .. } = &with_context.context_expr.kind {
                let qualified_name = definition.identifier.to_qualified_name();
                let name_ref = self.store_qualified_name(qualified_name);
                let info = WithContextInfo::Transform {
                    source_path: VariablePath::from_name(name_ref),
                    source_scope: definition.scope_level,
                    transforms: with_context.transforms.clone(),
                    context_expr: with_context.context_expr,
                };
                self.with_stack.push(info);
                self.scope_level += 1;
            }
        }
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
    ) -> Result<VariableNode<'src>, SolverError> {
        match typ {
            ResolvedType::I32 { .. } | ResolvedType::F64 { .. } | ResolvedType::Bool { .. } => {
                // Leaf node: create Z3 primitive
                #[cfg(feature = "solver-debug")]
                eprintln!("[SOLVER-DEBUG]     Creating Z3 variable: {}", path);

                let z3_var = self.create_z3_primitive(path, typ)?;
                Ok(VariableNode::Primitive { z3_var })
            }

            ResolvedType::UserDefined { definition, .. } => {
                // Branch node: recursively create children
                let mut children = HashMap::new();
                for field in &definition.fields {
                    let child_path = path.with_field(field.name);
                    let child_node = self.build_variable_tree(&child_path, &field.field_type)?;
                    children.insert(field.name, child_node);
                }

                // If the struct has a container field, create an empty struct node for it
                // This allows variables to be added to the container later
                if let Some(container_field) = definition.container_field {
                    children.insert(
                        container_field.name,
                        VariableNode::Struct {
                            children: HashMap::new(),
                        },
                    );
                }

                Ok(VariableNode::Struct { children })
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
                Ok(VariableNode::Array { children })
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
    // Alias Management
    // ========================================================================

    /// Register an alias mapping for reference types
    ///
    /// When we have `let r = &x`, we call `register_alias(r_path, x_path)`.
    /// This means that `r` is an alias to `x`, and they should share the same Z3 variable.
    pub fn register_alias(&mut self, alias: VariablePath<'src>, target: VariablePath<'src>) {
        self.alias_map.insert(alias, target);
    }

    /// Resolve an alias to its ultimate target path
    ///
    /// Follows the alias chain until we find a path that is not an alias.
    /// This handles transitive aliases (r1 -> r2 -> x).
    ///
    /// Returns the resolved path, or the original path if it's not an alias.
    pub fn resolve_alias(&self, path: &VariablePath<'src>) -> VariablePath<'src> {
        let mut current = path.clone();
        let mut visited = std::collections::HashSet::new();

        // Follow the alias chain
        while let Some(target) = self.alias_map.get(&current) {
            // Detect cycles in alias chain
            if !visited.insert(current.clone()) {
                // Cycle detected - return the original path
                // This shouldn't happen in a well-formed program
                return path.clone();
            }
            current = target.clone();
        }

        current
    }

    // ========================================================================
    // Variable Lookup
    // ========================================================================

    /// Lookup variable by path
    ///
    /// Automatically resolves aliases. If the path is an alias to another path,
    /// returns the variable at the target path.
    pub fn get_variable(&self, path: &VariablePath<'src>) -> Option<&VariableNode<'src>> {
        if path.is_empty() {
            return None;
        }

        // Resolve aliases first
        let resolved_path = self.resolve_alias(path);

        // Extract root name
        let root_name = match resolved_path.components().first()? {
            PathComponent::Field(name) => name,
            _ => return None, // Root must be a field name
        };

        // Navigate from root
        let root = self.variables.get(root_name)?;
        root.get_at_path(&resolved_path.components()[1..])
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

    /// Register a rune block for execution after constraint solving
    ///
    /// This compiles the rune code immediately and caches the compiled unit
    /// to avoid recompilation on every iteration or execution.
    pub fn register_rune_block(
        &mut self,
        result_path: VariablePath<'src>,
        params: Vec<crate::hir::expr::ResolvedRuneParam<'src, 'arena>>,
        body: &'src str,
        return_type: &'arena crate::hir::types::ResolvedType<'src, 'arena>,
    ) -> Result<(), SolverError> {
        use crate::solver::rune_executor::RuneExecutor;

        // Compile the rune code once and cache it
        let executor = RuneExecutor::new()?;
        let compiled_unit = executor.compile_rune_block(body, &params)?;

        self.rune_blocks.push(RuneBlockExecution {
            result_path,
            params,
            compiled_unit,
            return_type,
        });

        Ok(())
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
        // Skip shadow variables and view variables (internal transform implementation details)
        for (root_name, root_node) in &self.variables {
            if root_name.starts_with("__shadow_") || root_name.contains("__view") {
                continue;
            }
            let root_path = VariablePath::from_name(root_name);
            self.extract_node_values(&root_path, root_node, &model, &mut solution)?;
        }

        Ok(solution)
    }

    /// Recursively extract values from a variable node
    fn extract_node_values(
        &self,
        path: &VariablePath<'src>,
        node: &VariableNode<'src>,
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
                                // as_rational() failed - this could mean:
                                // 1. The value is a symbolic expression (under-constrained)
                                // 2. The value cannot be represented as a rational
                                // In either case, treat it as under-constrained
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

    /// Execute all registered rune blocks and add their results to the solution
    ///
    /// This is called after Z3 solving completes successfully. It:
    /// 1. Extracts parameter values from the Z3 solution
    /// 2. Executes each rune block with those values
    /// 3. Adds the results to the solution
    fn execute_rune_blocks(&mut self, solution: &mut Solution<'src>) -> Result<(), SolverError> {
        use crate::solver::rune_executor::RuneExecutor;

        if self.rune_blocks.is_empty() {
            return Ok(());
        }

        // Create rune executor
        let executor = RuneExecutor::new()?;

        // Execute each rune block
        // Clone the rune blocks to avoid borrowing issues
        let rune_blocks = self.rune_blocks.clone();

        for rune_block in &rune_blocks {
            // Extract parameter values from solution
            let mut param_values = Vec::new();

            for param in &rune_block.params {
                // Resolve the parameter expression to get its value
                // For Phase 4 MVP, parameters should be simple variables
                let value = match &param.value.kind {
                    ResolvedExprKind::Var { definition, .. } => {
                        // Build path from the variable's identifier using the same
                        // identifier-aware builder that declarations use, so container
                        // variables and transformed views resolve correctly
                        let path = self.build_var_path_from_identifier(definition.identifier)?;

                        solution.assignments.get(&path).cloned().ok_or_else(|| {
                            SolverError::RuneExecutionError(format!(
                                "Rune block parameter '{}' not found in solution",
                                path
                            ))
                        })?
                    }
                    _ => {
                        return Err(SolverError::RuneExecutionError(
                            "Complex parameter expressions in rune blocks not yet supported (Phase 4 MVP)".to_string(),
                        ));
                    }
                };

                param_values.push(value);
            }

            // Execute the rune block with pre-compiled unit (no recompilation)
            let result = executor.execute_compiled_block(
                rune_block.compiled_unit.clone(),
                &rune_block.params,
                param_values,
            )?;

            // Convert result back to solver value
            let solver_value = executor.convert_from_rune_value(result, rune_block.return_type)?;

            // Add result to solution
            solution
                .assignments
                .insert(rune_block.result_path.clone(), solver_value);
        }

        Ok(())
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
                    let mut solution = self.extract_solution()?;
                    let current_solved_count = solution.resolved_count();

                    // Check progress
                    let made_progress = current_solved_count > self.previous_solved_count;
                    self.previous_solved_count = current_solved_count;
                    self.current_solution = Some(solution.clone());

                    // Return if complete (no deferred constraints)
                    if self.deferred_constraints.is_empty() {
                        // Execute rune blocks with the solution values
                        self.execute_rune_blocks(&mut solution)?;

                        return Ok(SolveResult::Complete {
                            solution,
                            iterations: self.iteration,
                        });
                    }

                    // If we have deferred constraints but made no progress, stop
                    if !made_progress && deferred_before > 0 {
                        // Execute rune blocks even in partial solutions (if parameters are available)
                        // Errors here are acceptable - we're already in a partial state
                        let _ = self.execute_rune_blocks(&mut solution);

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
    use assert_matches::assert_matches;

    #[test]
    fn test_path_from_name() {
        let path = VariablePath::from_name("x");
        assert_eq!(path.to_z3_name(), "x");
    }

    #[test]
    fn test_path_with_field() {
        let path = VariablePath::from_name("p").with_field("x");
        assert_eq!(path.to_z3_name(), "p.x");
    }

    #[test]
    fn test_path_with_index() {
        let path = VariablePath::from_name("arr").with_index(0);
        assert_eq!(path.to_z3_name(), "arr[0]");
    }

    #[test]
    fn test_path_complex() {
        let path = VariablePath::from_name("points")
            .with_index(0)
            .with_field("x");
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

        let nested = path.with_field("field");
        assert_eq!(nested.to_z3_name(), "test.field");

        let indexed = nested.with_index(5);
        assert_eq!(indexed.to_z3_name(), "test.field[5]");
    }

    #[test]
    fn test_path_components() {
        let path = VariablePath::from_name("x").with_field("y").with_index(0);
        let components = path.components();

        assert_matches!(components[0], PathComponent::Field("x"));
        assert_matches!(components[1], PathComponent::Field("y"));
        assert_matches!(components[2], PathComponent::Index(0));
    }
}
