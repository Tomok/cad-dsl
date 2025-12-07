use crate::ast::resolved::*;
use crate::ast::typed::*;
use crate::ast::unresolved::LiteralKind;
use crate::span::Span;
use std::collections::HashMap;
use thiserror::Error;

mod tests;

#[derive(Error, Debug, Clone)]
pub enum TypeErrorKind {
    #[error("Type mismatch: expected {expected}, found {found}")]
    TypeMismatch { expected: String, found: String },

    #[error("Unknown type: {0}")]
    UnknownType(String),

    #[error("Cannot apply operator {op} to types {left} and {right}")]
    InvalidOperation {
        op: String,
        left: String,
        right: String,
    },

    #[error("Function expects {expected} arguments, found {found}")]
    ArgumentCountMismatch { expected: usize, found: usize },

    #[error("Function parameter {index} expects type {expected}, found {found}")]
    ArgumentTypeMismatch {
        index: usize,
        expected: String,
        found: String,
    },

    #[error("Cannot call non-function type {ty}")]
    NotCallable { ty: String },

    #[error("Undefined function: {name}")]
    UndefinedFunction { name: String },

    #[error("Cannot reference {0} in this context")]
    InvalidReference(String),

    #[error("Cannot dereference non-reference type {0}")]
    InvalidDereference(String),

    #[error("Undefined symbol: {0}")]
    UndefinedSymbol(String),

    #[error("Field {field} not found in type {ty}")]
    FieldNotFound { field: String, ty: String },

    #[error("Cannot index non-array type {0}")]
    InvalidArrayIndex(String),

    #[error("Array index must be an integer, found {0}")]
    InvalidIndexType(String),
}

#[derive(Debug, Clone)]
pub struct TypeError {
    pub kind: TypeErrorKind,
    pub span: Span,
}

impl TypeError {
    pub fn new(kind: TypeErrorKind, span: Span) -> Self {
        Self { kind, span }
    }
}

pub struct TypeChecker<'a> {
    symbol_table: &'a SymbolTable,
    ident_arena: &'a crate::ident::IdentArena,
    errors: Vec<TypeError>,
    type_table: TypeTable,
    current_scope: ScopeId,
}

impl<'a> TypeChecker<'a> {
    pub fn new(symbol_table: &'a SymbolTable, ident_arena: &'a crate::ident::IdentArena) -> Self {
        Self {
            symbol_table,
            ident_arena,
            errors: Vec::new(),
            type_table: TypeTable {
                types: HashMap::new(),
            },
            current_scope: ScopeId(0), // Global scope
        }
    }

    pub fn check_types(mut self, ast: ResolvedAst) -> (TypedIr, Vec<TypeError>) {
        let mut typed_sketches = Vec::new();

        // First pass: collect struct definitions for type table
        for struct_def in &ast.structs {
            self.collect_struct_type(struct_def);
        }

        // Second pass: type check sketches
        for sketch in ast.sketches {
            match self.type_check_sketch(sketch) {
                Ok(typed_sketch) => typed_sketches.push(typed_sketch),
                Err(_) => {
                    // Error already recorded, continue with recovery
                }
            }
        }

        let typed_ir = TypedIr {
            sketches: typed_sketches,
            type_table: self.type_table,
        };

        (typed_ir, self.errors)
    }

    fn collect_struct_type(&mut self, struct_def: &ResolvedStructDef) {
        let mut fields = Vec::new();

        for field in &struct_def.fields {
            let field_type = self.resolve_type_ref(&field.ty);
            let field_name = self.ident_arena.resolve(field.name).to_string();
            fields.push((field_name, field_type));
        }

        let type_info = TypeInfo {
            name: self.ident_arena.resolve(struct_def.name).to_string(),
            kind: TypeKind::Struct { fields },
        };

        let type_id = self.type_table.types.len();
        self.type_table.types.insert(type_id, type_info);
    }

    fn type_check_sketch(&mut self, sketch: ResolvedSketchDef) -> Result<TypedSketch, ()> {
        self.current_scope = sketch.scope_id;
        let mut typed_body = Vec::new();

        for stmt in sketch.body {
            match self.type_check_stmt(stmt) {
                Ok(typed_stmt) => typed_body.push(typed_stmt),
                Err(_) => {
                    // Error already recorded, continue with recovery
                }
            }
        }

        Ok(TypedSketch {
            name: sketch.name,
            body: typed_body,
            scope: sketch.scope_id,
            span: sketch.span,
        })
    }

