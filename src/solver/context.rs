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

use super::{PathComponent, SolverError, VariablePath};
use crate::hir::definitions::FieldDefinition;
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
        typ: &'arena ResolvedType<'src, 'arena>,

        /// The Z3 variable
        z3_var: Z3Primitive,

        /// Scope level where this variable was declared
        scope_level: usize,
    },

    /// Struct variable (branch node with named fields)
    Struct {
        /// The resolved type of this variable
        typ: &'arena ResolvedType<'src, 'arena>,

        /// Child nodes (struct fields)
        children: HashMap<&'src str, VariableNode<'src, 'arena>>,

        /// Scope level where this variable was declared
        scope_level: usize,
    },

    /// Array variable (branch node with indexed elements)
    Array {
        /// The resolved type of this variable
        typ: &'arena ResolvedType<'src, 'arena>,

        /// Child nodes (array elements)
        children: Vec<VariableNode<'src, 'arena>>,

        /// Scope level where this variable was declared
        scope_level: usize,
    },
}

impl<'src, 'arena> VariableNode<'src, 'arena> {
    /// Get the scope level of this node
    pub fn scope_level(&self) -> usize {
        match self {
            Self::Primitive { scope_level, .. } => *scope_level,
            Self::Struct { scope_level, .. } => *scope_level,
            Self::Array { scope_level, .. } => *scope_level,
        }
    }

    /// Get the type of this node
    pub fn typ(&self) -> &'arena ResolvedType<'src, 'arena> {
        match self {
            Self::Primitive { typ, .. } => typ,
            Self::Struct { typ, .. } => typ,
            Self::Array { typ, .. } => typ,
        }
    }

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

    /// Recursively collect all primitive leaves under this node
    ///
    /// Returns a vector of (path, z3_variable) pairs for all primitives
    /// reachable from this node.
    pub fn collect_primitives(
        &self,
        base_path: &VariablePath<'src>,
    ) -> Vec<(VariablePath<'src>, &Z3Primitive)> {
        match self {
            Self::Primitive { z3_var, .. } => {
                vec![(base_path.clone(), z3_var)]
            }
            Self::Struct { children, .. } => children
                .iter()
                .flat_map(|(field_name, child)| {
                    child.collect_primitives(&base_path.with_field(field_name))
                })
                .collect(),
            Self::Array { children, .. } => children
                .iter()
                .enumerate()
                .flat_map(|(idx, child)| child.collect_primitives(&base_path.with_index(idx)))
                .collect(),
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
        container_field: &'arena FieldDefinition<'src, 'arena>,
    },

    /// Transform with-statement: coordinate transformations (Phase 3+)
    ///
    /// Variables get shadow variables linked by transform constraints.
    #[allow(dead_code)] // Will be used in Phase 3+
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

    /// Root variable tree (maps root variable names to their trees)
    variables: HashMap<&'src str, VariableNode<'src, 'arena>>,

    /// Current scope depth (incremented on scope entry)
    scope_level: usize,

    /// Stack of active with-statement contexts
    with_stack: Vec<WithContextInfo<'src, 'arena>>,
}

impl<'src, 'arena> SolverContext<'src, 'arena> {
    /// Create a new solver context
    pub fn new(z3_ctx: z3::Context, z3_solver: z3::Solver) -> Self {
        Self {
            z3_ctx,
            z3_solver,
            variables: HashMap::new(),
            scope_level: 0,
            with_stack: Vec::new(),
        }
    }

    /// Get current scope level
    pub fn scope_level(&self) -> usize {
        self.scope_level
    }

    /// Get current with-statement context (if any)
    pub fn current_with_context(&self) -> Option<&WithContextInfo<'src, 'arena>> {
        self.with_stack.last()
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

    /// Mutable variable lookup
    pub fn get_variable_mut(
        &mut self,
        path: &VariablePath<'src>,
    ) -> Option<&mut VariableNode<'src, 'arena>> {
        if path.is_empty() {
            return None;
        }

        let root_name = match path.components().first()? {
            PathComponent::Field(name) => name,
            _ => return None,
        };

        let root = self.variables.get_mut(root_name)?;
        root.get_at_path_mut(&path.components()[1..])
    }

    // ========================================================================
    // Scope Management
    // ========================================================================

    /// Remove all variables from current scope level
    ///
    /// Called automatically by scope guards when they drop.
    fn pop_scope(&mut self) {
        self.variables
            .retain(|_, node| node.scope_level() < self.scope_level);
        self.scope_level = self.scope_level.saturating_sub(1);
    }
}

// ============================================================================
// RAII Scope Guards
// ============================================================================

/// General scope guard
///
/// Automatically increments scope level on creation and pops scope on drop.
/// This ensures scopes are always cleaned up properly using RAII.
pub struct ScopeGuard<'a, 'src, 'arena> {
    ctx: &'a mut SolverContext<'src, 'arena>,
    active: bool,
}

impl<'a, 'src, 'arena> ScopeGuard<'a, 'src, 'arena> {
    /// Create a new scope guard, incrementing the scope level
    pub fn new(ctx: &'a mut SolverContext<'src, 'arena>) -> Self {
        ctx.scope_level += 1;
        ScopeGuard { ctx, active: true }
    }

    /// Get mutable access to the context
    pub fn context(&mut self) -> &mut SolverContext<'src, 'arena> {
        self.ctx
    }

    /// Manually disable the guard if needed (advanced usage)
    #[allow(dead_code)]
    pub fn disarm(mut self) {
        self.active = false;
    }
}

impl Drop for ScopeGuard<'_, '_, '_> {
    fn drop(&mut self) {
        if self.active {
            self.ctx.pop_scope();
        }
    }
}

/// With-statement guard
///
/// Handles both container and transform with-statement contexts.
/// Automatically pushes with-context on creation and pops on drop.
pub struct WithGuard<'a, 'src, 'arena> {
    ctx: &'a mut SolverContext<'src, 'arena>,
    active: bool,
}

impl<'a, 'src, 'arena> WithGuard<'a, 'src, 'arena> {
    /// Create a new with-statement guard
    pub fn new(
        ctx: &'a mut SolverContext<'src, 'arena>,
        with_info: WithContextInfo<'src, 'arena>,
    ) -> Self {
        ctx.with_stack.push(with_info);
        ctx.scope_level += 1;
        WithGuard { ctx, active: true }
    }

    /// Get mutable access to the context
    pub fn context(&mut self) -> &mut SolverContext<'src, 'arena> {
        self.ctx
    }
}

impl Drop for WithGuard<'_, '_, '_> {
    fn drop(&mut self) {
        if self.active {
            self.ctx.pop_scope();
            self.ctx.with_stack.pop();
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
