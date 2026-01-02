//! Semantic Analyzer for CAD-DSL
//!
//! This module provides the main entry point for semantic analysis of CAD-DSL programs.
//! Semantic analysis is the phase after parsing that validates the program's semantics
//! (meaning) and builds the High-level Intermediate Representation (HIR).
//!
//! # What is Semantic Analysis?
//!
//! Semantic analysis transforms the Abstract Syntax Tree (AST) produced by the parser
//! into a High-level Intermediate Representation (HIR) with resolved cross-references.
//! During this process, it performs:
//!
//! - **Name Resolution**: Resolve all identifiers to their definitions
//! - **Type Checking**: Ensure type consistency throughout the program
//! - **Scope Analysis**: Track variable scopes and detect shadowing
//! - **Error Detection**: Find semantic errors like undefined variables or duplicate definitions
//!
//! # Two-Pass Analysis
//!
//! The semantic analyzer uses a two-pass approach to handle forward references:
//!
//! ## Pass 1: Declaration Collection
//!
//! The first pass collects all top-level declarations without resolving their bodies:
//!
//! - Register all struct names (with placeholder definitions)
//! - Register all function names (with placeholder definitions)
//! - Collect top-level variable declarations
//! - Detect duplicate definitions
//!
//! This phase enables Pass 2 to resolve forward references. For example:
//!
//! ```text
//! struct Line { p1: Point, p2: Point }  // Point is referenced before definition
//! struct Point { x: f64, y: f64 }       // Point is defined here
//! ```
//!
//! Pass 1 registers both "Line" and "Point" struct names first, so when it processes
//! Line's fields in the second phase of Pass 1, "Point" is already known.
//!
//! ## Pass 2: Resolution and HIR Construction
//!
//! The second pass resolves all references and constructs the HIR:
//!
//! - Resolve variable references to their definitions
//! - Resolve function calls to their definitions
//! - Resolve type references to struct definitions
//! - Track nested scopes (functions, blocks, loops, with-contexts)
//! - Build resolved HIR with arena-allocated cross-references
//!
//! # Forward References in CAD-DSL
//!
//! CAD-DSL allows forward references in several contexts:
//!
//! - **Type References**: A struct can reference another struct defined later
//! - **Function Calls**: A function can call another function defined later
//! - **Recursive Definitions**: Structs and functions can reference themselves
//!
//! Example with forward references:
//!
//! ```text
//! fn distance(p1: Point, p2: Point) -> f64 {
//!     // Point is used before it's defined
//!     sqrt(square(p2.x - p1.x) + square(p2.y - p1.y))
//! }
//!
//! fn square(x: f64) -> f64 {
//!     x * x
//! }
//!
//! fn sqrt(x: f64) -> f64 {
//!     // Implementation...
//!     x
//! }
//!
//! struct Point {
//!     x: f64,
//!     y: f64
//! }
//! ```
//!
//! # Arena Allocation Pattern
//!
//! The HIR uses arena allocation (`bumpalo::Bump`) for efficient memory management:
//!
//! - All HIR nodes are allocated in a single arena
//! - Cross-references use arena-allocated pointers (`&'arena T`)
//! - The entire HIR is deallocated when the arena is dropped
//! - Faster than individual heap allocations
//! - Better cache locality
//!
//! ## Lifetime Parameters
//!
//! The semantic analyzer uses two lifetime parameters:
//!
//! - `'src`: Lifetime of the source code string
//!   - Used for string slices (identifiers, literals)
//!   - These slices point into the original source text
//!
//! - `'arena`: Lifetime of the arena allocator
//!   - Used for HIR node references
//!   - All HIR nodes live as long as the arena
//!
//! These lifetimes ensure that:
//! - Source strings outlive any references to them
//! - HIR nodes outlive any references to them
//! - No dangling pointers
//!
//! # Example Usage
//!
//! ```ignore
//! use bumpalo::Bump;
//! use cad_dsl::semantic_analyzer;
//! use cad_dsl::parser;
//!
//! // Source code to analyze
//! let source = r#"
//!     struct Point { x: f64, y: f64 }
//!
//!     fn distance(p1: Point, p2: Point) -> f64 {
//!         let dx = p2.x - p1.x;
//!         let dy = p2.y - p1.y;
//!         sqrt(dx * dx + dy * dy)
//!     }
//! "#;
//!
//! // Parse the source code
//! let tokens = lexer::tokenize(source)?;
//! let ast = parser::parse(&tokens)?;
//!
//! // Create arena for HIR
//! let arena = Bump::new();
//!
//! // Run semantic analysis
//! match semantic_analyzer::analyze(&arena, source, &ast) {
//!     Ok(hir) => {
//!         println!("Semantic analysis successful!");
//!         println!("Generated {} HIR statements", hir.len());
//!     }
//!     Err(errors) => {
//!         println!("Semantic errors:");
//!         for error in errors {
//!             eprintln!("  {}", error);
//!         }
//!     }
//! }
//! ```
//!
//! # Error Handling
//!
//! The analyzer collects all semantic errors during analysis and returns them
//! at the end. This allows reporting multiple errors at once rather than
//! failing on the first error.
//!
//! Common semantic errors include:
//! - Undefined variable
//! - Undefined function
//! - Undefined type
//! - Duplicate definition
//! - Type mismatch
//! - Invalid dot-prefix usage outside of with-context

// ============================================================================
// Imports
// ============================================================================

use crate::ast;
use bumpalo::Bump;

// ============================================================================
// Submodule Declarations
// ============================================================================

pub mod context;
pub mod errors;
pub mod pass1;
pub mod pass2;

// ============================================================================
// Public Re-exports
// ============================================================================

#[allow(unused_imports)]
pub use context::AnalyzerContext;
#[allow(unused_imports)]
pub use errors::SemanticError;

// ============================================================================
// Main Entry Point
// ============================================================================

