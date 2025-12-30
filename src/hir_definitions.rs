//! High-level Intermediate Representation (HIR) definition nodes
//!
//! This module defines the core definition types for the HIR, including variables,
//! functions, structs, and fields. All nodes use arena allocation for efficient
//! memory management and to enable safe cross-references using lifetimes.
//!
#![allow(dead_code)] // Public API with methods for future compiler phases
//! # Arena Allocation
//!
//! All HIR nodes are allocated in a `bumpalo::Bump` arena allocator, providing:
//! - Fast allocation (just bumping a pointer)
//! - No individual deallocations (arena is freed all at once)
//! - Safe cross-references using arena lifetimes
//! - Reduced memory fragmentation
//!
//! # Lifetimes
//!
//! HIR nodes use two lifetime parameters:
//! - `'src`: Lifetime of the source text (for string slices from the original source)
//! - `'arena`: Lifetime of the arena allocator (for references to other HIR nodes)
//!
//! # Design Philosophy
//!
//! HIR definitions are created during semantic analysis after parsing. Unlike the AST,
//! which is a direct representation of the source syntax, the HIR:
//! - Resolves names to their definitions
//! - Tracks scope levels for proper shadowing and lookup
//! - Connects related entities (methods to structs, etc.)
//! - Provides a foundation for type checking and constraint solving

use crate::ast::span::HasSpan;
use crate::hir_expr::ResolvedExpr;
use crate::hir_types::ResolvedType;
use crate::lexer::Span;
use std::collections::HashMap;

// ============================================================================
// Type Aliases for HIR References
// ============================================================================

/// HIR expression reference - points to a resolved expression in the arena
pub type HirExpr<'src, 'arena> = &'arena ResolvedExpr<'src, 'arena>;

/// HIR type reference - represents a resolved type
pub type HirType<'src, 'arena> = ResolvedType<'src, 'arena>;

/// HIR statement placeholder - to be defined in hir_stmt module
/// For now, statements are represented as AST statements
pub type HirStmt<'src, 'arena> = crate::ast::types::Stmt<'src>;

// ============================================================================
// Scope Level
// ============================================================================

/// Scope level for tracking variable shadowing and lookup
///
/// The scope level is a simple integer that increases as we enter nested scopes
/// (function bodies, blocks, loops, etc.). This allows us to:
/// - Detect variable shadowing (same name at different levels)
/// - Implement proper lexical scoping rules
/// - Clean up variables when exiting a scope
///
/// # Examples
///
/// ```text
/// // Scope level 0 (global)
/// let x = 1;
///
/// fn foo() {           // Scope level 1 (function body)
///     let x = 2;       // Shadows the global x
///     {                // Scope level 2 (block)
///         let x = 3;   // Shadows the function-level x
///     }
/// }
/// ```
pub type ScopeLevel = usize;

// ============================================================================
// Variable Definition
// ============================================================================

/// Variable definition in the HIR
///
/// Represents a variable that has been declared with `let`. Variables can be:
/// - Uninitialized: declared but not assigned a value
/// - Initialized: declared with an initial value expression
/// - Type-annotated: explicitly typed, or type can be inferred
///
/// # Examples
///
/// ```text
/// let x: i32 = 42;           // Initialized with type annotation
/// let y: bool;               // Uninitialized with type annotation
/// let z = 3.14;              // Initialized with inferred type
/// let container.field = p;   // Container field (handled differently)
/// ```
///
/// # Scope Tracking
///
/// Each variable tracks its `scope_level` to enable proper shadowing and lookup:
/// - Variables at the same level in the same scope must have unique names
/// - Variables at different levels can shadow outer variables
/// - When looking up a variable, we search from innermost to outermost scope
#[derive(Debug, Clone, PartialEq)]
pub struct VarDefinition<'src, 'arena> {
    /// Variable name as it appears in source
    pub name: &'src str,

    /// Span of the variable name for error reporting
    pub name_span: Span,

    /// Type of the variable (either explicit or inferred)
    /// None during initial creation, filled in during type checking
    pub var_type: Option<HirType<'src, 'arena>>,

