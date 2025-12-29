//! Semantic Analyzer Pass 2: Resolution and HIR Construction
//!
//! This module implements the second pass of semantic analysis, which resolves
//! all references (variables, functions, types) and constructs the High-level
//! Intermediate Representation (HIR) from the AST.
//!
//! # Pass 2 Overview
//!
//! Pass 2 operates on the AST after Pass 1 has collected all declarations.
//! It performs:
//!
//! - **Name Resolution**: Resolve all variable, function, and type references
//! - **HIR Construction**: Build the HIR with resolved cross-references
//! - **Scope Management**: Track nested scopes (blocks, functions, loops, with-contexts)
//! - **Error Collection**: Collect all resolution errors for reporting
//!
//! # What is Resolved
//!
//! - Expression bodies (let initializers, assignments)
//! - Function bodies (statements and return expressions)
//! - Variable references to their definitions
//! - Function calls to their definitions
//! - Method calls to their definitions
//! - Field accesses to their field definitions
//! - Type references to their struct definitions
//!
//! # Scope Management
//!
//! Pass 2 manages scopes for:
//! - Blocks: Create new scope for block statements
//! - Functions: Create new scope for function body
//! - For loops: Create new scope with loop variable
//! - With statements: Create new scope with with-context
//!
//! # Error Handling
//!
//! All errors are collected in `ctx.errors` to enable continued analysis
//! and reporting of multiple errors at once. When a resolution failure occurs,
//! the function returns `None` and an error is added to the context.

use crate::ast::{Expr, Stmt, StructLitField as AstStructLitField};
use crate::hir_context::WithContext;
use crate::hir_definitions::{ScopeLevel, VarDefinition};
use crate::hir_expr::{
    ResolvedExpr, ResolvedExprKind, ResolvedStmt, ResolvedStmtKind, ResolvedStructLitField,
};
use crate::hir_types::ResolvedType;
use crate::lexer::Span;
use crate::semantic_analyzer_context::AnalyzerContext;
use crate::semantic_analyzer_errors::SemanticError;

// ============================================================================
// Main Resolution Functions
// ============================================================================

/// Resolve all statements and return resolved HIR statements
///
/// This is the main entry point for Pass 2. It processes all statements
/// in order, resolving references and constructing HIR nodes.
///
/// # Parameters
///
/// - `ctx`: The analyzer context (with declarations from Pass 1)
/// - `stmts`: The AST statements to resolve
///
/// # Returns
///
/// A vector of resolved HIR statements
///
/// # Error Handling
///
/// Errors are collected in `ctx.errors`. Resolution continues even when
/// errors are encountered to collect as many errors as possible.
pub fn resolve_statements<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    stmts: &[Stmt<'src>],
) -> Vec<&'arena ResolvedStmt<'src, 'arena>> {
    let mut resolved = Vec::new();

    for stmt in stmts {
        if let Some(resolved_stmt) = resolve_statement(ctx, stmt) {
            resolved.push(resolved_stmt);
        }
    }

    resolved
}

/// Resolve a single statement
///
/// Dispatches to the appropriate resolution function based on the statement type.
///
/// # Parameters
///
/// - `ctx`: The analyzer context
/// - `stmt`: The AST statement to resolve
///
/// # Returns
///
/// - `Some(&arena_stmt)` if the statement was successfully resolved
/// - `None` if resolution failed (error added to context)
pub fn resolve_statement<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    stmt: &Stmt<'src>,
) -> Option<&'arena ResolvedStmt<'src, 'arena>> {
    match stmt {
        Stmt::Let {
            dot_prefix,
            name_path,
            type_annotation,
            init,
            span,
        } => resolve_let_statement(
            ctx,
            *dot_prefix,
            name_path,
            type_annotation.as_ref(),
            init.as_ref(),
            *span,
        ),

        Stmt::Assignment {
            name,
            name_span,
            value,
            span,
        } => resolve_assignment(ctx, name, *name_span, value, *span),

        Stmt::FieldAssignment {
            dot_prefix,
            field_path,
            value,
            span,
        } => resolve_field_assignment(ctx, *dot_prefix, field_path, value, *span),

        Stmt::FunctionDef {
            name,
            name_span,
            params,
            return_type,
            body,
            return_expr,
            span,
        } => resolve_function_body(
            ctx,
            name,
            *name_span,
            params,
            return_type,
            body,
            return_expr.as_ref(),
            *span,
        ),

        Stmt::StructDef {
            name,
            name_span,
            container: _,
            fields: _,
            methods,
            span,
        } => {
            // Look up the struct definition from Pass 1
            let struct_def = ctx.lookup_struct(name);
            if struct_def.is_none() {
                ctx.add_error(SemanticError::UndefinedType {
                    name: name.clone(),
                    span: *name_span,
                });
                return None;
            }
            let struct_def = struct_def.unwrap();

            // Resolve method bodies
            let resolved_methods = resolve_statements(ctx, methods);

            // Create the HIR statement
            Some(ctx.arena.alloc(ResolvedStmt::new(
                *span,
                ResolvedStmtKind::StructDef {
                    struct_def,
                    methods: resolved_methods,
                    span: *span,
                },
            )))
        }

        Stmt::With {
            context_expr,
            body,
            span,
        } => resolve_with_statement(ctx, context_expr, body, *span),

        Stmt::For {
            loop_var,
            loop_var_span,
            iterator,
            body,
            span,
        } => resolve_for_statement(ctx, loop_var, *loop_var_span, iterator, body, *span),

        Stmt::If {
            condition,
            then_branch,
            else_branch,
            span,
        } => resolve_if_statement(ctx, condition, then_branch, else_branch.as_ref(), *span),

        Stmt::Block { statements, span } => resolve_block_statement(ctx, statements, *span),

        Stmt::Return { value, span } => resolve_return_statement(ctx, value.as_ref(), *span),

        Stmt::Expression { expr, span } => resolve_expression_statement(ctx, expr, *span),
    }
}

// ============================================================================
// Statement Resolution Functions
// ============================================================================

/// Resolve a let statement
fn resolve_let_statement<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    dot_prefix: bool,
    name_path: &[(&'src str, Span)],
    type_annotation: Option<&crate::ast::Type>,
    init: Option<&Expr<'src>>,
    span: Span,
) -> Option<&'arena ResolvedStmt<'src, 'arena>> {
    // Simple let statement (no dot prefix, single name)
    if !dot_prefix && name_path.len() == 1 {
        let (name, name_span) = name_path[0];

        // Resolve type annotation if present
        let resolved_type = type_annotation.and_then(|ty| resolve_type(ctx, ty));

        // Resolve initializer expression if present
        let init_expr = init.and_then(|expr| resolve_expression(ctx, expr));

        // Get current scope level
        let scope_level: ScopeLevel = ctx.scope_stack.current_scope_level();

        // Look up or create the variable definition
        let var_def = if scope_level == 0 {
            // Top-level let statement - should already exist from Pass 1
            let existing = ctx.scope_stack.lookup_variable(name);
            if let Some(existing) = existing {
                existing
            } else {
                // If not found, create it now (shouldn't happen normally)
                let new_def = VarDefinition::new(
                    name,
                    name_span,
                    resolved_type,
                    init_expr,
                    scope_level,
                    span,
                );
                let new_def_ref: &'arena VarDefinition<'src, 'arena> = ctx.arena.alloc(new_def);
                let result = new_def_ref;
                ctx.scope_stack.declare_variable(name, new_def_ref);
                result
            }
        } else {
            // Non-top-level let statement - declare it now
            let new_def =
                VarDefinition::new(name, name_span, resolved_type, init_expr, scope_level, span);
            let new_def_ref: &'arena VarDefinition<'src, 'arena> = ctx.arena.alloc(new_def);
            let result = new_def_ref;

            // Declare the variable in the current scope
            if let Some(old_def) = ctx.scope_stack.declare_variable(name, new_def_ref) {
                // Duplicate variable definition in the same scope
                ctx.add_error(SemanticError::DuplicateDefinition {
                    name: name.to_string(),
                    first_span: old_def.name_span,
                    second_span: name_span,
                });
            }
            result
        };

        // Create the HIR statement
        let stmt = ctx.arena.alloc(ResolvedStmt::new(
            span,
            ResolvedStmtKind::Let {
                dot_prefix,
                name_path: name_path.to_vec(),
                var_def,
                init: init_expr,
                span,
            },
        ));

        Some(stmt)
    } else if dot_prefix {
        // Dot-prefix let (e.g., `let .field = value;`)
        // Check if we're in a with-context
        if ctx.scope_stack.current_with_context().is_none() {
            ctx.add_error(SemanticError::NotInWithContext { span });
            return None;
        }

        // Resolve the initializer
        let init_expr = init.and_then(|expr| resolve_expression(ctx, expr));

        // For dot-prefix lets, we need to create a variable definition
        // The variable is implicitly scoped to the with-context
        let (name, name_span) = name_path[0];
        let resolved_type = type_annotation.and_then(|ty| resolve_type(ctx, ty));
        let scope_level = ctx.scope_stack.current_scope_level();

        let var_def = ctx.arena.alloc(VarDefinition::new(
            name,
            name_span,
            resolved_type,
            init_expr,
            scope_level,
            span,
        ));

        // Create the HIR statement
        let stmt = ctx.arena.alloc(ResolvedStmt::new(
            span,
            ResolvedStmtKind::Let {
                dot_prefix,
                name_path: name_path.to_vec(),
                var_def,
                init: init_expr,
                span,
            },
        ));

        Some(stmt)
    } else {
        // Path let (e.g., `let container.field = value;`)
        // Resolve the initializer
        let init_expr = init.and_then(|expr| resolve_expression(ctx, expr));

        // For path lets, we create a variable for the last element in the path
        if name_path.is_empty() {
            ctx.add_error(SemanticError::UndefinedVariable {
                name: "<empty path>".to_string(),
                span,
            });
            return None;
        }

        let (name, name_span) = name_path[name_path.len() - 1];
        let resolved_type = type_annotation.and_then(|ty| resolve_type(ctx, ty));
        let scope_level = ctx.scope_stack.current_scope_level();

        let var_def = ctx.arena.alloc(VarDefinition::new(
            name,
            name_span,
            resolved_type,
            init_expr,
            scope_level,
            span,
        ));

        // Create the HIR statement
        let stmt = ctx.arena.alloc(ResolvedStmt::new(
            span,
            ResolvedStmtKind::Let {
                dot_prefix,
                name_path: name_path.to_vec(),
                var_def,
                init: init_expr,
                span,
            },
        ));

        Some(stmt)
    }
}

