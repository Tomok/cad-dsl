//! Symbol table and scope management for the High-level Intermediate Representation (HIR).
//!
//! This module provides the infrastructure for tracking variables, scopes, and context
//! information during semantic analysis of CAD-DSL programs. It uses arena allocation
//! for efficient memory management and safe cross-references.

#![allow(dead_code)]
// Public API for scope management in future phases
// Allow dead code for now since this module is not yet fully integrated
//!
//! # Scope Management Overview
//!
//! The scope system implements lexical scoping with support for:
//! - **Nested scopes**: Blocks, functions, and with-statements create new scopes
//! - **Variable shadowing**: Inner scopes can shadow variables from outer scopes
//! - **With-contexts**: Special scoping for transform and container contexts
//! - **Forward references**: CAD-DSL allows referencing variables before declaration
//!
//! # Lexical Scoping Rules
//!
//! CAD-DSL follows standard lexical scoping rules:
//!
//! 1. **Variables are visible from declaration point to end of scope**
//!    ```cad
//!    {
//!        // x not visible here
//!        let x = 10;
//!        // x visible here
//!    }
//!    // x not visible here
//!    ```
//!
//! 2. **Inner scopes can access outer scope variables**
//!    ```cad
//!    let x = 10;
//!    {
//!        let y = x + 5;  // Can access x from outer scope
//!    }
//!    ```
//!
//! 3. **Inner scopes can shadow outer scope variables**
//!    ```cad
//!    let x = 10;
//!    {
//!        let x = 20;     // Shadows outer x
//!        // x is 20 here
//!    }
//!    // x is 10 here
//!    ```
//!
//! # Variable Shadowing
//!
//! When a variable is declared with the same name as a variable in an outer scope,
//! the inner variable shadows (hides) the outer one within its scope:
//!
//! ```cad
//! let x = 1;              // Scope level 0
//! fn foo() {              // Scope level 1
//!     let x = 2;          // Shadows level 0's x
//!     {                   // Scope level 2
//!         let x = 3;      // Shadows level 1's x
//!         // x is 3 here
//!     }
//!     // x is 2 here
//! }
//! // x is 1 here
//! ```
//!
//! The scope stack searches from innermost to outermost scope, so the most recently
//! declared variable with a given name is found first.
//!
//! # With-Context Scoping
//!
//! With-statements create special scopes that affect name resolution in two ways:
//!
//! ## 1. Container Field Access with Dot-Prefix
//!
//! Inside a with-statement with a container context, the dot-prefix syntax (`.field`)
//! provides shorthand access to container fields:
//!
//! ```cad
//! with sketch {
//!     let .p1 = point(0, 0);      // Equivalent to: let sketch.p1 = point(0, 0);
//!     let .p2 = point(10, 10);    // Equivalent to: let sketch.p2 = point(10, 10);
//! }
//! ```
//!
//! The scope stack tracks the current with-context and resolves dot-prefix names
//! to the container expression.
//!
//! ## 2. Transform Contexts
//!
//! Transform contexts apply implicit transformations to coordinates:
//!
//! ```cad
//! with transform1 {
//!     with transform2 {
//!         // Coordinates are implicitly transformed through both transforms
//!         point(10, 20)  // Passed through transform1 then transform2
//!     }
//! }
//! ```
//!
//! The scope stack maintains a chain of with-contexts to enable proper transform
//! composition.
//!
//! # Forward References
//!
//! **IMPORTANT**: CAD-DSL allows forward references within the same scope for certain
//! declarations (particularly in constraint contexts):
//!
//! ```cad
//! {
//!     constraint distance(p1, p2) == 10;  // References p1 and p2 before declaration
//!     let p1 = point(0, 0);
//!     let p2 = point(10, 0);
//! }
//! ```
//!
//! To support forward references:
//!
//! 1. **Two-pass resolution**: The semantic analyzer makes multiple passes:
//!    - First pass: Collect all declarations in a scope
//!    - Second pass: Resolve expressions and type-check
//!
//! 2. **Uninitialized variables**: Variables can be declared without initialization
//!    and initialized later (or left uninitialized for solver-assigned values)
//!
//! 3. **Constraint ordering**: Constraints are collected and solved as a system,
//!    not evaluated sequentially
//!
//! The scope stack supports forward references by allowing variable lookup before
//! the variable's initialization expression is processed.
//!
//! # Arena Allocation
//!
//! All scope-related data uses arena allocation (`bumpalo::Bump`):
//! - Variable definitions are allocated in the arena
//! - With-contexts are allocated in the arena
//! - References use arena lifetimes for safety
//!
//! This design enables:
//! - Fast allocation (pointer bumping)
//! - Safe cross-references without reference counting
//! - Automatic cleanup when analysis completes

