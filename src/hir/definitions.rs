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

use super::expr::ResolvedExpr;
use super::types::ResolvedType;
use crate::ast::span::HasSpan;
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
// Transform Step
// ============================================================================

/// A single step in a transform chain
///
/// Transform chains represent nested with-contexts where each level applies a
/// transformation. For example:
/// ```cad
/// with outer {          // Transform step 1
///     with inner {      // Transform step 2
///         let .p: T;    // Variable transformed through both steps
///     }
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformStep<'src, 'arena> {
    /// The transform method being applied in this step
    pub transform_method: &'arena FunctionDefinition<'src, 'arena>,

    /// The with-context that provides this transform
    pub with_context: &'arena super::context::WithContext<'src, 'arena>,

    /// Input type for this transform step
    pub input_type: &'arena ResolvedType<'src, 'arena>,

    /// Output type for this transform step
    pub output_type: &'arena ResolvedType<'src, 'arena>,
}

// ============================================================================
// Variable Identifier
// ============================================================================

/// Identifies a variable structurally without string concatenation
///
/// This enum represents variable identity using structural chains of references
/// rather than heap-allocated qualified name strings. This eliminates memory leaks
/// from Box::leak() and better represents the semantic structure of variable access.
///
/// # Design Principles
///
/// 1. **Store Structure, Not Strings:** Variable identity is represented by chains of references
/// 2. **Compute Names On-Demand:** Generate string names only when needed for display/Z3
/// 3. **Use Arena Allocation:** All structural elements use `&'arena` references
/// 4. **Preserve Source Lifetimes:** Simple names from source remain `&'src str`
///
/// # Examples
///
/// ```text
/// Simple("x")                                    // Simple variable: x
/// FieldAccess { base: Simple("p"), field: "x" }  // Field access: p.x
/// ContainerAccess { container: ..., entity: "p" } // Container: sketch.entities.p
/// ArrayIndex { array: Simple("arr"), index: 0 }  // Array element: arr[0]
/// TransformedView { view_name: "p", ... }        // Transform view (replaces shadows)
/// ```
#[derive(Debug, Clone)]
pub enum VariableIdentifier<'src, 'arena> {
    /// Simple variable from source: `x`, `p`, `sketch`
    Simple(&'src str),

    /// Field access chain: `p.x`, `sketch.origin`
    FieldAccess {
        /// The base expression being accessed
        base: &'arena VariableIdentifier<'src, 'arena>,
        /// The field name
        field_name: &'src str,
    },

    /// Container field access: `sketch.entities.p`
    ///
    /// Created when declaring dot-prefix variables in with-statements.
    /// Represents the full qualified path to an entity within a container.
    ContainerAccess {
        /// The container variable identifier
        container_var: &'arena VariableIdentifier<'src, 'arena>,
        /// The container field within the struct
        container_field: &'arena ContainerField<'src, 'arena>,
        /// The entity name within the container
        entity_name: &'src str,
    },

    /// Array element: `points[0]`, `arr[i]` (index is constant)
    ArrayIndex {
        /// The array being indexed
        array: &'arena VariableIdentifier<'src, 'arena>,
        /// The constant index
        index: usize,
    },

    /// Transformed view variable (replaces shadow variables!)
    ///
    /// When a variable is declared in a transform context (with-statement with __transform__),
    /// this variant stores the relationship between the view variable and its container
    /// variable directly in the identifier. This eliminates the need for generated shadow
    /// variable names like "__shadow_0".
    ///
    /// # Example
    ///
    /// ```text
    /// with sketch {
    ///     let .p: Point2D;  // Creates view variable with TransformedView identifier
    /// }
    /// // View: p (Point2D)
    /// // Container: sketch.entities.p (Point3D)
    /// // Transform: sketch.__transform__(&sketch.entities.p) -> Point2D
    /// ```
    TransformedView {
        /// The view variable's simple name (e.g., "p")
        view_name: &'src str,
        /// The underlying container variable being viewed
        container_var: &'arena VariableIdentifier<'src, 'arena>,
        /// Transform chain metadata
        transform_chain: &'arena [TransformStep<'src, 'arena>],
    },
}

impl<'src, 'arena> VariableIdentifier<'src, 'arena> {
    /// Generate qualified name for display/Z3
    ///
    /// This is the ONLY point where String allocation happens for variable names.
    /// All other operations use the structural representation.
    ///
    /// # Examples
    ///
    /// ```text
    /// Simple("x")                  -> "x"
    /// FieldAccess(Simple("p"), "x") -> "p.x"
    /// ArrayIndex(Simple("arr"), 0)  -> "arr[0]"
    /// ```
    pub fn to_qualified_name(&self) -> String {
        match self {
            Self::Simple(name) => name.to_string(),
            Self::FieldAccess { base, field_name } => {
                format!("{}.{}", base.to_qualified_name(), field_name)
            }
            Self::ContainerAccess {
                container_var,
                container_field,
                entity_name,
            } => {
                format!(
                    "{}.{}.{}",
                    container_var.to_qualified_name(),
                    container_field.name,
                    entity_name
                )
            }
            Self::ArrayIndex { array, index } => {
                format!("{}[{}]", array.to_qualified_name(), index)
            }
            Self::TransformedView { container_var, .. } => {
                // For transformed views, use the container variable's name with a __view suffix
                // This ensures view variables have unique names and won't conflict with each other
                // or with container variables. View variables are internal and will be filtered
                // from the final solution output.
                format!("{}__view", container_var.to_qualified_name())
            }
        }
    }