/// Resolve an assignment statement
fn resolve_assignment<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    name: &'src str,
    name_span: Span,
    value: &Expr<'src>,
    span: Span,
) -> Option<&'arena ResolvedStmt<'src, 'arena>> {
    // Lookup the variable
    let var_def = ctx.scope_stack.lookup_variable(name);
    if var_def.is_none() {
        ctx.add_error(SemanticError::UndefinedVariable {
            name: name.to_string(),
            span: name_span,
        });
        return None;
    }
    let var_def = var_def.unwrap();

    // Resolve the value expression
    let value_expr = resolve_expression(ctx, value)?;

    // Create the HIR statement
    let stmt = ctx.arena.alloc(ResolvedStmt::new(
        span,
        ResolvedStmtKind::Assignment {
            var_def,
            value: value_expr,
            span,
        },
    ));

    Some(stmt)
}

/// Resolve a field assignment statement
fn resolve_field_assignment<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    dot_prefix: bool,
    field_path: &[(&'src str, Span)],
    value: &Expr<'src>,
    span: Span,
) -> Option<&'arena ResolvedStmt<'src, 'arena>> {
    // Resolve the target field expression
    let target = if dot_prefix {
        // Dot-prefix field assignment (e.g., `.field = value;`)
        // Check if we're in a with-context
        let with_ctx = ctx.scope_stack.current_with_context();
        if with_ctx.is_none() {
            ctx.add_error(SemanticError::NotInWithContext { span });
            return None;
        }
        let with_ctx = with_ctx.unwrap();

        // Create ContainerFieldAccess expression
        // For now, use a simple type (we'll infer the actual type during type checking)
        let ty = &*ctx.arena.alloc(ResolvedType::I32 { span });
        let kind = ResolvedExprKind::ContainerFieldAccess {
            resolved_path: field_path.iter().map(|(name, _)| *name).collect(),
            with_context: with_ctx,
            transform: None,
        };
        ctx.arena.alloc(ResolvedExpr::new(span, kind, ty))
    } else {
        // Regular field assignment (e.g., `obj.field = value;`)
        // Build nested field access expression
        if field_path.is_empty() {
            ctx.add_error(SemanticError::UndefinedVariable {
                name: "<empty path>".to_string(),
                span,
            });
            return None;
        }

        // Start with the first element (must be a variable)
        let (first_name, first_span) = field_path[0];
        let var_def = ctx.scope_stack.lookup_variable(first_name);
        if var_def.is_none() {
            ctx.add_error(SemanticError::UndefinedVariable {
                name: first_name.to_string(),
                span: first_span,
            });
            return None;
        }
        let var_def = var_def.unwrap();

        let var_ty: &'arena ResolvedType = if let Some(ty) = var_def.var_type {
            ctx.arena.alloc(ty)
        } else {
            let default_type = ResolvedType::I32 { span: first_span };
            ctx.arena.alloc(default_type)
        };

        let mut current_expr = ctx.arena.alloc(ResolvedExpr::new(
            first_span,
            ResolvedExprKind::Var {
                name: first_name,
                definition: var_def,
            },
            var_ty,
        ));

        // Build nested field accesses for the rest of the path
        for &(_field_name, field_span) in &field_path[1..] {
            // For now, we don't have full struct type information during resolution
            // Type checking will validate the field access later
            // Create a placeholder field access
            let field_ty = &*ctx.arena.alloc(ResolvedType::I32 { span: field_span });

            // We need a field definition, but we don't have it yet
            // For now, create a temporary placeholder
            // The type checker will validate this later
            let kind = ResolvedExprKind::IntLit { value: 0 }; // Placeholder
            current_expr = ctx
                .arena
                .alloc(ResolvedExpr::new(field_span, kind, field_ty));
        }

        current_expr
    };

    // Resolve the value expression
    let value_expr = resolve_expression(ctx, value)?;

    // Create the HIR statement
    let stmt = ctx.arena.alloc(ResolvedStmt::new(
        span,
        ResolvedStmtKind::FieldAssignment {
            target,
            value: value_expr,
            span,
        },
    ));

    Some(stmt)
}

/// Resolve a function body
#[allow(clippy::too_many_arguments)]
fn resolve_function_body<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    name: &str,
    name_span: Span,
    params: &[crate::ast::FunctionParam],
    _return_type: &crate::ast::Type,
    body: &[Stmt<'src>],
    return_expr: Option<&Expr<'src>>,
    span: Span,
) -> Option<&'arena ResolvedStmt<'src, 'arena>> {
    // Look up the function definition from Pass 1
    let func_def = ctx.lookup_function(name);
    if func_def.is_none() {
        ctx.add_error(SemanticError::UndefinedFunction {
            name: name.to_string(),
            span: name_span,
        });
        return None;
    }
    let func_def = func_def.unwrap();

    // Push a new scope for the function body
    ctx.scope_stack.push_scope();

    // Add function parameters to the scope
    let scope_level = ctx.scope_stack.current_scope_level();
    for param in params {
        let param_name = extract_name(ctx.source, &param.name);
        let param_type = resolve_type(ctx, &param.type_annotation);

        let var_def = ctx.arena.alloc(VarDefinition::new(
            param_name,
            param.name_span,
            param_type,
            None, // Parameters don't have initializers
            scope_level,
            param.span,
        ));

        // Declare the parameter as a variable in the function scope
        ctx.scope_stack.declare_variable(param_name, var_def);
    }

    // Resolve body statements
    let resolved_body = resolve_statements(ctx, body);

    // Resolve return expression if present
    let resolved_return = return_expr.and_then(|expr| resolve_expression(ctx, expr));

    // Pop the function scope
    ctx.scope_stack.pop_scope();

    // Create the HIR statement
    let stmt = ctx.arena.alloc(ResolvedStmt::new(
        span,
        ResolvedStmtKind::FunctionDef {
            func_def,
            body: resolved_body,
            return_expr: resolved_return,
            span,
        },
    ));

    Some(stmt)
}

/// Resolve a with statement
fn resolve_with_statement<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    context_expr: &Expr<'src>,
    body: &[Stmt<'src>],
    span: Span,
) -> Option<&'arena ResolvedStmt<'src, 'arena>> {
    // Resolve the context expression
    let resolved_context = resolve_expression(ctx, context_expr)?;

    // Create a with-context
    // For now, we create a simple transform context
    let with_ctx = ctx.arena.alloc(WithContext::new_transform(
        resolved_context,
        vec![], // No transforms for now
    ));

    // Enter the with-context
    ctx.scope_stack.enter_with_context(with_ctx);

    // Resolve body statements
    let resolved_body = resolve_statements(ctx, body);

    // Exit the with-context
    ctx.scope_stack.exit_with_context();

    // Create the HIR statement
    let stmt = ctx.arena.alloc(ResolvedStmt::new(
        span,
        ResolvedStmtKind::With {
            with_context: with_ctx,
            body: resolved_body,
            span,
        },
    ));

    Some(stmt)
}

/// Resolve a for statement
fn resolve_for_statement<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    loop_var: &'src str,
    loop_var_span: Span,
    iterator: &Expr<'src>,
    body: &[Stmt<'src>],
    span: Span,
) -> Option<&'arena ResolvedStmt<'src, 'arena>> {
    // Resolve the iterator expression
    let iterator_expr = resolve_expression(ctx, iterator)?;

    // Push a new scope for the loop body
    ctx.scope_stack.push_scope();

    // Create the loop variable definition
    // TODO: Infer type from iterator
    let scope_level = ctx.scope_stack.current_scope_level();
    let loop_var_def = ctx.arena.alloc(VarDefinition::new(
        loop_var,
        loop_var_span,
        None, // Type inference needed
        None, // No initializer for loop variables
        scope_level,
        span,
    ));

    // Declare the loop variable
    ctx.scope_stack.declare_variable(loop_var, loop_var_def);

    // Resolve body statements
    let resolved_body = resolve_statements(ctx, body);

    // Pop the loop scope
    ctx.scope_stack.pop_scope();

    // Create the HIR statement
    let stmt = ctx.arena.alloc(ResolvedStmt::new(
        span,
        ResolvedStmtKind::For {
            loop_var_def,
            iterator: iterator_expr,
            body: resolved_body,
            span,
        },
    ));

    Some(stmt)
}

