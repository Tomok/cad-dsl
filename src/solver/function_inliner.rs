//! Function Call Inlining for Constraint Solver
//!
//! This module implements function call inlining to transform HIR before constraint
//! extraction. Function calls are replaced with their function bodies, with parameters
//! substituted by argument expressions.
//!
//! # Purpose
//!
//! The constraint solver cannot directly handle function calls, so we inline them
//! before constraint extraction. This allows functions to be used for common patterns
//! while keeping the solver simple.
//!
//! # Algorithm
//!
//! 1. Walk the HIR statements and expressions
//! 2. When a function call is found:
//!    - Extract the return expression from the function body
//!    - Create a mapping of parameters to argument expressions
//!    - Recursively substitute parameters with arguments in the return expression
//!    - Replace the function call with the inlined expression
//! 3. Detect and reject recursive function calls
//!
//! # Example
//!
//! ```text
//! // Input HIR:
//! fn add(a: i32, b: i32) -> i32 {
//!     return a + b;
//! }
//! let x = add(10, 20);
//!
//! // After inlining:
//! let x = 10 + 20;
//! ```
//!
//! # Limitations
//!
//! - Only functions with a single return statement are supported
//! - No recursive function calls
//! - No complex control flow (if, for, etc.) in function bodies
//! - Parameters must be used exactly once (no sharing/cloning issues)

use crate::hir::definitions::FunctionDefinition;
use crate::hir::expr::{
    ResolvedExpr, ResolvedExprKind, ResolvedStmt, ResolvedStmtKind, ResolvedStructLitField,
};
use crate::lexer::Span;
use bumpalo::Bump;
use std::collections::HashMap;
use std::fmt;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during function inlining
#[derive(Debug, Clone, PartialEq)]
pub enum FunctionInlinerError {
    /// Recursive function call detected
    RecursiveCall {
        function_name: String,
        call_chain: Vec<String>,
        span: Span,
    },

    /// Function has no return statement
    NoReturnStatement { function_name: String, span: Span },

    /// Function has unsupported control flow
    UnsupportedControlFlow {
        function_name: String,
        statement_type: String,
        span: Span,
    },

    /// Function body has multiple statements before return
    ComplexFunctionBody { function_name: String, span: Span },
}

impl fmt::Display for FunctionInlinerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FunctionInlinerError::RecursiveCall {
                function_name,
                call_chain,
                span,
            } => {
                write!(
                    f,
                    "Recursive function call to '{}' at line {}, column {}. Call chain: {}",
                    function_name,
                    span.start.line,
                    span.start.column,
                    call_chain.join(" -> ")
                )
            }
            FunctionInlinerError::NoReturnStatement {
                function_name,
                span,
            } => {
                write!(
                    f,
                    "Function '{}' at line {}, column {} has no return statement",
                    function_name, span.start.line, span.start.column
                )
            }
            FunctionInlinerError::UnsupportedControlFlow {
                function_name,
                statement_type,
                span,
            } => {
                write!(
                    f,
                    "Function '{}' at line {}, column {} has unsupported control flow: {}",
                    function_name, span.start.line, span.start.column, statement_type
                )
            }
            FunctionInlinerError::ComplexFunctionBody {
                function_name,
                span,
            } => {
                write!(
                    f,
                    "Function '{}' at line {}, column {} has complex body with multiple statements. Only simple functions with a single return statement are supported.",
                    function_name, span.start.line, span.start.column
                )
            }
        }
    }
}

impl std::error::Error for FunctionInlinerError {}

// ============================================================================
// Function Inliner Context
// ============================================================================

/// Information about a resolved function for inlining
#[derive(Debug, Clone)]
struct FunctionInfo<'src, 'arena> {
    /// Function definition
    definition: &'arena FunctionDefinition<'src, 'arena>,
    /// The return expression extracted from the function body
    return_expr: &'arena ResolvedExpr<'src, 'arena>,
    /// Function body span for error reporting
    span: Span,
}

/// Context for function inlining, tracking call stack and arena
struct InlinerContext<'src, 'arena> {
    /// Arena for allocating new HIR nodes
    arena: &'arena Bump,

    /// Current call stack for recursion detection
    /// Stores function names currently being inlined
    call_stack: Vec<&'src str>,

    /// Map of qualified function name to function information
    /// Built during the first pass over statements
    /// Methods use "StructName::method_name" as keys to avoid collisions
    function_map: HashMap<String, FunctionInfo<'src, 'arena>>,
}

/// Get a qualified name for a function or method
/// For methods, returns "StructName::method_name"
/// For functions, returns just the function name
fn get_qualified_name<'src, 'arena>(func_def: &FunctionDefinition<'src, 'arena>) -> String {
    if let Some(parent_struct) = func_def.parent_struct {
        format!("{}::{}", parent_struct.name, func_def.name)
    } else {
        func_def.name.to_string()
    }
}

