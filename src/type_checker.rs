//! # Type Checker
//!
//! This module implements type checking for the CAD-DSL language, ensuring that
//! all types are used correctly throughout the program.
//!
//! ## What is Type Checking?
//!
//! Type checking is the process of verifying that a program's type system is
//! sound—that is, that all operations are performed on values of compatible types.
//! The type checker validates that:
//!
//! - Variables are assigned values of compatible types
//! - Function arguments match parameter types
//! - Binary operations use operands of compatible types
//! - Control flow conditions are boolean expressions
//! - Struct fields are initialized with correct types
//!
//! ## Type Inference vs Type Validation
//!
//! The CAD-DSL type checker operates in two complementary modes:
//!
//! ### Type Inference
//!
//! Type inference automatically deduces the types of expressions without requiring
//! explicit type annotations. For example:
//!
//! ```text
//! let x = 42;        // Inferred as i32
//! let y = 3.14;      // Inferred as f64
//! let z = x + 5;     // Inferred as i32 (both operands are i32)
//! ```
//!
//! The inference engine uses a constraint-based approach inspired by Hindley-Milner
//! type inference, collecting constraints from expression contexts and solving them
//! to determine the most general type for each expression.
//!
//! ### Type Validation
//!
//! Type validation checks that inferred or explicitly declared types are used
//! correctly. It validates:
//!
//! - **Type annotations**: Declared types match expression types
//! - **Assignments**: Right-hand side types are compatible with left-hand side
//! - **Function calls**: Argument types match parameter types
//! - **Operations**: Operand types are valid for the operation
//! - **Control flow**: Conditions are boolean, branches have compatible types
//!
//! Example validation:
//!
//! ```text
//! let x: i32 = 42;      // OK: 42 inferred as i32, matches annotation
//! let y: i32 = true;    // ERROR: bool is not compatible with i32
//! let z: i32 = 3.14;    // ERROR: f64 is not compatible with i32
//! ```
//!
//! ## Hindley-Milner Inspiration
//!
//! This type checker is inspired by the Hindley-Milner (HM) type system, which
//! powers languages like ML, Haskell, and OCaml. Key concepts borrowed from HM:
//!
//! - **Type Variables**: Unknown types represented as variables that are solved
//!   through constraint collection and unification
//! - **Constraint Generation**: Type relationships are expressed as constraints
//!   (e.g., "type of x equals type of y")
//! - **Unification**: Constraints are solved to find the most general type that
//!   satisfies all constraints
//! - **Let-Polymorphism**: Variables can have polymorphic types that are
//!   instantiated at each use site (planned feature)
//!
//! However, CAD-DSL simplifies the HM system:
//!
//! - **Simpler Type System**: No higher-kinded types or rank-N polymorphism
//! - **Explicit Primitives**: Built-in types (i32, f64, bool, string) are known
//! - **Numeric Coercion**: Automatic promotion from i32 to f64 where appropriate
//! - **Constraint-Oriented**: Designed to integrate with Z3 constraint solver
//!
//! ## Architecture
//!
//! The type checker is organized into four submodules:
//!
//! ### 1. `type_checker_errors` - Error Types
//!
//! Defines all type checking errors with detailed context:
//! - Type mismatches (expected vs found)
//! - Inference failures
//! - Incompatible operations
//! - Argument mismatches
//! - Numeric/boolean requirement violations
//!
//! ### 2. `type_checker_context` - Analysis Context
//!
//! Maintains state during type checking:
//! - Arena allocator for type data structures
//! - Source code reference for error reporting
//! - Constraint collection (type equality, compatibility)
//! - Error accumulation
//! - Type variable generation
//!
//! ### 3. `type_checker_inference` - Type Inference
//!
//! Implements type inference for expressions:
//! - Literal type inference (integers, floats, booleans, strings)
//! - Binary operation type inference with numeric promotion
//! - Variable reference type lookup
//! - Function call return type inference
//! - Struct literal type inference
//! - Array and range type inference
//!
//! ### 4. `type_checker_validation` - Type Validation
//!
//! Validates types in statements:
//! - Let statement initialization checking
//! - Assignment compatibility validation
//! - Function call argument checking (count and types)
//! - Conditional expression boolean validation
//! - Control flow type consistency
//!
//! ## Usage
//!
//! The main entry point is the `type_check` function:
//!
//! ```rust,ignore
//! use bumpalo::Bump;
//! use cad_dsl::{lexer, parser, semantic_analyzer, type_checker};
//!
//! let source = r#"
//!     let x: i32 = 42;
//!     let y: f64 = 3.14;
//!     let z = x + 10;  // Infers i32
//! "#;
//!
//! // Parse and analyze
//! let tokens = lexer::tokenize(source)?;
//! let ast = parser::parse(&tokens)?;
//! let arena = Bump::new();
//! let hir = semantic_analyzer::analyze(&arena, source, &ast)?;
//!
//! // Type check
//! match type_checker::type_check(&arena, source, &hir) {
//!     Ok(()) => println!("Type checking succeeded!"),
//!     Err(errors) => {
//!         for error in errors {
//!             eprintln!("Type error: {}", error);
//!         }
//!     }
//! }
//! ```
//!
//! ## Error Reporting
//!
//! All type errors include:
//! - Descriptive error messages
//! - Source code spans (line, column positions)
//! - Expected vs found type information
//! - Contextual information (operation, variable name, etc.)
//!
//! Errors can be formatted with the Ariadne library for rich diagnostics
//! with source code snippets and highlighting.
//!
//! ## Future Enhancements
//!
//! Planned features for the type checker:
//!
//! - **Generics**: Parametric polymorphism for functions and structs
//! - **Type Aliases**: User-defined type names
//! - **Union Types**: Sum types for variant data
//! - **Trait/Interface System**: Structural typing for constraints
//! - **Z3 Integration**: Constraint solving for geometric relationships
//! - **Error Recovery**: Continue type checking after errors for better IDE support