    /// Optional initialization expression
    /// None for uninitialized variables (e.g., `let x: i32;`)
    pub init: Option<HirExpr<'src, 'arena>>,

    /// Scope level where this variable was defined
    /// Used for shadowing detection and variable lookup
    pub scope_level: ScopeLevel,

    /// Full span of the variable definition for error reporting
    pub span: Span,
}

impl<'src, 'arena> VarDefinition<'src, 'arena> {
    /// Create a new variable definition
    pub fn new(
        name: &'src str,
        name_span: Span,
        var_type: Option<HirType<'src, 'arena>>,
        init: Option<HirExpr<'src, 'arena>>,
        scope_level: ScopeLevel,
        span: Span,
    ) -> Self {
        Self {
            name,
            name_span,
            var_type,
            init,
            scope_level,
            span,
        }
    }

    /// Check if this variable is initialized
    pub fn is_initialized(&self) -> bool {
        self.init.is_some()
    }

    /// Check if this variable has an explicit type annotation
    pub fn has_type_annotation(&self) -> bool {
        self.var_type.is_some()
    }
}

impl<'src, 'arena> HasSpan for VarDefinition<'src, 'arena> {
    fn span(&self) -> Span {
        self.span
    }
}

// ============================================================================
// Field Definition
// ============================================================================

/// Field definition within a struct
///
/// Represents a named, typed field in a struct definition. Fields are the
/// data members of a struct and define its internal state.
///
/// # Examples
///
/// ```text
/// struct Point {
///     x: f64,        // FieldDefinition { name: "x", type: F64 }
///     y: f64,        // FieldDefinition { name: "y", type: F64 }
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDefinition<'src, 'arena> {
    /// Field name as it appears in the struct definition
    pub name: &'src str,

    /// Span of the field name for error reporting
    pub name_span: Span,

    /// Type of the field
    pub field_type: HirType<'src, 'arena>,

    /// Full span of the field definition for error reporting
    pub span: Span,

    /// Phantom data to maintain lifetime parameters consistency
    _phantom: std::marker::PhantomData<&'arena ()>,
}

impl<'src, 'arena> FieldDefinition<'src, 'arena> {
    /// Create a new field definition
    pub fn new(
        name: &'src str,
        name_span: Span,
        field_type: HirType<'src, 'arena>,
        span: Span,
    ) -> Self {
        Self {
            name,
            name_span,
            field_type,
            span,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<'src, 'arena> HasSpan for FieldDefinition<'src, 'arena> {
    fn span(&self) -> Span {
        self.span
    }
}

// ============================================================================
// Container Field
// ============================================================================

/// Container field for dynamically storing entities
///
/// A container field is a special type of field declared with the `container` keyword
/// in struct definitions. It provides a namespace for dynamically adding entities
/// (points, lines, etc.) at runtime without pre-declaring them.
///
/// # Purpose
///
/// Container fields enable patterns like:
/// - Collecting sketch entities (points, lines, curves)
/// - Building up constraint systems incrementally
/// - Dynamically naming and organizing related objects
///
/// # Examples
///
/// ```text
/// struct Sketch {
///     container entities,    // ContainerField for dynamic entities
///     origin: Point,
/// }
///
/// // Later, entities can be added:
/// let sketch = Sketch { origin: point(0, 0) };
/// let sketch.entities.p1 = point(10, 10);
/// let sketch.entities.line1 = line(p1, p2);
/// ```
///
/// # Implementation
///
/// The container uses a HashMap to store dynamically created entities. Each entity
/// is stored with its name as the key and a reference to its definition as the value.
#[derive(Debug, Clone)]
pub struct ContainerField<'src, 'arena> {
    /// Container field name
    pub name: &'src str,

    /// Span of the container field name for error reporting
    pub name_span: Span,

    /// Map of dynamically added entities
    /// Key: entity name, Value: reference to the entity's variable definition
    ///
    /// This map is populated as entities are added to the container at runtime
    /// (or during semantic analysis for compile-time declarations).
    pub entities: HashMap<&'src str, &'arena VarDefinition<'src, 'arena>>,

    /// Full span of the container field declaration
    pub span: Span,
}

impl<'src, 'arena> ContainerField<'src, 'arena> {
    /// Create a new container field
    pub fn new(name: &'src str, name_span: Span, span: Span) -> Self {
        Self {
            name,
            name_span,
            entities: HashMap::new(),
            span,
        }
    }

    /// Add an entity to this container
    ///
    /// Returns `Some(&old_definition)` if an entity with this name already existed,
    /// `None` otherwise.
    pub fn add_entity(
        &mut self,
        entity_name: &'src str,
        definition: &'arena VarDefinition<'src, 'arena>,
    ) -> Option<&'arena VarDefinition<'src, 'arena>> {
        self.entities.insert(entity_name, definition)
    }

    /// Look up an entity by name
    pub fn get_entity(&self, entity_name: &str) -> Option<&'arena VarDefinition<'src, 'arena>> {
        self.entities.get(entity_name).copied()
    }

    /// Check if an entity with the given name exists
    pub fn has_entity(&self, entity_name: &str) -> bool {
        self.entities.contains_key(entity_name)
    }

    /// Get the number of entities in this container
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Iterate over all entities in this container
    pub fn entities_iter(
        &self,
    ) -> impl Iterator<Item = (&'src str, &'arena VarDefinition<'src, 'arena>)> + '_ {
        self.entities.iter().map(|(k, v)| (*k, *v))
    }
}

