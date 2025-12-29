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
use crate::semantic_analyzer_pass1;
use crate::semantic_analyzer_pass2;
use bumpalo::Bump;

// ============================================================================
// Re-exports for convenience
// ============================================================================

pub use crate::semantic_analyzer_context::AnalyzerContext;
pub use crate::semantic_analyzer_errors::SemanticError;

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
#[allow(dead_code)] // Will be used when integrated with main CLI
pub fn analyze<'src, 'arena>(
    arena: &'arena Bump,
    source: &'src str,
    ast: &[ast::Stmt<'src>],
) -> Result<Vec<&'arena ast::Stmt<'src>>, Vec<SemanticError>> {
    // Create analyzer context
    let mut ctx = AnalyzerContext::new(arena, source);

    // Pass 1: Collect all declarations
    semantic_analyzer_pass1::collect_declarations(&mut ctx, ast);

    // Check for errors from Pass 1
    if ctx.has_errors() {
        return Err(ctx.take_errors());
    }

    // Pass 2: Resolve all references and build HIR
    let resolved = semantic_analyzer_pass2::resolve_statements(&mut ctx, ast);

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
}
