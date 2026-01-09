//! Semantic Analyzer Pass 1: Declaration Collection
//!
//! This module implements the first pass of semantic analysis, which collects
//! all top-level declarations (structs, functions, variables) without resolving
//! references or type-checking expressions.

// Allow dead code for now since this module is not yet fully integrated
//!
//! # Two-Phase Collection
//!
//! Pass 1 operates in two phases:
//!
//! ## Phase 1: Name Registration
//! - Register all struct names (with placeholder definitions)
//! - Register all function names (with placeholder definitions)
//!
//! This phase ensures that all type names are known before we try to resolve
//! UserDefined types in field and parameter declarations.
//!
//! ## Phase 2: Definition Completion
//! - Create complete struct definitions with fields
//! - Create complete function definitions with parameters
//! - Collect top-level variable declarations
//!
//! Now that all struct names are registered, we can resolve UserDefined types
//! in field type annotations and function parameter types.
//!
//! # What is NOT Resolved
//!
//! - Expression bodies (let initializers)
//! - Function bodies (statements and return expressions)
//! - Complex type resolution (happens in pass 2)
//!
//! # Duplicate Detection
//!
//! This pass detects and reports:
//! - Duplicate struct definitions (same name)
//! - Duplicate function definitions (same name)
//! - Duplicate variable definitions (in the same scope)

use super::context::AnalyzerContext;
use super::errors::SemanticError;
use crate::ast::{
    FunctionParam as AstFunctionParam, Stmt, StructField as AstStructField, Type as AstType,
};
use crate::hir::definitions::{
    FieldDefinition, FunctionDefinition, FunctionParam, ScopeLevel, StructDefinition, VarDefinition,
};
use crate::hir::types::ResolvedType;
use crate::lexer::Span;

// ============================================================================
// Main Collection Function
// ============================================================================