    fn type_check_stmt(&mut self, stmt: ResolvedStmt) -> Result<TypedStmt, ()> {
        match stmt {
            ResolvedStmt::Let {
                name,
                symbol_id,
                ty,
                init,
                span,
            } => {
                let expected_type = if let Some(type_ref) = ty {
                    self.resolve_type_ref(&type_ref)
                } else {
                    // Type inference from initialization
                    if let Some(ref init_expr) = init {
                        let typed_expr = self.type_check_expr(init_expr.clone())?;
                        typed_expr.ty
                    } else {
                        Type::Unknown
                    }
                };

                let init_expr = match init {
                    Some(expr) => {
                        let typed_expr = self.type_check_expr(expr)?;
                        if !matches!(expected_type, Type::Unknown) {
                            self.check_type_compatibility(
                                &expected_type,
                                &typed_expr.ty,
                                typed_expr.span,
                            )?;
                        }
                        Some(typed_expr)
                    }
                    None => None,
                };

                Ok(TypedStmt::Let {
                    name,
                    symbol_id,
                    ty: expected_type,
                    init: init_expr,
                    span,
                })
            }
            ResolvedStmt::Assign {
                target,
                value,
                span,
            } => {
                let typed_target = self.type_check_expr(target)?;
                let typed_value = self.type_check_expr(value)?;

                self.check_type_compatibility(&typed_target.ty, &typed_value.ty, span)?;

                let constraint_kind = ConstraintKind::Equality;
                Ok(TypedStmt::Constraint {
                    target: typed_target,
                    value: typed_value,
                    constraint_kind,
                    span,
                })
            }
            ResolvedStmt::For {
                var,
                var_symbol_id,
                range,
                body,
                scope_id: _,
                span,
            } => {
                let typed_range = self.type_check_expr(range)?;

                // TODO: Implement proper range type checking
                let var_ty = Type::I32; // Simplified for now

                let mut typed_body = Vec::new();
                for stmt in body {
                    match self.type_check_stmt(stmt) {
                        Ok(typed_stmt) => typed_body.push(typed_stmt),
                        Err(_) => {
                            // Continue with error recovery
                        }
                    }
                }

                Ok(TypedStmt::For {
                    var,
                    symbol_id: var_symbol_id,
                    var_ty,
                    range: typed_range,
                    body: typed_body,
                    span,
                })
            }
            ResolvedStmt::With {
                view,
                body,
                scope_id: _,
                span,
            } => {
                let typed_view = self.type_check_expr(view)?;

                // Check that view expression has View type
                if !matches!(typed_view.ty, Type::View) {
                    self.report_error(
                        TypeErrorKind::TypeMismatch {
                            expected: "View".to_string(),
                            found: Self::type_to_string(&typed_view.ty),
                        },
                        typed_view.span,
                    );
                    return Err(());
                }

                let mut typed_body = Vec::new();
                for stmt in body {
                    match self.type_check_stmt(stmt) {
                        Ok(typed_stmt) => typed_body.push(typed_stmt),
                        Err(_) => {
                            // Continue with error recovery
                        }
                    }
                }

                Ok(TypedStmt::With {
                    view: typed_view,
                    view_ty: Type::View,
                    body: typed_body,
                    context_id: 0, // TODO: Implement proper view context tracking
                    span,
                })
            }
            ResolvedStmt::Return { value: _, span } => {
                // TODO: Handle return statements - for now, just ignore
                Ok(TypedStmt::Expr(TypedExpr {
                    expr: ExprKind::LogicalOr(TypedLogicalOrExpr::LogicalAnd(
                        TypedLogicalAndExpr::Comparison(TypedComparisonExpr::Addition(
                            TypedAdditionExpr::Multiplication(TypedMultiplicationExpr::Power(
                                TypedPowerExpr::Unary(TypedUnaryExpr::Primary(
                                    TypedPrimaryExpr::Literal {
                                        kind: LiteralKind::Int(0),
                                    },
                                )),
                            )),
                        )),
                    )),
                    ty: Type::Unknown,
                    span,
                }))
            }
            ResolvedStmt::Expr(expr) => {
                let typed_expr = self.type_check_expr(expr)?;
                Ok(TypedStmt::Expr(typed_expr))
            }
        }
    }

    fn type_check_expr(&mut self, expr: ResolvedExpr) -> Result<TypedExpr, ()> {
        let span = expr.span();
        let (expr_kind, ty) = match expr {
            ResolvedExpr::LogicalOr(logical_or_expr) => {
                let (typed_expr, ty) = self.type_check_logical_or(logical_or_expr)?;
                (ExprKind::LogicalOr(typed_expr), ty)
            }
        };

        Ok(TypedExpr {
            expr: expr_kind,
            ty,
            span,
        })
    }

    fn type_check_logical_or(
        &mut self,
        expr: ResolvedLogicalOrExpr,
    ) -> Result<(TypedLogicalOrExpr, Type), ()> {
        match expr {
            ResolvedLogicalOrExpr::LogicalOr { left, right, span } => {
                let (typed_left, left_ty) = self.type_check_logical_and(*left)?;
                let (typed_right, right_ty) = self.type_check_logical_and(*right)?;

                // Both operands must be boolean
                self.check_type_compatibility(&Type::Bool, &left_ty, span)?;
                self.check_type_compatibility(&Type::Bool, &right_ty, span)?;

                Ok((
                    TypedLogicalOrExpr::LogicalOr {
                        left: Box::new(typed_left),
                        right: Box::new(typed_right),
                    },
                    Type::Bool,
                ))
            }
            ResolvedLogicalOrExpr::LogicalAnd(and_expr) => {
                let (typed_expr, ty) = self.type_check_logical_and(and_expr)?;
                Ok((TypedLogicalOrExpr::LogicalAnd(typed_expr), ty))
            }
        }
    }