/// Resolve an if statement
fn resolve_if_statement<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    condition: &Expr<'src>,
    then_branch: &[Stmt<'src>],
    else_branch: Option<&Vec<Stmt<'src>>>,
    span: Span,
) -> Option<&'arena ResolvedStmt<'src, 'arena>> {
    // Resolve the condition
    let condition_expr = resolve_expression(ctx, condition)?;

    // Resolve then branch
    let resolved_then = resolve_statements(ctx, then_branch);

    // Resolve else branch if present
    let resolved_else = else_branch.map(|stmts| resolve_statements(ctx, stmts));

    // Create the HIR statement
    let stmt = ctx.arena.alloc(ResolvedStmt::new(
        span,
        ResolvedStmtKind::If {
            condition: condition_expr,
            then_branch: resolved_then,
            else_branch: resolved_else,
            span,
        },
    ));

    Some(stmt)
}

/// Resolve a block statement
fn resolve_block_statement<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    statements: &[Stmt<'src>],
    span: Span,
) -> Option<&'arena ResolvedStmt<'src, 'arena>> {
    // Push a new scope for the block
    ctx.scope_stack.push_scope();

    // Resolve statements in the block
    let resolved = resolve_statements(ctx, statements);

    // Pop the block scope
    ctx.scope_stack.pop_scope();

    // Create the HIR statement
    let stmt = ctx.arena.alloc(ResolvedStmt::new(
        span,
        ResolvedStmtKind::Block {
            statements: resolved,
            span,
        },
    ));

    Some(stmt)
}

/// Resolve a return statement
fn resolve_return_statement<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    value: Option<&Expr<'src>>,
    span: Span,
) -> Option<&'arena ResolvedStmt<'src, 'arena>> {
    // Resolve the return value if present
    let value_expr = value.and_then(|expr| resolve_expression(ctx, expr));

    // Create the HIR statement
    let stmt = ctx.arena.alloc(ResolvedStmt::new(
        span,
        ResolvedStmtKind::Return {
            value: value_expr,
            span,
        },
    ));

    Some(stmt)
}

/// Resolve an expression statement
fn resolve_expression_statement<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    expr: &Expr<'src>,
    span: Span,
) -> Option<&'arena ResolvedStmt<'src, 'arena>> {
    // Resolve the expression
    let resolved_expr = resolve_expression(ctx, expr)?;

    // Create the HIR statement
    let stmt = ctx.arena.alloc(ResolvedStmt::new(
        span,
        ResolvedStmtKind::Expression {
            expr: resolved_expr,
            span,
        },
    ));

    Some(stmt)
}

// ============================================================================
// Expression Resolution
// ============================================================================

