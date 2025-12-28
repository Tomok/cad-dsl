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

// Allow dead code for now since this module is not yet fully integrated
#![allow(dead_code)]

use crate::ast::{Expr, Stmt, StructLitField as AstStructLitField};
use crate::hir_context::WithContext;
use crate::hir_definitions::{ScopeLevel, VarDefinition};
use crate::hir_expr::{ResolvedExpr, ResolvedExprKind, ResolvedStructLitField};
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
/// A vector of resolved HIR statements (currently using AST as placeholder)
///
/// # Error Handling
///
/// Errors are collected in `ctx.errors`. Resolution continues even when
/// errors are encountered to collect as many errors as possible.
pub fn resolve_statements<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    stmts: &[Stmt<'src>],
) -> Vec<&'arena Stmt<'src>> {
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
) -> Option<&'arena Stmt<'src>> {
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

        Stmt::StructDef { .. } => {
            // Struct definitions are already processed in Pass 1
            // Return the statement as-is (arena-allocated)
            Some(ctx.arena.alloc(stmt.clone()))
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
) -> Option<&'arena Stmt<'src>> {
    // Simple let statement (no dot prefix, single name)
    if !dot_prefix && name_path.len() == 1 {
        let (name, name_span) = name_path[0];

        // Resolve type annotation if present
        let resolved_type = type_annotation.and_then(|ty| resolve_type(ctx, ty));

        // Resolve initializer expression if present
        let init_expr = init.and_then(|expr| resolve_expression(ctx, expr));

        // Get current scope level
        let scope_level: ScopeLevel = ctx.scope_stack.current_scope_level();

        // Create variable definition
        let var_def = ctx.arena.alloc(VarDefinition::new(
            name,
            name_span,
            resolved_type,
            init_expr,
            scope_level,
            span,
        ));

        // Declare the variable in the current scope
        if let Some(old_def) = ctx.scope_stack.declare_variable(name, var_def) {
            // Duplicate variable definition in the same scope
            ctx.add_error(SemanticError::DuplicateDefinition {
                name: name.to_string(),
                first_span: old_def.name_span,
                second_span: name_span,
            });
        }

        // Return the statement (for now, just clone the AST)
        Some(ctx.arena.alloc(Stmt::Let {
            dot_prefix,
            name_path: name_path.to_vec(),
            type_annotation: type_annotation.cloned(),
            init: init.cloned(),
            span,
        }))
    } else if dot_prefix {
        // Dot-prefix let (e.g., `let .field = value;`)
        // Check if we're in a with-context
        if ctx.scope_stack.current_with_context().is_none() {
            ctx.add_error(SemanticError::NotInWithContext { span });
            return None;
        }

        // Resolve the initializer
        let _init_expr = init.and_then(|expr| resolve_expression(ctx, expr));

        // TODO: Handle dot-prefix let statements properly
        // For now, just return the statement
        Some(ctx.arena.alloc(Stmt::Let {
            dot_prefix,
            name_path: name_path.to_vec(),
            type_annotation: type_annotation.cloned(),
            init: init.cloned(),
            span,
        }))
    } else {
        // Path let (e.g., `let container.field = value;`)
        // Resolve the initializer
        let _init_expr = init.and_then(|expr| resolve_expression(ctx, expr));

        // TODO: Handle container field let statements properly
        // For now, just return the statement
        Some(ctx.arena.alloc(Stmt::Let {
            dot_prefix,
            name_path: name_path.to_vec(),
            type_annotation: type_annotation.cloned(),
            init: init.cloned(),
            span,
        }))
    }
}

/// Resolve an assignment statement
fn resolve_assignment<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    name: &'src str,
    name_span: Span,
    value: &Expr<'src>,
    span: Span,
) -> Option<&'arena Stmt<'src>> {
    // Lookup the variable
    if ctx.scope_stack.lookup_variable(name).is_none() {
        ctx.add_error(SemanticError::UndefinedVariable {
            name: name.to_string(),
            span: name_span,
        });
    }

    // Resolve the value expression
    let _value_expr = resolve_expression(ctx, value);

    // Return the statement (for now, just clone the AST)
    Some(ctx.arena.alloc(Stmt::Assignment {
        name,
        name_span,
        value: value.clone(),
        span,
    }))
}