    fn type_check_logical_and(
        &mut self,
        expr: ResolvedLogicalAndExpr,
    ) -> Result<(TypedLogicalAndExpr, Type), ()> {
        match expr {
            ResolvedLogicalAndExpr::LogicalAnd { left, right, span } => {
                let (typed_left, left_ty) = self.type_check_comparison(*left)?;
                let (typed_right, right_ty) = self.type_check_comparison(*right)?;

                // Both operands must be boolean
                self.check_type_compatibility(&Type::Bool, &left_ty, span)?;
                self.check_type_compatibility(&Type::Bool, &right_ty, span)?;

                Ok((
                    TypedLogicalAndExpr::LogicalAnd {
                        left: Box::new(typed_left),
                        right: Box::new(typed_right),
                    },
                    Type::Bool,
                ))
            }
            ResolvedLogicalAndExpr::Comparison(comp_expr) => {
                let (typed_expr, ty) = self.type_check_comparison(comp_expr)?;
                Ok((TypedLogicalAndExpr::Comparison(typed_expr), ty))
            }
        }
    }

    fn type_check_comparison(
        &mut self,
        expr: ResolvedComparisonExpr,
    ) -> Result<(TypedComparisonExpr, Type), ()> {
        match expr {
            ResolvedComparisonExpr::Equal { left, right, span } => {
                let (typed_left, left_ty) = self.type_check_addition(*left)?;
                let (typed_right, right_ty) = self.type_check_addition(*right)?;

                self.check_type_compatibility(&left_ty, &right_ty, span)?;

                Ok((
                    TypedComparisonExpr::Equal {
                        left: Box::new(typed_left),
                        right: Box::new(typed_right),
                    },
                    Type::Bool,
                ))
            }
            ResolvedComparisonExpr::NotEqual { left, right, span } => {
                let (typed_left, left_ty) = self.type_check_addition(*left)?;
                let (typed_right, right_ty) = self.type_check_addition(*right)?;

                self.check_type_compatibility(&left_ty, &right_ty, span)?;

                Ok((
                    TypedComparisonExpr::NotEqual {
                        left: Box::new(typed_left),
                        right: Box::new(typed_right),
                    },
                    Type::Bool,
                ))
            }
            ResolvedComparisonExpr::LessThan { left, right, span } => {
                let (typed_left, left_ty) = self.type_check_addition(*left)?;
                let (typed_right, right_ty) = self.type_check_addition(*right)?;

                // Both operands must be numeric
                if !left_ty.is_numeric() || !right_ty.is_numeric() {
                    self.report_error(
                        TypeErrorKind::InvalidOperation {
                            op: "<".to_string(),
                            left: Self::type_to_string(&left_ty),
                            right: Self::type_to_string(&right_ty),
                        },
                        span,
                    );
                    return Err(());
                }

                Ok((
                    TypedComparisonExpr::LessThan {
                        left: Box::new(typed_left),
                        right: Box::new(typed_right),
                    },
                    Type::Bool,
                ))
            }
            ResolvedComparisonExpr::GreaterThan { left, right, span } => {
                let (typed_left, left_ty) = self.type_check_addition(*left)?;
                let (typed_right, right_ty) = self.type_check_addition(*right)?;

                if !left_ty.is_numeric() || !right_ty.is_numeric() {
                    self.report_error(
                        TypeErrorKind::InvalidOperation {
                            op: ">".to_string(),
                            left: Self::type_to_string(&left_ty),
                            right: Self::type_to_string(&right_ty),
                        },
                        span,
                    );
                    return Err(());
                }

                Ok((
                    TypedComparisonExpr::GreaterThan {
                        left: Box::new(typed_left),
                        right: Box::new(typed_right),
                    },
                    Type::Bool,
                ))
            }
            ResolvedComparisonExpr::LessEqual { left, right, span } => {
                let (typed_left, left_ty) = self.type_check_addition(*left)?;
                let (typed_right, right_ty) = self.type_check_addition(*right)?;

                if !left_ty.is_numeric() || !right_ty.is_numeric() {
                    self.report_error(
                        TypeErrorKind::InvalidOperation {
                            op: "<=".to_string(),
                            left: Self::type_to_string(&left_ty),
                            right: Self::type_to_string(&right_ty),
                        },
                        span,
                    );
                    return Err(());
                }

                Ok((
                    TypedComparisonExpr::LessEqual {
                        left: Box::new(typed_left),
                        right: Box::new(typed_right),
                    },
                    Type::Bool,
                ))
            }
            ResolvedComparisonExpr::GreaterEqual { left, right, span } => {
                let (typed_left, left_ty) = self.type_check_addition(*left)?;
                let (typed_right, right_ty) = self.type_check_addition(*right)?;

                if !left_ty.is_numeric() || !right_ty.is_numeric() {
                    self.report_error(
                        TypeErrorKind::InvalidOperation {
                            op: ">=".to_string(),
                            left: Self::type_to_string(&left_ty),
                            right: Self::type_to_string(&right_ty),
                        },
                        span,
                    );
                    return Err(());
                }

                Ok((
                    TypedComparisonExpr::GreaterEqual {
                        left: Box::new(typed_left),
                        right: Box::new(typed_right),
                    },
                    Type::Bool,
                ))
            }
            ResolvedComparisonExpr::Addition(add_expr) => {
                let (typed_expr, ty) = self.type_check_addition(add_expr)?;
                Ok((TypedComparisonExpr::Addition(typed_expr), ty))
            }
        }
    }

