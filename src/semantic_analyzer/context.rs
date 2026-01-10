//! Semantic analyzer context for managing analysis state
//!
//! This module provides the `AnalyzerContext` struct, which tracks the state
//! during semantic analysis of CAD-DSL programs. It manages:
//! - Arena allocation for HIR nodes
//! - Lexical scope tracking
//! - Symbol tables for structs and functions
//! - Error collection

// Allow dead code for now since this module is not yet fully integrated

use super::errors::SemanticError;
use crate::hir::definitions::{FunctionDefinition, StructDefinition};
use crate::hir::scope::ScopeStack;
use bumpalo::Bump;
use std::collections::HashMap;

// ============================================================================
// AnalyzerContext
// ============================================================================

/// Context for semantic analysis
///
/// The analyzer context maintains all the state needed during semantic analysis:
/// - Arena allocator for HIR nodes
/// - Source code reference for string slices
/// - Scope stack for variable tracking
/// - Symbol tables for structs and functions
/// - Error collection
///
/// # Lifetimes
///
/// - `'src`: Lifetime of the source text (for string slices)
/// - `'arena`: Lifetime of the arena allocator (for HIR node references)
///
/// # Example
///
/// ```ignore
/// let arena = Bump::new();
/// let source = "struct Point { x: f64, y: f64 }";
/// let mut ctx = AnalyzerContext::new(&arena, source);
///
/// // Register a struct definition
/// let struct_def = /* ... */;
/// ctx.register_struct("Point", struct_def)?;
///
/// // Look up the struct
/// let found = ctx.lookup_struct("Point");
/// assert!(found.is_some());
/// ```
#[derive(Debug)]
pub struct AnalyzerContext<'src, 'arena> {
    /// Arena allocator for HIR nodes
    ///
    /// All HIR nodes (expressions, types, definitions) are allocated in this arena
    /// for efficient memory management and safe cross-references.
    pub arena: &'arena Bump,

    /// Source code for string slices
    ///
    /// The original source text, used to create string slices for identifiers,
    /// literals, and other tokens. These slices have the `'src` lifetime.
    pub source: &'src str,

    /// Lexical scope management
    ///
    /// Tracks the current scope stack for variable declarations and lookups.
    /// Handles nested scopes, with-contexts, and variable shadowing.
    pub scope_stack: ScopeStack<'src, 'arena>,

    /// Struct symbol table
    ///
    /// Maps struct names to their definitions. Used for type resolution and
    /// struct member access validation.
    pub struct_definitions: HashMap<&'src str, &'arena StructDefinition<'src, 'arena>>,

    /// Function symbol table
    ///
    /// Maps function names to their definitions. Used for function call
    /// resolution and type checking.
    pub function_definitions: HashMap<&'src str, &'arena FunctionDefinition<'src, 'arena>>,

    /// Collected semantic errors
    ///
    /// All errors encountered during semantic analysis are collected here.
    /// This allows analysis to continue and report multiple errors at once.
    pub errors: Vec<SemanticError>,

    /// Resolved method bodies
    ///
    /// Maps function definitions (by address) to their resolved bodies and return expressions.
    /// Used when collecting transform methods to access the resolved HIR bodies.
    /// The key is a raw pointer to allow using function definition addresses as identifiers.
    pub resolved_method_bodies: HashMap<
        *const FunctionDefinition<'src, 'arena>,
        (
            Vec<&'arena ResolvedStmt<'src, 'arena>>,
            Option<&'arena ResolvedExpr<'src, 'arena>>,
        ),
    >,
}

impl<'src, 'arena> AnalyzerContext<'src, 'arena> {
    /// Create a new analyzer context
    ///
    /// Initializes the context with:
    /// - Reference to the arena allocator
    /// - Reference to the source code
    /// - Empty scope stack (with global scope)
    /// - Empty symbol tables
    /// - No errors
    ///
    /// # Parameters
    ///
    /// - `arena`: Arena allocator for HIR nodes
    /// - `source`: Source code text
    ///
    /// # Returns
    ///
    /// A new analyzer context ready for semantic analysis
    pub fn new(arena: &'arena Bump, source: &'src str) -> Self {
        Self {
            arena,
            source,
            scope_stack: ScopeStack::new(),
            struct_definitions: HashMap::new(),
            function_definitions: HashMap::new(),
            errors: Vec::new(),
            resolved_method_bodies: HashMap::new(),
        }
    }