/// Resolve a field assignment statement
fn resolve_field_assignment<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    dot_prefix: bool,
    field_path: &[(&'src str, Span)],
    value: &Expr<'src>,
    span: Span,
) -> Option<&'arena Stmt<'src>> {
    if dot_prefix {
        // Dot-prefix field assignment (e.g., `.field = value;`)
        // Check if we're in a with-context
        if ctx.scope_stack.current_with_context().is_none() {
            ctx.add_error(SemanticError::NotInWithContext { span });
            return None;
        }

        // TODO: Resolve field path using with-context
    } else {
        // Regular field assignment (e.g., `obj.field = value;`)
        // TODO: Resolve field path
    }

    // Resolve the value expression
    let _value_expr = resolve_expression(ctx, value);

    // Return the statement (for now, just clone the AST)
    Some(ctx.arena.alloc(Stmt::FieldAssignment {
        dot_prefix,
        field_path: field_path.to_vec(),
        value: value.clone(),
        span,
    }))
}

/// Resolve a function body
#[allow(clippy::too_many_arguments)]
fn resolve_function_body<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    name: &str,
    _name_span: Span,
    _params: &[crate::ast::FunctionParam],
    _return_type: &crate::ast::Type,
    body: &[Stmt<'src>],
    return_expr: Option<&Expr<'src>>,
    span: Span,
) -> Option<&'arena Stmt<'src>> {
    // Function definition is already in the symbol table from Pass 1
    // Now we resolve the body

    // Push a new scope for the function body
    ctx.scope_stack.push_scope();

    // TODO: Add function parameters to the scope

    // Resolve body statements
    let _resolved_body = resolve_statements(ctx, body);

    // Resolve return expression if present
    let _resolved_return = return_expr.and_then(|expr| resolve_expression(ctx, expr));

    // Pop the function scope
    ctx.scope_stack.pop_scope();

    // Return the statement (for now, just clone the AST)
    // In a full implementation, we would update the function definition with the resolved body
    let stmt = ctx.arena.alloc(Stmt::FunctionDef {
        name: name.to_string(),
        name_span: _name_span,
        params: _params.to_vec(),
        return_type: _return_type.clone(),
        body: body.to_vec(),
        return_expr: return_expr.cloned(),
        span,
    });
    Some(stmt)
}

/// Resolve a with statement
fn resolve_with_statement<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    context_expr: &Expr<'src>,
    body: &[Stmt<'src>],
    span: Span,
) -> Option<&'arena Stmt<'src>> {
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
    let _resolved_body = resolve_statements(ctx, body);

    // Exit the with-context
    ctx.scope_stack.exit_with_context();

    // Return the statement (for now, just clone the AST)
    Some(ctx.arena.alloc(Stmt::With {
        context_expr: context_expr.clone(),
        body: body.to_vec(),
        span,
    }))
}

/// Resolve a for statement
fn resolve_for_statement<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    loop_var: &'src str,
    loop_var_span: Span,
    iterator: &Expr<'src>,
    body: &[Stmt<'src>],
    span: Span,
) -> Option<&'arena Stmt<'src>> {
    // Resolve the iterator expression
    let _iterator_expr = resolve_expression(ctx, iterator);

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
    let _resolved_body = resolve_statements(ctx, body);

    // Pop the loop scope
    ctx.scope_stack.pop_scope();

    // Return the statement (for now, just clone the AST)
    Some(ctx.arena.alloc(Stmt::For {
        loop_var,
        loop_var_span,
        iterator: iterator.clone(),
        body: body.to_vec(),
        span,
    }))
}

/// Resolve an if statement
fn resolve_if_statement<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    condition: &Expr<'src>,
    then_branch: &[Stmt<'src>],
    else_branch: Option<&Vec<Stmt<'src>>>,
    span: Span,
) -> Option<&'arena Stmt<'src>> {
    // Resolve the condition
    let _condition_expr = resolve_expression(ctx, condition);

    // Resolve then branch
    let _resolved_then = resolve_statements(ctx, then_branch);

    // Resolve else branch if present
    let _resolved_else = else_branch.map(|stmts| resolve_statements(ctx, stmts));

    // Return the statement (for now, just clone the AST)
    Some(ctx.arena.alloc(Stmt::If {
        condition: condition.clone(),
        then_branch: then_branch.to_vec(),
        else_branch: else_branch.cloned(),
        span,
    }))
}

/// Resolve a block statement
fn resolve_block_statement<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    statements: &[Stmt<'src>],
    span: Span,
) -> Option<&'arena Stmt<'src>> {
    // Push a new scope for the block
    ctx.scope_stack.push_scope();

    // Resolve statements in the block
    let _resolved = resolve_statements(ctx, statements);

    // Pop the block scope
    ctx.scope_stack.pop_scope();

    // Return the statement (for now, just clone the AST)
    Some(ctx.arena.alloc(Stmt::Block {
        statements: statements.to_vec(),
        span,
    }))
}

/// Resolve a return statement
fn resolve_return_statement<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    value: Option<&Expr<'src>>,
    span: Span,
) -> Option<&'arena Stmt<'src>> {
    // Resolve the return value if present
    let _value_expr = value.and_then(|expr| resolve_expression(ctx, expr));

    // Return the statement (for now, just clone the AST)
    Some(ctx.arena.alloc(Stmt::Return {
        value: value.cloned(),
        span,
    }))
}

/// Resolve an expression statement
fn resolve_expression_statement<'src, 'arena>(
    ctx: &mut AnalyzerContext<'src, 'arena>,
    expr: &Expr<'src>,
    span: Span,
) -> Option<&'arena Stmt<'src>> {
    // Resolve the expression
    let _resolved_expr = resolve_expression(ctx, expr);

    // Return the statement (for now, just clone the AST)
    Some(ctx.arena.alloc(Stmt::Expression {
        expr: expr.clone(),
        span,
    }))
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

        use crate::ast::expr::{PowLhs, PowRhs};

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
}