    fn type_check_addition(
        &mut self,
        expr: ResolvedAdditionExpr,
    ) -> Result<(TypedAdditionExpr, Type), ()> {
        match expr {
            ResolvedAdditionExpr::Add { left, right, span } => {
                let (typed_left, left_ty) = self.type_check_multiplication(*left)?;
                let (typed_right, right_ty) = self.type_check_multiplication(*right)?;

                let result_ty = self.check_binary_op_types("+", &left_ty, &right_ty, span)?;

                Ok((
                    TypedAdditionExpr::Add {
                        left: Box::new(typed_left),
                        right: Box::new(typed_right),
                    },
                    result_ty,
                ))
            }
            ResolvedAdditionExpr::Subtract { left, right, span } => {
                let (typed_left, left_ty) = self.type_check_multiplication(*left)?;
                let (typed_right, right_ty) = self.type_check_multiplication(*right)?;

                let result_ty = self.check_binary_op_types("-", &left_ty, &right_ty, span)?;

                Ok((
                    TypedAdditionExpr::Subtract {
                        left: Box::new(typed_left),
                        right: Box::new(typed_right),
                    },
                    result_ty,
                ))
            }
            ResolvedAdditionExpr::Multiplication(mult_expr) => {
                let (typed_expr, ty) = self.type_check_multiplication(mult_expr)?;
                Ok((TypedAdditionExpr::Multiplication(typed_expr), ty))
            }
        }
    }

    fn type_check_multiplication(
        &mut self,
        expr: ResolvedMultiplicationExpr,
    ) -> Result<(TypedMultiplicationExpr, Type), ()> {
        match expr {
            ResolvedMultiplicationExpr::Multiply { left, right, span } => {
                let (typed_left, left_ty) = self.type_check_multiplication(*left)?;
                let (typed_right, right_ty) = self.type_check_power(*right)?;

                let result_ty = self.check_binary_op_types("*", &left_ty, &right_ty, span)?;

                Ok((
                    TypedMultiplicationExpr::Multiply {
                        left: Box::new(typed_left),
                        right: Box::new(typed_right),
                    },
                    result_ty,
                ))
            }
            ResolvedMultiplicationExpr::Divide { left, right, span } => {
                let (typed_left, left_ty) = self.type_check_multiplication(*left)?;
                let (typed_right, right_ty) = self.type_check_power(*right)?;

                let result_ty = self.check_binary_op_types("/", &left_ty, &right_ty, span)?;

                Ok((
                    TypedMultiplicationExpr::Divide {
                        left: Box::new(typed_left),
                        right: Box::new(typed_right),
                    },
                    result_ty,
                ))
            }
            ResolvedMultiplicationExpr::Modulo { left, right, span } => {
                let (typed_left, left_ty) = self.type_check_multiplication(*left)?;
                let (typed_right, right_ty) = self.type_check_power(*right)?;

                // Modulo only works with integers
                if !matches!(left_ty, Type::I32) || !matches!(right_ty, Type::I32) {
                    self.report_error(
                        TypeErrorKind::InvalidOperation {
                            op: "%".to_string(),
                            left: Self::type_to_string(&left_ty),
                            right: Self::type_to_string(&right_ty),
                        },
                        span,
                    );
                    return Err(());
                }

                Ok((
                    TypedMultiplicationExpr::Modulo {
                        left: Box::new(typed_left),
                        right: Box::new(typed_right),
                    },
                    Type::I32,
                ))
            }
            ResolvedMultiplicationExpr::Power(power_expr) => {
                let (typed_expr, ty) = self.type_check_power(power_expr)?;
                Ok((TypedMultiplicationExpr::Power(typed_expr), ty))
            }
        }
    }

    fn type_check_power(&mut self, expr: ResolvedPowerExpr) -> Result<(TypedPowerExpr, Type), ()> {
        match expr {
            ResolvedPowerExpr::Power { left, right, span } => {
                let (typed_left, left_ty) = self.type_check_unary(*left)?;
                let (typed_right, right_ty) = self.type_check_power(*right)?;

                let result_ty = self.check_binary_op_types("^", &left_ty, &right_ty, span)?;

                Ok((
                    TypedPowerExpr::Power {
                        left: Box::new(typed_left),
                        right: Box::new(typed_right),
                    },
                    result_ty,
                ))
            }
            ResolvedPowerExpr::Unary(unary_expr) => {
                let (typed_expr, ty) = self.type_check_unary(unary_expr)?;
                Ok((TypedPowerExpr::Unary(typed_expr), ty))
            }
        }
    }