/// Collect all top-level declarations from a list of statements
///
/// This is the main entry point for Pass 1. It performs a two-phase collection:
/// 1. Register all struct and function names
/// 2. Create complete definitions with fields and parameters
///
/// # Parameters
///
/// - `ctx`: The analyzer context (for registration and error collection)
/// - `stmts`: The top-level statements to process
///
/// # Error Handling
///
/// Errors are collected in the context rather than returned, allowing
/// analysis to continue and report multiple errors at once.
pub fn collect_declarations<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    stmts: &[Stmt<'src>],
) {
    // Phase 1: Register all struct and function names
    // This allows UserDefined types to reference these structs
    for stmt in stmts {
        match stmt {
            Stmt::StructDef {
                name,
                name_span,
                span,
                ..
            } => {
                // Create placeholder struct definition with empty fields
                let struct_def = ctx.arena.alloc(StructDefinition::new(
                    extract_name(ctx.source, name),
                    *name_span,
                    vec![], // Empty fields for now
                    vec![], // Empty methods for now
                    None,   // No container field for now
                    *span,
                ));

                // Try to register the struct
                if ctx
                    .register_struct(extract_name(ctx.source, name), struct_def)
                    .is_err()
                {
                    // Duplicate struct definition
                    if let Some(first_def) = ctx.lookup_struct(name) {
                        ctx.add_error(SemanticError::DuplicateDefinition {
                            name: name.clone(),
                            first_span: first_def.name_span,
                            second_span: *name_span,
                        });
                    }
                }
            }
            Stmt::FunctionDef {
                name,
                name_span,
                span,
                ..
            } => {
                // Create placeholder function definition with empty params
                // We use a dummy return type for now (will be resolved in phase 2)
                let func_def = ctx.arena.alloc(FunctionDefinition::new(
                    extract_name(ctx.source, name),
                    *name_span,
                    vec![],                            // Empty params for now
                    ResolvedType::I32 { span: *span }, // Placeholder return type
                    vec![],                            // Empty body
                    None,                              // Not a method
                    *span,
                ));

                // Try to register the function
                if ctx
                    .register_function(extract_name(ctx.source, name), func_def)
                    .is_err()
                {
                    // Duplicate function definition
                    if let Some(first_def) = ctx.lookup_function(name) {
                        ctx.add_error(SemanticError::DuplicateDefinition {
                            name: name.clone(),
                            first_span: first_def.name_span,
                            second_span: *name_span,
                        });
                    }
                }
            }
            _ => {
                // Not a struct or function definition, will be processed in phase 2
            }
        }
    }

    // Phase 2: Create complete definitions with fields and parameters
    // Now that all struct names are registered, we can resolve UserDefined types
    for stmt in stmts {
        match stmt {
            Stmt::StructDef {
                name,
                name_span,
                container,
                fields,
                methods,
                span,
            } => {
                collect_struct_def(
                    ctx,
                    extract_name(ctx.source, name),
                    *name_span,
                    container.as_ref(),
                    fields,
                    methods,
                    *span,
                );
            }
            Stmt::FunctionDef {
                name,
                name_span,
                params,
                return_type,
                span,
                ..
            } => {
                collect_function_def(
                    ctx,
                    extract_name(ctx.source, name),
                    *name_span,
                    params,
                    return_type,
                    *span,
                );
            }
            Stmt::Let {
                dot_prefix,
                name_path,
                type_annotation,
                span,
                ..
            } => {
                // Only collect simple let statements (no dot prefix, single name)
                if !dot_prefix && name_path.len() == 1 {
                    let (name, name_span) = name_path[0];
                    collect_let_stmt(ctx, name, name_span, type_annotation.as_ref(), *span);
                }
                // Complex let statements (dot prefix, paths) are handled in later passes
            }
            _ => {
                // Other statement types are not declarations
            }
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Collect a struct definition with fields
///
/// This creates a complete StructDefinition with resolved field types and
/// updates the registration in the context.
///
/// # Parameters
///
/// - `ctx`: The analyzer context
/// - `name`: The struct name (from source)
/// - `name_span`: Span of the struct name
/// - `container`: Optional container field
/// - `fields`: AST field definitions
/// - `methods`: AST method definitions (not processed in pass 1)
/// - `span`: Full span of the struct definition
fn collect_struct_def<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    name: &'src str,
    name_span: Span,
    container: Option<&(String, Span)>,
    fields: &[AstStructField],
    methods: &[Stmt<'src>],
    span: Span,
) {
    // Resolve field types and create field definitions
    let mut resolved_fields = Vec::new();

    for field in fields {
        // Resolve the field type
        if let Some(field_type) = resolve_type(ctx, &field.type_annotation) {
            // Create field definition
            let field_name = extract_name(ctx.source, &field.name);
            let field_def: &'arena FieldDefinition<'src, 'arena> = ctx.arena.alloc(
                FieldDefinition::new(field_name, field.name_span, field_type, field.span),
            );
            resolved_fields.push(field_def);
        }
        // If type resolution failed, error was already added to context
    }

    // Handle container field if present
    let container_field: Option<&'arena crate::hir::definitions::ContainerField<'src, 'arena>> =
        container.map(|(container_name, container_span)| {
            let container_name_src = extract_name(ctx.source, container_name);
            let cf: &'arena crate::hir::definitions::ContainerField<'src, 'arena> = ctx
                .arena
                .alloc(crate::hir::definitions::ContainerField::new(
                    container_name_src,
                    *container_span,
                    *container_span,
                ));
            cf
        });

    // Create a placeholder struct definition first (needed for parent_struct reference in methods)
    let placeholder_struct_def = ctx.arena.alloc(StructDefinition::new(
        name,
        name_span,
        resolved_fields.clone(),
        vec![],
        container_field,
        span,
    ));

    // Process methods and create method definitions
    let mut resolved_methods: Vec<&'arena FunctionDefinition<'src, 'arena>> = Vec::new();

    for method_stmt in methods {
        if let Stmt::FunctionDef {
            name: method_name,
            name_span: method_name_span,
            params,
            return_type,
            body: _body,
            return_expr: _return_expr,
            span: method_span,
        } = method_stmt
        {
            // Resolve parameter types
            let mut resolved_params = Vec::new();
            for param in params {
                if let Some(param_type) = resolve_type(ctx, &param.type_annotation) {
                    let param_name = extract_name(ctx.source, &param.name);
                    let param_def =
                        FunctionParam::new(param_name, param.name_span, param_type, param.span);
                    resolved_params.push(param_def);
                }
            }

            // Resolve return type
            let method_name_src = extract_name(ctx.source, method_name);
            let resolved_return_type =
                resolve_type(ctx, return_type).unwrap_or(ResolvedType::I32 { span: *method_span });

            // Create method definition with parent_struct reference
            let method_def = ctx.arena.alloc(FunctionDefinition::new(
                method_name_src,
                *method_name_span,
                resolved_params,
                resolved_return_type,
                vec![], // Body not processed in pass 1
                Some(placeholder_struct_def),
                *method_span,
            ));

            resolved_methods.push(method_def);

            // Also register the method as a function so it can be found during inlining
            let _ = ctx.function_definitions.insert(method_name_src, method_def);
        }
    }

    // Create complete struct definition with methods
    let struct_def = ctx.arena.alloc(StructDefinition::new(
        name,
        name_span,
        resolved_fields,
        resolved_methods,
        container_field,
        span,
    ));

    // Replace the placeholder with the complete definition
    // Note: This will succeed because we already registered the name in phase 1
    // If it was a duplicate, we already reported the error
    let _ = ctx.struct_definitions.insert(name, struct_def);
}

/// Collect a function definition with parameters
///
/// This creates a complete FunctionDefinition with resolved parameter types
/// and updates the registration in the context.
///
/// # Parameters
///
/// - `ctx`: The analyzer context
/// - `name`: The function name (from source)
/// - `name_span`: Span of the function name
/// - `params`: AST parameter definitions
/// - `return_type`: AST return type
/// - `span`: Full span of the function definition
fn collect_function_def<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    name: &'src str,
    name_span: Span,
    params: &[AstFunctionParam],
    return_type: &AstType,
    span: Span,
) {
    // Resolve parameter types
    let mut resolved_params = Vec::new();

    for param in params {
        // Resolve the parameter type
        if let Some(param_type) = resolve_type(ctx, &param.type_annotation) {
            // Create parameter
            let param_name = extract_name(ctx.source, &param.name);
            let param_def = FunctionParam::new(param_name, param.name_span, param_type, param.span);
            resolved_params.push(param_def);
        }
        // If type resolution failed, error was already added to context
    }

    // Resolve return type
    let resolved_return_type = resolve_type(ctx, return_type).unwrap_or(ResolvedType::I32 { span }); // Fallback to i32 if resolution fails

    // Create complete function definition
    let func_def = ctx.arena.alloc(FunctionDefinition::new(
        name,
        name_span,
        resolved_params,
        resolved_return_type,
        vec![], // Body not processed in pass 1
        None,   // Not a method (top-level functions only in pass 1)
        span,
    ));

    // Replace the placeholder with the complete definition
    let _ = ctx.function_definitions.insert(name, func_def);
}

/// Collect a top-level let statement
///
/// This creates a VarDefinition for a global variable without resolving
/// the type or initializer expression.
///
/// # Parameters
///
/// - `ctx`: The analyzer context
/// - `name`: The variable name (from source)
/// - `name_span`: Span of the variable name
/// - `type_annotation`: Optional type annotation
/// - `span`: Full span of the let statement
fn collect_let_stmt<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    name: &'src str,
    name_span: Span,
    type_annotation: Option<&AstType>,
    span: Span,
) {
    // Resolve type annotation if present
    let var_type = type_annotation.and_then(|ty| resolve_type(ctx, ty));

    // Get current scope level
    let scope_level: ScopeLevel = ctx.scope_stack.current_scope_level();

    // Create variable definition (no initializer in pass 1)
    let var_def = ctx.arena.alloc(VarDefinition::new(
        name,
        name_span,
        var_type,
        None, // Initializer not resolved in pass 1
        scope_level,
        span,
    ));

    // Try to declare the variable in the current scope
    if let Some(old_def) = ctx.scope_stack.declare_variable(name, var_def) {
        // Duplicate variable definition in the same scope
        ctx.add_error(SemanticError::DuplicateDefinition {
            name: name.to_string(),
            first_span: old_def.name_span,
            second_span: name_span,
        });
    }
}