impl<'src, 'arena> InlinerContext<'src, 'arena> {
    /// Create a new inliner context
    fn new(arena: &'arena Bump) -> Self {
        Self {
            arena,
            call_stack: Vec::new(),
            function_map: HashMap::new(),
        }
    }

    /// Push a function onto the call stack
    /// Returns an error if recursion is detected
    fn push_call(
        &mut self,
        function_name: &'src str,
        span: Span,
    ) -> Result<(), FunctionInlinerError> {
        // Check if this function is already in the call stack (recursion)
        if self.call_stack.contains(&function_name) {
            let mut call_chain = self
                .call_stack
                .clone()
                .into_iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>();
            call_chain.push(function_name.to_string());
            return Err(FunctionInlinerError::RecursiveCall {
                function_name: function_name.to_string(),
                call_chain,
                span,
            });
        }

        self.call_stack.push(function_name);
        Ok(())
    }

    /// Pop a function from the call stack
    fn pop_call(&mut self) {
        self.call_stack.pop();
    }

    /// Register a function definition for inlining
    fn register_function(
        &mut self,
        func_def: &'arena FunctionDefinition<'src, 'arena>,
        body: &[&'arena ResolvedStmt<'src, 'arena>],
        return_expr: Option<&'arena ResolvedExpr<'src, 'arena>>,
        span: Span,
    ) -> Result<(), FunctionInlinerError> {
        // Extract the return expression from the body
        let ret_expr = if let Some(expr) = return_expr {
            // Implicit return
            expr
        } else {
            // Look for a return statement in the body
            extract_return_from_body(func_def.name, body, span)?
        };

        // Use qualified name for methods to avoid collisions
        let key = get_qualified_name(func_def);
        self.function_map.insert(
            key,
            FunctionInfo {
                definition: func_def,
                return_expr: ret_expr,
                span,
            },
        );
        Ok(())
    }

    /// Get function information by name
    fn get_function(&self, name: &str) -> Option<&FunctionInfo<'src, 'arena>> {
        self.function_map.get(name)
    }

    /// Get function information by function definition
    /// This is used for method calls where we have the resolved method definition
    fn get_function_by_def(
        &self,
        func_def: &FunctionDefinition<'src, 'arena>,
    ) -> Option<&FunctionInfo<'src, 'arena>> {
        let key = get_qualified_name(func_def);
        self.function_map.get(&key)
    }
}

// ============================================================================
// Main Entry Point
// ============================================================================

/// Inline all function calls in a list of HIR statements
///
/// This is the main entry point for function inlining. It performs two passes:
/// 1. First pass: collect all function definitions and their return expressions
/// 2. Second pass: inline function calls in all statements
///
/// # Arguments
///
/// * `statements` - The HIR statements to process
/// * `arena` - Arena allocator for creating new HIR nodes
///
/// # Returns
///
/// * `Ok(Vec<HirStmt>)` - The transformed statements with inlined functions
/// * `Err(FunctionInlinerError)` - If inlining fails (recursion, unsupported features, etc.)
pub fn inline_functions<'src, 'arena>(
    statements: &[&'arena ResolvedStmt<'src, 'arena>],
    arena: &'arena Bump,
) -> Result<Vec<&'arena ResolvedStmt<'src, 'arena>>, FunctionInlinerError> {
    let mut context = InlinerContext::new(arena);

    // Pass 1: Collect function definitions
    for stmt in statements {
        collect_functions(stmt, &mut context)?;
    }

    // Pass 2: Inline function calls and filter out definitions
    let mut result = Vec::new();
    for stmt in statements {
        let inlined_stmt = inline_statement(stmt, &mut context)?;

        // Filter out function and struct definitions - they're not executable statements
        // and have already been processed during collection phase
        match &inlined_stmt.kind {
            ResolvedStmtKind::FunctionDef { .. } | ResolvedStmtKind::StructDef { .. } => {
                // Skip definitions - they don't contribute to constraints
            }
            _ => {
                result.push(inlined_stmt);
            }
        }
    }

    Ok(result)
}

// ============================================================================
// Function Collection (Pass 1)
// ============================================================================

/// Collect function definitions from statements
fn collect_functions<'src, 'arena>(
    stmt: &'arena ResolvedStmt<'src, 'arena>,
    context: &mut InlinerContext<'src, 'arena>,
) -> Result<(), FunctionInlinerError> {
    match &stmt.kind {
        ResolvedStmtKind::FunctionDef {
            func_def,
            body,
            return_expr,
            span,
        } => {
            // Register this function
            context.register_function(func_def, body, *return_expr, *span)?;
        }

        // Recursively collect from nested statements
        ResolvedStmtKind::Block { statements, .. } => {
            for s in statements {
                collect_functions(s, context)?;
            }
        }

        ResolvedStmtKind::If {
            then_branch,
            else_branch,
            ..
        } => {
            for s in then_branch {
                collect_functions(s, context)?;
            }
            if let Some(else_stmts) = else_branch {
                for s in else_stmts {
                    collect_functions(s, context)?;
                }
            }
        }

        // Collect methods from struct definitions
        ResolvedStmtKind::StructDef { methods, .. } => {
            for method_stmt in methods {
                collect_functions(method_stmt, context)?;
            }
        }

        // Other statement types don't contain function definitions
        _ => {}
    }

    Ok(())
}

// ============================================================================
// Statement Inlining
// ============================================================================

/// Inline function calls in a single statement
fn inline_statement<'src, 'arena>(
    stmt: &'arena ResolvedStmt<'src, 'arena>,
    context: &mut InlinerContext<'src, 'arena>,
) -> Result<&'arena ResolvedStmt<'src, 'arena>, FunctionInlinerError> {
    let kind = match &stmt.kind {
        // Let statement with initializer - inline the initializer
        ResolvedStmtKind::Let {
            dot_prefix,
            name_path,
            var_def,
            init: Some(init_expr),
            span,
        } => {
            let inlined_init = inline_expression(init_expr, context)?;
            ResolvedStmtKind::Let {
                dot_prefix: *dot_prefix,
                name_path: name_path.clone(),
                var_def,
                init: Some(inlined_init),
                span: *span,
            }
        }

        // Let statement without initializer - no inlining needed
        ResolvedStmtKind::Let { .. } => stmt.kind.clone(),

        // Assignment - inline the value expression
        ResolvedStmtKind::Assignment {
            var_def,
            value,
            span,
        } => {
            let inlined_value = inline_expression(value, context)?;
            ResolvedStmtKind::Assignment {
                var_def,
                value: inlined_value,
                span: *span,
            }
        }

        // Field assignment - inline both target and value
        ResolvedStmtKind::FieldAssignment {
            target,
            value,
            span,
        } => {
            let inlined_target = inline_expression(target, context)?;
            let inlined_value = inline_expression(value, context)?;
            ResolvedStmtKind::FieldAssignment {
                target: inlined_target,
                value: inlined_value,
                span: *span,
            }
        }

        // Expression statement - inline the expression
        ResolvedStmtKind::Expression { expr, span } => {
            let inlined_expr = inline_expression(expr, context)?;
            ResolvedStmtKind::Expression {
                expr: inlined_expr,
                span: *span,
            }
        }

        // If statement - inline condition and branches
        ResolvedStmtKind::If {
            condition,
            then_branch,
            else_branch,
            span,
        } => {
            let inlined_condition = inline_expression(condition, context)?;
            let inlined_then = then_branch
                .iter()
                .map(|s| inline_statement(s, context))
                .collect::<Result<Vec<_>, _>>()?;
            let inlined_else = if let Some(else_stmts) = else_branch {
                Some(
                    else_stmts
                        .iter()
                        .map(|s| inline_statement(s, context))
                        .collect::<Result<Vec<_>, _>>()?,
                )
            } else {
                None
            };
            ResolvedStmtKind::If {
                condition: inlined_condition,
                then_branch: inlined_then,
                else_branch: inlined_else,
                span: *span,
            }
        }

        // Block statement - inline all statements in the block
        ResolvedStmtKind::Block { statements, span } => {
            let inlined_statements = statements
                .iter()
                .map(|s| inline_statement(s, context))
                .collect::<Result<Vec<_>, _>>()?;
            ResolvedStmtKind::Block {
                statements: inlined_statements,
                span: *span,
            }
        }

        // Return statement - inline the return value
        ResolvedStmtKind::Return { value, span } => {
            let inlined_value = if let Some(v) = value {
                Some(inline_expression(v, context)?)
            } else {
                None
            };
            ResolvedStmtKind::Return {
                value: inlined_value,
                span: *span,
            }
        }

        // Function definitions, struct definitions - pass through unchanged
        // These are definitions, not executable statements
        ResolvedStmtKind::FunctionDef { .. } | ResolvedStmtKind::StructDef { .. } => {
            stmt.kind.clone()
        }

        // For loops and With statements - pass through for now
        // These will be handled by other transformation passes
        ResolvedStmtKind::For { .. } | ResolvedStmtKind::With { .. } => stmt.kind.clone(),
    };