    fn type_check_unary(&mut self, expr: ResolvedUnaryExpr) -> Result<(TypedUnaryExpr, Type), ()> {
        match expr {
            ResolvedUnaryExpr::Negation { expr, span } => {
                let (typed_expr, ty) = self.type_check_unary(*expr)?;

                if !ty.is_numeric() {
                    self.report_error(
                        TypeErrorKind::InvalidOperation {
                            op: "-".to_string(),
                            left: "".to_string(),
                            right: Self::type_to_string(&ty),
                        },
                        span,
                    );
                    return Err(());
                }

                Ok((
                    TypedUnaryExpr::Negation {
                        expr: Box::new(typed_expr),
                    },
                    ty,
                ))
            }
            ResolvedUnaryExpr::Not { expr, span } => {
                let (typed_expr, ty) = self.type_check_unary(*expr)?;

                self.check_type_compatibility(&Type::Bool, &ty, span)?;

                Ok((
                    TypedUnaryExpr::Not {
                        expr: Box::new(typed_expr),
                    },
                    Type::Bool,
                ))
            }
            ResolvedUnaryExpr::Reference { expr, span } => {
                let (typed_expr, ty) = self.type_check_unary(*expr)?;

                // Can only take references to entity types
                if !ty.is_entity_type() {
                    self.report_error(
                        TypeErrorKind::InvalidReference(Self::type_to_string(&ty)),
                        span,
                    );
                    return Err(());
                }

                Ok((
                    TypedUnaryExpr::Reference {
                        expr: Box::new(typed_expr),
                    },
                    Type::Reference(Box::new(ty)),
                ))
            }
            ResolvedUnaryExpr::Dereference { expr, span } => {
                let (typed_expr, ty) = self.type_check_unary(*expr)?;

                match ty {
                    Type::Reference(inner_ty) => Ok((
                        TypedUnaryExpr::Dereference {
                            expr: Box::new(typed_expr),
                        },
                        *inner_ty,
                    )),
                    _ => {
                        self.report_error(
                            TypeErrorKind::InvalidDereference(Self::type_to_string(&ty)),
                            span,
                        );
                        Err(())
                    }
                }
            }
            ResolvedUnaryExpr::Primary(primary_expr) => {
                let (typed_expr, ty) = self.type_check_primary(primary_expr)?;
                Ok((TypedUnaryExpr::Primary(typed_expr), ty))
            }
        }
    }