    /// Get the root variable name (for HashMap lookups)
    ///
    /// Returns the simple name at the root of the identifier chain.
    /// Used when we need to look up the root variable in a HashMap.
    ///
    /// # Examples
    ///
    /// ```text
    /// Simple("x")                  -> "x"
    /// FieldAccess(Simple("p"), "x") -> "p"
    /// ArrayIndex(Simple("arr"), 0)  -> "arr"
    /// ```
    pub fn root_name(&self) -> &'src str {
        match self {
            Self::Simple(name) => name,
            Self::FieldAccess { base, .. } => base.root_name(),
            Self::ContainerAccess { container_var, .. } => container_var.root_name(),
            Self::ArrayIndex { array, .. } => array.root_name(),
            Self::TransformedView { view_name, .. } => view_name,
        }
    }

    /// Create a simple identifier (convenience constructor)
    pub fn simple(name: &'src str) -> Self {
        Self::Simple(name)
    }
}

// Manual PartialEq implementation using pointer comparison for arena-allocated references
impl<'src, 'arena> PartialEq for VariableIdentifier<'src, 'arena> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Simple(a), Self::Simple(b)) => a == b,
            (
                Self::FieldAccess {
                    base: base_a,
                    field_name: field_a,
                },
                Self::FieldAccess {
                    base: base_b,
                    field_name: field_b,
                },
            ) => {
                // Compare bases structurally (recursively)
                base_a == base_b && field_a == field_b
            }
            (
                Self::ContainerAccess {
                    container_var: container_a,
                    container_field: field_a,
                    entity_name: entity_a,
                },
                Self::ContainerAccess {
                    container_var: container_b,
                    container_field: field_b,
                    entity_name: entity_b,
                },
            ) => {
                // Use pointer comparison for container_field (identity equality)
                container_a == container_b
                    && std::ptr::eq(*field_a, *field_b)
                    && entity_a == entity_b
            }
            (
                Self::ArrayIndex {
                    array: array_a,
                    index: index_a,
                },
                Self::ArrayIndex {
                    array: array_b,
                    index: index_b,
                },
            ) => array_a == array_b && index_a == index_b,
            (
                Self::TransformedView {
                    view_name: view_a,
                    container_var: container_a,
                    transform_chain: chain_a,
                },
                Self::TransformedView {
                    view_name: view_b,
                    container_var: container_b,
                    transform_chain: chain_b,
                },
            ) => {
                // Use pointer comparison for transform_chain (identity equality)
                view_a == view_b && container_a == container_b && std::ptr::eq(*chain_a, *chain_b)
            }
            _ => false, // Different variants are not equal
        }
    }
}

impl<'src, 'arena> Eq for VariableIdentifier<'src, 'arena> {}