impl<'src, 'arena> HasSpan for ContainerField<'src, 'arena> {
    fn span(&self) -> Span {
        self.span
    }
}

impl<'src, 'arena> PartialEq for ContainerField<'src, 'arena> {
    fn eq(&self, other: &Self) -> bool {
        // Compare all fields except the HashMap for basic equality
        // For full equality, we'd need to compare HashMap contents as well
        self.name == other.name
            && self.name_span == other.name_span
            && self.span == other.span
            && self.entities.len() == other.entities.len()
            && self.entities.iter().all(|(k, v)| {
                other
                    .entities
                    .get(k)
                    .map(|ov| std::ptr::eq(*v, *ov))
                    .unwrap_or(false)
            })
    }
}

// ============================================================================
// Function Definition
// ============================================================================

/// Function definition in the HIR
///
/// Represents a function that can be:
/// - A top-level function (standalone)
/// - A method within a struct (has `parent_struct`)
///
/// Functions have parameters, a return type, and a body consisting of statements.
///
/// # Examples
///
/// ```text
/// // Top-level function
/// fn distance(p1: &Point, p2: &Point) -> f64 {
///     let dx = p2.x - p1.x;
///     let dy = p2.y - p1.y;
///     sqrt(dx * dx + dy * dy)
/// }
///
/// // Method within a struct
/// struct Circle {
///     center: Point,
///     radius: f64,
///
///     fn area() -> f64 {
///         3.14159 * self.radius * self.radius
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDefinition<'src, 'arena> {
    /// Function name
    pub name: &'src str,

    /// Span of the function name for error reporting
    pub name_span: Span,

    /// Function parameters
    /// Each parameter has a name and type
    pub params: Vec<FunctionParam<'src, 'arena>>,

    /// Return type of the function
    pub return_type: HirType<'src, 'arena>,

    /// Function body statements
    /// The body is a sequence of statements executed when the function is called
    pub body: Vec<HirStmt<'src, 'arena>>,

    /// Optional reference to the parent struct if this is a method
    /// None for top-level functions, Some(&struct_def) for methods
    pub parent_struct: Option<&'arena StructDefinition<'src, 'arena>>,

    /// Full span of the function definition for error reporting
    pub span: Span,
}

impl<'src, 'arena> FunctionDefinition<'src, 'arena> {
    /// Create a new function definition
    pub fn new(
        name: &'src str,
        name_span: Span,
        params: Vec<FunctionParam<'src, 'arena>>,
        return_type: HirType<'src, 'arena>,
        body: Vec<HirStmt<'src, 'arena>>,
        parent_struct: Option<&'arena StructDefinition<'src, 'arena>>,
        span: Span,
    ) -> Self {
        Self {
            name,
            name_span,
            params,
            return_type,
            body,
            parent_struct,
            span,
        }
    }