    fn type_check_primary(
        &mut self,
        expr: ResolvedPrimaryExpr,
    ) -> Result<(TypedPrimaryExpr, Type), ()> {
        match expr {
            ResolvedPrimaryExpr::Literal { kind, span: _ } => {
                let ty = self.infer_literal_type(&kind);
                Ok((TypedPrimaryExpr::Literal { kind }, ty))
            }
            ResolvedPrimaryExpr::Ident {
                name,
                symbol_id,
                span,
            } => {
                let symbol = self.symbol_table.get_symbol(symbol_id).ok_or_else(|| {
                    self.report_error(
                        TypeErrorKind::UndefinedSymbol(self.ident_arena.resolve(name).to_string()),
                        span,
                    );
                })?;

                let ty = self.get_symbol_type(symbol);
                Ok((TypedPrimaryExpr::Ident { name, symbol_id }, ty))
            }
            ResolvedPrimaryExpr::Call { func, args, span } => {
                // First, type check the function expression
                let (typed_func, func_ty) = self.type_check_primary(*func)?;

                // Type check all arguments
                let mut typed_args = Vec::new();
                let mut arg_types = Vec::new();

                for arg in args {
                    match self.type_check_expr(arg) {
                        Ok(typed_arg) => {
                            arg_types.push(typed_arg.ty.clone());
                            typed_args.push(typed_arg);
                        }
                        Err(_) => {
                            // Continue with error recovery
                            arg_types.push(Type::Error);
                            typed_args.push(TypedExpr {
                                expr: ExprKind::LogicalOr(TypedLogicalOrExpr::LogicalAnd(
                                    TypedLogicalAndExpr::Comparison(TypedComparisonExpr::Addition(
                                        TypedAdditionExpr::Multiplication(
                                            TypedMultiplicationExpr::Power(TypedPowerExpr::Unary(
                                                TypedUnaryExpr::Primary(
                                                    TypedPrimaryExpr::Literal {
                                                        kind: LiteralKind::Int(0),
                                                    },
                                                ),
                                            )),
                                        ),
                                    )),
                                )),
                                ty: Type::Error,
                                span,
                            });
                        }
                    }
                }

                // Check if function type is callable and validate signature
                let (func_id, return_type) = match func_ty {
                    Type::Function {
                        params,
                        return_type,
                    } => {
                        // Validate argument count
                        if params.len() != arg_types.len() {
                            self.report_error(
                                TypeErrorKind::ArgumentCountMismatch {
                                    expected: params.len(),
                                    found: arg_types.len(),
                                },
                                span,
                            );
                            return Err(());
                        }

                        // Validate argument types
                        for (i, (expected, found)) in
                            params.iter().zip(arg_types.iter()).enumerate()
                        {
                            if !self.types_compatible(expected, found) {
                                self.report_error(
                                    TypeErrorKind::ArgumentTypeMismatch {
                                        index: i,
                                        expected: Self::type_to_string(expected),
                                        found: Self::type_to_string(found),
                                    },
                                    typed_args[i].span,
                                );
                            }
                        }

                        (None, *return_type) // TODO: Get actual function ID from symbol resolution
                    }
                    _ => {
                        // Try to resolve if this is a function identifier
                        if let TypedPrimaryExpr::Ident { symbol_id, .. } = &typed_func {
                            if let Some(symbol) = self.symbol_table.get_symbol(*symbol_id) {
                                if let Some((param_types, return_type)) =
                                    self.get_function_signature(symbol)
                                {
                                    // Validate argument count
                                    if param_types.len() != arg_types.len() {
                                        self.report_error(
                                            TypeErrorKind::ArgumentCountMismatch {
                                                expected: param_types.len(),
                                                found: arg_types.len(),
                                            },
                                            span,
                                        );
                                        return Err(());
                                    }

                                    // Validate argument types
                                    for (i, (expected, found)) in
                                        param_types.iter().zip(arg_types.iter()).enumerate()
                                    {
                                        if !self.types_compatible(expected, found) {
                                            self.report_error(
                                                TypeErrorKind::ArgumentTypeMismatch {
                                                    index: i,
                                                    expected: Self::type_to_string(expected),
                                                    found: Self::type_to_string(found),
                                                },
                                                typed_args[i].span,
                                            );
                                        }
                                    }

                                    (Some(symbol_id.0), return_type)
                                } else {
                                    self.report_error(
                                        TypeErrorKind::NotCallable {
                                            ty: Self::type_to_string(&func_ty),
                                        },
                                        span,
                                    );
                                    return Err(());
                                }
                            } else {
                                self.report_error(
                                    TypeErrorKind::UndefinedFunction {
                                        name: "unknown".to_string(),
                                    },
                                    span,
                                );
                                return Err(());
                            }
                        } else {
                            self.report_error(
                                TypeErrorKind::NotCallable {
                                    ty: Self::type_to_string(&func_ty),
                                },
                                span,
                            );
                            return Err(());
                        }
                    }
                };

                Ok((
                    TypedPrimaryExpr::Call {
                        func: Box::new(typed_func),
                        func_id,
                        args: typed_args,
                    },
                    return_type,
                ))
            }
            ResolvedPrimaryExpr::FieldAccess {
                base,
                field,
                field_symbol_id,
                span: _,
            } => {
                let (typed_base, _base_ty) = self.type_check_primary(*base)?;

                // TODO: Implement proper field access type checking
                let field_type = Type::Unknown; // Simplified for now

                Ok((
                    TypedPrimaryExpr::FieldAccess {
                        base: Box::new(typed_base),
                        field,
                        field_symbol_id: Some(field_symbol_id),
                    },
                    field_type,
                ))
            }
            ResolvedPrimaryExpr::ArrayIndex { array, index, span } => {
                let (typed_array, array_ty) = self.type_check_primary(*array)?;
                let typed_index = self.type_check_expr(*index)?;

                // Check that index is an integer
                self.check_type_compatibility(&Type::I32, &typed_index.ty, typed_index.span)?;

                // Extract element type from array
                let element_type = match array_ty {
                    Type::Array { element_type, .. } => *element_type,
                    _ => {
                        self.report_error(
                            TypeErrorKind::InvalidArrayIndex(Self::type_to_string(&array_ty)),
                            span,
                        );
                        return Err(());
                    }
                };

                Ok((
                    TypedPrimaryExpr::ArrayIndex {
                        array: Box::new(typed_array),
                        index: Box::new(typed_index),
                    },
                    element_type,
                ))
            }
            ResolvedPrimaryExpr::StructLiteral {
                ty,
                fields,
                span: _,
            } => {
                let mut typed_fields = Vec::new();

                for (field_name, field_expr) in fields {
                    match self.type_check_expr(field_expr) {
                        Ok(typed_expr) => typed_fields.push((field_name, typed_expr)),
                        Err(_) => {
                            // Continue with error recovery
                        }
                    }
                }

                // Get struct ID from type reference
                let struct_id = match &ty.symbol_id {
                    Some(symbol_id) => symbol_id.0, // Access inner usize
                    None => 0,                      // Default struct ID
                };

                let struct_type = Type::Struct { struct_id };
                Ok((
                    TypedPrimaryExpr::StructLiteral {
                        struct_id,
                        fields: typed_fields,
                    },
                    struct_type,
                ))
            }
            ResolvedPrimaryExpr::ArrayLiteral { elements, span: _ } => {
                let mut typed_elements = Vec::new();
                let mut element_type = None;

                for element in elements {
                    match self.type_check_expr(element) {
                        Ok(typed_element) => {
                            if element_type.is_none() {
                                element_type = Some(typed_element.ty.clone());
                            } else {
                                // Check that all elements have the same type
                                self.check_type_compatibility(
                                    element_type.as_ref().unwrap(),
                                    &typed_element.ty,
                                    typed_element.span,
                                )?;
                            }
                            typed_elements.push(typed_element);
                        }
                        Err(_) => {
                            // Continue with error recovery
                        }
                    }
                }

                let array_type = Type::Array {
                    element_type: Box::new(element_type.unwrap_or(Type::Unknown)),
                    size: typed_elements.len(),
                };

                Ok((
                    TypedPrimaryExpr::ArrayLiteral {
                        elements: typed_elements,
                    },
                    array_type,
                ))
            }
            ResolvedPrimaryExpr::Range {
                start,
                end,
                span: _,
            } => {
                let typed_start = self.type_check_expr(*start)?;
                let typed_end = self.type_check_expr(*end)?;

                // Both start and end must be integers
                self.check_type_compatibility(&Type::I32, &typed_start.ty, typed_start.span)?;
                self.check_type_compatibility(&Type::I32, &typed_end.ty, typed_end.span)?;

                // Range type is array of integers
                let range_type = Type::Array {
                    element_type: Box::new(Type::I32),
                    size: 0, // Size determined at runtime
                };

                Ok((
                    TypedPrimaryExpr::Range {
                        start: Box::new(typed_start),
                        end: Box::new(typed_end),
                    },
                    range_type,
                ))
            }
            ResolvedPrimaryExpr::Parenthesized(expr) => {
                let typed_expr = self.type_check_expr(*expr)?;
                let ty = typed_expr.ty.clone();
                Ok((TypedPrimaryExpr::Parenthesized(Box::new(typed_expr)), ty))
            }
        }
    }