// Manual Hash implementation using pointer hashing for arena-allocated references
impl<'src, 'arena> std::hash::Hash for VariableIdentifier<'src, 'arena> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // First, hash the discriminant to differentiate variants
        std::mem::discriminant(self).hash(state);

        match self {
            Self::Simple(name) => {
                name.hash(state);
            }
            Self::FieldAccess { base, field_name } => {
                base.hash(state);
                field_name.hash(state);
            }
            Self::ContainerAccess {
                container_var,
                container_field,
                entity_name,
            } => {
                container_var.hash(state);
                // Hash the pointer address for container_field
                std::ptr::hash(*container_field, state);
                entity_name.hash(state);
            }
            Self::ArrayIndex { array, index } => {
                array.hash(state);
                index.hash(state);
            }
            Self::TransformedView {
                view_name,
                container_var,
                transform_chain,
            } => {
                view_name.hash(state);
                container_var.hash(state);
                // Hash the pointer address for transform_chain
                std::ptr::hash(*transform_chain, state);
            }
        }
    }
}

// ============================================================================
// Variable Definition Kind
// ============================================================================

/// How a variable is defined
///
/// This enum describes the three ways a variable can be defined in CAD-DSL:
/// 1. Uninitialized - declared but not assigned, solver will find a value
/// 2. Initialized - declared with an explicit expression
/// 3. TransformedView - a temporary view of a container variable through a transform
///
/// # Transform Semantics
///
/// When a variable is declared with dot-prefix syntax inside a with-block,
/// two variables are created:
/// - **Container variable**: The real, persistent entity (e.g., `sketch.entities.p: Point3D`)
/// - **View variable**: A temporary transformed view (e.g., `p: Point2D`)
///
/// The view variable shadows the container variable by name inside the with-block.
#[derive(Debug, Clone, PartialEq)]
pub enum VarDefinitionKind<'src, 'arena> {
    /// Uninitialized variable: `let x: i32;`
    ///
    /// The solver will find a value satisfying all constraints.
    /// This is the default for free variables in constraint systems.
    Uninitialized,

    /// Initialized with explicit expression: `let x = 5;`
    ///
    /// The variable's value is determined by the initialization expression.
    Initialized {
        /// The initialization expression
        init: &'arena ResolvedExpr<'src, 'arena>,
    },

    /// Temporary transformed view: `with sketch { let .p: Point2D; }`
    ///
    /// Creates TWO variables:
    /// 1. Container variable: `sketch.entities.p: Point3D` (persistent, accessible outside)
    /// 2. View variable: `.p: Point2D` (temporary, only in with-block, shadows container)
    ///
    /// The view's value is the transform of the container variable.
    /// Supports nested transforms via transform_chain.
    TransformedView {
        /// The persistent container variable (e.g., `sketch.entities.p: Point3D`)
        ///
        /// This is the real entity that exists in the container's namespace.
        /// Outside the with-block, this variable is accessible as Point3D.
        /// Inside the with-block, it's shadowed by the view.
        container_var: &'arena VarDefinition<'src, 'arena>,

        /// The complete transform chain from outermost to innermost
        ///
        /// For single transform: vec has one element
        /// For nested transforms: `with outer { with inner { ... } }`
        /// Chain is [outer_transform, inner_transform] applied in order
        transform_chain: Vec<TransformStep<'src, 'arena>>,

        /// The transform expression defining the view
        ///
        /// Represents: view_var == innermost_transform(...(outermost_transform(&container_var)))
        /// Example: `.p == inner.__transform__(outer.__transform__(&sketch.entities.p))`
        transform_expr: &'arena ResolvedExpr<'src, 'arena>,
    },
}

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
/// - TransformedView: a temporary view of a container variable through a transform
///
/// # Examples
///
/// ```text
/// let x: i32 = 42;           // Initialized with type annotation
/// let y: bool;               // Uninitialized with type annotation
/// let z = 3.14;              // Initialized with inferred type
/// with sketch {
///     let .p: Point2D;       // TransformedView (creates container + view)
/// }
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
    /// Variable identifier (structural representation)
    ///
    /// This replaces the simple string name with a structural identifier that
    /// can represent qualified paths (e.g., field access, container access, arrays).
    /// Eliminates heap allocations for qualified names.
    pub identifier: &'arena VariableIdentifier<'src, 'arena>,

    /// Simple display name for error messages
    ///
    /// This is the "short name" without qualification, used primarily for
    /// user-facing error messages. For simple variables, this is the same as
    /// the source name. For complex identifiers, this is the most relevant part.
    pub display_name: &'src str,

    /// Span of the variable name for error reporting
    pub name_span: Span,

    /// Type of the variable (either explicit or inferred)
    /// None during initial creation, filled in during type checking
    pub var_type: Option<HirType<'src, 'arena>>,

    /// How this variable is defined (uninitialized, initialized, or transformed view)
    pub definition_kind: VarDefinitionKind<'src, 'arena>,

    /// Scope level where this variable was defined
    /// Used for shadowing detection and variable lookup
    pub scope_level: ScopeLevel,

    /// Full span of the variable definition for error reporting
    pub span: Span,
}