    /// Register a struct definition
    ///
    /// Adds a struct to the symbol table. If a struct with the same name
    /// already exists, returns an error.
    ///
    /// # Parameters
    ///
    /// - `name`: Struct name (must be from the source text)
    /// - `def`: Struct definition (allocated in the arena)
    ///
    /// # Returns
    ///
    /// - `Ok(())` if the struct was registered successfully
    /// - `Err(())` if a struct with this name already exists
    ///
    /// # Example
    ///
    /// ```ignore
    /// let struct_def = arena.alloc(StructDefinition::new(/* ... */));
    /// match ctx.register_struct("Point", struct_def) {
    ///     Ok(()) => println!("Registered struct Point"),
    ///     Err(()) => println!("Struct Point already defined"),
    /// }
    /// ```
    pub fn register_struct(
        &mut self,
        name: &'src str,
        def: &'arena StructDefinition<'src, 'arena>,
    ) -> Result<(), ()> {
        if self.struct_definitions.contains_key(name) {
            Err(())
        } else {
            self.struct_definitions.insert(name, def);
            Ok(())
        }
    }

    /// Look up a struct by name
    ///
    /// Searches the struct symbol table for a struct with the given name.
    ///
    /// # Parameters
    ///
    /// - `name`: Struct name to look up
    ///
    /// # Returns
    ///
    /// - `Some(&def)` if the struct is found
    /// - `None` if no struct with this name exists
    ///
    /// # Example
    ///
    /// ```ignore
    /// if let Some(struct_def) = ctx.lookup_struct("Point") {
    ///     println!("Found struct Point with {} fields", struct_def.field_count());
    /// }
    /// ```
    pub fn lookup_struct(&self, name: &str) -> Option<&'arena StructDefinition<'src, 'arena>> {
        self.struct_definitions.get(name).copied()
    }

    /// Register a function definition
    ///
    /// Adds a function to the symbol table. If a function with the same name
    /// already exists, returns an error.
    ///
    /// # Parameters
    ///
    /// - `name`: Function name (must be from the source text)
    /// - `def`: Function definition (allocated in the arena)
    ///
    /// # Returns
    ///
    /// - `Ok(())` if the function was registered successfully
    /// - `Err(())` if a function with this name already exists
    ///
    /// # Example
    ///
    /// ```ignore
    /// let func_def = arena.alloc(FunctionDefinition::new(/* ... */));
    /// match ctx.register_function("distance", func_def) {
    ///     Ok(()) => println!("Registered function distance"),
    ///     Err(()) => println!("Function distance already defined"),
    /// }
    /// ```
    pub fn register_function(
        &mut self,
        name: &'src str,
        def: &'arena FunctionDefinition<'src, 'arena>,
    ) -> Result<(), ()> {
        if self.function_definitions.contains_key(name) {
            Err(())
        } else {
            self.function_definitions.insert(name, def);
            Ok(())
        }
    }

    /// Look up a function by name
    ///
    /// Searches the function symbol table for a function with the given name.
    ///
    /// # Parameters
    ///
    /// - `name`: Function name to look up
    ///
    /// # Returns
    ///
    /// - `Some(&def)` if the function is found
    /// - `None` if no function with this name exists
    ///
    /// # Example
    ///
    /// ```ignore
    /// if let Some(func_def) = ctx.lookup_function("distance") {
    ///     println!("Found function distance with {} parameters", func_def.param_count());
    /// }
    /// ```
    pub fn lookup_function(&self, name: &str) -> Option<&'arena FunctionDefinition<'src, 'arena>> {
        self.function_definitions.get(name).copied()
    }

    /// Add an error to the error collection
    ///
    /// Semantic errors are collected during analysis so that multiple errors
    /// can be reported at once, rather than failing on the first error.
    ///
    /// # Parameters
    ///
    /// - `error`: The semantic error to add
    ///
    /// # Example
    ///
    /// ```ignore
    /// ctx.add_error(SemanticError::UndefinedVariable {
    ///     name: "x".to_string(),
    ///     span: var_span,
    /// });
    /// ```
    pub fn add_error(&mut self, error: SemanticError) {
        self.errors.push(error);
    }

    /// Check if any errors have been collected
    ///
    /// # Returns
    ///
    /// `true` if there are any errors, `false` otherwise
    ///
    /// # Example
    ///
    /// ```ignore
    /// if ctx.has_errors() {
    ///     println!("Semantic analysis failed with {} errors", ctx.errors.len());
    /// }
    /// ```
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Extract all collected errors
    ///
    /// Removes and returns all errors from the context, leaving it empty.
    /// This is useful for reporting errors after analysis completes.
    ///
    /// # Returns
    ///
    /// A vector of all collected semantic errors
    ///
    /// # Example
    ///
    /// ```ignore
    /// let errors = ctx.take_errors();
    /// for error in errors {
    ///     println!("Error: {}", error);
    /// }
    /// ```
    pub fn take_errors(&mut self) -> Vec<SemanticError> {
        std::mem::take(&mut self.errors)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::definitions::{FieldDefinition, FunctionParam};
    use crate::hir::types::ResolvedType;
    use crate::lexer::{LineColumn, Span};

    /// Helper to create a dummy span for testing
    fn dummy_span() -> Span {
        Span {
            start: LineColumn { line: 1, column: 1 },
            lines: 0,
            end_column: 1,
        }
    }

    /// Helper to create a dummy type for testing
    fn dummy_type<'src, 'arena>() -> ResolvedType<'src, 'arena> {
        ResolvedType::I32 { span: dummy_span() }
    }

    #[test]
    fn test_context_creation() {
        let arena = Bump::new();
        let source = "struct Point { x: f64, y: f64 }";
        let ctx = AnalyzerContext::new(&arena, source);

        assert_eq!(ctx.source, source);
        assert_eq!(ctx.struct_definitions.len(), 0);
        assert_eq!(ctx.function_definitions.len(), 0);
        assert_eq!(ctx.errors.len(), 0);
        assert!(!ctx.has_errors());
        assert_eq!(ctx.scope_stack.scope_count(), 1);
    }

    #[test]
    fn test_struct_registration_and_lookup() {
        let arena = Bump::new();
        let source = "struct Point {}";
        let mut ctx = AnalyzerContext::new(&arena, source);

        // Create a simple struct definition
        let struct_def = arena.alloc(StructDefinition::new(
            "Point",
            dummy_span(),
            vec![],
            vec![],
            None,
            dummy_span(),
        ));

        // Register should succeed
        assert!(ctx.register_struct("Point", struct_def).is_ok());
        assert_eq!(ctx.struct_definitions.len(), 1);

        // Lookup should find the struct
        let found = ctx.lookup_struct("Point");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Point");

        // Lookup non-existent struct should return None
        assert!(ctx.lookup_struct("Circle").is_none());
    }

    #[test]
    fn test_struct_duplicate_detection() {
        let arena = Bump::new();
        let source = "struct Point {}";
        let mut ctx = AnalyzerContext::new(&arena, source);

        let struct_def1 = arena.alloc(StructDefinition::new(
            "Point",
            dummy_span(),
            vec![],
            vec![],
            None,
            dummy_span(),
        ));

        let struct_def2 = arena.alloc(StructDefinition::new(
            "Point",
            dummy_span(),
            vec![],
            vec![],
            None,
            dummy_span(),
        ));

        // First registration succeeds
        assert!(ctx.register_struct("Point", struct_def1).is_ok());

        // Second registration fails
        assert!(ctx.register_struct("Point", struct_def2).is_err());

        // Still only one struct registered
        assert_eq!(ctx.struct_definitions.len(), 1);
    }

    #[test]
    fn test_function_registration_and_lookup() {
        let arena = Bump::new();
        let source = "fn distance() -> f64 {}";
        let mut ctx = AnalyzerContext::new(&arena, source);

        // Create a simple function definition
        let func_def = arena.alloc(FunctionDefinition::new(
            "distance",
            dummy_span(),
            vec![],
            dummy_type(),
            vec![],
            None,
            dummy_span(),
        ));

        // Register should succeed
        assert!(ctx.register_function("distance", func_def).is_ok());
        assert_eq!(ctx.function_definitions.len(), 1);

        // Lookup should find the function
        let found = ctx.lookup_function("distance");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "distance");

        // Lookup non-existent function should return None
        assert!(ctx.lookup_function("area").is_none());
    }

    #[test]
    fn test_function_duplicate_detection() {
        let arena = Bump::new();
        let source = "fn distance() -> f64 {}";
        let mut ctx = AnalyzerContext::new(&arena, source);

        let func_def1 = arena.alloc(FunctionDefinition::new(
            "distance",
            dummy_span(),
            vec![],
            dummy_type(),
            vec![],
            None,
            dummy_span(),
        ));

        let func_def2 = arena.alloc(FunctionDefinition::new(
            "distance",
            dummy_span(),
            vec![],
            dummy_type(),
            vec![],
            None,
            dummy_span(),
        ));

        // First registration succeeds
        assert!(ctx.register_function("distance", func_def1).is_ok());

        // Second registration fails
        assert!(ctx.register_function("distance", func_def2).is_err());

        // Still only one function registered
        assert_eq!(ctx.function_definitions.len(), 1);
    }

    #[test]
    fn test_error_collection() {
        let arena = Bump::new();
        let source = "let x = y;";
        let mut ctx = AnalyzerContext::new(&arena, source);

        // Initially no errors
        assert!(!ctx.has_errors());
        assert_eq!(ctx.errors.len(), 0);

        // Add first error
        ctx.add_error(SemanticError::UndefinedVariable {
            name: "y".to_string(),
            span: dummy_span(),
        });

        assert!(ctx.has_errors());
        assert_eq!(ctx.errors.len(), 1);

        // Add second error
        ctx.add_error(SemanticError::UndefinedFunction {
            name: "foo".to_string(),
            span: dummy_span(),
        });

        assert!(ctx.has_errors());
        assert_eq!(ctx.errors.len(), 2);
    }

    #[test]
    fn test_take_errors() {
        let arena = Bump::new();
        let source = "let x = y;";
        let mut ctx = AnalyzerContext::new(&arena, source);

        // Add some errors
        ctx.add_error(SemanticError::UndefinedVariable {
            name: "y".to_string(),
            span: dummy_span(),
        });
        ctx.add_error(SemanticError::UndefinedFunction {
            name: "foo".to_string(),
            span: dummy_span(),
        });

        assert_eq!(ctx.errors.len(), 2);

        // Take errors
        let errors = ctx.take_errors();

        // Context should now have no errors
        assert!(!ctx.has_errors());
        assert_eq!(ctx.errors.len(), 0);

        // Taken errors should have the correct count
        assert_eq!(errors.len(), 2);

        // Verify error types
        assert!(matches!(errors[0], SemanticError::UndefinedVariable { .. }));
        assert!(matches!(errors[1], SemanticError::UndefinedFunction { .. }));
    }

    #[test]
    fn test_multiple_structs_and_functions() {
        let arena = Bump::new();
        let source = "struct Point {} struct Circle {} fn foo() {} fn bar() {}";
        let mut ctx = AnalyzerContext::new(&arena, source);

        // Register multiple structs
        let point = arena.alloc(StructDefinition::new(
            "Point",
            dummy_span(),
            vec![],
            vec![],
            None,
            dummy_span(),
        ));
        let circle = arena.alloc(StructDefinition::new(
            "Circle",
            dummy_span(),
            vec![],
            vec![],
            None,
            dummy_span(),
        ));

        assert!(ctx.register_struct("Point", point).is_ok());
        assert!(ctx.register_struct("Circle", circle).is_ok());
        assert_eq!(ctx.struct_definitions.len(), 2);

        // Register multiple functions
        let foo = arena.alloc(FunctionDefinition::new(
            "foo",
            dummy_span(),
            vec![],
            dummy_type(),
            vec![],
            None,
            dummy_span(),
        ));
        let bar = arena.alloc(FunctionDefinition::new(
            "bar",
            dummy_span(),
            vec![],
            dummy_type(),
            vec![],
            None,
            dummy_span(),
        ));

        assert!(ctx.register_function("foo", foo).is_ok());
        assert!(ctx.register_function("bar", bar).is_ok());
        assert_eq!(ctx.function_definitions.len(), 2);

        // All lookups should work
        assert!(ctx.lookup_struct("Point").is_some());
        assert!(ctx.lookup_struct("Circle").is_some());
        assert!(ctx.lookup_function("foo").is_some());
        assert!(ctx.lookup_function("bar").is_some());
    }

    #[test]
    fn test_struct_with_fields() {
        let arena = Bump::new();
        let source = "struct Point { x: f64, y: f64 }";
        let mut ctx = AnalyzerContext::new(&arena, source);

        // Create fields
        let field_x = arena.alloc(FieldDefinition::new(
            "x",
            dummy_span(),
            ResolvedType::F64 { span: dummy_span() },
            dummy_span(),
        ));
        let field_y = arena.alloc(FieldDefinition::new(
            "y",
            dummy_span(),
            ResolvedType::F64 { span: dummy_span() },
            dummy_span(),
        ));

        // Create struct with fields
        let struct_def = arena.alloc(StructDefinition::new(
            "Point",
            dummy_span(),
            vec![field_x, field_y],
            vec![],
            None,
            dummy_span(),
        ));

        assert!(ctx.register_struct("Point", struct_def).is_ok());

        let found = ctx.lookup_struct("Point").unwrap();
        assert_eq!(found.field_count(), 2);
        assert!(found.has_field("x"));
        assert!(found.has_field("y"));
    }

    #[test]
    fn test_function_with_parameters() {
        let arena = Bump::new();
        let source = "fn distance(p1: Point, p2: Point) -> f64 {}";
        let mut ctx = AnalyzerContext::new(&arena, source);

        // Create a Point struct definition for the type
        let point_struct = arena.alloc(StructDefinition::new(
            "Point",
            dummy_span(),
            vec![],
            vec![],
            None,
            dummy_span(),
        ));

        // Create a Point type referencing the struct
        let point_type = ResolvedType::UserDefined {
            name: "Point",
            definition: point_struct,
            span: dummy_span(),
        };

        // Create parameters
        let params = vec![
            FunctionParam::new("p1", dummy_span(), point_type, dummy_span()),
            FunctionParam::new("p2", dummy_span(), point_type, dummy_span()),
        ];

        // Create function with parameters
        let func_def = arena.alloc(FunctionDefinition::new(
            "distance",
            dummy_span(),
            params,
            ResolvedType::F64 { span: dummy_span() },
            vec![],
            None,
            dummy_span(),
        ));

        assert!(ctx.register_function("distance", func_def).is_ok());

        let found = ctx.lookup_function("distance").unwrap();
        assert_eq!(found.param_count(), 2);
        assert_eq!(found.params[0].name, "p1");
        assert_eq!(found.params[1].name, "p2");
    }

    #[test]
    fn test_scope_stack_integration() {
        let arena = Bump::new();
        let source = "let x = 10;";
        let ctx = AnalyzerContext::new(&arena, source);

        // Verify scope stack is initialized correctly
        assert_eq!(ctx.scope_stack.scope_count(), 1);
        assert_eq!(ctx.scope_stack.current_scope_level(), 0);
        assert!(!ctx.scope_stack.in_with_context());
    }
}