    fn infer_literal_type(&self, literal: &LiteralKind) -> Type {
        match literal {
            LiteralKind::Int(_) => Type::I32,
            LiteralKind::Float(_) => Type::F64,
            LiteralKind::Bool(_) => Type::Bool,
            LiteralKind::Length { .. } => Type::Length,
            LiteralKind::Angle { .. } => Type::Angle,
        }
    }

    fn resolve_type_ref(&self, type_ref: &ResolvedTypeRef) -> Type {
        let ResolvedTypeRef {
            name,
            symbol_id,
            is_reference,
            array_size,
            ..
        } = type_ref;

        let base_type = if let Some(_symbol_id) = symbol_id {
            // TODO: Look up struct type from symbol table
            Type::Unknown // Simplified for now
        } else {
            // Map identifier to built-in type
            let type_name = self.ident_arena.resolve(*name);
            match type_name {
                "Point" => Type::Point,
                "Length" => Type::Length,
                "Angle" => Type::Angle,
                "Area" => Type::Area,
                "Bool" => Type::Bool,
                "I32" => Type::I32,
                "F64" => Type::F64,
                "Real" => Type::Real,
                "Algebraic" => Type::Algebraic,
                "View" => Type::View,
                _ => Type::Unknown,
            }
        };

        let array_type = if let Some(_array_size_expr) = array_size {
            // TODO: Evaluate array size expression
            Type::Array {
                element_type: Box::new(base_type),
                size: 0, // Placeholder size
            }
        } else {
            base_type
        };

        if *is_reference {
            Type::Reference(Box::new(array_type))
        } else {
            array_type
        }
    }

    fn get_symbol_type(&self, symbol: &Symbol) -> Type {
        match &symbol.kind {
            SymbolKind::Variable { type_ref, .. } => {
                if let Some(type_ref) = type_ref {
                    self.resolve_type_ref(type_ref)
                } else {
                    Type::Unknown
                }
            }
            SymbolKind::Function {
                params,
                return_type,
            } => {
                // Resolve parameter types from symbol IDs
                let mut param_types = Vec::new();
                for param_id in params {
                    if let Some(param_symbol) = self.symbol_table.get_symbol(*param_id) {
                        param_types.push(self.get_symbol_type(param_symbol));
                    } else {
                        param_types.push(Type::Unknown);
                    }
                }

                let return_ty = return_type
                    .as_ref()
                    .map(|t| Box::new(self.resolve_type_ref(t)))
                    .unwrap_or_else(|| Box::new(Type::Unknown));
                Type::Function {
                    params: param_types,
                    return_type: return_ty,
                }
            }
            SymbolKind::Struct { .. } => {
                // For struct symbols, return the struct type
                Type::Unknown // TODO: Need to properly handle struct types
            }
            SymbolKind::Field { type_ref } => self.resolve_type_ref(type_ref),
            SymbolKind::Parameter { type_ref } => self.resolve_type_ref(type_ref),
        }
    }

    /// Get the function signature for a symbol, if it's a function
    fn get_function_signature(&self, symbol: &Symbol) -> Option<(Vec<Type>, Type)> {
        match &symbol.kind {
            SymbolKind::Function {
                params,
                return_type,
            } => {
                let mut param_types = Vec::new();
                for param_id in params {
                    if let Some(param_symbol) = self.symbol_table.get_symbol(*param_id) {
                        param_types.push(self.get_symbol_type(param_symbol));
                    } else {
                        param_types.push(Type::Unknown);
                    }
                }

                let return_type = return_type
                    .as_ref()
                    .map(|t| self.resolve_type_ref(t))
                    .unwrap_or(Type::Unknown);

                Some((param_types, return_type))
            }
            _ => None,
        }
    }