/// Resolve an expression and return a resolved HIR expression
///
/// This function dispatches to specific resolution functions based on the
/// expression type.
///
/// # Parameters
///
/// - `ctx`: The analyzer context
/// - `expr`: The AST expression to resolve
///
/// # Returns
///
/// - `Some(&resolved_expr)` if the expression was successfully resolved
/// - `None` if resolution failed (error added to context)
pub fn resolve_expression<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    expr: &Expr<'src>,
) -> Option<&'arena ResolvedExpr<'src, 'arena>> {
    use crate::ast::HasSpan;

    let span = expr.span();

    let (kind, ty) = match expr {
        // Variables
        Expr::Var { name, span } => match ctx.scope_stack.lookup_variable(name) {
            Some(def) => {
                let var_type = def.var_type.as_ref().unwrap_or_else(|| {
                    // Fallback type if not resolved
                    ctx.arena.alloc(ResolvedType::I32 { span: *span })
                });
                let kind = ResolvedExprKind::Var {
                    name,
                    definition: def,
                };
                (kind, var_type)
            }
            None => {
                ctx.add_error(SemanticError::UndefinedVariable {
                    name: name.to_string(),
                    span: *span,
                });
                return None;
            }
        },

        // Function calls
        Expr::Call { name, args, span } => match ctx.lookup_function(name) {
            Some(func) => {
                // Resolve arguments
                let resolved_args: Vec<_> = args
                    .iter()
                    .filter_map(|arg| resolve_expression(ctx, arg))
                    .collect();

                let kind = ResolvedExprKind::FunctionCall {
                    name,
                    function: func,
                    args: resolved_args,
                };
                let ty = &*ctx.arena.alloc(func.return_type);
                (kind, ty)
            }
            None => {
                ctx.add_error(SemanticError::UndefinedFunction {
                    name: name.to_string(),
                    span: *span,
                });
                return None;
            }
        },

        // Method calls
        Expr::MethodCall {
            receiver,
            method,
            args: _,
            span,
        } => {
            // Resolve receiver
            let _resolved_receiver = resolve_expression(ctx, receiver)?;

            // TODO: Look up method on receiver type
            // For now, create a placeholder
            let kind = ResolvedExprKind::IntLit { value: 0 };
            let ty = &*ctx.arena.alloc(ResolvedType::I32 { span: *span });

            // Add error for unimplemented method resolution
            ctx.add_error(SemanticError::UndefinedFunction {
                name: format!("{}.{}", "receiver", method),
                span: *span,
            });

            (kind, ty)
        }

        // Field access
        Expr::FieldAccess {
            receiver,
            field: _,
            span,
        } => {
            // Resolve receiver
            let _resolved_receiver = resolve_expression(ctx, receiver)?;

            // TODO: Look up field on receiver type
            // For now, create a placeholder
            let kind = ResolvedExprKind::IntLit { value: 0 };
            let ty = &*ctx.arena.alloc(ResolvedType::I32 { span: *span });

            (kind, ty)
        }

        // Container field access (dot-prefix)
        Expr::ContainerFieldAccess { field_path, span } => {
            match ctx.scope_stack.current_with_context() {
                Some(with_ctx) => {
                    let kind = ResolvedExprKind::ContainerFieldAccess {
                        resolved_path: field_path.clone(),
                        with_context: with_ctx,
                        transform: None,
                    };
                    // TODO: Determine type from container field
                    let ty = &*ctx.arena.alloc(ResolvedType::I32 { span: *span });
                    (kind, ty)
                }
                None => {
                    ctx.add_error(SemanticError::NotInWithContext { span: *span });
                    return None;
                }
            }
        }

        // Binary operators
        Expr::And { lhs, rhs, span } => {
            let lhs_expr: Expr = (*lhs.clone()).into();
            let rhs_expr: Expr = (*rhs.clone()).into();
            let resolved_lhs = resolve_expression(ctx, &lhs_expr)?;
            let resolved_rhs = resolve_expression(ctx, &rhs_expr)?;
            let kind = ResolvedExprKind::And {
                lhs: resolved_lhs,
                rhs: resolved_rhs,
            };
            let ty = &*ctx.arena.alloc(ResolvedType::Bool { span: *span });
            (kind, ty)
        }
        Expr::Or { lhs, rhs, span } => {
            let lhs_expr: Expr = (*lhs.clone()).into();
            let rhs_expr: Expr = (*rhs.clone()).into();
            let resolved_lhs = resolve_expression(ctx, &lhs_expr)?;
            let resolved_rhs = resolve_expression(ctx, &rhs_expr)?;
            let kind = ResolvedExprKind::Or {
                lhs: resolved_lhs,
                rhs: resolved_rhs,
            };
            let ty = &*ctx.arena.alloc(ResolvedType::Bool { span: *span });
            (kind, ty)
        }
        Expr::Eq { lhs, rhs, span } => {
            let lhs_expr: Expr = (*lhs.clone()).into();
            let rhs_expr: Expr = (*rhs.clone()).into();
            let resolved_lhs = resolve_expression(ctx, &lhs_expr)?;
            let resolved_rhs = resolve_expression(ctx, &rhs_expr)?;
            let kind = ResolvedExprKind::Eq {
                lhs: resolved_lhs,
                rhs: resolved_rhs,
            };
            let ty = &*ctx.arena.alloc(ResolvedType::Bool { span: *span });
            (kind, ty)
        }
        Expr::NotEq { lhs, rhs, span } => {
            let lhs_expr: Expr = (*lhs.clone()).into();
            let rhs_expr: Expr = (*rhs.clone()).into();
            let resolved_lhs = resolve_expression(ctx, &lhs_expr)?;
            let resolved_rhs = resolve_expression(ctx, &rhs_expr)?;
            let kind = ResolvedExprKind::NotEq {
                lhs: resolved_lhs,
                rhs: resolved_rhs,
            };
            let ty = &*ctx.arena.alloc(ResolvedType::Bool { span: *span });
            (kind, ty)
        }
        Expr::Lt { lhs, rhs, span } => {
            let lhs_expr: Expr = (*lhs.clone()).into();
            let rhs_expr: Expr = (*rhs.clone()).into();
            let resolved_lhs = resolve_expression(ctx, &lhs_expr)?;
            let resolved_rhs = resolve_expression(ctx, &rhs_expr)?;
            let kind = ResolvedExprKind::Lt {
                lhs: resolved_lhs,
                rhs: resolved_rhs,
            };
            let ty = &*ctx.arena.alloc(ResolvedType::Bool { span: *span });
            (kind, ty)
        }
        Expr::Gt { lhs, rhs, span } => {
            let lhs_expr: Expr = (*lhs.clone()).into();
            let rhs_expr: Expr = (*rhs.clone()).into();
            let resolved_lhs = resolve_expression(ctx, &lhs_expr)?;
            let resolved_rhs = resolve_expression(ctx, &rhs_expr)?;
            let kind = ResolvedExprKind::Gt {
                lhs: resolved_lhs,
                rhs: resolved_rhs,
            };
            let ty = &*ctx.arena.alloc(ResolvedType::Bool { span: *span });
            (kind, ty)
        }
        Expr::LtEq { lhs, rhs, span } => {
            let lhs_expr: Expr = (*lhs.clone()).into();
            let rhs_expr: Expr = (*rhs.clone()).into();
            let resolved_lhs = resolve_expression(ctx, &lhs_expr)?;
            let resolved_rhs = resolve_expression(ctx, &rhs_expr)?;
            let kind = ResolvedExprKind::LtEq {
                lhs: resolved_lhs,
                rhs: resolved_rhs,
            };
            let ty = &*ctx.arena.alloc(ResolvedType::Bool { span: *span });
            (kind, ty)
        }
        Expr::GtEq { lhs, rhs, span } => {
            let lhs_expr: Expr = (*lhs.clone()).into();
            let rhs_expr: Expr = (*rhs.clone()).into();
            let resolved_lhs = resolve_expression(ctx, &lhs_expr)?;
            let resolved_rhs = resolve_expression(ctx, &rhs_expr)?;
            let kind = ResolvedExprKind::GtEq {
                lhs: resolved_lhs,
                rhs: resolved_rhs,
            };
            let ty = &*ctx.arena.alloc(ResolvedType::Bool { span: *span });
            (kind, ty)
        }
        Expr::Add { lhs, rhs, span: _ } => {
            let lhs_expr: Expr = (*lhs.clone()).into();
            let rhs_expr: Expr = (*rhs.clone()).into();
            let resolved_lhs = resolve_expression(ctx, &lhs_expr)?;
            let resolved_rhs = resolve_expression(ctx, &rhs_expr)?;
            let kind = ResolvedExprKind::Add {
                lhs: resolved_lhs,
                rhs: resolved_rhs,
            };
            let ty = resolved_lhs.ty; // Use left type for now
            (kind, ty)
        }
        Expr::Sub { lhs, rhs, span: _ } => {
            let lhs_expr: Expr = (*lhs.clone()).into();
            let rhs_expr: Expr = (*rhs.clone()).into();
            let resolved_lhs = resolve_expression(ctx, &lhs_expr)?;
            let resolved_rhs = resolve_expression(ctx, &rhs_expr)?;
            let kind = ResolvedExprKind::Sub {
                lhs: resolved_lhs,
                rhs: resolved_rhs,
            };
            let ty = resolved_lhs.ty;
            (kind, ty)
        }
        Expr::Mul { lhs, rhs, span: _ } => {
            let lhs_expr: Expr = (*lhs.clone()).into();
            let rhs_expr: Expr = (*rhs.clone()).into();
            let resolved_lhs = resolve_expression(ctx, &lhs_expr)?;
            let resolved_rhs = resolve_expression(ctx, &rhs_expr)?;
            let kind = ResolvedExprKind::Mul {
                lhs: resolved_lhs,
                rhs: resolved_rhs,
            };
            let ty = resolved_lhs.ty;
            (kind, ty)
        }
        Expr::Div { lhs, rhs, span: _ } => {
            let lhs_expr: Expr = (*lhs.clone()).into();
            let rhs_expr: Expr = (*rhs.clone()).into();
            let resolved_lhs = resolve_expression(ctx, &lhs_expr)?;
            let resolved_rhs = resolve_expression(ctx, &rhs_expr)?;
            let kind = ResolvedExprKind::Div {
                lhs: resolved_lhs,
                rhs: resolved_rhs,
            };
            let ty = resolved_lhs.ty;
            (kind, ty)
        }
        Expr::Mod { lhs, rhs, span: _ } => {
            let lhs_expr: Expr = (*lhs.clone()).into();
            let rhs_expr: Expr = (*rhs.clone()).into();
            let resolved_lhs = resolve_expression(ctx, &lhs_expr)?;
            let resolved_rhs = resolve_expression(ctx, &rhs_expr)?;
            let kind = ResolvedExprKind::Mod {
                lhs: resolved_lhs,
                rhs: resolved_rhs,
            };
            let ty = resolved_lhs.ty;
            (kind, ty)
        }
        Expr::Pow { lhs, rhs, span: _ } => {
            let lhs_expr: Expr = (*lhs.clone()).into();
            let rhs_expr: Expr = (*rhs.clone()).into();
            let resolved_lhs = resolve_expression(ctx, &lhs_expr)?;
            let resolved_rhs = resolve_expression(ctx, &rhs_expr)?;
            let kind = ResolvedExprKind::Pow {
                lhs: resolved_lhs,
                rhs: resolved_rhs,
            };
            let ty = resolved_lhs.ty;
            (kind, ty)
        }

        // Unary operators
        Expr::Neg { inner, span: _ } => {
            let inner_expr: Expr = (*inner.clone()).into();
            let resolved_inner = resolve_expression(ctx, &inner_expr)?;
            let kind = ResolvedExprKind::Neg {
                inner: resolved_inner,
            };
            let ty = resolved_inner.ty;
            (kind, ty)
        }
        Expr::Ref { inner, span } => {
            let inner_expr: Expr = (*inner.clone()).into();
            let resolved_inner = resolve_expression(ctx, &inner_expr)?;
            let inner_ty = resolved_inner.ty;
            let ref_ty = &*ctx.arena.alloc(ResolvedType::Reference {
                inner: &*ctx.arena.alloc(*inner_ty),
                span: *span,
            });
            let kind = ResolvedExprKind::Ref {
                inner: resolved_inner,
            };
            (kind, ref_ty)
        }

        // Literals
        Expr::IntLit { value, span } => {
            let kind = ResolvedExprKind::IntLit { value: *value };
            let ty = &*ctx.arena.alloc(ResolvedType::I32 { span: *span });
            (kind, ty)
        }
        Expr::FloatLit { value, span } => {
            let kind = ResolvedExprKind::FloatLit { value: *value };
            let ty = &*ctx.arena.alloc(ResolvedType::F64 { span: *span });
            (kind, ty)
        }
        Expr::BoolLit { value, span } => {
            let kind = ResolvedExprKind::BoolLit { value: *value };
            let ty = &*ctx.arena.alloc(ResolvedType::Bool { span: *span });
            (kind, ty)
        }

        // Struct literal
        Expr::StructLit { name, fields, span } => match ctx.lookup_struct(name) {
            Some(struct_def) => {
                // Resolve fields
                let resolved_fields: Vec<_> = fields
                    .iter()
                    .filter_map(|field| resolve_struct_lit_field(ctx, field, struct_def))
                    .collect();

                let kind = ResolvedExprKind::StructLit {
                    name,
                    fields: resolved_fields,
                };
                let ty = &*ctx.arena.alloc(ResolvedType::UserDefined {
                    name,
                    definition: struct_def,
                    span: *span,
                });
                (kind, ty)
            }
            None => {
                ctx.add_error(SemanticError::UndefinedType {
                    name: name.to_string(),
                    span: *span,
                });
                return None;
            }
        },

        // Array literal
        Expr::ArrayLit { elements, span } => {
            let resolved_elements: Vec<_> = elements
                .iter()
                .filter_map(|elem| resolve_expression(ctx, elem))
                .collect();

            let kind = ResolvedExprKind::ArrayLit {
                elements: resolved_elements,
            };
            // TODO: Determine array element type
            let ty = &*ctx.arena.alloc(ResolvedType::I32 { span: *span });
            (kind, ty)
        }

        // Index
        Expr::Index { array, index, span } => {
            let resolved_array = resolve_expression(ctx, array)?;
            let resolved_index = resolve_expression(ctx, index)?;

            let kind = ResolvedExprKind::Index {
                array: resolved_array,
                index: resolved_index,
            };
            // TODO: Determine element type from array type
            let ty = &*ctx.arena.alloc(ResolvedType::I32 { span: *span });
            (kind, ty)
        }

        // Range
        Expr::Range { start, end, span } => {
            let resolved_start = resolve_expression(ctx, start)?;
            let resolved_end = resolve_expression(ctx, end)?;

            let kind = ResolvedExprKind::Range {
                start: resolved_start,
                end: resolved_end,
            };
            // TODO: Define Range type
            let ty = &*ctx.arena.alloc(ResolvedType::I32 { span: *span });
            (kind, ty)
        }

        // Closure
        Expr::Closure { params, body, span } => {
            // Push a new scope for closure parameters
            ctx.scope_stack.push_scope();

            // TODO: Add closure parameters to scope

            let resolved_body = resolve_expression(ctx, body)?;

            ctx.scope_stack.pop_scope();

            let kind = ResolvedExprKind::Closure {
                params: params.clone(),
                body: resolved_body,
            };
            // TODO: Define closure type
            let ty = &*ctx.arena.alloc(ResolvedType::I32 { span: *span });
            (kind, ty)
        }

        // Parentheses
        Expr::Paren { inner, span: _ } => {
            let resolved_inner = resolve_expression(ctx, inner)?;
            let kind = ResolvedExprKind::Paren {
                inner: resolved_inner,
            };
            let ty = resolved_inner.ty;
            (kind, ty)
        }
    };

    let resolved = ctx.arena.alloc(ResolvedExpr::new(span, kind, ty));
    Some(resolved)
}