// ============================================================================
// Type Resolution
// ============================================================================

/// Resolve an AST type to a HIR ResolvedType
///
/// For primitive types, this creates the corresponding ResolvedType immediately.
/// For UserDefined types, this looks up the struct definition and creates a reference.
///
/// # Parameters
///
/// - `ctx`: The analyzer context (for struct lookups and error reporting)
/// - `ast_type`: The AST type to resolve
///
/// # Returns
///
/// - `Some(resolved_type)` if the type was successfully resolved
/// - `None` if the type could not be resolved (error added to context)
fn resolve_type<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    ast_type: &AstType,
) -> Option<ResolvedType<'src, 'arena>> {
    match ast_type {
        AstType::Bool { span } => Some(ResolvedType::Bool { span: *span }),
        AstType::I32 { span } => Some(ResolvedType::I32 { span: *span }),
        AstType::F64 { span } => Some(ResolvedType::F64 { span: *span }),
        AstType::Real { span } => Some(ResolvedType::Real { span: *span }),
        AstType::Algebraic { span } => Some(ResolvedType::Algebraic { span: *span }),
        AstType::Reference { inner, span } => {
            // Recursively resolve the inner type
            let inner_resolved = resolve_type(ctx, inner)?;
            let inner_allocated = ctx.arena.alloc(inner_resolved);
            Some(ResolvedType::Reference {
                inner: inner_allocated,
                span: *span,
            })
        }
        AstType::Array {
            element_type,
            size,
            span,
        } => {
            // Recursively resolve the element type
            let element_resolved = resolve_type(ctx, element_type)?;
            let element_allocated = ctx.arena.alloc(element_resolved);
            Some(ResolvedType::Array {
                element_type: element_allocated,
                size: *size,
                span: *span,
            })
        }
        AstType::UserDefined { name, span } => {
            // Look up the struct definition
            let name_src = extract_name(ctx.source, name);
            if let Some(struct_def) = ctx.lookup_struct(name_src) {
                Some(ResolvedType::UserDefined {
                    name: name_src,
                    definition: struct_def,
                    span: *span,
                })
            } else {
                // Undefined type
                ctx.add_error(SemanticError::UndefinedType {
                    name: name.clone(),
                    span: *span,
                });
                None
            }
        }
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Extract a name from the source text
///
/// This function ensures that names are `&'src str` references into the
/// source text rather than owned Strings.
///
/// # Parameters
///
/// - `source`: The source text
/// - `name`: The name as a String (from AST)
///
/// # Returns
///
/// A string slice from the source text with the correct lifetime
///
/// # Implementation Note
///
/// This is a temporary approach. Ideally, the AST should use `&'src str`
/// directly, but for now we work with the current String-based AST.
fn extract_name<'src>(source: &'src str, name: &str) -> &'src str {
    // Find the name in the source text
    // This is safe because all AST names come from the source text
    if let Some(idx) = source.find(name) {
        &source[idx..idx + name.len()]
    } else {
        // Fallback: Use a static string if not found
        // This should never happen in practice
        Box::leak(name.to_string().into_boxed_str())
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
    use bumpalo::Bump;

    /// Helper to create a test span
    fn make_span(line: usize, column: usize) -> Span {
        Span {
            start: LineColumn { line, column },
            lines: 0,
            end_column: column + 5,
        }
    }

    #[test]
    fn test_collect_struct_with_primitive_fields() {
        let arena = Bump::new();
        let source = "struct Point { x: f64, y: f64 }";
        let mut ctx = AnalyzerContext::new(&arena, source);

        let stmts = vec![Stmt::StructDef {
            name: "Point".to_string(),
            name_span: make_span(1, 8),
            container: None,
            fields: vec![
                AstStructField {
                    name: "x".to_string(),
                    name_span: make_span(1, 16),
                    type_annotation: AstType::F64 {
                        span: make_span(1, 19),
                    },
                    span: make_span(1, 16),
                },
                AstStructField {
                    name: "y".to_string(),
                    name_span: make_span(1, 24),
                    type_annotation: AstType::F64 {
                        span: make_span(1, 27),
                    },
                    span: make_span(1, 24),
                },
            ],
            methods: vec![],
            span: make_span(1, 1),
        }];

        collect_declarations(&mut ctx, &stmts);

        // Should have registered the struct
        assert_eq!(ctx.struct_definitions.len(), 1);
        assert!(!ctx.has_errors());

        // Lookup the struct
        let point = ctx.lookup_struct("Point").unwrap();
        assert_eq!(point.name, "Point");
        assert_eq!(point.field_count(), 2);
        assert!(point.has_field("x"));
        assert!(point.has_field("y"));
    }

    #[test]
    fn test_collect_struct_duplicate_detection() {
        let arena = Bump::new();
        let source = "struct Point {} struct Point {}";
        let mut ctx = AnalyzerContext::new(&arena, source);

        let stmts = vec![
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
                name_span: make_span(1, 24),
                container: None,
                fields: vec![],
                methods: vec![],
                span: make_span(1, 17),
            },
        ];

        collect_declarations(&mut ctx, &stmts);

        // Should have only one struct registered
        assert_eq!(ctx.struct_definitions.len(), 1);

        // Should have one duplicate definition error
        assert!(ctx.has_errors());
        let errors = ctx.take_errors();
        assert_eq!(errors.len(), 1);
        assert_matches!(
            &errors[0],
            SemanticError::DuplicateDefinition { name, .. } if name == "Point"
        );
    }

    #[test]
    fn test_collect_function_with_parameters() {
        let arena = Bump::new();
        let source = "fn distance(x1: f64, x2: f64) -> f64 {}";
        let mut ctx = AnalyzerContext::new(&arena, source);

        let stmts = vec![Stmt::FunctionDef {
            name: "distance".to_string(),
            name_span: make_span(1, 4),
            params: vec![
                AstFunctionParam {
                    name: "x1".to_string(),
                    name_span: make_span(1, 13),
                    type_annotation: AstType::F64 {
                        span: make_span(1, 17),
                    },
                    span: make_span(1, 13),
                },
                AstFunctionParam {
                    name: "x2".to_string(),
                    name_span: make_span(1, 22),
                    type_annotation: AstType::F64 {
                        span: make_span(1, 26),
                    },
                    span: make_span(1, 22),
                },
            ],
            return_type: AstType::F64 {
                span: make_span(1, 34),
            },
            body: vec![],
            return_expr: None,
            span: make_span(1, 1),
        }];

        collect_declarations(&mut ctx, &stmts);

        // Should have registered the function
        assert_eq!(ctx.function_definitions.len(), 1);
        assert!(!ctx.has_errors());

        // Lookup the function
        let distance = ctx.lookup_function("distance").unwrap();
        assert_eq!(distance.name, "distance");
        assert_eq!(distance.param_count(), 2);
    }

    #[test]
    fn test_collect_function_duplicate_detection() {
        let arena = Bump::new();
        let source = "fn foo() -> i32 {} fn foo() -> i32 {}";
        let mut ctx = AnalyzerContext::new(&arena, source);

        let stmts = vec![
            Stmt::FunctionDef {
                name: "foo".to_string(),
                name_span: make_span(1, 4),
                params: vec![],
                return_type: AstType::I32 {
                    span: make_span(1, 13),
                },
                body: vec![],
                return_expr: None,
                span: make_span(1, 1),
            },
            Stmt::FunctionDef {
                name: "foo".to_string(),
                name_span: make_span(1, 23),
                params: vec![],
                return_type: AstType::I32 {
                    span: make_span(1, 32),
                },
                body: vec![],
                return_expr: None,
                span: make_span(1, 20),
            },
        ];

        collect_declarations(&mut ctx, &stmts);

        // Should have only one function registered
        assert_eq!(ctx.function_definitions.len(), 1);

        // Should have one duplicate definition error
        assert!(ctx.has_errors());
        let errors = ctx.take_errors();
        assert_eq!(errors.len(), 1);
        assert_matches!(
            &errors[0],
            SemanticError::DuplicateDefinition { name, .. } if name == "foo"
        );
    }

    #[test]
    fn test_collect_variable() {
        let arena = Bump::new();
        let source = "let x: i32;";
        let mut ctx = AnalyzerContext::new(&arena, source);

        let stmts = vec![Stmt::Let {
            dot_prefix: false,
            name_path: vec![("x", make_span(1, 5))],
            type_annotation: Some(AstType::I32 {
                span: make_span(1, 8),
            }),
            init: None,
            span: make_span(1, 1),
        }];

        collect_declarations(&mut ctx, &stmts);

        // Should have registered the variable
        assert!(!ctx.has_errors());
        let var = ctx.scope_stack.lookup_variable("x");
        assert!(var.is_some());
        assert_eq!(var.unwrap().name, "x");
    }

    #[test]
    fn test_collect_variable_duplicate_detection() {
        let arena = Bump::new();
        let source = "let x: i32; let x: i32;";
        let mut ctx = AnalyzerContext::new(&arena, source);

        let stmts = vec![
            Stmt::Let {
                dot_prefix: false,
                name_path: vec![("x", make_span(1, 5))],
                type_annotation: Some(AstType::I32 {
                    span: make_span(1, 8),
                }),
                init: None,
                span: make_span(1, 1),
            },
            Stmt::Let {
                dot_prefix: false,
                name_path: vec![("x", make_span(1, 17))],
                type_annotation: Some(AstType::I32 {
                    span: make_span(1, 20),
                }),
                init: None,
                span: make_span(1, 13),
            },
        ];

        collect_declarations(&mut ctx, &stmts);

        // Should have one duplicate definition error
        assert!(ctx.has_errors());
        let errors = ctx.take_errors();
        assert_eq!(errors.len(), 1);
        assert_matches!(
            &errors[0],
            SemanticError::DuplicateDefinition { name, .. } if name == "x"
        );
    }

    #[test]
    fn test_struct_with_user_defined_field() {
        let arena = Bump::new();
        let source = "struct Point { x: f64 } struct Line { p1: Point, p2: Point }";
        let mut ctx = AnalyzerContext::new(&arena, source);

        let stmts = vec![
            Stmt::StructDef {
                name: "Point".to_string(),
                name_span: make_span(1, 8),
                container: None,
                fields: vec![AstStructField {
                    name: "x".to_string(),
                    name_span: make_span(1, 16),
                    type_annotation: AstType::F64 {
                        span: make_span(1, 19),
                    },
                    span: make_span(1, 16),
                }],
                methods: vec![],
                span: make_span(1, 1),
            },
            Stmt::StructDef {
                name: "Line".to_string(),
                name_span: make_span(2, 8),
                container: None,
                fields: vec![
                    AstStructField {
                        name: "p1".to_string(),
                        name_span: make_span(2, 16),
                        type_annotation: AstType::UserDefined {
                            name: "Point".to_string(),
                            span: make_span(2, 20),
                        },
                        span: make_span(2, 16),
                    },
                    AstStructField {
                        name: "p2".to_string(),
                        name_span: make_span(2, 27),
                        type_annotation: AstType::UserDefined {
                            name: "Point".to_string(),
                            span: make_span(2, 31),
                        },
                        span: make_span(2, 27),
                    },
                ],
                methods: vec![],
                span: make_span(2, 1),
            },
        ];

        collect_declarations(&mut ctx, &stmts);

        // Should have registered both structs
        assert_eq!(ctx.struct_definitions.len(), 2);
        assert!(!ctx.has_errors());

        // Check Line struct has Point-typed fields
        let line = ctx.lookup_struct("Line").unwrap();
        assert_eq!(line.field_count(), 2);

        let p1_field = line.find_field("p1").unwrap();
        assert!(p1_field.field_type.is_user_defined());
    }

    #[test]
    fn test_undefined_type_error() {
        let arena = Bump::new();
        let source = "struct Line { p1: Point }";
        let mut ctx = AnalyzerContext::new(&arena, source);

        let stmts = vec![Stmt::StructDef {
            name: "Line".to_string(),
            name_span: make_span(1, 8),
            container: None,
            fields: vec![AstStructField {
                name: "p1".to_string(),
                name_span: make_span(1, 15),
                type_annotation: AstType::UserDefined {
                    name: "Point".to_string(),
                    span: make_span(1, 19),
                },
                span: make_span(1, 15),
            }],
            methods: vec![],
            span: make_span(1, 1),
        }];

        collect_declarations(&mut ctx, &stmts);

        // Should have registered Line struct
        assert_eq!(ctx.struct_definitions.len(), 1);

        // Should have one undefined type error
        assert!(ctx.has_errors());
        let errors = ctx.take_errors();
        assert_eq!(errors.len(), 1);
        assert_matches!(
            &errors[0],
            SemanticError::UndefinedType { name, .. } if name == "Point"
        );
    }

    #[test]
    fn test_mixed_declarations() {
        let arena = Bump::new();
        let source = "struct Point { x: f64 } fn distance() -> f64 {} let origin: Point;";
        let mut ctx = AnalyzerContext::new(&arena, source);

        let stmts = vec![
            Stmt::StructDef {
                name: "Point".to_string(),
                name_span: make_span(1, 8),
                container: None,
                fields: vec![AstStructField {
                    name: "x".to_string(),
                    name_span: make_span(1, 16),
                    type_annotation: AstType::F64 {
                        span: make_span(1, 19),
                    },
                    span: make_span(1, 16),
                }],
                methods: vec![],
                span: make_span(1, 1),
            },
            Stmt::FunctionDef {
                name: "distance".to_string(),
                name_span: make_span(1, 28),
                params: vec![],
                return_type: AstType::F64 {
                    span: make_span(1, 42),
                },
                body: vec![],
                return_expr: None,
                span: make_span(1, 25),
            },
            Stmt::Let {
                dot_prefix: false,
                name_path: vec![("origin", make_span(1, 53))],
                type_annotation: Some(AstType::UserDefined {
                    name: "Point".to_string(),
                    span: make_span(1, 61),
                }),
                init: None,
                span: make_span(1, 49),
            },
        ];

        collect_declarations(&mut ctx, &stmts);

        // Should have registered all declarations
        assert_eq!(ctx.struct_definitions.len(), 1);
        assert_eq!(ctx.function_definitions.len(), 1);
        assert!(ctx.scope_stack.lookup_variable("origin").is_some());
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_empty_struct() {
        let arena = Bump::new();
        let source = "struct Empty {}";
        let mut ctx = AnalyzerContext::new(&arena, source);

        let stmts = vec![Stmt::StructDef {
            name: "Empty".to_string(),
            name_span: make_span(1, 8),
            container: None,
            fields: vec![],
            methods: vec![],
            span: make_span(1, 1),
        }];

        collect_declarations(&mut ctx, &stmts);

        // Should have registered the empty struct
        assert_eq!(ctx.struct_definitions.len(), 1);
        assert!(!ctx.has_errors());

        let empty = ctx.lookup_struct("Empty").unwrap();
        assert_eq!(empty.field_count(), 0);
    }

    #[test]
    fn test_function_no_parameters() {
        let arena = Bump::new();
        let source = "fn foo() -> bool {}";
        let mut ctx = AnalyzerContext::new(&arena, source);

        let stmts = vec![Stmt::FunctionDef {
            name: "foo".to_string(),
            name_span: make_span(1, 4),
            params: vec![],
            return_type: AstType::Bool {
                span: make_span(1, 13),
            },
            body: vec![],
            return_expr: None,
            span: make_span(1, 1),
        }];

        collect_declarations(&mut ctx, &stmts);

        // Should have registered the parameterless function
        assert_eq!(ctx.function_definitions.len(), 1);
        assert!(!ctx.has_errors());

        let foo = ctx.lookup_function("foo").unwrap();
        assert_eq!(foo.param_count(), 0);
    }
}
