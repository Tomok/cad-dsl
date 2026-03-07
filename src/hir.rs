//! High-level Intermediate Representation (HIR)
//!
//! The HIR is the semantic representation of a CAD-DSL program after parsing and
//! name resolution. It serves as the bridge between the Abstract Syntax Tree (AST)
//! and later compilation stages like type checking and constraint solving.
//!
//! # What is HIR?
//!
//! The High-level Intermediate Representation (HIR) transforms the syntax-focused
//! AST into a semantically-enriched representation where:
//!
//! - **Names are resolved**: Variables, functions, types, and fields are linked to
//!   their definitions through direct references rather than just names
//! - **Scopes are tracked**: Each definition knows its scope level for proper shadowing
//!   and lookup semantics
//! - **Types are associated**: Every expression carries its resolved type information
//! - **Cross-references exist**: Methods know their parent structs, fields know their
//!   containing types, and expressions reference their definitions
//!
//! # Differences from AST
//!
//! While the AST is a faithful representation of source syntax, the HIR provides:
//!
//! ## 1. Name Resolution
//!
//! **AST**: Names are just strings
//! ```text
//! Expr::Var { name: "x" }  // Just a string reference
//! ```
//!
//! **HIR**: Names resolve to their definitions
//! ```text
//! ResolvedExpr {
//!     kind: Var {
//!         name: "x",
//!         definition: &VarDefinition { ... }  // Direct reference
//!     }
//! }
//! ```
//!
//! ## 2. Type Information
//!
//! **AST**: Types are syntactic annotations
//! ```text
//! Type::Named { name: "Point" }  // Just a name
//! ```
//!
//! **HIR**: Types include full definition information
//! ```text
//! ResolvedType::UserDefined {
//!     name: "Point",
//!     definition: &StructDefinition { ... }  // Link to struct
//! }
//! ```
//!
//! ## 3. Scope Tracking
//!
//! The HIR tracks scope levels to enable:
//! - Variable shadowing detection
//! - Proper lexical scoping rules
//! - Efficient name lookup (search from innermost to outermost scope)
//!
//! ## 4. Structural Simplification
//!
//! The AST uses type-level precedence hierarchies (via subenums) to enforce parse
//! tree correctness. The HIR uses a single flat enum since precedence is already
//! resolved.
//!
//! # Arena Allocation Strategy
//!
//! The HIR uses arena allocation via `bumpalo::Bump` for superior performance and
//! memory management:
//!
//! ## Why Arena Allocation?
//!
//! ### 1. Performance Benefits
//!
//! - **Fast allocation**: Allocating is just bumping a pointer - O(1) with minimal overhead
//! - **Fast deallocation**: The entire HIR is freed at once when the arena drops
//! - **No individual frees**: Eliminates per-node deallocation overhead
//! - **Cache locality**: Related data allocated together is stored nearby in memory,
//!   improving CPU cache hit rates
//!
//! ### 2. Safety Guarantees
//!
//! - **Lifetime tracking**: The `'arena` lifetime ensures HIR data can't outlive the arena
//! - **No use-after-free**: Borrow checker prevents accessing deallocated HIR data
//! - **No reference cycles**: Arena references are acyclic by construction
//!
//! ### 3. Simplicity
//!
//! - **Direct references**: Use `&'arena T` instead of `Box<T>` or `Rc<T>`
//! - **No reference counting**: Eliminates runtime overhead of `Rc`/`Arc`
//! - **Easier to reason about**: Single ownership model via arena
//!
//! ## How It Works
//!
//! ```rust,ignore
//! use bumpalo::Bump;
//!
//! // Create an arena for the compilation session
//! let arena = Bump::new();
//!
//! // Allocate HIR nodes in the arena
//! let var_def = arena.alloc(VarDefinition {
//!     name: "x",
//!     var_type: Some(ResolvedType::I32 { span }),
//!     // ...
//! });
//!
//! // Use arena-allocated references in other nodes
//! let expr = arena.alloc(ResolvedExpr {
//!     kind: Var {
//!         name: "x",
//!         definition: var_def,  // Reference to arena-allocated data
//!     },
//!     // ...
//! });
//!
//! // All HIR data is freed when arena is dropped
//! ```
//!
//! # Lifetime Parameters
//!
//! HIR types use two lifetime parameters that serve distinct purposes:
//!
//! ## `'src` - Source Code Lifetime
//!
//! - Represents the lifetime of the original source code string
//! - Used for string slices (`&'src str`) that point directly into the source
//! - Avoids allocating copies of identifiers, keywords, etc.
//! - Examples: variable names, struct names, field names
//!
//! ```rust,ignore
//! struct VarDefinition<'src, 'arena> {
//!     name: &'src str,  // Points into original source
//!     // ...
//! }
//! ```
//!
//! ## `'arena` - Arena Allocator Lifetime
//!
//! - Represents the lifetime of the arena allocator
//! - Used for references to other HIR nodes (`&'arena T`)
//! - Ensures HIR nodes can't outlive the arena that allocated them
//! - Enables safe cross-references between HIR nodes
//!
//! ```rust,ignore
//! struct ResolvedExpr<'src, 'arena> {
//!     ty: &'arena ResolvedType<'src, 'arena>,  // Arena-allocated type
//!     // ...
//! }
//! ```
//!
//! ## Lifetime Relationships
//!
//! Typically, `'src` outlives `'arena`:
//! - Source code is loaded first and kept in memory
//! - Arena is created for compilation session
//! - Arena is dropped after compilation (while source remains)
//!
//! The borrow checker ensures these lifetimes are used correctly throughout the HIR.
//!
//! # Semantic Analysis Phase
//!
//! The HIR is built during semantic analysis, which consists of several phases:
//!
//! ## 1. Name Resolution
//!
//! - Build symbol tables mapping names to definitions
//! - Resolve variable, function, type, and field references
//! - Detect duplicate definitions and undefined references
//! - Handle scoping rules (shadowing, nested scopes)
//!
//! ## 2. Type Checking
//!
//! - Assign types to all expressions
//! - Verify type compatibility in operations and assignments
//! - Check function argument types against parameter types
//! - Resolve method calls based on receiver types
//! - Handle type inference where types are not explicitly annotated
//!
//! ## 3. Scope Analysis
//!
//! - Track scope levels for each definition
//! - Build scope stacks for nested contexts (functions, blocks, with-statements)
//! - Validate scope-dependent constructs (e.g., dot-prefix in with-blocks)
//! - Clean up scope entries when exiting nested scopes
//!
//! ## 4. Constraint Analysis
//!
//! - Identify constraint expressions in with-blocks
//! - Resolve transform chains for coordinate transformation
//! - Link container field accesses to their containing structs
//! - Prepare constraint system for solver
//!
//! # Cross-References Between Nodes
//!
//! The HIR enables efficient traversal through direct references:
//!
//! ## Expression → Definition
//!
//! ```text
//! ResolvedExpr::Var { definition: &VarDefinition }
//! ResolvedExpr::FunctionCall { function: &FunctionDefinition }
//! ResolvedExpr::MethodCall { method: &FunctionDefinition }
//! ResolvedExpr::FieldAccess { field: &FieldDefinition }
//! ```
//!
//! ## Definition → Type
//!
//! ```text
//! VarDefinition { var_type: ResolvedType }
//! FieldDefinition { field_type: ResolvedType }
//! FunctionParam { param_type: ResolvedType }
//! ```
//!
//! ## Type → Definition
//!
//! ```text
//! ResolvedType::UserDefined { definition: &StructDefinition }
//! ResolvedType::Reference { inner: &ResolvedType }
//! ```
//!
//! ## Method → Struct
//!
//! ```text
//! FunctionDefinition { parent_struct: Option<&StructDefinition> }
//! ```
//!
//! ## With-Context → Container
//!
//! ```text
//! WithContext { container_field: Option<&ContainerField> }
//! ResolvedExpr::ContainerFieldAccess { with_context: &WithContext }
//! ```
//!
//! These cross-references enable:
//! - Fast type checking (no hash map lookups)
//! - Efficient constraint solving (direct access to definitions)
//! - Better error messages (full context available)
//! - Simplified compiler passes (no separate symbol tables needed)
//!
//! # Module Organization
//!
//! The HIR is organized into several submodules:
//!
//! - **hir_types**: Type system (`ResolvedType`)
//! - **hir_definitions**: Definitions (`VarDefinition`, `FunctionDefinition`,
//!   `StructDefinition`, etc.)
//! - **hir_expr**: Expressions (`ResolvedExpr`, `ResolvedExprKind`)
//! - **hir_context**: With-statement contexts (`WithContext`, `TransformMethod`)
//! - **hir_scope**: Scope management (`Scope`, `ScopeStack`)
//!
//! # Example
//!
//! ```text
//! // Source code
//! struct Point { x: f64, y: f64 }
//! let p = Point { x: 10.0, y: 20.0 };
//!
//! // AST (simplified)
//! StructDef { name: "Point", fields: [...] }
//! VarDecl { name: "p", init: StructLit { name: "Point", ... } }
//!
//! // HIR (simplified)
//! StructDefinition { name: "Point", fields: [&FieldDef, &FieldDef] }
//! VarDefinition {
//!     name: "p",
//!     var_type: UserDefined {
//!         name: "Point",
//!         definition: &StructDefinition  // Direct link!
//!     },
//!     init: &ResolvedExpr {
//!         kind: StructLit { ... },
//!         ty: &UserDefined { ... }  // Direct link to type!
//!     }
//! }
//! ```

// ============================================================================
// Submodule Declarations
// ============================================================================

pub mod context;
pub mod definitions;
pub mod expr;
pub mod scope;
pub mod types;

// ============================================================================
// Public Re-exports
// ============================================================================
//
// These re-exports provide a clean public API for the HIR module.
// The `#[allow(unused_imports)]` annotation is necessary because these items
// are re-exported for external use, even though they may not be used within
// this module itself.

#[allow(unused_imports)]
pub use types::ResolvedType;

#[allow(unused_imports)]
pub use definitions::{
    ContainerField, FieldDefinition, FunctionDefinition, FunctionParam, ScopeLevel,
    StructDefinition, VarDefinition,
};

#[allow(unused_imports)]
pub use expr::{
    ResolvedExpr, ResolvedExprKind, ResolvedOptimizeDirective, ResolvedOptimizeDirectiveKind,
    ResolvedStmt, ResolvedStmtKind, ResolvedStructLitField,
};

#[allow(unused_imports)]
pub use context::{TransformMethod, TransformMethodKind, WithContext};

#[allow(unused_imports)]
pub use scope::{Scope, ScopeStack};