// ============================================================================
// Submodule Declarations
// ============================================================================

pub mod context;
pub mod errors;
pub mod inference;
pub mod rune_integration;
pub mod validation;

// ============================================================================
// Public Re-exports
// ============================================================================

#[allow(unused_imports)]
pub use crate::hir::expr::ResolvedStmt;
#[allow(unused_imports)]
pub use context::TypeCheckContext;
#[allow(unused_imports)]
pub use errors::TypeCheckError;

// ============================================================================
// Imports
// ============================================================================

use bumpalo::Bump;

/// Type check a CAD-DSL program
///
/// This is the main entry point for type checking. It takes a slice of resolved
/// statements from the HIR and validates that all types are used correctly
/// throughout the program.
///
/// # Type Checking Process
///
/// The type checker performs the following steps:
///
/// 1. **Initialize Context**: Create a type checking context with the arena
///    allocator, source code reference, and empty error collection.
///
/// 2. **Validate Each Statement**: Iterate through all statements and validate
///    them using the validation module. This checks:
///    - Let statements have compatible initializers
///    - Assignments use compatible types
///    - Function calls have correct argument types and counts
///    - Conditionals have boolean conditions
///    - All expressions have valid types
///
/// 3. **Collect Errors**: Any type errors discovered during validation are
///    accumulated in the context.
///
/// 4. **Return Results**:
///    - If no errors: Return `Ok(())`
///    - If errors exist: Return `Err(errors)` with all type errors
///
/// # Parameters
///
/// - `arena`: Arena allocator for type checking data structures
/// - `source`: Source code string for error reporting
/// - `hir`: Slice of resolved statements from the HIR to type check
///
/// # Returns
///
/// - `Ok(Vec<String>)`: Type checking succeeded, program is well-typed. Returns any warnings.
/// - `Err(Vec<TypeCheckError>)`: Type checking failed, returns all errors
///
/// # Example
///
/// ```rust,ignore
/// use bumpalo::Bump;
/// use cad_dsl::type_checker::type_check;
///
/// let arena = Bump::new();
/// let source = "let x: i32 = 42;";
/// // ... parse and analyze to get HIR ...
/// let hir = vec![/* ... */];
///
/// match type_check(&arena, source, &hir) {
///     Ok(()) => println!("Type checking succeeded!"),
///     Err(errors) => {
///         println!("Type errors found:");
///         for error in errors {
///             eprintln!("  {}", error);
///         }
///     }
/// }
/// ```
///
/// # Error Recovery
///
/// The type checker uses error recovery to find as many type errors as possible
/// in a single pass. Even if an error is found in one statement, type checking
/// continues for subsequent statements. This provides better feedback to users
/// by showing all type errors at once rather than requiring multiple compile cycles.
///
/// # Integration with Semantic Analyzer
///
/// Type checking typically follows semantic analysis in the compilation pipeline:
///
/// 1. **Lexing**: Source → Tokens
/// 2. **Parsing**: Tokens → AST
/// 3. **Semantic Analysis**: AST → HIR (with name resolution)
/// 4. **Type Checking**: HIR → Validated HIR (with type information)
/// 5. **Code Generation**: Validated HIR → Executable code or constraints
///
/// The semantic analyzer resolves all names before type checking begins, so the
/// type checker can assume all references are valid. This separation of concerns
/// makes both phases simpler and more maintainable.
pub fn type_check<'src, 'arena>(
    arena: &'arena Bump,
    source: &'src str,
    hir: &[&'arena ResolvedStmt<'src, 'arena>],
) -> Result<Vec<String>, Vec<TypeCheckError>> {
    // Create type checking context
    let mut ctx = TypeCheckContext::new(arena, source);

    // Validate each statement in the HIR
    for stmt in hir {
        validation::validate_stmt(&mut ctx, stmt);
    }

    // Check if any errors were collected
    if ctx.has_errors() {
        Err(ctx.take_errors())
    } else {
        // Return any warnings collected
        Ok(ctx.take_warnings())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::definitions::VarDefinition;
    use crate::hir::expr::{ResolvedExpr, ResolvedExprKind, ResolvedStmtKind};
    use crate::hir::types::ResolvedType;
    use crate::lexer::{LineColumn, Span};

    /// Helper to create a span for testing
    fn test_span() -> Span {
        Span {
            start: LineColumn { line: 1, column: 1 },
            lines: 0,
            end_column: 1,
        }
    }

    #[test]
    fn test_type_check_empty_program() {
        let arena = Bump::new();
        let source = "";
        let hir: &[&ResolvedStmt] = &[];

        let result = type_check(&arena, source, hir);
        assert!(
            result.is_ok(),
            "Empty program should type check successfully"
        );
    }

    #[test]
    fn test_type_check_simple_let_with_annotation() {
        let arena = Bump::new();
        let source = "let x: i32 = 42;";

        let var_type = arena.alloc(ResolvedType::I32 { span: test_span() });
        let identifier = arena.alloc(crate::hir::definitions::VariableIdentifier::Simple("x"));
        let var_def = arena.alloc(VarDefinition::new(
            identifier,
            "x",
            test_span(),
            Some(ResolvedType::I32 { span: test_span() }),
            crate::hir::definitions::VarDefinitionKind::Uninitialized,
            0,
            test_span(),
        ));
        let init_expr = arena.alloc(ResolvedExpr {
            span: test_span(),
            kind: ResolvedExprKind::IntLit { value: 42 },
            ty: var_type,
        });

        let stmt: &ResolvedStmt = arena.alloc(ResolvedStmt {
            span: test_span(),
            kind: ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("x", test_span())],
                var_def,
                init: Some(init_expr),
                span: test_span(),
            },
        });
        let hir = &[stmt];

        let result = type_check(&arena, source, hir);
        assert!(
            result.is_ok(),
            "Simple let with annotation should type check"
        );
    }

    #[test]
    fn test_type_check_let_without_init() {
        let arena = Bump::new();
        let source = "let x: i32;";

        let identifier = arena.alloc(crate::hir::definitions::VariableIdentifier::Simple("x"));
        let var_def = arena.alloc(VarDefinition::new(
            identifier,
            "x",
            test_span(),
            Some(ResolvedType::I32 { span: test_span() }),
            crate::hir::definitions::VarDefinitionKind::Uninitialized,
            0,
            test_span(),
        ));

        let stmt: &ResolvedStmt = arena.alloc(ResolvedStmt {
            span: test_span(),
            kind: ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("x", test_span())],
                var_def,
                init: None,
                span: test_span(),
            },
        });
        let hir = &[stmt];

        let result = type_check(&arena, source, hir);
        assert!(result.is_ok(), "Let without init should type check");
    }

    #[test]
    fn test_type_check_multiple_statements() {
        let arena = Bump::new();
        let source = "let x: i32 = 42;\nlet y: f64 = 3.14;";

        // First statement: let x: i32 = 42;
        let i32_type = arena.alloc(ResolvedType::I32 { span: test_span() });
        let identifier1 = arena.alloc(crate::hir::definitions::VariableIdentifier::Simple("x"));
        let var_def1 = arena.alloc(VarDefinition::new(
            identifier1,
            "x",
            test_span(),
            Some(ResolvedType::I32 { span: test_span() }),
            crate::hir::definitions::VarDefinitionKind::Uninitialized,
            0,
            test_span(),
        ));
        let init_expr1 = arena.alloc(ResolvedExpr {
            span: test_span(),
            kind: ResolvedExprKind::IntLit { value: 42 },
            ty: i32_type,
        });
        let stmt1: &ResolvedStmt = arena.alloc(ResolvedStmt {
            span: test_span(),
            kind: ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("x", test_span())],
                var_def: var_def1,
                init: Some(init_expr1),
                span: test_span(),
            },
        });

        // Second statement: let y: f64 = 3.14;
        let f64_type = arena.alloc(ResolvedType::F64 { span: test_span() });
        let identifier2 = arena.alloc(crate::hir::definitions::VariableIdentifier::Simple("y"));
        let var_def2 = arena.alloc(VarDefinition::new(
            identifier2,
            "y",
            test_span(),
            Some(ResolvedType::F64 { span: test_span() }),
            crate::hir::definitions::VarDefinitionKind::Uninitialized,
            0,
            test_span(),
        ));
        let init_expr2 = arena.alloc(ResolvedExpr {
            span: test_span(),
            kind: ResolvedExprKind::FloatLit { value: 3.14 },
            ty: f64_type,
        });
        let stmt2: &ResolvedStmt = arena.alloc(ResolvedStmt {
            span: test_span(),
            kind: ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("y", test_span())],
                var_def: var_def2,
                init: Some(init_expr2),
                span: test_span(),
            },
        });

        let hir = &[stmt1, stmt2];

        let result = type_check(&arena, source, hir);
        assert!(
            result.is_ok(),
            "Multiple let statements should type check successfully"
        );
    }

    #[test]
    fn test_type_check_expression_statement() {
        let arena = Bump::new();
        let source = "42;";

        let i32_type = arena.alloc(ResolvedType::I32 { span: test_span() });
        let expr = arena.alloc(ResolvedExpr {
            span: test_span(),
            kind: ResolvedExprKind::IntLit { value: 42 },
            ty: i32_type,
        });

        let stmt: &ResolvedStmt = arena.alloc(ResolvedStmt {
            span: test_span(),
            kind: ResolvedStmtKind::Expression {
                expr,
                span: test_span(),
            },
        });
        let hir = &[stmt];

        let result = type_check(&arena, source, hir);
        assert!(
            result.is_ok(),
            "Expression statement should type check successfully"
        );
    }

    #[test]
    fn test_type_check_block_statement() {
        let arena = Bump::new();
        let source = "{ let x: i32 = 42; }";

        let i32_type = arena.alloc(ResolvedType::I32 { span: test_span() });
        let identifier = arena.alloc(crate::hir::definitions::VariableIdentifier::Simple("x"));
        let var_def = arena.alloc(VarDefinition::new(
            identifier,
            "x",
            test_span(),
            Some(ResolvedType::I32 { span: test_span() }),
            crate::hir::definitions::VarDefinitionKind::Uninitialized,
            1,
            test_span(),
        ));
        let init_expr = arena.alloc(ResolvedExpr {
            span: test_span(),
            kind: ResolvedExprKind::IntLit { value: 42 },
            ty: i32_type,
        });

        let inner_stmt = arena.alloc(ResolvedStmt {
            span: test_span(),
            kind: ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("x", test_span())],
                var_def,
                init: Some(init_expr),
                span: test_span(),
            },
        });

        let stmt: &ResolvedStmt = arena.alloc(ResolvedStmt {
            span: test_span(),
            kind: ResolvedStmtKind::Block {
                statements: vec![inner_stmt],
                span: test_span(),
            },
        });
        let hir = &[stmt];

        let result = type_check(&arena, source, hir);
        assert!(
            result.is_ok(),
            "Block statement should type check successfully"
        );
    }

    // Note: More comprehensive tests that check for actual type errors would require
    // the validation and inference modules to be fully implemented. These tests verify
    // that the type_check function correctly processes statements and returns the
    // appropriate result type.
}