use super::context::WithContext;
use super::definitions::VarDefinition;
use std::collections::HashMap;

// ============================================================================
// Scope
// ============================================================================

/// A single scope level in the scope stack.
///
/// A scope represents a lexical scope boundary (function body, block, with-statement, etc.)
/// and tracks:
/// - Variables declared in this scope
/// - The current with-context (if inside a with-statement)
/// - The nesting level for debugging and error messages
///
/// # Lifetimes
///
/// - `'src`: Lifetime of the source text (for variable names)
/// - `'arena`: Lifetime of the arena allocator (for variable definitions and contexts)
#[derive(Debug)]
pub struct Scope<'src, 'arena> {
    /// Variables declared in this scope.
    ///
    /// Maps variable names to their definitions. Only variables declared directly
    /// in this scope are stored here; variables from outer scopes are not included.
    ///
    /// When looking up a variable, we search scopes from innermost to outermost,
    /// so the first match found is the correct one (implementing shadowing).
    pub variables: HashMap<&'src str, &'arena VarDefinition<'src, 'arena>>,

    /// Optional with-context active in this scope.
    ///
    /// When a with-statement is entered, a new scope is created with a with-context.
    /// This context affects name resolution for:
    /// - Dot-prefix field access (`.fieldname`)
    /// - Implicit coordinate transformations
    ///
    /// The with-context is `None` for regular scopes (functions, blocks) and `Some`
    /// for scopes created by with-statements.
    pub with_context: Option<&'arena WithContext<'src, 'arena>>,

    /// The nesting level of this scope.
    ///
    /// Scope levels start at 0 (global scope) and increase as we enter nested scopes:
    /// - 0: Global/module scope
    /// - 1: Function bodies, top-level blocks
    /// - 2+: Nested blocks, with-statements, etc.
    ///
    /// This is used for:
    /// - Debugging and error messages
    /// - Tracking variable shadowing
    /// - Determining scope boundaries
    pub scope_level: usize,
}

impl<'src, 'arena> Scope<'src, 'arena> {
    /// Creates a new scope at the given nesting level.
    ///
    /// # Parameters
    ///
    /// - `scope_level`: The nesting level for this scope (0 for global, 1+ for nested)
    pub fn new(scope_level: usize) -> Self {
        Self {
            variables: HashMap::new(),
            with_context: None,
            scope_level,
        }
    }

    /// Creates a new scope with a with-context.
    ///
    /// This is used when entering a with-statement to create a scope that tracks
    /// the context expression and enables special name resolution rules.
    ///
    /// # Parameters
    ///
    /// - `scope_level`: The nesting level for this scope
    /// - `with_context`: The with-context information for this scope
    pub fn new_with_context(
        scope_level: usize,
        with_context: &'arena WithContext<'src, 'arena>,
    ) -> Self {
        Self {
            variables: HashMap::new(),
            with_context: Some(with_context),
            scope_level,
        }
    }

    /// Declares a new variable in this scope.
    ///
    /// If a variable with the same name already exists in this scope, returns
    /// `Some(&old_definition)` for error reporting. If the variable is new,
    /// returns `None`.
    ///
    /// # Parameters
    ///
    /// - `name`: The variable name
    /// - `definition`: The variable definition (allocated in the arena)
    ///
    /// # Returns
    ///
    /// - `None` if the variable is new in this scope
    /// - `Some(&old_definition)` if a variable with this name already exists in this scope
    pub fn declare_variable(
        &mut self,
        name: &'src str,
        definition: &'arena VarDefinition<'src, 'arena>,
    ) -> Option<&'arena VarDefinition<'src, 'arena>> {
        self.variables.insert(name, definition)
    }

    /// Looks up a variable by name in this scope only.
    ///
    /// Does not search outer scopes; use `ScopeStack::lookup_variable()` for
    /// full lexical scope lookup.
    ///
    /// # Parameters
    ///
    /// - `name`: The variable name to look up
    ///
    /// # Returns
    ///
    /// - `Some(&definition)` if the variable is declared in this scope
    /// - `None` if the variable is not in this scope
    pub fn lookup_variable(&self, name: &str) -> Option<&'arena VarDefinition<'src, 'arena>> {
        self.variables.get(name).copied()
    }

    /// Returns the number of variables declared in this scope.
    pub fn variable_count(&self) -> usize {
        self.variables.len()
    }

    /// Returns true if this scope has a with-context.
    pub fn has_with_context(&self) -> bool {
        self.with_context.is_some()
    }

    /// Returns true if this is a container context (with-context with a container field).
    pub fn is_container_context(&self) -> bool {
        self.with_context
            .map(|ctx| ctx.is_container_context())
            .unwrap_or(false)
    }

    /// Returns true if this is a transform context (with-context with transforms).
    pub fn is_transform_context(&self) -> bool {
        self.with_context
            .map(|ctx| ctx.is_transform_context())
            .unwrap_or(false)
    }
}