/// Resolve a struct literal field
fn resolve_struct_lit_field<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    field: &AstStructLitField<'src>,
    struct_def: &'arena crate::hir_definitions::StructDefinition<'src, 'arena>,
) -> Option<ResolvedStructLitField<'src, 'arena>> {
    match field {
        AstStructLitField::Field { name, value, span } => {
            // Look up field in struct
            let field_def = struct_def.find_field(name);
            if let Some(field_def) = field_def {
                // Resolve value expression
                let resolved_value = resolve_expression(ctx, value)?;

                Some(ResolvedStructLitField::Field {
                    name,
                    value: resolved_value,
                    field_def,
                    span: *span,
                })
            } else {
                ctx.add_error(SemanticError::UndefinedField {
                    struct_name: struct_def.name.to_string(),
                    field_name: name.to_string(),
                    span: *span,
                });
                None
            }
        }
        AstStructLitField::ComputedProperty { name, value, span } => {
            // Look up method in struct
            let method_def = struct_def.find_method(name);
            if let Some(method_def) = method_def {
                // Resolve value expression
                let resolved_value = resolve_expression(ctx, value)?;

                Some(ResolvedStructLitField::ComputedProperty {
                    name,
                    value: resolved_value,
                    method_def,
                    span: *span,
                })
            } else {
                ctx.add_error(SemanticError::UndefinedFunction {
                    name: format!("{}.{}", struct_def.name, name),
                    span: *span,
                });
                None
            }
        }
    }
}

// ============================================================================
// Type Resolution
// ============================================================================