    fn check_type_compatibility(
        &mut self,
        expected: &Type,
        found: &Type,
        span: Span,
    ) -> Result<(), ()> {
        if self.types_compatible(expected, found) {
            Ok(())
        } else {
            self.report_error(
                TypeErrorKind::TypeMismatch {
                    expected: Self::type_to_string(expected),
                    found: Self::type_to_string(found),
                },
                span,
            );
            Err(())
        }
    }

    fn types_compatible(&self, expected: &Type, found: &Type) -> bool {
        match (expected, found) {
            (Type::Unknown, _) | (_, Type::Unknown) => true,
            (Type::Error, _) | (_, Type::Error) => true,
            (a, b) => a == b,
        }
    }

    fn check_binary_op_types(
        &mut self,
        op: &str,
        left: &Type,
        right: &Type,
        span: Span,
    ) -> Result<Type, ()> {
        let result = match (op, left, right) {
            // Arithmetic operations
            ("+", Type::Length, Type::Length) => Type::Length,
            ("+", Type::Angle, Type::Angle) => Type::Angle,
            ("+", Type::Area, Type::Area) => Type::Area,
            ("+", Type::I32, Type::I32) => Type::I32,
            ("+", Type::F64, Type::F64) => Type::F64,
            ("+", Type::Real, Type::Real) => Type::Real,

            ("-", Type::Length, Type::Length) => Type::Length,
            ("-", Type::Angle, Type::Angle) => Type::Angle,
            ("-", Type::Area, Type::Area) => Type::Area,
            ("-", Type::I32, Type::I32) => Type::I32,
            ("-", Type::F64, Type::F64) => Type::F64,
            ("-", Type::Real, Type::Real) => Type::Real,

            ("*", Type::Length, Type::Length) => Type::Area,
            ("*", Type::Length, Type::F64) => Type::Length,
            ("*", Type::F64, Type::Length) => Type::Length,
            ("*", Type::I32, Type::I32) => Type::I32,
            ("*", Type::F64, Type::F64) => Type::F64,
            ("*", Type::Real, Type::Real) => Type::Real,

            ("/", Type::Length, Type::Length) => Type::F64,
            ("/", Type::Length, Type::F64) => Type::Length,
            ("/", Type::Area, Type::Length) => Type::Length,
            ("/", Type::I32, Type::I32) => Type::I32,
            ("/", Type::F64, Type::F64) => Type::F64,
            ("/", Type::Real, Type::Real) => Type::Real,

            ("^", Type::F64, Type::F64) => Type::F64,
            ("^", Type::Real, Type::Real) => Type::Real,

            // Handle unknown/error types gracefully
            (_, Type::Unknown, _) | (_, _, Type::Unknown) => Type::Unknown,
            (_, Type::Error, _) | (_, _, Type::Error) => Type::Error,

            // Invalid operations
            _ => {
                self.report_error(
                    TypeErrorKind::InvalidOperation {
                        op: op.to_string(),
                        left: Self::type_to_string(left),
                        right: Self::type_to_string(right),
                    },
                    span,
                );
                return Err(());
            }
        };

        Ok(result)
    }

    fn type_to_string(ty: &Type) -> String {
        match ty {
            Type::Point => "Point".to_string(),
            Type::Length => "Length".to_string(),
            Type::Angle => "Angle".to_string(),
            Type::Area => "Area".to_string(),
            Type::Bool => "Bool".to_string(),
            Type::I32 => "I32".to_string(),
            Type::F64 => "F64".to_string(),
            Type::Real => "Real".to_string(),
            Type::Algebraic => "Algebraic".to_string(),
            Type::View => "View".to_string(),
            Type::Array { element_type, size } => {
                format!("Array<{}, {}>", Self::type_to_string(element_type), size)
            }
            Type::Struct { struct_id } => format!("Struct#{}", struct_id),
            Type::Function {
                params,
                return_type,
            } => {
                let param_strs: Vec<String> = params.iter().map(Self::type_to_string).collect();
                format!(
                    "({}) -> {}",
                    param_strs.join(", "),
                    Self::type_to_string(return_type)
                )
            }
            Type::Reference(inner) => format!("&{}", Self::type_to_string(inner)),
            Type::Unknown => "Unknown".to_string(),
            Type::Error => "Error".to_string(),
        }
    }

    fn report_error(&mut self, kind: TypeErrorKind, span: Span) {
        self.errors.push(TypeError::new(kind, span));
    }
}

pub fn check_types(
    ast: ResolvedAst,
    symbol_table: &SymbolTable,
    ident_arena: &crate::ident::IdentArena,
) -> (TypedIr, Vec<TypeError>) {
    let checker = TypeChecker::new(symbol_table, ident_arena);
    checker.check_types(ast)
}