    /// Check if this is a method (has a parent struct)
    pub fn is_method(&self) -> bool {
        self.parent_struct.is_some()
    }

    /// Check if this is a top-level function
    pub fn is_top_level(&self) -> bool {
        self.parent_struct.is_none()
    }

    /// Get the number of parameters
    pub fn param_count(&self) -> usize {
        self.params.len()
    }
}

impl<'src, 'arena> HasSpan for FunctionDefinition<'src, 'arena> {
    fn span(&self) -> Span {
        self.span
    }
}

// ============================================================================
// Function Parameter
// ============================================================================

/// Function parameter with name and type
///
/// Parameters are the inputs to a function, each with a name and type.
/// Unlike the AST version, this HIR version includes the arena lifetime.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionParam<'src, 'arena> {
    /// Parameter name
    pub name: &'src str,

    /// Span of the parameter name for error reporting
    pub name_span: Span,

    /// Parameter type
    pub param_type: HirType<'src, 'arena>,

    /// Full span of the parameter for error reporting
    pub span: Span,

    /// Phantom data to maintain lifetime parameters consistency
    _phantom: std::marker::PhantomData<&'arena ()>,
}

impl<'src, 'arena> FunctionParam<'src, 'arena> {
    /// Create a new function parameter
    pub fn new(
        name: &'src str,
        name_span: Span,
        param_type: HirType<'src, 'arena>,
        span: Span,
    ) -> Self {
        Self {
            name,
            name_span,
            param_type,
            span,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<'src, 'arena> HasSpan for FunctionParam<'src, 'arena> {
    fn span(&self) -> Span {
        self.span
    }
}

// ============================================================================
// Struct Definition
// ============================================================================

/// Struct definition in the HIR
///
/// Represents a user-defined composite type with:
/// - Named fields (regular data members)
/// - Methods (functions associated with the struct)
/// - Optional container field (for dynamic entity storage)
///
/// # Container Fields
///
/// Structs can have at most one container field, declared with the `container` keyword.
/// This field provides a namespace for dynamically adding entities.
///
/// # Examples
///
/// ```text
/// // Simple struct with fields
/// struct Point {
///     x: f64,
///     y: f64,
/// }
///
/// // Struct with methods
/// struct Circle {
///     center: Point,
///     radius: f64,
///
///     fn area() -> f64 {
///         3.14159 * self.radius * self.radius
///     }
///
///     fn circumference() -> f64 {
///         2.0 * 3.14159 * self.radius
///     }
/// }
///
/// // Struct with container field
/// struct Sketch {
///     container entities,
///     origin: Point,
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct StructDefinition<'src, 'arena> {
    /// Struct name
    pub name: &'src str,

    /// Span of the struct name for error reporting
    pub name_span: Span,

    /// Regular fields (data members)
    pub fields: Vec<&'arena FieldDefinition<'src, 'arena>>,

    /// Methods (functions associated with this struct)
    pub methods: Vec<&'arena FunctionDefinition<'src, 'arena>>,

    /// Optional container field for dynamic entities
    /// At most one container field is allowed per struct
    pub container_field: Option<&'arena ContainerField<'src, 'arena>>,

    /// Full span of the struct definition for error reporting
    pub span: Span,
}

impl<'src, 'arena> StructDefinition<'src, 'arena> {
    /// Create a new struct definition
    pub fn new(
        name: &'src str,
        name_span: Span,
        fields: Vec<&'arena FieldDefinition<'src, 'arena>>,
        methods: Vec<&'arena FunctionDefinition<'src, 'arena>>,
        container_field: Option<&'arena ContainerField<'src, 'arena>>,
        span: Span,
    ) -> Self {
        Self {
            name,
            name_span,
            fields,
            methods,
            container_field,
            span,
        }
    }

    /// Check if this struct has a container field
    pub fn has_container(&self) -> bool {
        self.container_field.is_some()
    }

    /// Get the number of regular fields
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// Get the number of methods
    pub fn method_count(&self) -> usize {
        self.methods.len()
    }

    /// Look up a field by name
    pub fn find_field(&self, field_name: &str) -> Option<&'arena FieldDefinition<'src, 'arena>> {
        self.fields.iter().find(|f| f.name == field_name).copied()
    }

    /// Look up a method by name
    pub fn find_method(
        &self,
        method_name: &str,
    ) -> Option<&'arena FunctionDefinition<'src, 'arena>> {
        self.methods.iter().find(|m| m.name == method_name).copied()
    }

    /// Check if a field with the given name exists
    pub fn has_field(&self, field_name: &str) -> bool {
        self.find_field(field_name).is_some()
    }

    /// Check if a method with the given name exists
    pub fn has_method(&self, method_name: &str) -> bool {
        self.find_method(method_name).is_some()
    }
}

impl<'src, 'arena> HasSpan for StructDefinition<'src, 'arena> {
    fn span(&self) -> Span {
        self.span
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;

    /// Helper to create a dummy span for testing
    fn dummy_span() -> Span {
        Span {
            start: crate::lexer::LineColumn { line: 1, column: 1 },
            lines: 0,
            end_column: 1,
        }
    }

    /// Helper to create a dummy type for testing
    fn dummy_type<'src, 'arena>() -> HirType<'src, 'arena> {
        ResolvedType::I32 { span: dummy_span() }
    }

    #[test]
    fn test_var_definition_uninitialized() {
        let var_def = VarDefinition::<'_, '_>::new(
            "x",
            dummy_span(),
            Some(dummy_type()),
            None,
            0,
            dummy_span(),
        );

        assert_eq!(var_def.name, "x");
        assert!(!var_def.is_initialized());
        assert!(var_def.has_type_annotation());
        assert_eq!(var_def.scope_level, 0);
    }

    #[test]
    fn test_container_field_operations() {
        let arena = Bump::new();
        let mut container = ContainerField::new("entities", dummy_span(), dummy_span());

        assert_eq!(container.name, "entities");
        assert_eq!(container.entity_count(), 0);
        assert!(!container.has_entity("p1"));

        let var_def = arena.alloc(VarDefinition::new(
            "p1",
            dummy_span(),
            Some(dummy_type()),
            None,
            0,
            dummy_span(),
        ));

        let old = container.add_entity("p1", var_def);
        assert!(old.is_none());
        assert_eq!(container.entity_count(), 1);
        assert!(container.has_entity("p1"));

        let found = container.get_entity("p1");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "p1");
    }