/// Resolve an AST type to a HIR ResolvedType
///
/// This is a simplified version that just converts AST types to HIR types.
/// More complex type resolution (generics, etc.) would go here.
fn resolve_type<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    ast_type: &crate::ast::Type,
) -> Option<ResolvedType<'src, 'arena>> {
    match ast_type {
        crate::ast::Type::Bool { span } => Some(ResolvedType::Bool { span: *span }),
        crate::ast::Type::I32 { span } => Some(ResolvedType::I32 { span: *span }),
        crate::ast::Type::F64 { span } => Some(ResolvedType::F64 { span: *span }),
        crate::ast::Type::Real { span } => Some(ResolvedType::Real { span: *span }),
        crate::ast::Type::Algebraic { span } => Some(ResolvedType::Algebraic { span: *span }),
        crate::ast::Type::Reference { inner, span } => {
            let inner_resolved = resolve_type(ctx, inner)?;
            let inner_allocated = ctx.arena.alloc(inner_resolved);
            Some(ResolvedType::Reference {
                inner: inner_allocated,
                span: *span,
            })
        }
        crate::ast::Type::UserDefined { name, span } => {
            let struct_def = ctx.lookup_struct(name);
            if let Some(struct_def) = struct_def {
                // Extract name from source
                let name_src = extract_name(ctx.source, name);
                Some(ResolvedType::UserDefined {
                    name: name_src,
                    definition: struct_def,
                    span: *span,
                })
            } else {
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
/// This ensures names are `&'src str` references into the source text.
fn extract_name<'src>(source: &'src str, name: &str) -> &'src str {
    // Find the name in the source text
    if let Some(idx) = source.find(name) {
        &source[idx..idx + name.len()]
    } else {
        // Fallback: Use a static string if not found
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
    fn test_resolve_int_literal() {
        let arena = Bump::new();
        let source = "42";
        let mut ctx = AnalyzerContext::new(&arena, source);

        let expr = Expr::IntLit {
            value: 42,
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved.kind, ResolvedExprKind::IntLit { value: 42 });
        assert_matches!(resolved.ty, ResolvedType::I32 { .. });
    }

    #[test]
    fn test_resolve_variable_undefined() {
        let arena = Bump::new();
        let source = "x";
        let mut ctx = AnalyzerContext::new(&arena, source);

        let expr = Expr::Var {
            name: "x",
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_none());
        assert!(ctx.has_errors());

        let errors = ctx.take_errors();
        assert_eq!(errors.len(), 1);
        assert_matches!(
            &errors[0],
            SemanticError::UndefinedVariable { name, .. } if name == "x"
        );
    }

    #[test]
    fn test_resolve_variable_defined() {
        let arena = Bump::new();
        let source = "let x: i32 = 42;";
        let mut ctx = AnalyzerContext::new(&arena, source);

        // Define variable
        let var_def = arena.alloc(VarDefinition::new(
            "x",
            make_span(1, 5),
            Some(ResolvedType::I32 {
                span: make_span(1, 8),
            }),
            None,
            0,
            make_span(1, 1),
        ));
        ctx.scope_stack.declare_variable("x", var_def);

        let expr = Expr::Var {
            name: "x",
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved.kind, ResolvedExprKind::Var { .. });
    }

    #[test]
    fn test_resolve_binary_add() {
        let arena = Bump::new();
        let source = "1 + 2";
        let mut ctx = AnalyzerContext::new(&arena, source);

        use crate::ast::expr::{AddLhs, AddRhs};

        let expr = Expr::Add {
            lhs: Box::new(AddLhs::IntLit {
                value: 1,
                span: make_span(1, 1),
            }),
            rhs: Box::new(AddRhs::IntLit {
                value: 2,
                span: make_span(1, 5),
            }),
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved.kind, ResolvedExprKind::Add { .. });
    }

    #[test]
    fn test_resolve_function_call_undefined() {
        let arena = Bump::new();
        let source = "foo()";
        let mut ctx = AnalyzerContext::new(&arena, source);

        let expr = Expr::Call {
            name: "foo",
            args: vec![],
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_none());
        assert!(ctx.has_errors());

        let errors = ctx.take_errors();
        assert_eq!(errors.len(), 1);
        assert_matches!(
            &errors[0],
            SemanticError::UndefinedFunction { name, .. } if name == "foo"
        );
    }

    #[test]
    fn test_resolve_block_with_scoping() {
        let arena = Bump::new();
        let source = "{ let x = 1; }";
        let mut ctx = AnalyzerContext::new(&arena, source);

        let stmts = vec![Stmt::Let {
            dot_prefix: false,
            name_path: vec![("x", make_span(1, 7))],
            type_annotation: None,
            init: Some(Expr::IntLit {
                value: 1,
                span: make_span(1, 11),
            }),
            span: make_span(1, 3),
        }];

        let block_stmt = Stmt::Block {
            statements: stmts,
            span: make_span(1, 1),
        };

        // Scope level should be 0 before resolving block
        assert_eq!(ctx.scope_stack.current_scope_level(), 0);

        let resolved = resolve_statement(&mut ctx, &block_stmt);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        // Scope level should be back to 0 after resolving block
        assert_eq!(ctx.scope_stack.current_scope_level(), 0);
    }

    #[test]
    fn test_resolve_for_loop() {
        let arena = Bump::new();
        let source = "for i in 0..10 { }";
        let mut ctx = AnalyzerContext::new(&arena, source);

        let for_stmt = Stmt::For {
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
            body: vec![],
            span: make_span(1, 1),
        };

        let resolved = resolve_statement(&mut ctx, &for_stmt);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_resolve_with_statement_no_context_error() {
        let arena = Bump::new();
        let source = "let .field = 42;";
        let mut ctx = AnalyzerContext::new(&arena, source);

        // Try to resolve a dot-prefix let outside of a with context
        let let_stmt = Stmt::Let {
            dot_prefix: true,
            name_path: vec![("field", make_span(1, 6))],
            type_annotation: None,
            init: Some(Expr::IntLit {
                value: 42,
                span: make_span(1, 14),
            }),
            span: make_span(1, 1),
        };

        let resolved = resolve_statement(&mut ctx, &let_stmt);
        assert!(resolved.is_none());
        assert!(ctx.has_errors());

        let errors = ctx.take_errors();
        assert_eq!(errors.len(), 1);
        assert_matches!(&errors[0], SemanticError::NotInWithContext { .. });
    }

    #[test]
    fn test_resolve_shadowing() {
        let arena = Bump::new();
        let source = "let x = 1; { let x = 2; }";
        let mut ctx = AnalyzerContext::new(&arena, source);

        // Declare x in outer scope
        let outer_x = arena.alloc(VarDefinition::new(
            "x",
            make_span(1, 5),
            None,
            None,
            0,
            make_span(1, 1),
        ));
        ctx.scope_stack.declare_variable("x", outer_x);

        // Enter inner scope
        ctx.scope_stack.push_scope();

        // Declare x in inner scope (shadows outer x)
        let inner_x = arena.alloc(VarDefinition::new(
            "x",
            make_span(1, 17),
            None,
            None,
            1,
            make_span(1, 13),
        ));
        let old = ctx.scope_stack.declare_variable("x", inner_x);
        assert!(old.is_none()); // No duplicate in same scope

        // Lookup should find inner x
        let found = ctx.scope_stack.lookup_variable("x");
        assert!(found.is_some());
        assert!(std::ptr::eq(found.unwrap(), inner_x));

        // Exit inner scope
        ctx.scope_stack.pop_scope();

        // Lookup should now find outer x
        let found = ctx.scope_stack.lookup_variable("x");
        assert!(found.is_some());
        assert!(std::ptr::eq(found.unwrap(), outer_x));
    }

    // ========================================================================
    // Literal Expression Tests
    // ========================================================================

    #[test]
    fn test_resolve_float_literal() {
        let arena = Bump::new();
        let source = "3.14";
        let mut ctx = AnalyzerContext::new(&arena, source);

        let expr = Expr::FloatLit {
            value: 3.14,
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved.kind, ResolvedExprKind::FloatLit { value } if (value - 3.14).abs() < 0.001);
        assert_matches!(resolved.ty, ResolvedType::F64 { .. });
    }

    #[test]
    fn test_resolve_bool_literal_true() {
        let arena = Bump::new();
        let source = "true";
        let mut ctx = AnalyzerContext::new(&arena, source);

        let expr = Expr::BoolLit {
            value: true,
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved.kind, ResolvedExprKind::BoolLit { value: true });
        assert_matches!(resolved.ty, ResolvedType::Bool { .. });
    }

    #[test]
    fn test_resolve_bool_literal_false() {
        let arena = Bump::new();
        let source = "false";
        let mut ctx = AnalyzerContext::new(&arena, source);

        let expr = Expr::BoolLit {
            value: false,
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved.kind, ResolvedExprKind::BoolLit { value: false });
        assert_matches!(resolved.ty, ResolvedType::Bool { .. });
    }

    // ========================================================================
    // Comparison Operator Tests
    // ========================================================================

    #[test]
    fn test_resolve_eq() {
        let arena = Bump::new();
        let source = "1 == 2";
        let mut ctx = AnalyzerContext::new(&arena, source);

        use crate::ast::expr::{CmpLhs, CmpRhs};

        let expr = Expr::Eq {
            lhs: Box::new(CmpLhs::IntLit {
                value: 1,
                span: make_span(1, 1),
            }),
            rhs: Box::new(CmpRhs::IntLit {
                value: 2,
                span: make_span(1, 6),
            }),
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved.kind, ResolvedExprKind::Eq { .. });
        assert_matches!(resolved.ty, ResolvedType::Bool { .. });
    }

    #[test]
    fn test_resolve_not_eq() {
        let arena = Bump::new();
        let source = "1 != 2";
        let mut ctx = AnalyzerContext::new(&arena, source);

        use crate::ast::expr::{CmpLhs, CmpRhs};

        let expr = Expr::NotEq {
            lhs: Box::new(CmpLhs::IntLit {
                value: 1,
                span: make_span(1, 1),
            }),
            rhs: Box::new(CmpRhs::IntLit {
                value: 2,
                span: make_span(1, 6),
            }),
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved.kind, ResolvedExprKind::NotEq { .. });
        assert_matches!(resolved.ty, ResolvedType::Bool { .. });
    }

    #[test]
    fn test_resolve_lt() {
        let arena = Bump::new();
        let source = "1 < 2";
        let mut ctx = AnalyzerContext::new(&arena, source);

        use crate::ast::expr::{CmpLhs, CmpRhs};

        let expr = Expr::Lt {
            lhs: Box::new(CmpLhs::IntLit {
                value: 1,
                span: make_span(1, 1),
            }),
            rhs: Box::new(CmpRhs::IntLit {
                value: 2,
                span: make_span(1, 5),
            }),
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved.kind, ResolvedExprKind::Lt { .. });
        assert_matches!(resolved.ty, ResolvedType::Bool { .. });
    }

    #[test]
    fn test_resolve_gt() {
        let arena = Bump::new();
        let source = "1 > 2";
        let mut ctx = AnalyzerContext::new(&arena, source);

        use crate::ast::expr::{CmpLhs, CmpRhs};

        let expr = Expr::Gt {
            lhs: Box::new(CmpLhs::IntLit {
                value: 1,
                span: make_span(1, 1),
            }),
            rhs: Box::new(CmpRhs::IntLit {
                value: 2,
                span: make_span(1, 5),
            }),
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved.kind, ResolvedExprKind::Gt { .. });
        assert_matches!(resolved.ty, ResolvedType::Bool { .. });
    }

    #[test]
    fn test_resolve_lt_eq() {
        let arena = Bump::new();
        let source = "1 <= 2";
        let mut ctx = AnalyzerContext::new(&arena, source);

        use crate::ast::expr::{CmpLhs, CmpRhs};

        let expr = Expr::LtEq {
            lhs: Box::new(CmpLhs::IntLit {
                value: 1,
                span: make_span(1, 1),
            }),
            rhs: Box::new(CmpRhs::IntLit {
                value: 2,
                span: make_span(1, 6),
            }),
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved.kind, ResolvedExprKind::LtEq { .. });
        assert_matches!(resolved.ty, ResolvedType::Bool { .. });
    }

    #[test]
    fn test_resolve_gt_eq() {
        let arena = Bump::new();
        let source = "1 >= 2";
        let mut ctx = AnalyzerContext::new(&arena, source);

        use crate::ast::expr::{CmpLhs, CmpRhs};

        let expr = Expr::GtEq {
            lhs: Box::new(CmpLhs::IntLit {
                value: 1,
                span: make_span(1, 1),
            }),
            rhs: Box::new(CmpRhs::IntLit {
                value: 2,
                span: make_span(1, 6),
            }),
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved.kind, ResolvedExprKind::GtEq { .. });
        assert_matches!(resolved.ty, ResolvedType::Bool { .. });
    }

    // ========================================================================
    // Logical Operator Tests
    // ========================================================================

    #[test]
    fn test_resolve_and() {
        let arena = Bump::new();
        let source = "true && false";
        let mut ctx = AnalyzerContext::new(&arena, source);

        use crate::ast::expr::{CmpLhs, CmpRhs};

        let expr = Expr::And {
            lhs: Box::new(CmpLhs::BoolLit {
                value: true,
                span: make_span(1, 1),
            }),
            rhs: Box::new(CmpRhs::BoolLit {
                value: false,
                span: make_span(1, 9),
            }),
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved.kind, ResolvedExprKind::And { .. });
        assert_matches!(resolved.ty, ResolvedType::Bool { .. });
    }

    #[test]
    fn test_resolve_or() {
        let arena = Bump::new();
        let source = "true || false";
        let mut ctx = AnalyzerContext::new(&arena, source);

        use crate::ast::expr::{CmpLhs, CmpRhs};

        let expr = Expr::Or {
            lhs: Box::new(CmpLhs::BoolLit {
                value: true,
                span: make_span(1, 1),
            }),
            rhs: Box::new(CmpRhs::BoolLit {
                value: false,
                span: make_span(1, 9),
            }),
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved.kind, ResolvedExprKind::Or { .. });
        assert_matches!(resolved.ty, ResolvedType::Bool { .. });
    }

    // ========================================================================
    // Arithmetic Operator Tests
    // ========================================================================

    #[test]
    fn test_resolve_sub() {
        let arena = Bump::new();
        let source = "5 - 3";
        let mut ctx = AnalyzerContext::new(&arena, source);

        use crate::ast::expr::{AddLhs, AddRhs};

        let expr = Expr::Sub {
            lhs: Box::new(AddLhs::IntLit {
                value: 5,
                span: make_span(1, 1),
            }),
            rhs: Box::new(AddRhs::IntLit {
                value: 3,
                span: make_span(1, 5),
            }),
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved.kind, ResolvedExprKind::Sub { .. });
    }

    #[test]
    fn test_resolve_mul() {
        let arena = Bump::new();
        let source = "3 * 4";
        let mut ctx = AnalyzerContext::new(&arena, source);

        use crate::ast::expr::{MulLhs, MulRhs};

        let expr = Expr::Mul {
            lhs: Box::new(MulLhs::IntLit {
                value: 3,
                span: make_span(1, 1),
            }),
            rhs: Box::new(MulRhs::IntLit {
                value: 4,
                span: make_span(1, 5),
            }),
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved.kind, ResolvedExprKind::Mul { .. });
    }

    #[test]
    fn test_resolve_div() {
        let arena = Bump::new();
        let source = "10 / 2";
        let mut ctx = AnalyzerContext::new(&arena, source);

        use crate::ast::expr::{MulLhs, MulRhs};

        let expr = Expr::Div {
            lhs: Box::new(MulLhs::IntLit {
                value: 10,
                span: make_span(1, 1),
            }),
            rhs: Box::new(MulRhs::IntLit {
                value: 2,
                span: make_span(1, 6),
            }),
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved.kind, ResolvedExprKind::Div { .. });
    }

    #[test]
    fn test_resolve_mod() {
        let arena = Bump::new();
        let source = "10 % 3";
        let mut ctx = AnalyzerContext::new(&arena, source);

        use crate::ast::expr::{MulLhs, MulRhs};

        let expr = Expr::Mod {
            lhs: Box::new(MulLhs::IntLit {
                value: 10,
                span: make_span(1, 1),
            }),
            rhs: Box::new(MulRhs::IntLit {
                value: 3,
                span: make_span(1, 6),
            }),
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved.kind, ResolvedExprKind::Mod { .. });
    }

    #[test]
    fn test_resolve_pow() {
        let arena = Bump::new();
        let source = "2 ^ 3";
        let mut ctx = AnalyzerContext::new(&arena, source);

        use crate::ast::expr::{PowLhs, PowRhs};

        let expr = Expr::Pow {
            lhs: Box::new(PowLhs::IntLit {
                value: 2,
                span: make_span(1, 1),
            }),
            rhs: Box::new(PowRhs::IntLit {
                value: 3,
                span: make_span(1, 5),
            }),
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved.kind, ResolvedExprKind::Pow { .. });
    }

    // ========================================================================
    // Unary Operator Tests
    // ========================================================================

    #[test]
    fn test_resolve_neg() {
        let arena = Bump::new();
        let source = "-42";
        let mut ctx = AnalyzerContext::new(&arena, source);

        use crate::ast::expr::PowLhs;

        let expr = Expr::Neg {
            inner: Box::new(PowLhs::IntLit {
                value: 42,
                span: make_span(1, 2),
            }),
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved.kind, ResolvedExprKind::Neg { .. });
    }

    #[test]
    fn test_resolve_ref() {
        let arena = Bump::new();
        let source = "&42";
        let mut ctx = AnalyzerContext::new(&arena, source);

        use crate::ast::expr::PowLhs;

        let expr = Expr::Ref {
            inner: Box::new(PowLhs::IntLit {
                value: 42,
                span: make_span(1, 2),
            }),
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved.kind, ResolvedExprKind::Ref { .. });
        assert_matches!(resolved.ty, ResolvedType::Reference { .. });
    }

    // ========================================================================
    // Complex Expression Tests
    // ========================================================================

    #[test]
    fn test_resolve_paren() {
        let arena = Bump::new();
        let source = "(42)";
        let mut ctx = AnalyzerContext::new(&arena, source);

        let expr = Expr::Paren {
            inner: Box::new(Expr::IntLit {
                value: 42,
                span: make_span(1, 2),
            }),
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved.kind, ResolvedExprKind::Paren { .. });
    }

    #[test]
    fn test_resolve_array_lit() {
        let arena = Bump::new();
        let source = "[1, 2, 3]";
        let mut ctx = AnalyzerContext::new(&arena, source);

        let expr = Expr::ArrayLit {
            elements: vec![
                Expr::IntLit {
                    value: 1,
                    span: make_span(1, 2),
                },
                Expr::IntLit {
                    value: 2,
                    span: make_span(1, 5),
                },
                Expr::IntLit {
                    value: 3,
                    span: make_span(1, 8),
                },
            ],
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved.kind, ResolvedExprKind::ArrayLit { .. });
    }

    #[test]
    fn test_resolve_index() {
        let arena = Bump::new();
        let source = "arr[0]";
        let mut ctx = AnalyzerContext::new(&arena, source);

        // Define array variable
        let arr_def = arena.alloc(VarDefinition::new(
            "arr",
            make_span(1, 1),
            None,
            None,
            0,
            make_span(1, 1),
        ));
        ctx.scope_stack.declare_variable("arr", arr_def);

        let expr = Expr::Index {
            array: Box::new(Expr::Var {
                name: "arr",
                span: make_span(1, 1),
            }),
            index: Box::new(Expr::IntLit {
                value: 0,
                span: make_span(1, 5),
            }),
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved.kind, ResolvedExprKind::Index { .. });
    }

    #[test]
    fn test_resolve_range() {
        let arena = Bump::new();
        let source = "1..10";
        let mut ctx = AnalyzerContext::new(&arena, source);

        let expr = Expr::Range {
            start: Box::new(Expr::IntLit {
                value: 1,
                span: make_span(1, 1),
            }),
            end: Box::new(Expr::IntLit {
                value: 10,
                span: make_span(1, 4),
            }),
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved.kind, ResolvedExprKind::Range { .. });
    }

    #[test]
    fn test_resolve_closure() {
        let arena = Bump::new();
        let source = "|x| x + 1";
        let mut ctx = AnalyzerContext::new(&arena, source);

        let expr = Expr::Closure {
            params: vec!["x"],
            body: Box::new(Expr::IntLit {
                value: 1,
                span: make_span(1, 9),
            }),
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved.kind, ResolvedExprKind::Closure { .. });
    }

    #[test]
    fn test_resolve_struct_lit() {
        let arena = Bump::new();
        let source = "Point { x: 1, y: 2 }";
        let mut ctx = AnalyzerContext::new(&arena, source);

        use crate::hir_definitions::{FieldDefinition, StructDefinition};
        use crate::hir_types::ResolvedType;

        // Create struct definition
        let field_x = arena.alloc(FieldDefinition::new(
            "x",
            make_span(1, 9),
            ResolvedType::I32 {
                span: make_span(1, 9),
            },
            make_span(1, 9),
        ));
        let field_y = arena.alloc(FieldDefinition::new(
            "y",
            make_span(1, 15),
            ResolvedType::I32 {
                span: make_span(1, 15),
            },
            make_span(1, 15),
        ));
        let struct_def = arena.alloc(StructDefinition::new(
            "Point",
            make_span(1, 1),
            vec![field_x, field_y],
            vec![],
            None,
            make_span(1, 1),
        ));
        ctx.register_struct("Point", struct_def).unwrap();

        use crate::ast::StructLitField;
        let expr = Expr::StructLit {
            name: "Point",
            fields: vec![
                StructLitField::Field {
                    name: "x",
                    value: Expr::IntLit {
                        value: 1,
                        span: make_span(1, 12),
                    },
                    span: make_span(1, 9),
                },
                StructLitField::Field {
                    name: "y",
                    value: Expr::IntLit {
                        value: 2,
                        span: make_span(1, 18),
                    },
                    span: make_span(1, 15),
                },
            ],
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved.kind, ResolvedExprKind::StructLit { .. });
        assert_matches!(resolved.ty, ResolvedType::UserDefined { .. });
    }

    #[test]
    fn test_resolve_struct_lit_undefined_field() {
        let arena = Bump::new();
        let source = "Point { z: 1 }";
        let mut ctx = AnalyzerContext::new(&arena, source);

        use crate::hir_definitions::{FieldDefinition, StructDefinition};
        use crate::hir_types::ResolvedType;

        // Create struct definition without field z
        let field_x = arena.alloc(FieldDefinition::new(
            "x",
            make_span(1, 9),
            ResolvedType::I32 {
                span: make_span(1, 9),
            },
            make_span(1, 9),
        ));
        let struct_def = arena.alloc(StructDefinition::new(
            "Point",
            make_span(1, 1),
            vec![field_x],
            vec![],
            None,
            make_span(1, 1),
        ));
        ctx.register_struct("Point", struct_def).unwrap();

        use crate::ast::StructLitField;
        let expr = Expr::StructLit {
            name: "Point",
            fields: vec![StructLitField::Field {
                name: "z",
                value: Expr::IntLit {
                    value: 1,
                    span: make_span(1, 12),
                },
                span: make_span(1, 9),
            }],
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        // The struct literal resolves, but one field fails
        assert!(resolved.is_some());
        assert!(ctx.has_errors());

        let errors = ctx.take_errors();
        assert_eq!(errors.len(), 1);
        assert_matches!(&errors[0], SemanticError::UndefinedField { .. });
    }

    #[test]
    fn test_resolve_field_access() {
        let arena = Bump::new();
        let source = "p.x";
        let mut ctx = AnalyzerContext::new(&arena, source);

        // Define variable p
        let p_def = arena.alloc(VarDefinition::new(
            "p",
            make_span(1, 1),
            None,
            None,
            0,
            make_span(1, 1),
        ));
        ctx.scope_stack.declare_variable("p", p_def);

        let expr = Expr::FieldAccess {
            receiver: Box::new(Expr::Var {
                name: "p",
                span: make_span(1, 1),
            }),
            field: "x",
            span: make_span(1, 1),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_resolve_container_field_access_in_with() {
        let arena = Bump::new();
        let source = "with obj { .field }";
        let mut ctx = AnalyzerContext::new(&arena, source);

        // Create a variable for the with context
        let obj_def = arena.alloc(VarDefinition::new(
            "obj",
            make_span(1, 6),
            None,
            None,
            0,
            make_span(1, 6),
        ));
        ctx.scope_stack.declare_variable("obj", obj_def);

        // Create with context expression
        let context_expr = Expr::Var {
            name: "obj",
            span: make_span(1, 6),
        };

        let resolved_context = resolve_expression(&mut ctx, &context_expr).unwrap();

        use crate::hir_context::WithContext;
        let with_ctx = ctx
            .arena
            .alloc(WithContext::new_transform(resolved_context, vec![]));

        // Enter with context
        ctx.scope_stack.enter_with_context(with_ctx);

        // Now resolve container field access
        let expr = Expr::ContainerFieldAccess {
            field_path: vec!["field"],
            span: make_span(1, 12),
        };

        let resolved = resolve_expression(&mut ctx, &expr);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved.kind, ResolvedExprKind::ContainerFieldAccess { .. });

        ctx.scope_stack.exit_with_context();
    }

    // ========================================================================
    // Statement Tests
    // ========================================================================

    #[test]
    fn test_resolve_assignment_success() {
        let arena = Bump::new();
        let source = "let x = 1; x = 2;";
        let mut ctx = AnalyzerContext::new(&arena, source);

        // Define variable x
        let x_def = arena.alloc(VarDefinition::new(
            "x",
            make_span(1, 5),
            None,
            None,
            0,
            make_span(1, 1),
        ));
        ctx.scope_stack.declare_variable("x", x_def);

        let stmt = Stmt::Assignment {
            name: "x",
            name_span: make_span(1, 12),
            value: Expr::IntLit {
                value: 2,
                span: make_span(1, 16),
            },
            span: make_span(1, 12),
        };

        let resolved = resolve_statement(&mut ctx, &stmt);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_resolve_field_assignment_regular() {
        let arena = Bump::new();
        let source = "obj.field = 42;";
        let mut ctx = AnalyzerContext::new(&arena, source);

        // Declare the obj variable first
        let obj_def = arena.alloc(VarDefinition::new(
            "obj",
            make_span(1, 1),
            None,
            None,
            0,
            make_span(1, 1),
        ));
        ctx.scope_stack.declare_variable("obj", obj_def);

        let stmt = Stmt::FieldAssignment {
            dot_prefix: false,
            field_path: vec![("obj", make_span(1, 1)), ("field", make_span(1, 5))],
            value: Expr::IntLit {
                value: 42,
                span: make_span(1, 13),
            },
            span: make_span(1, 1),
        };

        let resolved = resolve_statement(&mut ctx, &stmt);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_resolve_field_assignment_dot_prefix() {
        let arena = Bump::new();
        let source = "with obj { .field = 42; }";
        let mut ctx = AnalyzerContext::new(&arena, source);

        // Create a variable for the with context
        let obj_def = arena.alloc(VarDefinition::new(
            "obj",
            make_span(1, 6),
            None,
            None,
            0,
            make_span(1, 6),
        ));
        ctx.scope_stack.declare_variable("obj", obj_def);

        let context_expr = Expr::Var {
            name: "obj",
            span: make_span(1, 6),
        };

        let resolved_context = resolve_expression(&mut ctx, &context_expr).unwrap();

        use crate::hir_context::WithContext;
        let with_ctx = ctx
            .arena
            .alloc(WithContext::new_transform(resolved_context, vec![]));

        ctx.scope_stack.enter_with_context(with_ctx);

        let stmt = Stmt::FieldAssignment {
            dot_prefix: true,
            field_path: vec![("field", make_span(1, 13))],
            value: Expr::IntLit {
                value: 42,
                span: make_span(1, 21),
            },
            span: make_span(1, 12),
        };

        let resolved = resolve_statement(&mut ctx, &stmt);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        ctx.scope_stack.exit_with_context();
    }

    #[test]
    fn test_resolve_function_body_with_params() {
        let arena = Bump::new();
        let source = "fn add(a: i32, b: i32) -> i32 { a + b }";
        let mut ctx = AnalyzerContext::new(&arena, source);

        use crate::ast::expr::{AddLhs, AddRhs};
        use crate::ast::{FunctionParam, Type};
        use crate::hir_definitions::FunctionDefinition;
        use crate::hir_types::ResolvedType;

        let params = vec![
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
                name_span: make_span(1, 16),
                type_annotation: Type::I32 {
                    span: make_span(1, 19),
                },
                span: make_span(1, 16),
            },
        ];

        // Declare the function first (simulating Pass 1)
        let func_def = arena.alloc(FunctionDefinition {
            name: "add",
            name_span: make_span(1, 4),
            params: vec![],
            return_type: ResolvedType::I32 {
                span: make_span(1, 27),
            },
            body: vec![],
            parent_struct: None,
            span: make_span(1, 1),
        });
        ctx.register_function("add", func_def);

        let return_expr = Expr::Add {
            lhs: Box::new(AddLhs::Var {
                name: "a",
                span: make_span(1, 33),
            }),
            rhs: Box::new(AddRhs::Var {
                name: "b",
                span: make_span(1, 37),
            }),
            span: make_span(1, 33),
        };

        let stmt = Stmt::FunctionDef {
            name: "add".to_string(),
            name_span: make_span(1, 4),
            params,
            return_type: Type::I32 {
                span: make_span(1, 27),
            },
            body: vec![],
            return_expr: Some(return_expr),
            span: make_span(1, 1),
        };

        let resolved = resolve_statement(&mut ctx, &stmt);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_resolve_with_statement_success() {
        let arena = Bump::new();
        let source = "with obj { let x = 1; }";
        let mut ctx = AnalyzerContext::new(&arena, source);

        // Create a variable for the with context
        let obj_def = arena.alloc(VarDefinition::new(
            "obj",
            make_span(1, 6),
            None,
            None,
            0,
            make_span(1, 6),
        ));
        ctx.scope_stack.declare_variable("obj", obj_def);

        let stmt = Stmt::With {
            context_expr: Expr::Var {
                name: "obj",
                span: make_span(1, 6),
            },
            body: vec![Stmt::Let {
                dot_prefix: false,
                name_path: vec![("x", make_span(1, 16))],
                type_annotation: None,
                init: Some(Expr::IntLit {
                    value: 1,
                    span: make_span(1, 20),
                }),
                span: make_span(1, 12),
            }],
            span: make_span(1, 1),
        };

        let resolved = resolve_statement(&mut ctx, &stmt);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_resolve_if_statement_then_only() {
        let arena = Bump::new();
        let source = "if true { let x = 1; }";
        let mut ctx = AnalyzerContext::new(&arena, source);

        let stmt = Stmt::If {
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
            else_branch: None,
            span: make_span(1, 1),
        };

        let resolved = resolve_statement(&mut ctx, &stmt);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_resolve_if_statement_with_else() {
        let arena = Bump::new();
        let source = "if true { let x = 1; } else { let y = 2; }";
        let mut ctx = AnalyzerContext::new(&arena, source);

        let stmt = Stmt::If {
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
        };

        let resolved = resolve_statement(&mut ctx, &stmt);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_resolve_return_statement_with_value() {
        let arena = Bump::new();
        let source = "return 42;";
        let mut ctx = AnalyzerContext::new(&arena, source);

        let stmt = Stmt::Return {
            value: Some(Expr::IntLit {
                value: 42,
                span: make_span(1, 8),
            }),
            span: make_span(1, 1),
        };

        let resolved = resolve_statement(&mut ctx, &stmt);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_resolve_return_statement_no_value() {
        let arena = Bump::new();
        let source = "return;";
        let mut ctx = AnalyzerContext::new(&arena, source);

        let stmt = Stmt::Return {
            value: None,
            span: make_span(1, 1),
        };

        let resolved = resolve_statement(&mut ctx, &stmt);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());
    }

    // ========================================================================
    // Type Resolution Tests
    // ========================================================================

    #[test]
    fn test_resolve_type_bool() {
        let arena = Bump::new();
        let source = "bool";
        let mut ctx = AnalyzerContext::new(&arena, source);

        use crate::ast::Type;
        let ast_type = Type::Bool {
            span: make_span(1, 1),
        };

        let resolved = resolve_type(&mut ctx, &ast_type);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved, ResolvedType::Bool { .. });
    }

    #[test]
    fn test_resolve_type_i32() {
        let arena = Bump::new();
        let source = "i32";
        let mut ctx = AnalyzerContext::new(&arena, source);

        use crate::ast::Type;
        let ast_type = Type::I32 {
            span: make_span(1, 1),
        };

        let resolved = resolve_type(&mut ctx, &ast_type);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved, ResolvedType::I32 { .. });
    }

    #[test]
    fn test_resolve_type_f64() {
        let arena = Bump::new();
        let source = "f64";
        let mut ctx = AnalyzerContext::new(&arena, source);

        use crate::ast::Type;
        let ast_type = Type::F64 {
            span: make_span(1, 1),
        };

        let resolved = resolve_type(&mut ctx, &ast_type);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved, ResolvedType::F64 { .. });
    }

    #[test]
    fn test_resolve_type_real() {
        let arena = Bump::new();
        let source = "real";
        let mut ctx = AnalyzerContext::new(&arena, source);

        use crate::ast::Type;
        let ast_type = Type::Real {
            span: make_span(1, 1),
        };

        let resolved = resolve_type(&mut ctx, &ast_type);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved, ResolvedType::Real { .. });
    }

    #[test]
    fn test_resolve_type_algebraic() {
        let arena = Bump::new();
        let source = "algebraic";
        let mut ctx = AnalyzerContext::new(&arena, source);

        use crate::ast::Type;
        let ast_type = Type::Algebraic {
            span: make_span(1, 1),
        };

        let resolved = resolve_type(&mut ctx, &ast_type);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved, ResolvedType::Algebraic { .. });
    }

    #[test]
    fn test_resolve_type_reference() {
        let arena = Bump::new();
        let source = "&i32";
        let mut ctx = AnalyzerContext::new(&arena, source);

        use crate::ast::Type;
        let ast_type = Type::Reference {
            inner: Box::new(Type::I32 {
                span: make_span(1, 2),
            }),
            span: make_span(1, 1),
        };

        let resolved = resolve_type(&mut ctx, &ast_type);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved, ResolvedType::Reference { .. });
    }

    #[test]
    fn test_resolve_type_user_defined() {
        let arena = Bump::new();
        let source = "Point";
        let mut ctx = AnalyzerContext::new(&arena, source);

        use crate::hir_definitions::StructDefinition;

        // Create and register a struct definition
        let struct_def = arena.alloc(StructDefinition::new(
            "Point",
            make_span(1, 1),
            vec![],
            vec![],
            None,
            make_span(1, 1),
        ));
        ctx.register_struct("Point", struct_def).unwrap();

        use crate::ast::Type;
        let ast_type = Type::UserDefined {
            name: "Point".to_string(),
            span: make_span(1, 1),
        };

        let resolved = resolve_type(&mut ctx, &ast_type);
        assert!(resolved.is_some());
        assert!(!ctx.has_errors());

        let resolved = resolved.unwrap();
        assert_matches!(resolved, ResolvedType::UserDefined { .. });
    }
}