// ============================================================================
// ScopeStack
// ============================================================================

/// A stack of scopes for lexical scope management.
///
/// The scope stack tracks the current nesting of scopes during semantic analysis.
/// As we enter new scopes (functions, blocks, with-statements), we push a new scope
/// onto the stack. As we exit scopes, we pop from the stack.
///
/// # Lexical Lookup
///
/// When looking up a variable, we search from the innermost scope (top of stack)
/// to the outermost scope (bottom of stack). The first match found is returned,
/// implementing proper shadowing semantics.
///
/// # With-Context Tracking
///
/// The stack maintains the current with-context by searching for the innermost
/// scope that has a with-context. This enables proper resolution of:
/// - Dot-prefix field access (`.field`)
/// - Nested transform contexts
///
/// # Lifetimes
///
/// - `'src`: Lifetime of the source text (for variable names)
/// - `'arena`: Lifetime of the arena allocator (for definitions and contexts)
#[derive(Debug)]
pub struct ScopeStack<'src, 'arena> {
    /// The stack of scopes, from outermost (index 0) to innermost (last index).
    ///
    /// The global scope is typically at index 0, and nested scopes are pushed onto the end.
    /// We maintain the invariant that this stack is never empty during analysis
    /// (there is always at least a global scope).
    scopes: Vec<Scope<'src, 'arena>>,
}