impl<'src, 'arena> VarDefinition<'src, 'arena> {
    /// Create a new variable definition
    pub fn new(
        identifier: &'arena VariableIdentifier<'src, 'arena>,
        display_name: &'src str,
        name_span: Span,
        var_type: Option<HirType<'src, 'arena>>,
        definition_kind: VarDefinitionKind<'src, 'arena>,
        scope_level: ScopeLevel,
        span: Span,
    ) -> Self {
        Self {
            identifier,
            display_name,
            name_span,
            var_type,
            definition_kind,
            scope_level,
            span,
        }
    }

    /// Get the simple name for backward compatibility
    ///
    /// Returns the display name, which is the short name without qualification.
    /// This method is provided for backward compatibility with code that expects
    /// a simple string name.
    pub fn name(&self) -> &'src str {
        self.display_name
    }

    /// Get the qualified name (allocates string)
    ///
    /// This generates the full qualified name from the structural identifier.
    /// Use sparingly as it allocates a String. Prefer using the identifier
    /// directly for most operations.
    pub fn qualified_name(&self) -> String {
        self.identifier.to_qualified_name()
    }

    /// Check if this variable is initialized
    pub fn is_initialized(&self) -> bool {
        matches!(self.definition_kind, VarDefinitionKind::Initialized { .. })
    }

    /// Check if this variable has an explicit type annotation
    pub fn has_type_annotation(&self) -> bool {
        self.var_type.is_some()
    }

    /// Check if this variable is a transformed view
    pub fn is_transformed_view(&self) -> bool {
        matches!(
            self.definition_kind,
            VarDefinitionKind::TransformedView { .. }
        )
    }

    /// Get the container variable if this is a transformed view
    pub fn get_container_var(&self) -> Option<&'arena VarDefinition<'src, 'arena>> {
        match &self.definition_kind {
            VarDefinitionKind::TransformedView { container_var, .. } => Some(container_var),
            _ => None,
        }
    }

    /// Get the initialization expression if this variable is initialized
    pub fn get_init_expr(&self) -> Option<&'arena ResolvedExpr<'src, 'arena>> {
        match &self.definition_kind {
            VarDefinitionKind::Initialized { init } => Some(init),
            _ => None,
        }
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
        let arena = Bump::new();
        let identifier = arena.alloc(VariableIdentifier::Simple("x"));

        let var_def = VarDefinition::<'_, '_>::new(
            identifier,
            "x",
            dummy_span(),
            Some(dummy_type()),
            VarDefinitionKind::Uninitialized,
            0,
            dummy_span(),
        );

        assert_eq!(var_def.name(), "x");
        assert_eq!(var_def.display_name, "x");
        assert_eq!(var_def.qualified_name(), "x");
        assert!(!var_def.is_initialized());
        assert!(var_def.has_type_annotation());
        assert_eq!(var_def.scope_level, 0);
        assert!(!var_def.is_transformed_view());
        assert_eq!(var_def.get_container_var(), None);
        assert_eq!(var_def.get_init_expr(), None);
    }

    #[test]
    fn test_container_field_operations() {
        let arena = Bump::new();
        let mut container = ContainerField::new("entities", dummy_span(), dummy_span());

        assert_eq!(container.name, "entities");
        assert_eq!(container.entity_count(), 0);
        assert!(!container.has_entity("p1"));

        let identifier = arena.alloc(VariableIdentifier::Simple("p1"));
        let var_def = arena.alloc(VarDefinition::new(
            identifier,
            "p1",
            dummy_span(),
            Some(dummy_type()),
            VarDefinitionKind::Uninitialized,
            0,
            dummy_span(),
        ));

        let old = container.add_entity("p1", var_def);
        assert!(old.is_none());
        assert_eq!(container.entity_count(), 1);
        assert!(container.has_entity("p1"));

        let found = container.get_entity("p1");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), "p1");
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

    // ========================================================================
    // VariableIdentifier Tests
    // ========================================================================

    #[test]
    fn test_variable_identifier_simple() {
        let id = VariableIdentifier::simple("x");

        assert_eq!(id.to_qualified_name(), "x");
        assert_eq!(id.root_name(), "x");
        assert!(matches!(id, VariableIdentifier::Simple("x")));
    }

    #[test]
    fn test_variable_identifier_field_access() {
        let arena = Bump::new();

        let base = arena.alloc(VariableIdentifier::Simple("p"));
        let id = VariableIdentifier::FieldAccess {
            base,
            field_name: "x",
        };

        assert_eq!(id.to_qualified_name(), "p.x");
        assert_eq!(id.root_name(), "p");
    }

    #[test]
    fn test_variable_identifier_nested_field_access() {
        let arena = Bump::new();

        // Build: sketch.origin.x
        let base = arena.alloc(VariableIdentifier::Simple("sketch"));
        let with_origin = arena.alloc(VariableIdentifier::FieldAccess {
            base,
            field_name: "origin",
        });
        let id = VariableIdentifier::FieldAccess {
            base: with_origin,
            field_name: "x",
        };

        assert_eq!(id.to_qualified_name(), "sketch.origin.x");
        assert_eq!(id.root_name(), "sketch");
    }

    #[test]
    fn test_variable_identifier_array_index() {
        let arena = Bump::new();

        let array = arena.alloc(VariableIdentifier::Simple("arr"));
        let id = VariableIdentifier::ArrayIndex { array, index: 0 };

        assert_eq!(id.to_qualified_name(), "arr[0]");
        assert_eq!(id.root_name(), "arr");
    }

    #[test]
    fn test_variable_identifier_array_of_structs() {
        let arena = Bump::new();

        // Build: points[1].x
        let base_array = arena.alloc(VariableIdentifier::Simple("points"));
        let with_index = arena.alloc(VariableIdentifier::ArrayIndex {
            array: base_array,
            index: 1,
        });
        let id = VariableIdentifier::FieldAccess {
            base: with_index,
            field_name: "x",
        };

        assert_eq!(id.to_qualified_name(), "points[1].x");
        assert_eq!(id.root_name(), "points");
    }

    #[test]
    fn test_variable_identifier_container_access() {
        let arena = Bump::new();

        let container_var = arena.alloc(VariableIdentifier::Simple("sketch"));
        let container_field =
            arena.alloc(ContainerField::new("entities", dummy_span(), dummy_span()));

        let id = VariableIdentifier::ContainerAccess {
            container_var,
            container_field,
            entity_name: "p",
        };

        assert_eq!(id.to_qualified_name(), "sketch.entities.p");
        assert_eq!(id.root_name(), "sketch");
    }

    #[test]
    fn test_variable_identifier_transformed_view() {
        let arena = Bump::new();

        // Container variable: sketch.entities.p
        let sketch_id = arena.alloc(VariableIdentifier::Simple("sketch"));
        let container_field =
            arena.alloc(ContainerField::new("entities", dummy_span(), dummy_span()));
        let container_var = arena.alloc(VariableIdentifier::ContainerAccess {
            container_var: sketch_id,
            container_field,
            entity_name: "p",
        });

        // Transform chain (empty for this test)
        let transform_chain: &[TransformStep] = &[];

        let id = VariableIdentifier::TransformedView {
            view_name: "p",
            container_var,
            transform_chain,
        };

        // TransformedView returns the container's qualified name with __view suffix
        // This ensures view variables have unique names and won't conflict with container variables
        assert_eq!(id.to_qualified_name(), "sketch.entities.p__view");
        // But root_name returns the view name
        assert_eq!(id.root_name(), "p");
    }

    #[test]
    fn test_variable_identifier_hash_and_eq() {
        use std::collections::HashSet;

        let arena = Bump::new();

        let id1 = VariableIdentifier::Simple("x");
        let id2 = VariableIdentifier::Simple("x");
        let id3 = VariableIdentifier::Simple("y");

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);

        // Test that identifiers can be used in HashSet
        let mut set = HashSet::new();
        set.insert(id1.clone());
        assert!(set.contains(&id2));
        assert!(!set.contains(&id3));

        // Test field access equality
        let base1 = arena.alloc(VariableIdentifier::Simple("p"));
        let base2 = arena.alloc(VariableIdentifier::Simple("p"));
        let field1 = VariableIdentifier::FieldAccess {
            base: base1,
            field_name: "x",
        };
        let field2 = VariableIdentifier::FieldAccess {
            base: base2,
            field_name: "x",
        };

        assert_eq!(field1, field2);
    }
}