    #[test]
    fn test_struct_definition_lookups() {
        let arena = Bump::new();

        let field1 = arena.alloc(FieldDefinition::new(
            "x",
            dummy_span(),
            dummy_type(),
            dummy_span(),
        ));

        let field2 = arena.alloc(FieldDefinition::new(
            "y",
            dummy_span(),
            dummy_type(),
            dummy_span(),
        ));

        let struct_def = StructDefinition::new(
            "Point",
            dummy_span(),
            vec![field1, field2],
            vec![],
            None,
            dummy_span(),
        );

        assert_eq!(struct_def.name, "Point");
        assert_eq!(struct_def.field_count(), 2);
        assert_eq!(struct_def.method_count(), 0);
        assert!(!struct_def.has_container());

        assert!(struct_def.has_field("x"));
        assert!(struct_def.has_field("y"));
        assert!(!struct_def.has_field("z"));

        let found_x = struct_def.find_field("x");
        assert!(found_x.is_some());
        assert_eq!(found_x.unwrap().name, "x");
    }

    #[test]
    fn test_function_definition_method_check() {
        let func_def = FunctionDefinition::<'_, '_>::new(
            "distance",
            dummy_span(),
            vec![],
            dummy_type(),
            vec![],
            None,
            dummy_span(),
        );

        assert_eq!(func_def.name, "distance");
        assert!(func_def.is_top_level());
        assert!(!func_def.is_method());
        assert_eq!(func_def.param_count(), 0);
    }
}