impl<'src, 'arena> ScopeStack<'src, 'arena> {
    /// Creates a new scope stack with a global scope at level 0.
    ///
    /// The stack always starts with at least one scope (the global scope).
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope::new(0)],
        }
    }

    /// Pushes a new scope onto the stack.
    ///
    /// The new scope's level is one greater than the current scope's level.
    /// After pushing, the new scope becomes the current scope.
    pub fn push_scope(&mut self) {
        let new_level = self.current_scope_level() + 1;
        self.scopes.push(Scope::new(new_level));
    }

    /// Pops the current scope from the stack.
    ///
    /// # Panics
    ///
    /// Panics if there is only one scope (the global scope) remaining, as we must
    /// maintain the invariant that the stack is never empty.
    ///
    /// # Returns
    ///
    /// The popped scope, which can be used for cleanup or error reporting.
    pub fn pop_scope(&mut self) -> Scope<'src, 'arena> {
        assert!(
            self.scopes.len() > 1,
            "Cannot pop the global scope; scope stack must never be empty"
        );
        self.scopes
            .pop()
            .expect("Checked that scopes has more than 1 element")
    }

    /// Enters a with-context by pushing a new scope with the given context.
    ///
    /// This is used when entering a with-statement. The new scope will have a
    /// with-context attached, enabling special name resolution rules.
    ///
    /// # Parameters
    ///
    /// - `with_context`: The with-context information for this scope
    pub fn enter_with_context(&mut self, with_context: &'arena WithContext<'src, 'arena>) {
        let new_level = self.current_scope_level() + 1;
        self.scopes
            .push(Scope::new_with_context(new_level, with_context));
    }

    /// Exits the current with-context by popping the scope.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - There is only the global scope remaining
    /// - The current scope does not have a with-context
    ///
    /// # Returns
    ///
    /// The popped scope with its with-context.
    pub fn exit_with_context(&mut self) -> Scope<'src, 'arena> {
        assert!(
            self.scopes.len() > 1,
            "Cannot pop the global scope; scope stack must never be empty"
        );
        let scope = self.scopes.pop().expect("Checked scopes is not empty");
        assert!(
            scope.has_with_context(),
            "exit_with_context called but current scope has no with-context"
        );
        scope
    }

    /// Looks up a variable by name, searching from innermost to outermost scope.
    ///
    /// This implements proper lexical scoping with shadowing: the first variable
    /// found (starting from the innermost scope) is returned.
    ///
    /// # Parameters
    ///
    /// - `name`: The variable name to look up
    ///
    /// # Returns
    ///
    /// - `Some(&definition)` if the variable is found in any scope
    /// - `None` if the variable is not declared in any scope
    pub fn lookup_variable(&self, name: &str) -> Option<&'arena VarDefinition<'src, 'arena>> {
        // Search from innermost (end of vec) to outermost (start of vec)
        for scope in self.scopes.iter().rev() {
            if let Some(var_def) = scope.lookup_variable(name) {
                return Some(var_def);
            }
        }
        None
    }

    /// Declares a new variable in the current scope.
    ///
    /// If a variable with the same name already exists in the current scope
    /// (but not in outer scopes), returns `Some(&old_definition)` for error reporting.
    ///
    /// Note: Variables in outer scopes can be shadowed; this only returns an error
    /// if the variable is redeclared in the *same* scope.
    ///
    /// # Parameters
    ///
    /// - `name`: The variable name
    /// - `definition`: The variable definition (allocated in the arena)
    ///
    /// # Returns
    ///
    /// - `None` if the variable is successfully declared (new in current scope)
    /// - `Some(&old_definition)` if a variable with this name already exists in the current scope
    pub fn declare_variable(
        &mut self,
        name: &'src str,
        definition: &'arena VarDefinition<'src, 'arena>,
    ) -> Option<&'arena VarDefinition<'src, 'arena>> {
        self.current_scope_mut().declare_variable(name, definition)
    }

    /// Returns the innermost with-context, if any.
    ///
    /// Searches from the innermost scope to the outermost scope and returns the
    /// first with-context found. This is used for:
    /// - Resolving dot-prefix field access (`.field`)
    /// - Determining the current transform chain
    ///
    /// # Returns
    ///
    /// - `Some(&with_context)` if we are inside a with-statement
    /// - `None` if there is no active with-context
    pub fn current_with_context(&self) -> Option<&'arena WithContext<'src, 'arena>> {
        // Search from innermost (end of vec) to outermost (start of vec)
        for scope in self.scopes.iter().rev() {
            if let Some(ctx) = scope.with_context {
                return Some(ctx);
            }
        }
        None
    }

    /// Returns the current scope level (nesting depth).
    ///
    /// The global scope is level 0, and nested scopes have increasing levels.
    ///
    /// # Returns
    ///
    /// The scope level of the current (innermost) scope.
    pub fn current_scope_level(&self) -> usize {
        self.current_scope().scope_level
    }

    /// Returns the current (innermost) scope.
    ///
    /// # Panics
    ///
    /// Panics if the scope stack is empty (should never happen as we maintain
    /// the invariant that at least the global scope exists).
    fn current_scope(&self) -> &Scope<'src, 'arena> {
        self.scopes
            .last()
            .expect("Scope stack should never be empty")
    }

    /// Returns a mutable reference to the current (innermost) scope.
    ///
    /// # Panics
    ///
    /// Panics if the scope stack is empty (should never happen as we maintain
    /// the invariant that at least the global scope exists).
    fn current_scope_mut(&mut self) -> &mut Scope<'src, 'arena> {
        self.scopes
            .last_mut()
            .expect("Scope stack should never be empty")
    }

    /// Returns the total number of scopes in the stack.
    ///
    /// This is primarily useful for debugging and testing. The count will always
    /// be at least 1 (the global scope).
    pub fn scope_count(&self) -> usize {
        self.scopes.len()
    }

    /// Returns true if we are currently inside a with-context.
    pub fn in_with_context(&self) -> bool {
        self.current_with_context().is_some()
    }

    /// Returns true if the current with-context (if any) is a container context.
    ///
    /// This is used to determine if dot-prefix syntax is allowed.
    pub fn in_container_context(&self) -> bool {
        self.current_with_context()
            .map(|ctx| ctx.is_container_context())
            .unwrap_or(false)
    }

    /// Returns true if the current with-context (if any) is a transform context.
    pub fn in_transform_context(&self) -> bool {
        self.current_with_context()
            .map(|ctx| ctx.is_transform_context())
            .unwrap_or(false)
    }

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
    ///
    /// # Returns
    ///
    /// A vector of with-context references in order from outermost (first) to innermost (last)
    pub fn all_with_contexts(&self) -> Vec<&'arena WithContext<'src, 'arena>> {
        self.scopes
            .iter()
            .filter_map(|scope| scope.with_context)
            .collect()
    }
}