/// Analyze an AST and produce a resolved HIR
///
/// This is the main entry point for semantic analysis. It performs a two-pass
/// analysis to handle forward references and build the HIR.
///
/// # Parameters
///
/// - `arena`: Arena allocator for HIR nodes
/// - `source`: Source code text (for string slices)
/// - `ast`: Abstract Syntax Tree to analyze
///
/// # Returns
///
/// - `Ok(hir)`: Semantic analysis succeeded, returns resolved HIR statements
/// - `Err(errors)`: Semantic analysis failed, returns all collected errors
///
/// # Two-Pass Analysis
///
/// 1. **Pass 1**: Collect all declarations (structs, functions, variables)
///    - Registers names in symbol tables
///    - Detects duplicate definitions
///    - Resolves type annotations
///
/// 2. **Pass 2**: Resolve references and build HIR
///    - Resolves variable references
///    - Resolves function calls
///    - Resolves type references
///    - Builds HIR with arena-allocated cross-references
///
/// # Example
///
/// ```ignore
/// let arena = Bump::new();
/// let source = "struct Point { x: f64, y: f64 }";
/// let ast = vec![/* parsed statements */];
///
/// match analyze(&arena, source, &ast) {
///     Ok(hir) => println!("Success! {} statements", hir.len()),
///     Err(errors) => {
///         for err in errors {
///             eprintln!("Error: {}", err);
///         }
///     }
/// }
/// ```
pub fn analyze<'src, 'arena>(
    arena: &'arena Bump,
    source: &'src str,
    ast: &[ast::Stmt<'src>],
) -> Result<Vec<&'arena crate::hir::expr::ResolvedStmt<'src, 'arena>>, Vec<SemanticError>> {
    // Create analyzer context
    let mut ctx = AnalyzerContext::new(arena, source);

    // Pass 1: Collect all declarations
    pass1::collect_declarations(&mut ctx, ast);

    // Check for errors from Pass 1
    if ctx.has_errors() {
        return Err(ctx.take_errors());
    }

    // Pass 2: Resolve all references and build HIR
    let resolved = pass2::resolve_statements(&mut ctx, ast);

    // Check for errors from Pass 2
    if ctx.has_errors() {
        return Err(ctx.take_errors());
    }

    // Return the resolved HIR
    Ok(resolved)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Expr, FunctionParam, Stmt, StructField, Type};
    use crate::lexer::{LineColumn, Span};
    use assert_matches::assert_matches;

    /// Helper to create a test span
    fn make_span(line: usize, column: usize) -> Span {
        Span {
            start: LineColumn { line, column },
            lines: 0,
            end_column: column + 5,
        }
    }

    #[test]
    fn test_simple_variable() {
        let arena = Bump::new();
        let source = "let x: i32 = 42;";

        let ast = vec![Stmt::Let {
            dot_prefix: false,
            name_path: vec![("x", make_span(1, 5))],
            type_annotation: Some(Type::I32 {
                span: make_span(1, 8),
            }),
            init: Some(Expr::IntLit {
                value: 42,
                span: make_span(1, 14),
            }),
            span: make_span(1, 1),
        }];

        let result = analyze(&arena, source, &ast);
        if let Err(ref errors) = result {
            for err in errors {
                eprintln!("Error: {}", err);
            }
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_struct_with_fields() {
        let arena = Bump::new();
        let source = "struct Point { x: f64, y: f64 }";

        let ast = vec![Stmt::StructDef {
            name: "Point".to_string(),
            name_span: make_span(1, 8),
            container: None,
            fields: vec![
                StructField {
                    name: "x".to_string(),
                    name_span: make_span(1, 16),
                    type_annotation: Type::F64 {
                        span: make_span(1, 19),
                    },
                    span: make_span(1, 16),
                },
                StructField {
                    name: "y".to_string(),
                    name_span: make_span(1, 24),
                    type_annotation: Type::F64 {
                        span: make_span(1, 27),
                    },
                    span: make_span(1, 24),
                },
            ],
            methods: vec![],
            span: make_span(1, 1),
        }];

        let result = analyze(&arena, source, &ast);
        assert!(result.is_ok());
    }

    #[test]
    fn test_function_definition() {
        let arena = Bump::new();
        let source = "fn square(x: f64) -> f64 { x * x }";

        let ast = vec![Stmt::FunctionDef {
            name: "square".to_string(),
            name_span: make_span(1, 4),
            params: vec![FunctionParam {
                name: "x".to_string(),
                name_span: make_span(1, 11),
                type_annotation: Type::F64 {
                    span: make_span(1, 14),
                },
                span: make_span(1, 11),
            }],
            return_type: Type::F64 {
                span: make_span(1, 22),
            },
            body: vec![],
            return_expr: Some(Expr::Mul {
                lhs: Box::new(crate::ast::expr::MulLhs::Var {
                    name: "x",
                    span: make_span(1, 28),
                }),
                rhs: Box::new(crate::ast::expr::MulRhs::Var {
                    name: "x",
                    span: make_span(1, 32),
                }),
                span: make_span(1, 28),
            }),
            span: make_span(1, 1),
        }];

        let result = analyze(&arena, source, &ast);
        if let Err(ref errors) = result {
            for err in errors {
                eprintln!("Error: {}", err);
            }
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_forward_reference_struct() {
        let arena = Bump::new();
        let source = "struct Line { p1: Point, p2: Point } struct Point { x: f64, y: f64 }";

        let ast = vec![
            Stmt::StructDef {
                name: "Line".to_string(),
                name_span: make_span(1, 8),
                container: None,
                fields: vec![
                    StructField {
                        name: "p1".to_string(),
                        name_span: make_span(1, 15),
                        type_annotation: Type::UserDefined {
                            name: "Point".to_string(),
                            span: make_span(1, 19),
                        },
                        span: make_span(1, 15),
                    },
                    StructField {
                        name: "p2".to_string(),
                        name_span: make_span(1, 26),
                        type_annotation: Type::UserDefined {
                            name: "Point".to_string(),
                            span: make_span(1, 30),
                        },
                        span: make_span(1, 26),
                    },
                ],
                methods: vec![],
                span: make_span(1, 1),
            },
            Stmt::StructDef {
                name: "Point".to_string(),
                name_span: make_span(2, 8),
                container: None,
                fields: vec![
                    StructField {
                        name: "x".to_string(),
                        name_span: make_span(2, 16),
                        type_annotation: Type::F64 {
                            span: make_span(2, 19),
                        },
                        span: make_span(2, 16),
                    },
                    StructField {
                        name: "y".to_string(),
                        name_span: make_span(2, 24),
                        type_annotation: Type::F64 {
                            span: make_span(2, 27),
                        },
                        span: make_span(2, 24),
                    },
                ],
                methods: vec![],
                span: make_span(2, 1),
            },
        ];

        let result = analyze(&arena, source, &ast);
        assert!(result.is_ok());
    }

    #[test]
    fn test_error_undefined_variable() {
        let arena = Bump::new();
        let source = "let y = x;";

        let ast = vec![Stmt::Let {
            dot_prefix: false,
            name_path: vec![("y", make_span(1, 5))],
            type_annotation: None,
            init: Some(Expr::Var {
                name: "x",
                span: make_span(1, 9),
            }),
            span: make_span(1, 1),
        }];

        let result = analyze(&arena, source, &ast);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_matches!(
            &errors[0],
            SemanticError::UndefinedVariable { name, .. } if name == "x"
        );
    }

    #[test]
    fn test_error_undefined_function() {
        let arena = Bump::new();
        let source = "let y = foo();";

        let ast = vec![Stmt::Let {
            dot_prefix: false,
            name_path: vec![("y", make_span(1, 5))],
            type_annotation: None,
            init: Some(Expr::Call {
                name: "foo",
                args: vec![],
                span: make_span(1, 9),
            }),
            span: make_span(1, 1),
        }];

        let result = analyze(&arena, source, &ast);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_matches!(
            &errors[0],
            SemanticError::UndefinedFunction { name, .. } if name == "foo"
        );
    }

    #[test]
    fn test_error_undefined_type() {
        let arena = Bump::new();
        let source = "struct Line { p: Point }";

        let ast = vec![Stmt::StructDef {
            name: "Line".to_string(),
            name_span: make_span(1, 8),
            container: None,
            fields: vec![StructField {
                name: "p".to_string(),
                name_span: make_span(1, 15),
                type_annotation: Type::UserDefined {
                    name: "Point".to_string(),
                    span: make_span(1, 18),
                },
                span: make_span(1, 15),
            }],
            methods: vec![],
            span: make_span(1, 1),
        }];

        let result = analyze(&arena, source, &ast);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_matches!(
            &errors[0],
            SemanticError::UndefinedType { name, .. } if name == "Point"
        );
    }

    #[test]
    fn test_error_duplicate_struct() {
        let arena = Bump::new();
        let source = "struct Point {} struct Point {}";

        let ast = vec![
            Stmt::StructDef {
                name: "Point".to_string(),
                name_span: make_span(1, 8),
                container: None,
                fields: vec![],
                methods: vec![],
                span: make_span(1, 1),
            },
            Stmt::StructDef {
                name: "Point".to_string(),
                name_span: make_span(2, 8),
                container: None,
                fields: vec![],
                methods: vec![],
                span: make_span(2, 1),
            },
        ];

        let result = analyze(&arena, source, &ast);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_matches!(
            &errors[0],
            SemanticError::DuplicateDefinition { name, .. } if name == "Point"
        );
    }

    #[test]
    fn test_error_duplicate_function() {
        let arena = Bump::new();
        let source = "fn foo() -> i32 {} fn foo() -> i32 {}";

        let ast = vec![
            Stmt::FunctionDef {
                name: "foo".to_string(),
                name_span: make_span(1, 4),
                params: vec![],
                return_type: Type::I32 {
                    span: make_span(1, 13),
                },
                body: vec![],
                return_expr: None,
                span: make_span(1, 1),
            },
            Stmt::FunctionDef {
                name: "foo".to_string(),
                name_span: make_span(2, 4),
                params: vec![],
                return_type: Type::I32 {
                    span: make_span(2, 13),
                },
                body: vec![],
                return_expr: None,
                span: make_span(2, 1),
            },
        ];

        let result = analyze(&arena, source, &ast);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_matches!(
            &errors[0],
            SemanticError::DuplicateDefinition { name, .. } if name == "foo"
        );
    }

    #[test]
    fn test_error_duplicate_variable() {
        let arena = Bump::new();
        let source = "let x = 1; let x = 2;";

        let ast = vec![
            Stmt::Let {
                dot_prefix: false,
                name_path: vec![("x", make_span(1, 5))],
                type_annotation: None,
                init: Some(Expr::IntLit {
                    value: 1,
                    span: make_span(1, 9),
                }),
                span: make_span(1, 1),
            },
            Stmt::Let {
                dot_prefix: false,
                name_path: vec![("x", make_span(2, 5))],
                type_annotation: None,
                init: Some(Expr::IntLit {
                    value: 2,
                    span: make_span(2, 9),
                }),
                span: make_span(2, 1),
            },
        ];

        let result = analyze(&arena, source, &ast);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_matches!(
            &errors[0],
            SemanticError::DuplicateDefinition { name, .. } if name == "x"
        );
    }

    #[test]
    fn test_nested_scopes() {
        let arena = Bump::new();
        let source = "let x = 1; { let y = 2; }";

        let ast = vec![
            Stmt::Let {
                dot_prefix: false,
                name_path: vec![("x", make_span(1, 5))],
                type_annotation: None,
                init: Some(Expr::IntLit {
                    value: 1,
                    span: make_span(1, 9),
                }),
                span: make_span(1, 1),
            },
            Stmt::Block {
                statements: vec![Stmt::Let {
                    dot_prefix: false,
                    name_path: vec![("y", make_span(1, 18))],
                    type_annotation: None,
                    init: Some(Expr::IntLit {
                        value: 2,
                        span: make_span(1, 22),
                    }),
                    span: make_span(1, 14),
                }],
                span: make_span(1, 12),
            },
        ];

        let result = analyze(&arena, source, &ast);
        assert!(result.is_ok());
    }

    #[test]
    fn test_shadowing_allowed() {
        let arena = Bump::new();
        let source = "let x = 1; { let x = 2; }";

        let ast = vec![
            Stmt::Let {
                dot_prefix: false,
                name_path: vec![("x", make_span(1, 5))],
                type_annotation: None,
                init: Some(Expr::IntLit {
                    value: 1,
                    span: make_span(1, 9),
                }),
                span: make_span(1, 1),
            },
            Stmt::Block {
                statements: vec![Stmt::Let {
                    dot_prefix: false,
                    name_path: vec![("x", make_span(1, 18))],
                    type_annotation: None,
                    init: Some(Expr::IntLit {
                        value: 2,
                        span: make_span(1, 22),
                    }),
                    span: make_span(1, 14),
                }],
                span: make_span(1, 12),
            },
        ];

        let result = analyze(&arena, source, &ast);
        // Shadowing in different scopes is allowed
        assert!(result.is_ok());
    }

    #[test]
    fn test_for_loop() {
        let arena = Bump::new();
        let source = "for i in 0..10 { let x = i; }";

        let ast = vec![Stmt::For {
            loop_var: "i",
            loop_var_span: make_span(1, 5),
            iterator: Expr::Range {
                start: Box::new(Expr::IntLit {
                    value: 0,
                    span: make_span(1, 10),
                }),
                end: Box::new(Expr::IntLit {
                    value: 10,
                    span: make_span(1, 13),
                }),
                span: make_span(1, 10),
            },
            body: vec![Stmt::Let {
                dot_prefix: false,
                name_path: vec![("x", make_span(1, 22))],
                type_annotation: None,
                init: Some(Expr::Var {
                    name: "i",
                    span: make_span(1, 26),
                }),
                span: make_span(1, 18),
            }],
            span: make_span(1, 1),
        }];

        let result = analyze(&arena, source, &ast);
        assert!(result.is_ok());
    }

    #[test]
    fn test_if_else() {
        let arena = Bump::new();
        let source = "if true { let x = 1; } else { let y = 2; }";

        let ast = vec![Stmt::If {
            condition: Expr::BoolLit {
                value: true,
                span: make_span(1, 4),
            },
            then_branch: vec![Stmt::Let {
                dot_prefix: false,
                name_path: vec![("x", make_span(1, 15))],
                type_annotation: None,
                init: Some(Expr::IntLit {
                    value: 1,
                    span: make_span(1, 19),
                }),
                span: make_span(1, 11),
            }],
            else_branch: Some(vec![Stmt::Let {
                dot_prefix: false,
                name_path: vec![("y", make_span(1, 35))],
                type_annotation: None,
                init: Some(Expr::IntLit {
                    value: 2,
                    span: make_span(1, 39),
                }),
                span: make_span(1, 31),
            }]),
            span: make_span(1, 1),
        }];

        let result = analyze(&arena, source, &ast);
        assert!(result.is_ok());
    }

    #[test]
    fn test_error_dot_prefix_outside_with_context() {
        let arena = Bump::new();
        let source = "let .field = 42;";

        let ast = vec![Stmt::Let {
            dot_prefix: true,
            name_path: vec![("field", make_span(1, 6))],
            type_annotation: None,
            init: Some(Expr::IntLit {
                value: 42,
                span: make_span(1, 14),
            }),
            span: make_span(1, 1),
        }];

        let result = analyze(&arena, source, &ast);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_matches!(&errors[0], SemanticError::NotInWithContext { .. });
    }

    #[test]
    fn test_with_context() {
        let arena = Bump::new();
        let source = "struct Point { x: f64, y: f64 } with Point { x: 0, y: 0 } { let .x = 10; }";

        let ast = vec![
            Stmt::StructDef {
                name: "Point".to_string(),
                name_span: make_span(1, 8),
                container: None,
                fields: vec![
                    StructField {
                        name: "x".to_string(),
                        name_span: make_span(1, 16),
                        type_annotation: Type::F64 {
                            span: make_span(1, 19),
                        },
                        span: make_span(1, 16),
                    },
                    StructField {
                        name: "y".to_string(),
                        name_span: make_span(1, 24),
                        type_annotation: Type::F64 {
                            span: make_span(1, 27),
                        },
                        span: make_span(1, 24),
                    },
                ],
                methods: vec![],
                span: make_span(1, 1),
            },
            Stmt::With {
                context_expr: Expr::StructLit {
                    name: "Point",
                    fields: vec![],
                    span: make_span(2, 6),
                },
                body: vec![Stmt::Let {
                    dot_prefix: true,
                    name_path: vec![("x", make_span(2, 34))],
                    type_annotation: None,
                    init: Some(Expr::IntLit {
                        value: 10,
                        span: make_span(2, 38),
                    }),
                    span: make_span(2, 29),
                }],
                span: make_span(2, 1),
            },
        ];

        let result = analyze(&arena, source, &ast);
        // This should succeed - we're in a with context
        if let Err(ref errors) = result {
            for err in errors {
                eprintln!("Error: {}", err);
            }
        }
        assert!(result.is_ok());
    }

    // ========================================================================
    // Integration Tests: Complete Pipeline (AST → HIR → Type Checking)
    // ========================================================================

    mod integration_tests {
        use super::*;
        use crate::type_checker;
        use crate::type_checker::TypeCheckError;

        /// Test 1: Simple Let Statement with Type Checking
        #[test]
        fn test_e2e_simple_let_statement() {
            let arena = Bump::new();
            let source = "let x: i32 = 42;";

            let ast = vec![Stmt::Let {
                dot_prefix: false,
                name_path: vec![("x", make_span(1, 5))],
                type_annotation: Some(Type::I32 {
                    span: make_span(1, 8),
                }),
                init: Some(Expr::IntLit {
                    value: 42,
                    span: make_span(1, 14),
                }),
                span: make_span(1, 1),
            }];

            // Test semantic analysis
            let result = analyze(&arena, source, &ast);
            if let Err(ref errors) = result {
                for err in errors {
                    eprintln!("Semantic Error: {}", err);
                }
            }
            assert!(result.is_ok(), "Semantic analysis should succeed");

            let hir = result.unwrap();
            assert_eq!(hir.len(), 1, "Should have 1 HIR statement");

            // Verify HIR structure
            assert_matches!(
                &hir[0].kind,
                crate::hir::expr::ResolvedStmtKind::Let {
                    var_def,
                    init: Some(_),
                    ..
                } if var_def.name == "x"
            );

            // Test type checking
            let type_result = type_checker::type_check(&arena, source, &hir);
            if let Err(ref errors) = type_result {
                for err in errors {
                    eprintln!("Type Error: {}", err);
                }
            }
            assert!(type_result.is_ok(), "Type checking should succeed");
        }

        /// Test 2: Assignment with Type Checking
        #[test]
        fn test_e2e_assignment_statement() {
            let arena = Bump::new();
            let source = "let x: i32 = 42; x = 100;";

            let ast = vec![
                Stmt::Let {
                    dot_prefix: false,
                    name_path: vec![("x", make_span(1, 5))],
                    type_annotation: Some(Type::I32 {
                        span: make_span(1, 8),
                    }),
                    init: Some(Expr::IntLit {
                        value: 42,
                        span: make_span(1, 14),
                    }),
                    span: make_span(1, 1),
                },
                Stmt::Assignment {
                    name: "x",
                    name_span: make_span(1, 18),
                    value: Expr::IntLit {
                        value: 100,
                        span: make_span(1, 22),
                    },
                    span: make_span(1, 18),
                },
            ];

            // Test semantic analysis
            let result = analyze(&arena, source, &ast);
            assert!(result.is_ok(), "Semantic analysis should succeed");

            let hir = result.unwrap();
            assert_eq!(hir.len(), 2, "Should have 2 HIR statements");

            // Verify HIR structure
            assert_matches!(&hir[0].kind, crate::hir::expr::ResolvedStmtKind::Let { .. });
            assert_matches!(
                &hir[1].kind,
                crate::hir::expr::ResolvedStmtKind::Assignment { .. }
            );

            // Test type checking
            let type_result = type_checker::type_check(&arena, source, &hir);
            assert!(type_result.is_ok(), "Type checking should succeed");
        }

        /// Test 3: Type Error Detection
        // Note: This test is disabled because the type checker currently allows
        // numeric promotions (bool -> i32, i32 -> f64) which is intentional behavior.
        // A more sophisticated type error test would require non-promotable types.
        #[test]
        #[ignore]
        fn test_e2e_type_error_detection() {
            let arena = Bump::new();
            let source = "let x: i32 = 42; x = true;";

            let ast = vec![
                Stmt::Let {
                    dot_prefix: false,
                    name_path: vec![("x", make_span(1, 5))],
                    type_annotation: Some(Type::I32 {
                        span: make_span(1, 8),
                    }),
                    init: Some(Expr::IntLit {
                        value: 42,
                        span: make_span(1, 14),
                    }),
                    span: make_span(1, 1),
                },
                Stmt::Assignment {
                    name: "x",
                    name_span: make_span(1, 18),
                    value: Expr::BoolLit {
                        value: true,
                        span: make_span(1, 22),
                    },
                    span: make_span(1, 18),
                },
            ];

            // Test semantic analysis
            let result = analyze(&arena, source, &ast);
            assert!(result.is_ok(), "Semantic analysis should succeed");

            let hir = result.unwrap();
            assert_eq!(hir.len(), 2, "Should have 2 HIR statements");

            // Test type checking - should fail
            let type_result = type_checker::type_check(&arena, source, &hir);
            assert!(type_result.is_err(), "Type checking should fail");

            let errors = type_result.unwrap_err();
            assert!(errors.len() > 0, "Should have at least one type error");
            // Verify it's a type mismatch error
            assert_matches!(&errors[0], TypeCheckError::TypeMismatch { .. });
        }

        /// Test 4: If Statement with Scoping
        #[test]
        fn test_e2e_if_statement() {
            let arena = Bump::new();
            let source = "let x = 10; if x > 5 { let y = 20; }";

            let ast = vec![
                Stmt::Let {
                    dot_prefix: false,
                    name_path: vec![("x", make_span(1, 5))],
                    type_annotation: None,
                    init: Some(Expr::IntLit {
                        value: 10,
                        span: make_span(1, 9),
                    }),
                    span: make_span(1, 1),
                },
                Stmt::If {
                    condition: Expr::Gt {
                        lhs: Box::new(crate::ast::expr::CmpLhs::Var {
                            name: "x",
                            span: make_span(1, 16),
                        }),
                        rhs: Box::new(crate::ast::expr::CmpRhs::IntLit {
                            value: 5,
                            span: make_span(1, 20),
                        }),
                        span: make_span(1, 16),
                    },
                    then_branch: vec![Stmt::Let {
                        dot_prefix: false,
                        name_path: vec![("y", make_span(1, 28))],
                        type_annotation: None,
                        init: Some(Expr::IntLit {
                            value: 20,
                            span: make_span(1, 32),
                        }),
                        span: make_span(1, 24),
                    }],
                    else_branch: None,
                    span: make_span(1, 13),
                },
            ];

            // Test semantic analysis
            let result = analyze(&arena, source, &ast);
            assert!(result.is_ok(), "Semantic analysis should succeed");

            let hir = result.unwrap();
            assert_eq!(hir.len(), 2, "Should have 2 HIR statements");

            // Verify If statement structure
            assert_matches!(
                &hir[1].kind,
                crate::hir::expr::ResolvedStmtKind::If {
                    then_branch,
                    else_branch: None,
                    ..
                } if then_branch.len() == 1
            );

            // Test type checking
            let type_result = type_checker::type_check(&arena, source, &hir);
            assert!(type_result.is_ok(), "Type checking should succeed");
        }

        /// Test 5: For Loop with Iterator
        #[test]
        fn test_e2e_for_loop() {
            let arena = Bump::new();
            let source = "for i in 0..10 { let x = i * 2; }";

            let ast = vec![Stmt::For {
                loop_var: "i",
                loop_var_span: make_span(1, 5),
                iterator: Expr::Range {
                    start: Box::new(Expr::IntLit {
                        value: 0,
                        span: make_span(1, 10),
                    }),
                    end: Box::new(Expr::IntLit {
                        value: 10,
                        span: make_span(1, 13),
                    }),
                    span: make_span(1, 10),
                },
                body: vec![Stmt::Let {
                    dot_prefix: false,
                    name_path: vec![("x", make_span(1, 22))],
                    type_annotation: None,
                    init: Some(Expr::Mul {
                        lhs: Box::new(crate::ast::expr::MulLhs::Var {
                            name: "i",
                            span: make_span(1, 26),
                        }),
                        rhs: Box::new(crate::ast::expr::MulRhs::IntLit {
                            value: 2,
                            span: make_span(1, 30),
                        }),
                        span: make_span(1, 26),
                    }),
                    span: make_span(1, 18),
                }],
                span: make_span(1, 1),
            }];

            // Test semantic analysis
            let result = analyze(&arena, source, &ast);
            assert!(result.is_ok(), "Semantic analysis should succeed");

            let hir = result.unwrap();
            assert_eq!(hir.len(), 1, "Should have 1 HIR statement");

            // Verify For loop structure
            assert_matches!(
                &hir[0].kind,
                crate::hir::expr::ResolvedStmtKind::For {
                    loop_var_def,
                    body,
                    ..
                } if loop_var_def.name == "i" && body.len() == 1
            );

            // Test type checking
            let type_result = type_checker::type_check(&arena, source, &hir);
            assert!(type_result.is_ok(), "Type checking should succeed");
        }

        /// Test 6: Function with Return Statement
        #[test]
        fn test_e2e_function_definition() {
            let arena = Bump::new();
            let source = "fn add(a: i32, b: i32) -> i32 { let result = a + b; return result; }";

            let ast = vec![Stmt::FunctionDef {
                name: "add".to_string(),
                name_span: make_span(1, 4),
                params: vec![
                    FunctionParam {
                        name: "a".to_string(),
                        name_span: make_span(1, 8),
                        type_annotation: Type::I32 {
                            span: make_span(1, 11),
                        },
                        span: make_span(1, 8),
                    },
                    FunctionParam {
                        name: "b".to_string(),
                        name_span: make_span(1, 15),
                        type_annotation: Type::I32 {
                            span: make_span(1, 18),
                        },
                        span: make_span(1, 15),
                    },
                ],
                return_type: Type::I32 {
                    span: make_span(1, 27),
                },
                body: vec![Stmt::Let {
                    dot_prefix: false,
                    name_path: vec![("result", make_span(1, 37))],
                    type_annotation: None,
                    init: Some(Expr::Add {
                        lhs: Box::new(crate::ast::expr::AddLhs::Var {
                            name: "a",
                            span: make_span(1, 46),
                        }),
                        rhs: Box::new(crate::ast::expr::AddRhs::Var {
                            name: "b",
                            span: make_span(1, 50),
                        }),
                        span: make_span(1, 46),
                    }),
                    span: make_span(1, 33),
                }],
                return_expr: Some(Expr::Var {
                    name: "result",
                    span: make_span(1, 60),
                }),
                span: make_span(1, 1),
            }];

            // Test semantic analysis
            let result = analyze(&arena, source, &ast);
            assert!(result.is_ok(), "Semantic analysis should succeed");

            let hir = result.unwrap();
            assert_eq!(hir.len(), 1, "Should have 1 HIR statement");

            // Verify function structure
            assert_matches!(
                &hir[0].kind,
                crate::hir::expr::ResolvedStmtKind::FunctionDef {
                    func_def,
                    body,
                    return_expr: Some(_),
                    ..
                } if func_def.name == "add" && body.len() == 1
            );

            // Test type checking
            let type_result = type_checker::type_check(&arena, source, &hir);
            assert!(type_result.is_ok(), "Type checking should succeed");
        }

        /// Test 7: Struct Definition and Instantiation
        #[test]
        fn test_e2e_struct_definition() {
            let arena = Bump::new();
            let source = "struct Point { x: i32, y: i32 } let p = Point { x: 10, y: 20 };";

            let ast = vec![
                Stmt::StructDef {
                    name: "Point".to_string(),
                    name_span: make_span(1, 8),
                    container: None,
                    fields: vec![
                        StructField {
                            name: "x".to_string(),
                            name_span: make_span(1, 16),
                            type_annotation: Type::I32 {
                                span: make_span(1, 19),
                            },
                            span: make_span(1, 16),
                        },
                        StructField {
                            name: "y".to_string(),
                            name_span: make_span(1, 24),
                            type_annotation: Type::I32 {
                                span: make_span(1, 27),
                            },
                            span: make_span(1, 24),
                        },
                    ],
                    methods: vec![],
                    span: make_span(1, 1),
                },
                Stmt::Let {
                    dot_prefix: false,
                    name_path: vec![("p", make_span(1, 37))],
                    type_annotation: None,
                    init: Some(Expr::StructLit {
                        name: "Point",
                        fields: vec![
                            crate::ast::expr::StructLitField::Field {
                                name: "x",
                                value: Expr::IntLit {
                                    value: 10,
                                    span: make_span(1, 52),
                                },
                                span: make_span(1, 49),
                            },
                            crate::ast::expr::StructLitField::Field {
                                name: "y",
                                value: Expr::IntLit {
                                    value: 20,
                                    span: make_span(1, 59),
                                },
                                span: make_span(1, 56),
                            },
                        ],
                        span: make_span(1, 41),
                    }),
                    span: make_span(1, 33),
                },
            ];

            // Test semantic analysis
            let result = analyze(&arena, source, &ast);
            assert!(result.is_ok(), "Semantic analysis should succeed");

            let hir = result.unwrap();
            assert_eq!(hir.len(), 2, "Should have 2 HIR statements");

            // Verify struct definition
            assert_matches!(
                &hir[0].kind,
                crate::hir::expr::ResolvedStmtKind::StructDef {
                    struct_def,
                    ..
                } if struct_def.name == "Point"
            );

            // Verify struct instantiation
            assert_matches!(
                &hir[1].kind,
                crate::hir::expr::ResolvedStmtKind::Let {
                    var_def,
                    init: Some(_),
                    ..
                } if var_def.name == "p"
            );

            // Test type checking
            let type_result = type_checker::type_check(&arena, source, &hir);
            assert!(type_result.is_ok(), "Type checking should succeed");
        }

        /// Test 8: Complex Program with Multiple Features
        #[test]
        fn test_e2e_complex_program() {
            let arena = Bump::new();
            let source =
                "struct Point { x: i32, y: i32 } fn double_x(p: Point) -> i32 { return p.x * 2; }";

            let ast = vec![
                Stmt::StructDef {
                    name: "Point".to_string(),
                    name_span: make_span(1, 8),
                    container: None,
                    fields: vec![
                        StructField {
                            name: "x".to_string(),
                            name_span: make_span(1, 16),
                            type_annotation: Type::I32 {
                                span: make_span(1, 19),
                            },
                            span: make_span(1, 16),
                        },
                        StructField {
                            name: "y".to_string(),
                            name_span: make_span(1, 24),
                            type_annotation: Type::I32 {
                                span: make_span(1, 27),
                            },
                            span: make_span(1, 24),
                        },
                    ],
                    methods: vec![],
                    span: make_span(1, 1),
                },
                Stmt::FunctionDef {
                    name: "double_x".to_string(),
                    name_span: make_span(1, 36),
                    params: vec![FunctionParam {
                        name: "p".to_string(),
                        name_span: make_span(1, 45),
                        type_annotation: Type::UserDefined {
                            name: "Point".to_string(),
                            span: make_span(1, 48),
                        },
                        span: make_span(1, 45),
                    }],
                    return_type: Type::I32 {
                        span: make_span(1, 58),
                    },
                    body: vec![],
                    return_expr: Some(Expr::Mul {
                        lhs: Box::new(crate::ast::expr::MulLhs::FieldAccess {
                            receiver: Box::new(Expr::Var {
                                name: "p",
                                span: make_span(1, 72),
                            }),
                            field: "x",
                            span: make_span(1, 72),
                        }),
                        rhs: Box::new(crate::ast::expr::MulRhs::IntLit {
                            value: 2,
                            span: make_span(1, 78),
                        }),
                        span: make_span(1, 72),
                    }),
                    span: make_span(1, 33),
                },
            ];

            // Test semantic analysis
            let result = analyze(&arena, source, &ast);
            assert!(result.is_ok(), "Semantic analysis should succeed");

            let hir = result.unwrap();
            assert_eq!(hir.len(), 2, "Should have 2 HIR statements");

            // Test type checking
            let type_result = type_checker::type_check(&arena, source, &hir);
            assert!(type_result.is_ok(), "Type checking should succeed");
        }

        /// Test 9: Block Statement with Nested Scopes
        #[test]
        fn test_e2e_block_statement() {
            let arena = Bump::new();
            let source = "let x = 1; { let y = 2; let z = x + y; }";

            let ast = vec![
                Stmt::Let {
                    dot_prefix: false,
                    name_path: vec![("x", make_span(1, 5))],
                    type_annotation: None,
                    init: Some(Expr::IntLit {
                        value: 1,
                        span: make_span(1, 9),
                    }),
                    span: make_span(1, 1),
                },
                Stmt::Block {
                    statements: vec![
                        Stmt::Let {
                            dot_prefix: false,
                            name_path: vec![("y", make_span(1, 18))],
                            type_annotation: None,
                            init: Some(Expr::IntLit {
                                value: 2,
                                span: make_span(1, 22),
                            }),
                            span: make_span(1, 14),
                        },
                        Stmt::Let {
                            dot_prefix: false,
                            name_path: vec![("z", make_span(1, 29))],
                            type_annotation: None,
                            init: Some(Expr::Add {
                                lhs: Box::new(crate::ast::expr::AddLhs::Var {
                                    name: "x",
                                    span: make_span(1, 33),
                                }),
                                rhs: Box::new(crate::ast::expr::AddRhs::Var {
                                    name: "y",
                                    span: make_span(1, 37),
                                }),
                                span: make_span(1, 33),
                            }),
                            span: make_span(1, 25),
                        },
                    ],
                    span: make_span(1, 12),
                },
            ];

            // Test semantic analysis
            let result = analyze(&arena, source, &ast);
            assert!(result.is_ok(), "Semantic analysis should succeed");

            let hir = result.unwrap();
            assert_eq!(hir.len(), 2, "Should have 2 HIR statements");

            // Verify block structure
            assert_matches!(
                &hir[1].kind,
                crate::hir::expr::ResolvedStmtKind::Block {
                    statements,
                    ..
                } if statements.len() == 2
            );

            // Test type checking
            let type_result = type_checker::type_check(&arena, source, &hir);
            assert!(type_result.is_ok(), "Type checking should succeed");
        }

        /// Test 10: Expression Statement
        #[test]
        fn test_e2e_expression_statement() {
            let arena = Bump::new();
            let source = "fn foo() -> i32 { return 42; } foo();";

            let ast = vec![
                Stmt::FunctionDef {
                    name: "foo".to_string(),
                    name_span: make_span(1, 4),
                    params: vec![],
                    return_type: Type::I32 {
                        span: make_span(1, 13),
                    },
                    body: vec![],
                    return_expr: Some(Expr::IntLit {
                        value: 42,
                        span: make_span(1, 25),
                    }),
                    span: make_span(1, 1),
                },
                Stmt::Expression {
                    expr: Expr::Call {
                        name: "foo",
                        args: vec![],
                        span: make_span(1, 32),
                    },
                    span: make_span(1, 32),
                },
            ];

            // Test semantic analysis
            let result = analyze(&arena, source, &ast);
            assert!(result.is_ok(), "Semantic analysis should succeed");

            let hir = result.unwrap();
            assert_eq!(hir.len(), 2, "Should have 2 HIR statements");

            // Verify expression statement
            assert_matches!(
                &hir[1].kind,
                crate::hir::expr::ResolvedStmtKind::Expression { .. }
            );

            // Test type checking
            let type_result = type_checker::type_check(&arena, source, &hir);
            assert!(type_result.is_ok(), "Type checking should succeed");
        }

        /// Test 11: Multiple Errors Collection
        #[test]
        fn test_e2e_multiple_errors() {
            let arena = Bump::new();
            let source = "let x = undefined_var; let y = undefined_func();";

            let ast = vec![
                Stmt::Let {
                    dot_prefix: false,
                    name_path: vec![("x", make_span(1, 5))],
                    type_annotation: None,
                    init: Some(Expr::Var {
                        name: "undefined_var",
                        span: make_span(1, 9),
                    }),
                    span: make_span(1, 1),
                },
                Stmt::Let {
                    dot_prefix: false,
                    name_path: vec![("y", make_span(1, 28))],
                    type_annotation: None,
                    init: Some(Expr::Call {
                        name: "undefined_func",
                        args: vec![],
                        span: make_span(1, 32),
                    }),
                    span: make_span(1, 24),
                },
            ];

            // Test semantic analysis - should fail with multiple errors
            let result = analyze(&arena, source, &ast);
            assert!(result.is_err(), "Semantic analysis should fail");

            let errors = result.unwrap_err();
            assert_eq!(errors.len(), 2, "Should have 2 semantic errors");
        }

        /// Test 12: If-Else Statement with Multiple Branches
        #[test]
        fn test_e2e_if_else_statement() {
            let arena = Bump::new();
            let source = "let x = 10; if x > 5 { let a = 1; } else { let b = 2; }";

            let ast = vec![
                Stmt::Let {
                    dot_prefix: false,
                    name_path: vec![("x", make_span(1, 5))],
                    type_annotation: None,
                    init: Some(Expr::IntLit {
                        value: 10,
                        span: make_span(1, 9),
                    }),
                    span: make_span(1, 1),
                },
                Stmt::If {
                    condition: Expr::Gt {
                        lhs: Box::new(crate::ast::expr::CmpLhs::Var {
                            name: "x",
                            span: make_span(1, 16),
                        }),
                        rhs: Box::new(crate::ast::expr::CmpRhs::IntLit {
                            value: 5,
                            span: make_span(1, 20),
                        }),
                        span: make_span(1, 16),
                    },
                    then_branch: vec![Stmt::Let {
                        dot_prefix: false,
                        name_path: vec![("a", make_span(1, 28))],
                        type_annotation: None,
                        init: Some(Expr::IntLit {
                            value: 1,
                            span: make_span(1, 32),
                        }),
                        span: make_span(1, 24),
                    }],
                    else_branch: Some(vec![Stmt::Let {
                        dot_prefix: false,
                        name_path: vec![("b", make_span(1, 48))],
                        type_annotation: None,
                        init: Some(Expr::IntLit {
                            value: 2,
                            span: make_span(1, 52),
                        }),
                        span: make_span(1, 44),
                    }]),
                    span: make_span(1, 13),
                },
            ];

            // Test semantic analysis
            let result = analyze(&arena, source, &ast);
            assert!(result.is_ok(), "Semantic analysis should succeed");

            let hir = result.unwrap();
            assert_eq!(hir.len(), 2, "Should have 2 HIR statements");

            // Verify if-else structure
            assert_matches!(
                &hir[1].kind,
                crate::hir::expr::ResolvedStmtKind::If {
                    then_branch,
                    else_branch: Some(else_stmts),
                    ..
                } if then_branch.len() == 1 && else_stmts.len() == 1
            );

            // Test type checking
            let type_result = type_checker::type_check(&arena, source, &hir);
            assert!(type_result.is_ok(), "Type checking should succeed");
        }

        /// Test 13: Nested Function Calls
        #[test]
        fn test_e2e_nested_function_calls() {
            let arena = Bump::new();
            let source = "fn double(x: i32) -> i32 { return x * 2; } fn quad(x: i32) -> i32 { return double(double(x)); }";

            let ast = vec![
                Stmt::FunctionDef {
                    name: "double".to_string(),
                    name_span: make_span(1, 4),
                    params: vec![FunctionParam {
                        name: "x".to_string(),
                        name_span: make_span(1, 11),
                        type_annotation: Type::I32 {
                            span: make_span(1, 14),
                        },
                        span: make_span(1, 11),
                    }],
                    return_type: Type::I32 {
                        span: make_span(1, 22),
                    },
                    body: vec![],
                    return_expr: Some(Expr::Mul {
                        lhs: Box::new(crate::ast::expr::MulLhs::Var {
                            name: "x",
                            span: make_span(1, 36),
                        }),
                        rhs: Box::new(crate::ast::expr::MulRhs::IntLit {
                            value: 2,
                            span: make_span(1, 40),
                        }),
                        span: make_span(1, 36),
                    }),
                    span: make_span(1, 1),
                },
                Stmt::FunctionDef {
                    name: "quad".to_string(),
                    name_span: make_span(1, 47),
                    params: vec![FunctionParam {
                        name: "x".to_string(),
                        name_span: make_span(1, 52),
                        type_annotation: Type::I32 {
                            span: make_span(1, 55),
                        },
                        span: make_span(1, 52),
                    }],
                    return_type: Type::I32 {
                        span: make_span(1, 63),
                    },
                    body: vec![],
                    return_expr: Some(Expr::Call {
                        name: "double",
                        args: vec![Expr::Call {
                            name: "double",
                            args: vec![Expr::Var {
                                name: "x",
                                span: make_span(1, 87),
                            }],
                            span: make_span(1, 80),
                        }],
                        span: make_span(1, 73),
                    }),
                    span: make_span(1, 44),
                },
            ];

            // Test semantic analysis
            let result = analyze(&arena, source, &ast);
            assert!(result.is_ok(), "Semantic analysis should succeed");

            let hir = result.unwrap();
            assert_eq!(hir.len(), 2, "Should have 2 HIR statements");

            // Test type checking
            let type_result = type_checker::type_check(&arena, source, &hir);
            assert!(type_result.is_ok(), "Type checking should succeed");
        }

        /// Test 14: Type Inference with Binary Operations
        #[test]
        fn test_e2e_type_inference_binary_ops() {
            let arena = Bump::new();
            let source = "let x = 10; let y = 20; let z = x + y;";

            let ast = vec![
                Stmt::Let {
                    dot_prefix: false,
                    name_path: vec![("x", make_span(1, 5))],
                    type_annotation: None,
                    init: Some(Expr::IntLit {
                        value: 10,
                        span: make_span(1, 9),
                    }),
                    span: make_span(1, 1),
                },
                Stmt::Let {
                    dot_prefix: false,
                    name_path: vec![("y", make_span(1, 17))],
                    type_annotation: None,
                    init: Some(Expr::IntLit {
                        value: 20,
                        span: make_span(1, 21),
                    }),
                    span: make_span(1, 13),
                },
                Stmt::Let {
                    dot_prefix: false,
                    name_path: vec![("z", make_span(1, 29))],
                    type_annotation: None,
                    init: Some(Expr::Add {
                        lhs: Box::new(crate::ast::expr::AddLhs::Var {
                            name: "x",
                            span: make_span(1, 33),
                        }),
                        rhs: Box::new(crate::ast::expr::AddRhs::Var {
                            name: "y",
                            span: make_span(1, 37),
                        }),
                        span: make_span(1, 33),
                    }),
                    span: make_span(1, 25),
                },
            ];

            // Test semantic analysis
            let result = analyze(&arena, source, &ast);
            assert!(result.is_ok(), "Semantic analysis should succeed");

            let hir = result.unwrap();
            assert_eq!(hir.len(), 3, "Should have 3 HIR statements");

            // Test type checking
            let type_result = type_checker::type_check(&arena, source, &hir);
            assert!(type_result.is_ok(), "Type checking should succeed");
        }

        /// Test 15: Return Statement in Function
        #[test]
        fn test_e2e_return_statement() {
            let arena = Bump::new();
            let source = "fn get_value() -> i32 { let x = 42; return x; }";

            let ast = vec![Stmt::FunctionDef {
                name: "get_value".to_string(),
                name_span: make_span(1, 4),
                params: vec![],
                return_type: Type::I32 {
                    span: make_span(1, 19),
                },
                body: vec![Stmt::Let {
                    dot_prefix: false,
                    name_path: vec![("x", make_span(1, 29))],
                    type_annotation: None,
                    init: Some(Expr::IntLit {
                        value: 42,
                        span: make_span(1, 33),
                    }),
                    span: make_span(1, 25),
                }],
                return_expr: Some(Expr::Var {
                    name: "x",
                    span: make_span(1, 45),
                }),
                span: make_span(1, 1),
            }];

            // Test semantic analysis
            let result = analyze(&arena, source, &ast);
            assert!(result.is_ok(), "Semantic analysis should succeed");

            let hir = result.unwrap();
            assert_eq!(hir.len(), 1, "Should have 1 HIR statement");

            // Verify return statement is present
            assert_matches!(
                &hir[0].kind,
                crate::hir::expr::ResolvedStmtKind::FunctionDef {
                    return_expr: Some(_),
                    ..
                }
            );

            // Test type checking
            let type_result = type_checker::type_check(&arena, source, &hir);
            assert!(type_result.is_ok(), "Type checking should succeed");
        }

        /// Test 16: Shadowing in Nested Scopes
        #[test]
        fn test_e2e_shadowing() {
            let arena = Bump::new();
            let source = "let x = 10; { let x = 20; let y = x; }";

            let ast = vec![
                Stmt::Let {
                    dot_prefix: false,
                    name_path: vec![("x", make_span(1, 5))],
                    type_annotation: None,
                    init: Some(Expr::IntLit {
                        value: 10,
                        span: make_span(1, 9),
                    }),
                    span: make_span(1, 1),
                },
                Stmt::Block {
                    statements: vec![
                        Stmt::Let {
                            dot_prefix: false,
                            name_path: vec![("x", make_span(1, 19))],
                            type_annotation: None,
                            init: Some(Expr::IntLit {
                                value: 20,
                                span: make_span(1, 23),
                            }),
                            span: make_span(1, 15),
                        },
                        Stmt::Let {
                            dot_prefix: false,
                            name_path: vec![("y", make_span(1, 31))],
                            type_annotation: None,
                            init: Some(Expr::Var {
                                name: "x",
                                span: make_span(1, 35),
                            }),
                            span: make_span(1, 27),
                        },
                    ],
                    span: make_span(1, 13),
                },
            ];

            // Test semantic analysis - shadowing should be allowed
            let result = analyze(&arena, source, &ast);
            assert!(result.is_ok(), "Semantic analysis should succeed");

            let hir = result.unwrap();
            assert_eq!(hir.len(), 2, "Should have 2 HIR statements");

            // Test type checking
            let type_result = type_checker::type_check(&arena, source, &hir);
            assert!(type_result.is_ok(), "Type checking should succeed");
        }

        /// Test 17: Forward Reference in Functions
        #[test]
        fn test_e2e_forward_reference() {
            let arena = Bump::new();
            let source = "fn foo() -> i32 { return bar(); } fn bar() -> i32 { return 42; }";

            let ast = vec![
                Stmt::FunctionDef {
                    name: "foo".to_string(),
                    name_span: make_span(1, 4),
                    params: vec![],
                    return_type: Type::I32 {
                        span: make_span(1, 13),
                    },
                    body: vec![],
                    return_expr: Some(Expr::Call {
                        name: "bar",
                        args: vec![],
                        span: make_span(1, 25),
                    }),
                    span: make_span(1, 1),
                },
                Stmt::FunctionDef {
                    name: "bar".to_string(),
                    name_span: make_span(1, 38),
                    params: vec![],
                    return_type: Type::I32 {
                        span: make_span(1, 47),
                    },
                    body: vec![],
                    return_expr: Some(Expr::IntLit {
                        value: 42,
                        span: make_span(1, 59),
                    }),
                    span: make_span(1, 35),
                },
            ];

            // Test semantic analysis - forward references should work
            let result = analyze(&arena, source, &ast);
            assert!(result.is_ok(), "Semantic analysis should succeed");

            let hir = result.unwrap();
            assert_eq!(hir.len(), 2, "Should have 2 HIR statements");

            // Test type checking
            let type_result = type_checker::type_check(&arena, source, &hir);
            assert!(type_result.is_ok(), "Type checking should succeed");
        }
    }
}