    Ok(context.arena.alloc(ResolvedStmt {
        span: stmt.span,
        kind,
    }))
}

// ============================================================================
// Expression Inlining
// ============================================================================

/// Inline function calls in an expression
fn inline_expression<'src, 'arena>(
    expr: &'arena ResolvedExpr<'src, 'arena>,
    context: &mut InlinerContext<'src, 'arena>,
) -> Result<&'arena ResolvedExpr<'src, 'arena>, FunctionInlinerError> {
    let kind = match &expr.kind {
        // Function call - this is what we inline!
        ResolvedExprKind::FunctionCall {
            name,
            function,
            args,
        } => {
            // Check for recursion
            context.push_call(name, expr.span)?;

            // Inline arguments first (they might contain function calls too)
            let inlined_args = args
                .iter()
                .map(|arg| inline_expression(arg, context))
                .collect::<Result<Vec<_>, _>>()?;

            // Get the function information from the map
            let func_info = context.get_function(name).ok_or_else(|| {
                FunctionInlinerError::NoReturnStatement {
                    function_name: name.to_string(),
                    span: expr.span,
                }
            })?;

            // Create parameter substitution map
            let mut param_map = HashMap::new();
            for (param, arg) in function.params.iter().zip(inlined_args.iter()) {
                param_map.insert(param.name, *arg);
            }

            // Substitute parameters in the return expression
            let inlined_body =
                substitute_parameters_impl(func_info.return_expr, &param_map, context)?;

            // Pop the call from the stack
            context.pop_call();

            // Return the inlined expression directly
            return Ok(inlined_body);
        }

        // Method call - similar to function call but with receiver as implicit first parameter
        ResolvedExprKind::MethodCall {
            receiver,
            method_name,
            method,
            args,
        } => {
            // Check for recursion using the method name
            context.push_call(method_name, expr.span)?;

            // Inline receiver first (it might contain function/method calls)
            let inlined_receiver = inline_expression(receiver, context)?;

            // Inline arguments (they might contain function/method calls too)
            let inlined_args = args
                .iter()
                .map(|arg| inline_expression(arg, context))
                .collect::<Result<Vec<_>, _>>()?;

            // Get the method information from the function map using the resolved definition
            // This ensures we get the correct method even if multiple structs have methods
            // with the same name
            let func_info = context.get_function_by_def(method).ok_or_else(|| {
                FunctionInlinerError::NoReturnStatement {
                    function_name: method_name.to_string(),
                    span: expr.span,
                }
            })?;

            // Create parameter substitution map
            // First, map "self" to the receiver
            let mut param_map = HashMap::new();
            param_map.insert("self", inlined_receiver);

            // Then, map the explicit parameters to the arguments
            for (param, arg) in method.params.iter().zip(inlined_args.iter()) {
                param_map.insert(param.name, *arg);
            }

            // Substitute parameters in the return expression
            let inlined_body =
                substitute_parameters_impl(func_info.return_expr, &param_map, context)?;

            // Pop the call from the stack
            context.pop_call();

            // Return the inlined expression directly
            return Ok(inlined_body);
        }

        // Binary operations - inline both operands
        ResolvedExprKind::And { lhs, rhs } => {
            let inlined_lhs = inline_expression(lhs, context)?;
            let inlined_rhs = inline_expression(rhs, context)?;
            ResolvedExprKind::And {
                lhs: inlined_lhs,
                rhs: inlined_rhs,
            }
        }

        ResolvedExprKind::Or { lhs, rhs } => {
            let inlined_lhs = inline_expression(lhs, context)?;
            let inlined_rhs = inline_expression(rhs, context)?;
            ResolvedExprKind::Or {
                lhs: inlined_lhs,
                rhs: inlined_rhs,
            }
        }

        ResolvedExprKind::Eq { lhs, rhs } => {
            let inlined_lhs = inline_expression(lhs, context)?;
            let inlined_rhs = inline_expression(rhs, context)?;
            ResolvedExprKind::Eq {
                lhs: inlined_lhs,
                rhs: inlined_rhs,
            }
        }

        ResolvedExprKind::NotEq { lhs, rhs } => {
            let inlined_lhs = inline_expression(lhs, context)?;
            let inlined_rhs = inline_expression(rhs, context)?;
            ResolvedExprKind::NotEq {
                lhs: inlined_lhs,
                rhs: inlined_rhs,
            }
        }

        ResolvedExprKind::Lt { lhs, rhs } => {
            let inlined_lhs = inline_expression(lhs, context)?;
            let inlined_rhs = inline_expression(rhs, context)?;
            ResolvedExprKind::Lt {
                lhs: inlined_lhs,
                rhs: inlined_rhs,
            }
        }

        ResolvedExprKind::Gt { lhs, rhs } => {
            let inlined_lhs = inline_expression(lhs, context)?;
            let inlined_rhs = inline_expression(rhs, context)?;
            ResolvedExprKind::Gt {
                lhs: inlined_lhs,
                rhs: inlined_rhs,
            }
        }

        ResolvedExprKind::LtEq { lhs, rhs } => {
            let inlined_lhs = inline_expression(lhs, context)?;
            let inlined_rhs = inline_expression(rhs, context)?;
            ResolvedExprKind::LtEq {
                lhs: inlined_lhs,
                rhs: inlined_rhs,
            }
        }

        ResolvedExprKind::GtEq { lhs, rhs } => {
            let inlined_lhs = inline_expression(lhs, context)?;
            let inlined_rhs = inline_expression(rhs, context)?;
            ResolvedExprKind::GtEq {
                lhs: inlined_lhs,
                rhs: inlined_rhs,
            }
        }

        ResolvedExprKind::Add { lhs, rhs } => {
            let inlined_lhs = inline_expression(lhs, context)?;
            let inlined_rhs = inline_expression(rhs, context)?;
            ResolvedExprKind::Add {
                lhs: inlined_lhs,
                rhs: inlined_rhs,
            }
        }

        ResolvedExprKind::Sub { lhs, rhs } => {
            let inlined_lhs = inline_expression(lhs, context)?;
            let inlined_rhs = inline_expression(rhs, context)?;
            ResolvedExprKind::Sub {
                lhs: inlined_lhs,
                rhs: inlined_rhs,
            }
        }

        ResolvedExprKind::Mul { lhs, rhs } => {
            let inlined_lhs = inline_expression(lhs, context)?;
            let inlined_rhs = inline_expression(rhs, context)?;
            ResolvedExprKind::Mul {
                lhs: inlined_lhs,
                rhs: inlined_rhs,
            }
        }

        ResolvedExprKind::Div { lhs, rhs } => {
            let inlined_lhs = inline_expression(lhs, context)?;
            let inlined_rhs = inline_expression(rhs, context)?;
            ResolvedExprKind::Div {
                lhs: inlined_lhs,
                rhs: inlined_rhs,
            }
        }

        ResolvedExprKind::Mod { lhs, rhs } => {
            let inlined_lhs = inline_expression(lhs, context)?;
            let inlined_rhs = inline_expression(rhs, context)?;
            ResolvedExprKind::Mod {
                lhs: inlined_lhs,
                rhs: inlined_rhs,
            }
        }

        ResolvedExprKind::Pow { lhs, rhs } => {
            let inlined_lhs = inline_expression(lhs, context)?;
            let inlined_rhs = inline_expression(rhs, context)?;
            ResolvedExprKind::Pow {
                lhs: inlined_lhs,
                rhs: inlined_rhs,
            }
        }

        // Unary operations - inline the inner expression
        ResolvedExprKind::Neg { inner } => {
            let inlined_inner = inline_expression(inner, context)?;
            ResolvedExprKind::Neg {
                inner: inlined_inner,
            }
        }

        ResolvedExprKind::Ref { inner } => {
            let inlined_inner = inline_expression(inner, context)?;
            ResolvedExprKind::Ref {
                inner: inlined_inner,
            }
        }

        // Struct literal - inline field values
        ResolvedExprKind::StructLit { name, fields } => {
            let inlined_fields = fields
                .iter()
                .map(|field| match field {
                    ResolvedStructLitField::Field {
                        name,
                        value,
                        field_def,
                        span,
                    } => {
                        let inlined_value = inline_expression(value, context)?;
                        Ok(ResolvedStructLitField::Field {
                            name,
                            value: inlined_value,
                            field_def,
                            span: *span,
                        })
                    }
                    ResolvedStructLitField::ComputedProperty {
                        name,
                        value,
                        method_def,
                        span,
                    } => {
                        let inlined_value = inline_expression(value, context)?;
                        Ok(ResolvedStructLitField::ComputedProperty {
                            name,
                            value: inlined_value,
                            method_def,
                            span: *span,
                        })
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            ResolvedExprKind::StructLit {
                name,
                fields: inlined_fields,
            }
        }

        // Array literal - inline elements
        ResolvedExprKind::ArrayLit { elements } => {
            let inlined_elements = elements
                .iter()
                .map(|elem| inline_expression(elem, context))
                .collect::<Result<Vec<_>, _>>()?;
            ResolvedExprKind::ArrayLit {
                elements: inlined_elements,
            }
        }

        // Array indexing - inline array and index
        ResolvedExprKind::Index { array, index } => {
            let inlined_array = inline_expression(array, context)?;
            let inlined_index = inline_expression(index, context)?;
            ResolvedExprKind::Index {
                array: inlined_array,
                index: inlined_index,
            }
        }

        // Range - inline start and end
        ResolvedExprKind::Range { start, end } => {
            let inlined_start = inline_expression(start, context)?;
            let inlined_end = inline_expression(end, context)?;
            ResolvedExprKind::Range {
                start: inlined_start,
                end: inlined_end,
            }
        }

        // Closure - inline body
        ResolvedExprKind::Closure { params, body } => {
            let inlined_body = inline_expression(body, context)?;
            ResolvedExprKind::Closure {
                params: params.clone(),
                body: inlined_body,
            }
        }

        // Parenthesized expression - inline inner
        ResolvedExprKind::Paren { inner } => {
            let inlined_inner = inline_expression(inner, context)?;
            ResolvedExprKind::Paren {
                inner: inlined_inner,
            }
        }

        // Literals and variables - no inlining needed
        ResolvedExprKind::IntLit { .. }
        | ResolvedExprKind::FloatLit { .. }
        | ResolvedExprKind::BoolLit { .. }
        | ResolvedExprKind::Var { .. } => expr.kind.clone(),

        // Field access, container field access - inline receivers
        ResolvedExprKind::FieldAccess {
            receiver,
            field_name,
            field,
        } => {
            let inlined_receiver = inline_expression(receiver, context)?;
            ResolvedExprKind::FieldAccess {
                receiver: inlined_receiver,
                field_name,
                field,
            }
        }

        ResolvedExprKind::ContainerFieldAccess {
            resolved_path,
            with_context,
            transform,
        } => {
            let inlined_transform = if let Some(t) = transform {
                Some(inline_expression(t, context)?)
            } else {
                None
            };
            ResolvedExprKind::ContainerFieldAccess {
                resolved_path: resolved_path.clone(),
                with_context,
                transform: inlined_transform,
            }
        }
    };

    Ok(context.arena.alloc(ResolvedExpr {
        span: expr.span,
        kind,
        ty: expr.ty,
    }))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract the return expression from a function body
///
/// Looks for a return statement in the body and extracts its value.
/// For simple functions, this should be the last (and only) statement.
fn extract_return_from_body<'src, 'arena>(
    function_name: &'src str,
    body: &[&'arena ResolvedStmt<'src, 'arena>],
    span: Span,
) -> Result<&'arena ResolvedExpr<'src, 'arena>, FunctionInlinerError> {
    // Simple case: single return statement
    if body.len() == 1
        && let ResolvedStmtKind::Return {
            value: Some(expr), ..
        } = &body[0].kind
    {
        return Ok(expr);
    }

    // Look for the last return statement
    for stmt in body.iter().rev() {
        if let ResolvedStmtKind::Return {
            value: Some(expr), ..
        } = &stmt.kind
        {
            return Ok(expr);
        }
    }

    // No return statement found
    Err(FunctionInlinerError::NoReturnStatement {
        function_name: function_name.to_string(),
        span,
    })
}

/// Substitute parameters with argument expressions in an expression
///
/// This recursively walks the expression tree and replaces variable references
/// to parameters with the corresponding argument expressions.
///
/// # Public API
///
/// This function is made public to support transform with-statement processing
/// in the constraint extractor, which needs to inline transform function bodies.
pub fn substitute_parameters<'src, 'arena>(
    expr: &'arena ResolvedExpr<'src, 'arena>,
    param_map: &HashMap<&'src str, &'arena ResolvedExpr<'src, 'arena>>,
    arena: &'arena Bump,
) -> Result<&'arena ResolvedExpr<'src, 'arena>, FunctionInlinerError> {
    // Create a minimal context for parameter substitution
    let context = InlinerContext {
        arena,
        call_stack: Vec::new(),
        function_map: HashMap::new(),
    };
    substitute_parameters_impl(expr, param_map, &context)
}

/// Implementation of parameter substitution
fn substitute_parameters_impl<'src, 'arena>(
    expr: &'arena ResolvedExpr<'src, 'arena>,
    param_map: &HashMap<&'src str, &'arena ResolvedExpr<'src, 'arena>>,
    context: &InlinerContext<'src, 'arena>,
) -> Result<&'arena ResolvedExpr<'src, 'arena>, FunctionInlinerError> {
    let kind = match &expr.kind {
        // Variable reference - check if it's a parameter
        ResolvedExprKind::Var { name, .. } => {
            // If this variable name is in the parameter map, substitute it
            if let Some(arg_expr) = param_map.get(name) {
                // Return the argument expression directly
                return Ok(*arg_expr);
            } else {
                // Not a parameter, keep as is
                expr.kind.clone()
            }
        }

        // Binary operations - recursively substitute in both operands
        ResolvedExprKind::And { lhs, rhs } => {
            let sub_lhs = substitute_parameters_impl(lhs, param_map, context)?;
            let sub_rhs = substitute_parameters_impl(rhs, param_map, context)?;
            ResolvedExprKind::And {
                lhs: sub_lhs,
                rhs: sub_rhs,
            }
        }

        ResolvedExprKind::Or { lhs, rhs } => {
            let sub_lhs = substitute_parameters_impl(lhs, param_map, context)?;
            let sub_rhs = substitute_parameters_impl(rhs, param_map, context)?;
            ResolvedExprKind::Or {
                lhs: sub_lhs,
                rhs: sub_rhs,
            }
        }

        ResolvedExprKind::Eq { lhs, rhs } => {
            let sub_lhs = substitute_parameters_impl(lhs, param_map, context)?;
            let sub_rhs = substitute_parameters_impl(rhs, param_map, context)?;
            ResolvedExprKind::Eq {
                lhs: sub_lhs,
                rhs: sub_rhs,
            }
        }

        ResolvedExprKind::NotEq { lhs, rhs } => {
            let sub_lhs = substitute_parameters_impl(lhs, param_map, context)?;
            let sub_rhs = substitute_parameters_impl(rhs, param_map, context)?;
            ResolvedExprKind::NotEq {
                lhs: sub_lhs,
                rhs: sub_rhs,
            }
        }

        ResolvedExprKind::Lt { lhs, rhs } => {
            let sub_lhs = substitute_parameters_impl(lhs, param_map, context)?;
            let sub_rhs = substitute_parameters_impl(rhs, param_map, context)?;
            ResolvedExprKind::Lt {
                lhs: sub_lhs,
                rhs: sub_rhs,
            }
        }

        ResolvedExprKind::Gt { lhs, rhs } => {
            let sub_lhs = substitute_parameters_impl(lhs, param_map, context)?;
            let sub_rhs = substitute_parameters_impl(rhs, param_map, context)?;
            ResolvedExprKind::Gt {
                lhs: sub_lhs,
                rhs: sub_rhs,
            }
        }

        ResolvedExprKind::LtEq { lhs, rhs } => {
            let sub_lhs = substitute_parameters_impl(lhs, param_map, context)?;
            let sub_rhs = substitute_parameters_impl(rhs, param_map, context)?;
            ResolvedExprKind::LtEq {
                lhs: sub_lhs,
                rhs: sub_rhs,
            }
        }

        ResolvedExprKind::GtEq { lhs, rhs } => {
            let sub_lhs = substitute_parameters_impl(lhs, param_map, context)?;
            let sub_rhs = substitute_parameters_impl(rhs, param_map, context)?;
            ResolvedExprKind::GtEq {
                lhs: sub_lhs,
                rhs: sub_rhs,
            }
        }

        ResolvedExprKind::Add { lhs, rhs } => {
            let sub_lhs = substitute_parameters_impl(lhs, param_map, context)?;
            let sub_rhs = substitute_parameters_impl(rhs, param_map, context)?;
            ResolvedExprKind::Add {
                lhs: sub_lhs,
                rhs: sub_rhs,
            }
        }

        ResolvedExprKind::Sub { lhs, rhs } => {
            let sub_lhs = substitute_parameters_impl(lhs, param_map, context)?;
            let sub_rhs = substitute_parameters_impl(rhs, param_map, context)?;
            ResolvedExprKind::Sub {
                lhs: sub_lhs,
                rhs: sub_rhs,
            }
        }

        ResolvedExprKind::Mul { lhs, rhs } => {
            let sub_lhs = substitute_parameters_impl(lhs, param_map, context)?;
            let sub_rhs = substitute_parameters_impl(rhs, param_map, context)?;
            ResolvedExprKind::Mul {
                lhs: sub_lhs,
                rhs: sub_rhs,
            }
        }

        ResolvedExprKind::Div { lhs, rhs } => {
            let sub_lhs = substitute_parameters_impl(lhs, param_map, context)?;
            let sub_rhs = substitute_parameters_impl(rhs, param_map, context)?;
            ResolvedExprKind::Div {
                lhs: sub_lhs,
                rhs: sub_rhs,
            }
        }

        ResolvedExprKind::Mod { lhs, rhs } => {
            let sub_lhs = substitute_parameters_impl(lhs, param_map, context)?;
            let sub_rhs = substitute_parameters_impl(rhs, param_map, context)?;
            ResolvedExprKind::Mod {
                lhs: sub_lhs,
                rhs: sub_rhs,
            }
        }

        ResolvedExprKind::Pow { lhs, rhs } => {
            let sub_lhs = substitute_parameters_impl(lhs, param_map, context)?;
            let sub_rhs = substitute_parameters_impl(rhs, param_map, context)?;
            ResolvedExprKind::Pow {
                lhs: sub_lhs,
                rhs: sub_rhs,
            }
        }

        // Unary operations - recursively substitute in the inner expression
        ResolvedExprKind::Neg { inner } => {
            let sub_inner = substitute_parameters_impl(inner, param_map, context)?;
            ResolvedExprKind::Neg { inner: sub_inner }
        }

        ResolvedExprKind::Ref { inner } => {
            let sub_inner = substitute_parameters_impl(inner, param_map, context)?;
            ResolvedExprKind::Ref { inner: sub_inner }
        }

        ResolvedExprKind::Paren { inner } => {
            let sub_inner = substitute_parameters_impl(inner, param_map, context)?;
            ResolvedExprKind::Paren { inner: sub_inner }
        }

        // Array indexing - substitute in array and index
        ResolvedExprKind::Index { array, index } => {
            let sub_array = substitute_parameters_impl(array, param_map, context)?;
            let sub_index = substitute_parameters_impl(index, param_map, context)?;
            ResolvedExprKind::Index {
                array: sub_array,
                index: sub_index,
            }
        }

        // Struct literal - substitute in field values
        ResolvedExprKind::StructLit { name, fields } => {
            let sub_fields = fields
                .iter()
                .map(|field| match field {
                    ResolvedStructLitField::Field {
                        name,
                        value,
                        field_def,
                        span,
                    } => {
                        let sub_value = substitute_parameters_impl(value, param_map, context)?;
                        Ok(ResolvedStructLitField::Field {
                            name,
                            value: sub_value,
                            field_def,
                            span: *span,
                        })
                    }
                    ResolvedStructLitField::ComputedProperty {
                        name,
                        value,
                        method_def,
                        span,
                    } => {
                        let sub_value = substitute_parameters_impl(value, param_map, context)?;
                        Ok(ResolvedStructLitField::ComputedProperty {
                            name,
                            value: sub_value,
                            method_def,
                            span: *span,
                        })
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            ResolvedExprKind::StructLit {
                name,
                fields: sub_fields,
            }
        }

        // Array literal - substitute in elements
        ResolvedExprKind::ArrayLit { elements } => {
            let sub_elements = elements
                .iter()
                .map(|elem| substitute_parameters_impl(elem, param_map, context))
                .collect::<Result<Vec<_>, _>>()?;
            ResolvedExprKind::ArrayLit {
                elements: sub_elements,
            }
        }

        // Field access - substitute in receiver
        ResolvedExprKind::FieldAccess {
            receiver,
            field_name,
            field,
        } => {
            let sub_receiver = substitute_parameters_impl(receiver, param_map, context)?;
            ResolvedExprKind::FieldAccess {
                receiver: sub_receiver,
                field_name,
                field,
            }
        }

        // Method call - substitute in receiver and arguments
        ResolvedExprKind::MethodCall {
            receiver,
            method_name,
            method,
            args,
        } => {
            let sub_receiver = substitute_parameters_impl(receiver, param_map, context)?;
            let sub_args = args
                .iter()
                .map(|arg| substitute_parameters_impl(arg, param_map, context))
                .collect::<Result<Vec<_>, _>>()?;
            ResolvedExprKind::MethodCall {
                receiver: sub_receiver,
                method_name,
                method,
                args: sub_args,
            }
        }

        // Function call - substitute in arguments
        ResolvedExprKind::FunctionCall {
            name,
            function,
            args,
        } => {
            let sub_args = args
                .iter()
                .map(|arg| substitute_parameters_impl(arg, param_map, context))
                .collect::<Result<Vec<_>, _>>()?;
            ResolvedExprKind::FunctionCall {
                name,
                function,
                args: sub_args,
            }
        }

        // Literals - no substitution needed
        ResolvedExprKind::IntLit { .. }
        | ResolvedExprKind::FloatLit { .. }
        | ResolvedExprKind::BoolLit { .. } => expr.kind.clone(),

        // Unsupported expression types - fail explicitly instead of silently
        // These would need proper implementation for full parameter substitution
        _ => {
            todo!(
                "Parameter substitution not implemented for this expression type: {:?}. \
                 This expression contains unhandled variants (Range, Closure, or ContainerFieldAccess). \
                 Parameters within these expressions will not be substituted, leading to undefined \
                 variable errors. Please report this case to the maintainers.",
                expr.kind
            )
        }
    };

    Ok(context.arena.alloc(ResolvedExpr {
        span: expr.span,
        kind,
        ty: expr.ty,
    }))
}