impl<'src, 'arena> Default for ScopeStack<'src, 'arena> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::{LineColumn, Span};
    use bumpalo::Bump;

    /// Helper to create a dummy span for testing
    fn dummy_span() -> Span {
        Span {
            start: LineColumn { line: 1, column: 1 },
            lines: 0,
            end_column: 1,
        }
    }

    #[test]
    fn test_scope_new() {
        let scope: Scope = Scope::new(0);
        assert_eq!(scope.scope_level, 0);
        assert_eq!(scope.variable_count(), 0);
        assert!(!scope.has_with_context());
    }

    #[test]
    fn test_scope_declare_and_lookup() {
        let arena = Bump::new();
        let mut scope = Scope::new(0);

        let var_def = arena.alloc(VarDefinition::new(
            "x",
            dummy_span(),
            None,
            crate::hir::definitions::VarDefinitionKind::Uninitialized,
            0,
            dummy_span(),
        ));

        // First declaration should succeed
        assert!(scope.declare_variable("x", var_def).is_none());
        assert_eq!(scope.variable_count(), 1);

        // Lookup should find the variable
        let found = scope.lookup_variable("x");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "x");

        // Redeclaring in same scope should return the old definition
        let var_def2 = arena.alloc(VarDefinition::new(
            "x",
            dummy_span(),
            None,
            crate::hir::definitions::VarDefinitionKind::Uninitialized,
            0,
            dummy_span(),
        ));
        let old = scope.declare_variable("x", var_def2);
        assert!(old.is_some());
        assert!(std::ptr::eq(old.unwrap(), var_def));

        // Still only 1 variable (replaced)
        assert_eq!(scope.variable_count(), 1);
    }

    #[test]
    fn test_scope_stack_new() {
        let stack: ScopeStack = ScopeStack::new();
        assert_eq!(stack.scope_count(), 1);
        assert_eq!(stack.current_scope_level(), 0);
        assert!(!stack.in_with_context());
    }

    #[test]
    fn test_scope_stack_push_pop() {
        let mut stack = ScopeStack::new();
        assert_eq!(stack.current_scope_level(), 0);

        stack.push_scope();
        assert_eq!(stack.current_scope_level(), 1);
        assert_eq!(stack.scope_count(), 2);

        stack.push_scope();
        assert_eq!(stack.current_scope_level(), 2);
        assert_eq!(stack.scope_count(), 3);

        let scope = stack.pop_scope();
        assert_eq!(scope.scope_level, 2);
        assert_eq!(stack.current_scope_level(), 1);

        let scope = stack.pop_scope();
        assert_eq!(scope.scope_level, 1);
        assert_eq!(stack.current_scope_level(), 0);
    }

    #[test]
    #[should_panic(expected = "Cannot pop the global scope")]
    fn test_scope_stack_cannot_pop_global() {
        let mut stack = ScopeStack::new();
        stack.pop_scope(); // Should panic
    }

    #[test]
    fn test_scope_stack_variable_lookup_with_shadowing() {
        let arena = Bump::new();
        let mut stack = ScopeStack::new();

        // Declare x in global scope
        let x_global = arena.alloc(VarDefinition::new(
            "x",
            dummy_span(),
            None,
            crate::hir::definitions::VarDefinitionKind::Uninitialized,
            0,
            dummy_span(),
        ));
        stack.declare_variable("x", x_global);

        // Lookup should find global x
        let found = stack.lookup_variable("x");
        assert!(found.is_some());
        assert!(std::ptr::eq(found.unwrap(), x_global));

        // Enter new scope
        stack.push_scope();

        // Declare x in nested scope (shadows global x)
        let x_nested = arena.alloc(VarDefinition::new(
            "x",
            dummy_span(),
            None,
            crate::hir::definitions::VarDefinitionKind::Uninitialized,
            1,
            dummy_span(),
        ));
        stack.declare_variable("x", x_nested);

        // Lookup should find nested x (shadowing global x)
        let found = stack.lookup_variable("x");
        assert!(found.is_some());
        assert!(std::ptr::eq(found.unwrap(), x_nested));

        // Pop scope
        stack.pop_scope();

        // Lookup should find global x again
        let found = stack.lookup_variable("x");
        assert!(found.is_some());
        assert!(std::ptr::eq(found.unwrap(), x_global));
    }

    #[test]
    fn test_scope_stack_declare_same_scope_error() {
        let arena = Bump::new();
        let mut stack = ScopeStack::new();

        let var1 = arena.alloc(VarDefinition::new(
            "x",
            dummy_span(),
            None,
            crate::hir::definitions::VarDefinitionKind::Uninitialized,
            0,
            dummy_span(),
        ));

        // First declaration succeeds
        assert!(stack.declare_variable("x", var1).is_none());

        let var2 = arena.alloc(VarDefinition::new(
            "x",
            dummy_span(),
            None,
            crate::hir::definitions::VarDefinitionKind::Uninitialized,
            0,
            dummy_span(),
        ));

        // Second declaration in same scope returns error
        let old = stack.declare_variable("x", var2);
        assert!(old.is_some());
        assert!(std::ptr::eq(old.unwrap(), var1));
    }

    #[test]
    fn test_scope_stack_with_context() {
        let arena = Bump::new();
        let mut stack = ScopeStack::new();

        // Initially not in with-context
        assert!(!stack.in_with_context());
        assert!(stack.current_with_context().is_none());

        // Create a dummy type for the expression
        let int_type = arena.alloc(crate::hir::types::ResolvedType::I32 { span: dummy_span() });

        // Create a dummy with-context (transform context)
        let ctx = arena.alloc(WithContext {
            context_expr: arena.alloc(crate::hir::expr::ResolvedExpr {
                span: dummy_span(),
                kind: crate::hir::expr::ResolvedExprKind::IntLit { value: 42 },
                ty: int_type,
            }),
            container_field: None,
            transforms: vec![],
        });

        // Enter with-context
        stack.enter_with_context(ctx);
        assert!(stack.in_with_context());
        assert_eq!(stack.scope_count(), 2);
        assert_eq!(stack.current_scope_level(), 1);

        let found_ctx = stack.current_with_context();
        assert!(found_ctx.is_some());
        assert!(std::ptr::eq(found_ctx.unwrap(), ctx));

        // Exit with-context
        let popped = stack.exit_with_context();
        assert!(popped.has_with_context());
        assert!(!stack.in_with_context());
        assert_eq!(stack.scope_count(), 1);
    }

    #[test]
    #[should_panic(expected = "exit_with_context called but current scope has no with-context")]
    fn test_scope_stack_exit_without_context() {
        let mut stack = ScopeStack::new();
        stack.push_scope();
        stack.exit_with_context(); // Should panic - no with-context in current scope
    }

    #[test]
    fn test_scope_stack_nested_with_contexts() {
        let arena = Bump::new();
        let mut stack = ScopeStack::new();

        // Create a dummy type for the expressions
        let int_type = arena.alloc(crate::hir::types::ResolvedType::I32 { span: dummy_span() });

        // Create first with-context
        let ctx1 = arena.alloc(WithContext {
            context_expr: arena.alloc(crate::hir::expr::ResolvedExpr {
                span: dummy_span(),
                kind: crate::hir::expr::ResolvedExprKind::IntLit { value: 1 },
                ty: int_type,
            }),
            container_field: None,
            transforms: vec![],
        });

        stack.enter_with_context(ctx1);
        assert_eq!(stack.current_scope_level(), 1);

        // Create nested with-context
        let ctx2 = arena.alloc(WithContext {
            context_expr: arena.alloc(crate::hir::expr::ResolvedExpr {
                span: dummy_span(),
                kind: crate::hir::expr::ResolvedExprKind::IntLit { value: 2 },
                ty: int_type,
            }),
            container_field: None,
            transforms: vec![],
        });

        stack.enter_with_context(ctx2);
        assert_eq!(stack.current_scope_level(), 2);

        // Current with-context should be the innermost (ctx2)
        let found = stack.current_with_context();
        assert!(found.is_some());
        assert!(std::ptr::eq(found.unwrap(), ctx2));

        // Exit innermost with-context
        stack.exit_with_context();
        assert_eq!(stack.current_scope_level(), 1);

        // Current with-context should now be ctx1
        let found = stack.current_with_context();
        assert!(found.is_some());
        assert!(std::ptr::eq(found.unwrap(), ctx1));

        // Exit outer with-context
        stack.exit_with_context();
        assert_eq!(stack.current_scope_level(), 0);
        assert!(!stack.in_with_context());
    }
}
