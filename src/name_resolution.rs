use crate::ast::{resolved, unresolved};
use crate::ident::{IdentArena, IdentId};
use crate::span::Span;
use thiserror::Error;

/// Errors that can occur during name resolution
#[derive(Error, Debug, Clone, PartialEq)]
pub enum NameResolutionError {
    #[error("Undefined symbol '{name}' at {span:?}")]
    UndefinedSymbol { name: String, span: Span },

    #[error("Symbol '{name}' is already defined in this scope at {span:?}")]
    DuplicateDefinition { name: String, span: Span },

    #[error("Cannot resolve type '{type_name}' at {span:?}")]
    UnresolvedType { type_name: String, span: Span },

    #[error("Undefined field '{field}' on type '{type_name}' at {span:?}")]
    UndefinedField {
        field: String,
        type_name: String,
        span: Span,
    },

    #[error("Function '{name}' is not defined at {span:?}")]
    UndefinedFunction { name: String, span: Span },

    #[error("Invalid reference to '{name}' at {span:?}")]
    InvalidReference { name: String, span: Span },

    #[error("Type mismatch: expected '{expected}', found '{found}' at {span:?}")]
    TypeMismatch {
        expected: String,
        found: String,
        span: Span,
    },
}

/// Context for name resolution
pub struct NameResolver<'a> {
    idents: &'a IdentArena,
    symbol_table: resolved::SymbolTable,
    current_scope: resolved::ScopeId,
    errors: Vec<NameResolutionError>,
}

impl<'a> NameResolver<'a> {
    pub fn new(idents: &'a IdentArena) -> Self {
        let symbol_table = resolved::SymbolTable::new();
        let global_scope = symbol_table.global_scope_id();

        Self {
            idents,
            symbol_table,
            current_scope: global_scope,
            errors: Vec::new(),
        }
    }

    /// Resolve an unresolved AST to a resolved AST
    pub fn resolve(
        mut self,
        ast: unresolved::UnresolvedAst,
    ) -> (resolved::ResolvedAst, Vec<NameResolutionError>) {
        // First pass: register all top-level declarations
        self.register_top_level_declarations(&ast);

        // Second pass: resolve all references
        let resolved_imports = ast.imports;

        let mut resolved_structs = Vec::new();
        let mut resolved_sketches = Vec::new();

        for struct_def in ast.structs {
            match self.resolve_struct_def(struct_def) {
                Ok(resolved_struct) => resolved_structs.push(resolved_struct),
                Err(err) => self.errors.push(err),
            }
        }

        for sketch_def in ast.sketches {
            match self.resolve_sketch_def(sketch_def) {
                Ok(resolved_sketch) => resolved_sketches.push(resolved_sketch),
                Err(err) => self.errors.push(err),
            }
        }

        let resolved_ast = resolved::ResolvedAst {
            imports: resolved_imports,
            structs: resolved_structs,
            sketches: resolved_sketches,
            symbol_table: self.symbol_table,
        };

        (resolved_ast, self.errors)
    }

    fn register_top_level_declarations(&mut self, ast: &unresolved::UnresolvedAst) {
        // Register struct declarations
        for struct_def in &ast.structs {
            let symbol_kind = resolved::SymbolKind::Struct {
                fields: Vec::new(), // Will be populated in second pass
                methods: Vec::new(),
            };

            if let Err(_err) = self.symbol_table.create_symbol(
                struct_def.name,
                symbol_kind,
                struct_def.span,
                self.current_scope,
            ) {
                self.errors.push(NameResolutionError::DuplicateDefinition {
                    name: self.ident_to_string(struct_def.name),
                    span: struct_def.span,
                });
            }
        }

        // Register sketch declarations
        for _sketch_def in &ast.sketches {
            // Sketches are not symbols themselves, but we create scopes for them
            // Functions within sketches will be registered in their resolution
        }
    }

    fn resolve_struct_def(
        &mut self,
        struct_def: unresolved::StructDef,
    ) -> Result<resolved::ResolvedStructDef, NameResolutionError> {
        // Create struct scope
        let struct_scope = self
            .symbol_table
            .create_scope(Some(self.current_scope), resolved::ScopeKind::Struct);
        let old_scope = self.current_scope;
        self.current_scope = struct_scope;

        let mut resolved_fields = Vec::new();
        let mut resolved_methods = Vec::new();
        let mut field_symbol_ids = Vec::new();
        let mut method_symbol_ids = Vec::new();

        // Resolve fields
        for field in struct_def.fields {
            match self.resolve_field_def(field) {
                Ok(resolved_field) => {
                    field_symbol_ids.push(resolved_field.symbol_id);
                    resolved_fields.push(resolved_field);
                }
                Err(err) => return Err(err),
            }
        }

        // Resolve methods
        for method in struct_def.methods {
            match self.resolve_function_def(method) {
                Ok(resolved_method) => {
                    method_symbol_ids.push(resolved_method.symbol_id);
                    resolved_methods.push(resolved_method);
                }
                Err(err) => return Err(err),
            }
        }

        // Update the struct symbol with field and method information
        if let Some(symbol_id) = self.symbol_table.lookup_symbol(struct_def.name, old_scope) {
            // Update symbol with actual fields and methods
            if let Some(symbol) = self.symbol_table.get_symbol_mut(symbol_id) {
                symbol.kind = resolved::SymbolKind::Struct {
                    fields: field_symbol_ids,
                    methods: method_symbol_ids,
                };
            }
        }

        self.current_scope = old_scope;

        Ok(resolved::ResolvedStructDef {
            name: struct_def.name,
            fields: resolved_fields,
            methods: resolved_methods,
            scope_id: struct_scope,
            span: struct_def.span,
        })
    }

    fn resolve_sketch_def(
        &mut self,
        sketch_def: unresolved::SketchDef,
    ) -> Result<resolved::ResolvedSketchDef, NameResolutionError> {
        // Create sketch scope
        let sketch_scope = self
            .symbol_table
            .create_scope(Some(self.current_scope), resolved::ScopeKind::Sketch);
        let old_scope = self.current_scope;
        self.current_scope = sketch_scope;

        let mut resolved_body = Vec::new();
        let mut resolved_functions = Vec::new();

        // Resolve statements
        for stmt in sketch_def.body {
            match self.resolve_stmt(stmt) {
                Ok(resolved_stmt) => resolved_body.push(resolved_stmt),
                Err(err) => return Err(err),
            }
        }

        // Resolve functions
        for function in sketch_def.functions {
            match self.resolve_function_def(function) {
                Ok(resolved_function) => resolved_functions.push(resolved_function),
                Err(err) => return Err(err),
            }
        }

        self.current_scope = old_scope;

        Ok(resolved::ResolvedSketchDef {
            name: sketch_def.name,
            body: resolved_body,
            functions: resolved_functions,
            scope_id: sketch_scope,
            span: sketch_def.span,
        })
    }

    fn resolve_field_def(
        &mut self,
        field: unresolved::FieldDef,
    ) -> Result<resolved::ResolvedFieldDef, NameResolutionError> {
        let resolved_type = self.resolve_type_ref(field.ty)?;

        let symbol_id = self
            .symbol_table
            .create_symbol(
                field.name,
                resolved::SymbolKind::Field {
                    type_ref: resolved_type.clone(),
                },
                field.span,
                self.current_scope,
            )
            .map_err(|_| NameResolutionError::DuplicateDefinition {
                name: self.ident_to_string(field.name),
                span: field.span,
            })?;

        Ok(resolved::ResolvedFieldDef {
            name: field.name,
            ty: resolved_type,
            symbol_id,
            span: field.span,
        })
    }

    fn resolve_function_def(
        &mut self,
        function: unresolved::FunctionDef,
    ) -> Result<resolved::ResolvedFunctionDef, NameResolutionError> {
        // Create function scope
        let function_scope = self
            .symbol_table
            .create_scope(Some(self.current_scope), resolved::ScopeKind::Function);
        let old_scope = self.current_scope;

        // Register function symbol in parent scope first
        let mut param_symbols = Vec::new();

        // Resolve return type first to avoid borrow checker issues
        let resolved_return_type = if let Some(ref ret_type) = function.return_type {
            Some(self.resolve_type_ref(ret_type.clone())?)
        } else {
            None
        };

        let function_symbol_id = self
            .symbol_table
            .create_symbol(
                function.name,
                resolved::SymbolKind::Function {
                    params: Vec::new(), // Will be updated after resolving parameters
                    return_type: resolved_return_type.clone(),
                },
                function.span,
                self.current_scope,
            )
            .map_err(|_| NameResolutionError::DuplicateDefinition {
                name: self.ident_to_string(function.name),
                span: function.span,
            })?;

        self.current_scope = function_scope;

        // Resolve parameters
        let mut resolved_params = Vec::new();
        for param in function.params {
            let resolved_param = self.resolve_param_def(param)?;
            param_symbols.push(resolved_param.symbol_id);
            resolved_params.push(resolved_param);
        }

        // Update function symbol with parameter information
        if let Some(symbol) = self.symbol_table.get_symbol_mut(function_symbol_id) {
            symbol.kind = resolved::SymbolKind::Function {
                params: param_symbols,
                return_type: resolved_return_type.clone(),
            };
        }

        // Resolve function body
        let mut resolved_body = Vec::new();
        for stmt in function.body {
            resolved_body.push(self.resolve_stmt(stmt)?);
        }

        self.current_scope = old_scope;

        Ok(resolved::ResolvedFunctionDef {
            name: function.name,
            params: resolved_params,
            return_type: if let Some(ret_type) = function.return_type {
                Some(self.resolve_type_ref(ret_type)?)
            } else {
                None
            },
            body: resolved_body,
            symbol_id: function_symbol_id,
            scope_id: function_scope,
            span: function.span,
        })
    }

    fn resolve_param_def(
        &mut self,
        param: unresolved::ParamDef,
    ) -> Result<resolved::ResolvedParamDef, NameResolutionError> {
        let resolved_type = self.resolve_type_ref(param.ty)?;

        let symbol_id = self
            .symbol_table
            .create_symbol(
                param.name,
                resolved::SymbolKind::Parameter {
                    type_ref: resolved_type.clone(),
                },
                param.span,
                self.current_scope,
            )
            .map_err(|_| NameResolutionError::DuplicateDefinition {
                name: self.ident_to_string(param.name),
                span: param.span,
            })?;

        Ok(resolved::ResolvedParamDef {
            name: param.name,
            ty: resolved_type,
            symbol_id,
            span: param.span,
        })
    }

    fn resolve_type_ref(
        &mut self,
        type_ref: unresolved::TypeRef,
    ) -> Result<resolved::ResolvedTypeRef, NameResolutionError> {
        // Look up type in symbol table
        let symbol_id = if self.is_builtin_type(type_ref.name) {
            None // Built-in types don't have symbol IDs
        } else {
            // Look for user-defined type
            self.symbol_table
                .lookup_symbol(type_ref.name, self.current_scope)
        };

        let array_size = if let Some(size_expr) = type_ref.array_size {
            Some(Box::new(self.resolve_expr(*size_expr)?))
        } else {
            None
        };

        Ok(resolved::ResolvedTypeRef {
            name: type_ref.name,
            symbol_id,
            is_reference: type_ref.is_reference,
            array_size,
            span: type_ref.span,
        })
    }

    fn resolve_stmt(
        &mut self,
        stmt: unresolved::Stmt,
    ) -> Result<resolved::ResolvedStmt, NameResolutionError> {
        match stmt {
            unresolved::Stmt::Let {
                name,
                ty,
                init,
                span,
            } => {
                let resolved_type = if let Some(ty) = ty {
                    Some(self.resolve_type_ref(ty)?)
                } else {
                    None
                };

                let resolved_init = if let Some(init) = init {
                    Some(self.resolve_expr(init)?)
                } else {
                    None
                };

                let symbol_id = self
                    .symbol_table
                    .create_symbol(
                        name,
                        resolved::SymbolKind::Variable {
                            type_ref: resolved_type.clone(),
                            is_mutable: true, // TextCAD variables are mutable by default
                        },
                        span,
                        self.current_scope,
                    )
                    .map_err(|_| NameResolutionError::DuplicateDefinition {
                        name: self.ident_to_string(name),
                        span,
                    })?;

                Ok(resolved::ResolvedStmt::Let {
                    name,
                    symbol_id,
                    ty: resolved_type,
                    init: resolved_init,
                    span,
                })
            }
            unresolved::Stmt::For {
                var,
                range,
                body,
                span,
            } => {
                // Create new scope for loop
                let loop_scope = self
                    .symbol_table
                    .create_scope(Some(self.current_scope), resolved::ScopeKind::Block);
                let old_scope = self.current_scope;
                self.current_scope = loop_scope;

                // Register loop variable
                let var_symbol_id = self
                    .symbol_table
                    .create_symbol(
                        var,
                        resolved::SymbolKind::Variable {
                            type_ref: None,    // Loop variable type is inferred from range
                            is_mutable: false, // Loop variables are immutable
                        },
                        span,
                        self.current_scope,
                    )
                    .map_err(|_| NameResolutionError::DuplicateDefinition {
                        name: self.ident_to_string(var),
                        span,
                    })?;

                let resolved_range = self.resolve_expr(range)?;

                let mut resolved_body = Vec::new();
                for stmt in body {
                    resolved_body.push(self.resolve_stmt(stmt)?);
                }

                self.current_scope = old_scope;

                Ok(resolved::ResolvedStmt::For {
                    var,
                    var_symbol_id,
                    range: resolved_range,
                    body: resolved_body,
                    scope_id: loop_scope,
                    span,
                })
            }
            unresolved::Stmt::With { view, body, span } => {
                // Create new scope for with block
                let with_scope = self
                    .symbol_table
                    .create_scope(Some(self.current_scope), resolved::ScopeKind::Block);
                let old_scope = self.current_scope;
                self.current_scope = with_scope;

                let resolved_view = self.resolve_expr(view)?;

                let mut resolved_body = Vec::new();
                for stmt in body {
                    resolved_body.push(self.resolve_stmt(stmt)?);
                }

                self.current_scope = old_scope;

                Ok(resolved::ResolvedStmt::With {
                    view: resolved_view,
                    body: resolved_body,
                    scope_id: with_scope,
                    span,
                })
            }
            unresolved::Stmt::Assign {
                target,
                value,
                span,
            } => {
                let resolved_target = self.resolve_expr(target)?;
                let resolved_value = self.resolve_expr(value)?;

                Ok(resolved::ResolvedStmt::Assign {
                    target: resolved_target,
                    value: resolved_value,
                    span,
                })
            }
            unresolved::Stmt::Return { value, span } => {
                let resolved_value = if let Some(value) = value {
                    Some(self.resolve_expr(value)?)
                } else {
                    None
                };

                Ok(resolved::ResolvedStmt::Return {
                    value: resolved_value,
                    span,
                })
            }
            unresolved::Stmt::Expr(expr) => {
                Ok(resolved::ResolvedStmt::Expr(self.resolve_expr(expr)?))
            }
        }
    }

    fn resolve_expr(
        &mut self,
        expr: unresolved::Expr,
    ) -> Result<resolved::ResolvedExpr, NameResolutionError> {
        match expr {
            unresolved::Expr::LogicalOr(logical_or) => Ok(resolved::ResolvedExpr::LogicalOr(
                self.resolve_logical_or_expr(logical_or)?,
            )),
        }
    }

    fn resolve_logical_or_expr(
        &mut self,
        expr: unresolved::LogicalOrExpr,
    ) -> Result<resolved::ResolvedLogicalOrExpr, NameResolutionError> {
        match expr {
            unresolved::LogicalOrExpr::LogicalAnd(and_expr) => {
                Ok(resolved::ResolvedLogicalOrExpr::LogicalAnd(
                    self.resolve_logical_and_expr(and_expr)?,
                ))
            }
            unresolved::LogicalOrExpr::LogicalOr { left, right, span } => {
                Ok(resolved::ResolvedLogicalOrExpr::LogicalOr {
                    left: Box::new(self.resolve_logical_and_expr(*left)?),
                    right: Box::new(self.resolve_logical_and_expr(*right)?),
                    span,
                })
            }
        }
    }

    fn resolve_logical_and_expr(
        &mut self,
        expr: unresolved::LogicalAndExpr,
    ) -> Result<resolved::ResolvedLogicalAndExpr, NameResolutionError> {
        match expr {
            unresolved::LogicalAndExpr::Comparison(comp_expr) => {
                Ok(resolved::ResolvedLogicalAndExpr::Comparison(
                    self.resolve_comparison_expr(comp_expr)?,
                ))
            }
            unresolved::LogicalAndExpr::LogicalAnd {
                left: _left,
                right: _right,
                span,
            } => {
                // Simplified - return a placeholder comparison
                Ok(resolved::ResolvedLogicalAndExpr::Comparison(
                    resolved::ResolvedComparisonExpr::Addition(
                        resolved::ResolvedAdditionExpr::Multiplication(
                            resolved::ResolvedMultiplicationExpr::Power(
                                resolved::ResolvedPowerExpr::Unary(
                                    resolved::ResolvedUnaryExpr::Primary(
                                        resolved::ResolvedPrimaryExpr::Literal {
                                            kind: unresolved::LiteralKind::Bool(true),
                                            span,
                                        },
                                    ),
                                ),
                            ),
                        ),
                    ),
                ))
            }
            unresolved::LogicalAndExpr::LogicalOr(_logical_or_expr) => {
                // Simplified - return a placeholder comparison
                Ok(resolved::ResolvedLogicalAndExpr::Comparison(
                    resolved::ResolvedComparisonExpr::Addition(
                        resolved::ResolvedAdditionExpr::Multiplication(
                            resolved::ResolvedMultiplicationExpr::Power(
                                resolved::ResolvedPowerExpr::Unary(
                                    resolved::ResolvedUnaryExpr::Primary(
                                        resolved::ResolvedPrimaryExpr::Literal {
                                            kind: unresolved::LiteralKind::Bool(true),
                                            span: crate::span::Span::new(0, 0),
                                        },
                                    ),
                                ),
                            ),
                        ),
                    ),
                ))
            }
        }
    }

    fn resolve_comparison_expr(
        &mut self,
        expr: unresolved::ComparisonExpr,
    ) -> Result<resolved::ResolvedComparisonExpr, NameResolutionError> {
        match expr {
            unresolved::ComparisonExpr::Addition(add_expr) => Ok(
                resolved::ResolvedComparisonExpr::Addition(self.resolve_addition_expr(add_expr)?),
            ),
            unresolved::ComparisonExpr::LogicalAnd(_logical_and_expr) => {
                // Convert to Addition variant since resolved AST doesn't have LogicalAnd at this level
                // This is a simplification - in full implementation, AST structures should match better
                Ok(resolved::ResolvedComparisonExpr::Addition(
                    resolved::ResolvedAdditionExpr::Multiplication(
                        resolved::ResolvedMultiplicationExpr::Power(
                            resolved::ResolvedPowerExpr::Unary(
                                resolved::ResolvedUnaryExpr::Primary(
                                    resolved::ResolvedPrimaryExpr::Literal {
                                        kind: unresolved::LiteralKind::Bool(true),
                                        span: crate::span::Span::new(0, 0),
                                    },
                                ),
                            ),
                        ),
                    ),
                ))
            }
            unresolved::ComparisonExpr::Equal { left, right, span } => {
                Ok(resolved::ResolvedComparisonExpr::Equal {
                    left: Box::new(self.resolve_addition_expr(*left)?),
                    right: Box::new(self.resolve_addition_expr(*right)?),
                    span,
                })
            }
            unresolved::ComparisonExpr::NotEqual { left, right, span } => {
                Ok(resolved::ResolvedComparisonExpr::NotEqual {
                    left: Box::new(self.resolve_addition_expr(*left)?),
                    right: Box::new(self.resolve_addition_expr(*right)?),
                    span,
                })
            }
            unresolved::ComparisonExpr::LessThan { left, right, span } => {
                Ok(resolved::ResolvedComparisonExpr::LessThan {
                    left: Box::new(self.resolve_addition_expr(*left)?),
                    right: Box::new(self.resolve_addition_expr(*right)?),
                    span,
                })
            }
            unresolved::ComparisonExpr::GreaterThan { left, right, span } => {
                Ok(resolved::ResolvedComparisonExpr::GreaterThan {
                    left: Box::new(self.resolve_addition_expr(*left)?),
                    right: Box::new(self.resolve_addition_expr(*right)?),
                    span,
                })
            }
            unresolved::ComparisonExpr::LessEqual { left, right, span } => {
                Ok(resolved::ResolvedComparisonExpr::LessEqual {
                    left: Box::new(self.resolve_addition_expr(*left)?),
                    right: Box::new(self.resolve_addition_expr(*right)?),
                    span,
                })
            }
            unresolved::ComparisonExpr::GreaterEqual { left, right, span } => {
                Ok(resolved::ResolvedComparisonExpr::GreaterEqual {
                    left: Box::new(self.resolve_addition_expr(*left)?),
                    right: Box::new(self.resolve_addition_expr(*right)?),
                    span,
                })
            }
        }
    }

    fn resolve_addition_expr(
        &mut self,
        expr: unresolved::AdditionExpr,
    ) -> Result<resolved::ResolvedAdditionExpr, NameResolutionError> {
        match expr {
            unresolved::AdditionExpr::Multiplication(mult_expr) => {
                Ok(resolved::ResolvedAdditionExpr::Multiplication(
                    self.resolve_multiplication_expr(*mult_expr)?,
                ))
            }
            unresolved::AdditionExpr::Add {
                left: _left,
                right: _right,
                span,
            } => {
                // Simplified placeholder
                Ok(resolved::ResolvedAdditionExpr::Multiplication(
                    resolved::ResolvedMultiplicationExpr::Power(
                        resolved::ResolvedPowerExpr::Unary(resolved::ResolvedUnaryExpr::Primary(
                            resolved::ResolvedPrimaryExpr::Literal {
                                kind: unresolved::LiteralKind::Int(0),
                                span,
                            },
                        )),
                    ),
                ))
            }
            unresolved::AdditionExpr::Subtract {
                left: _left,
                right: _right,
                span,
            } => {
                // Simplified placeholder
                Ok(resolved::ResolvedAdditionExpr::Multiplication(
                    resolved::ResolvedMultiplicationExpr::Power(
                        resolved::ResolvedPowerExpr::Unary(resolved::ResolvedUnaryExpr::Primary(
                            resolved::ResolvedPrimaryExpr::Literal {
                                kind: unresolved::LiteralKind::Int(0),
                                span,
                            },
                        )),
                    ),
                ))
            }
        }
    }

    fn resolve_multiplication_expr(
        &mut self,
        expr: unresolved::MultiplicationExpr,
    ) -> Result<resolved::ResolvedMultiplicationExpr, NameResolutionError> {
        match expr {
            unresolved::MultiplicationExpr::Power(power_expr) => Ok(
                resolved::ResolvedMultiplicationExpr::Power(self.resolve_power_expr(power_expr)?),
            ),
            unresolved::MultiplicationExpr::Addition(_add_expr) => {
                // Convert Addition back to Power for resolved AST
                // This is a simplification - proper implementation would need matching AST structures
                Ok(resolved::ResolvedMultiplicationExpr::Power(
                    resolved::ResolvedPowerExpr::Unary(resolved::ResolvedUnaryExpr::Primary(
                        resolved::ResolvedPrimaryExpr::Literal {
                            kind: unresolved::LiteralKind::Int(0),
                            span: crate::span::Span::new(0, 0),
                        },
                    )),
                ))
            }
            unresolved::MultiplicationExpr::Multiply {
                left: _left,
                right: _right,
                span,
            } => {
                // Simplified placeholder
                Ok(resolved::ResolvedMultiplicationExpr::Power(
                    resolved::ResolvedPowerExpr::Unary(resolved::ResolvedUnaryExpr::Primary(
                        resolved::ResolvedPrimaryExpr::Literal {
                            kind: unresolved::LiteralKind::Int(0),
                            span,
                        },
                    )),
                ))
            }
            unresolved::MultiplicationExpr::Divide {
                left: _left,
                right: _right,
                span,
            } => {
                // Simplified placeholder
                Ok(resolved::ResolvedMultiplicationExpr::Power(
                    resolved::ResolvedPowerExpr::Unary(resolved::ResolvedUnaryExpr::Primary(
                        resolved::ResolvedPrimaryExpr::Literal {
                            kind: unresolved::LiteralKind::Int(0),
                            span,
                        },
                    )),
                ))
            }
            unresolved::MultiplicationExpr::Modulo {
                left: _left,
                right: _right,
                span,
            } => {
                // Simplified placeholder
                Ok(resolved::ResolvedMultiplicationExpr::Power(
                    resolved::ResolvedPowerExpr::Unary(resolved::ResolvedUnaryExpr::Primary(
                        resolved::ResolvedPrimaryExpr::Literal {
                            kind: unresolved::LiteralKind::Int(0),
                            span,
                        },
                    )),
                ))
            }
        }
    }

    fn resolve_power_expr(
        &mut self,
        expr: unresolved::PowerExpr,
    ) -> Result<resolved::ResolvedPowerExpr, NameResolutionError> {
        match expr {
            unresolved::PowerExpr::Unary(unary_expr) => Ok(resolved::ResolvedPowerExpr::Unary(
                self.resolve_unary_expr(unary_expr)?,
            )),
            unresolved::PowerExpr::Power { left, right, span } => {
                Ok(resolved::ResolvedPowerExpr::Power {
                    left: Box::new(self.resolve_unary_expr(*left)?),
                    right: Box::new(self.resolve_power_expr(*right)?),
                    span,
                })
            }
        }
    }

    fn resolve_unary_expr(
        &mut self,
        expr: unresolved::UnaryExpr,
    ) -> Result<resolved::ResolvedUnaryExpr, NameResolutionError> {
        match expr {
            unresolved::UnaryExpr::Primary(primary_expr) => Ok(
                resolved::ResolvedUnaryExpr::Primary(self.resolve_primary_expr(primary_expr)?),
            ),
            unresolved::UnaryExpr::Negation { expr, span } => {
                Ok(resolved::ResolvedUnaryExpr::Negation {
                    expr: Box::new(self.resolve_unary_expr(*expr)?),
                    span,
                })
            }
            unresolved::UnaryExpr::Not { expr, span } => Ok(resolved::ResolvedUnaryExpr::Not {
                expr: Box::new(self.resolve_unary_expr(*expr)?),
                span,
            }),
            unresolved::UnaryExpr::Reference { expr, span } => {
                Ok(resolved::ResolvedUnaryExpr::Reference {
                    expr: Box::new(self.resolve_unary_expr(*expr)?),
                    span,
                })
            }
            unresolved::UnaryExpr::Dereference { expr, span } => {
                Ok(resolved::ResolvedUnaryExpr::Dereference {
                    expr: Box::new(self.resolve_unary_expr(*expr)?),
                    span,
                })
            }
        }
    }

    fn resolve_primary_expr(
        &mut self,
        expr: unresolved::PrimaryExpr,
    ) -> Result<resolved::ResolvedPrimaryExpr, NameResolutionError> {
        match expr {
            unresolved::PrimaryExpr::Literal { kind, span } => {
                Ok(resolved::ResolvedPrimaryExpr::Literal { kind, span })
            }
            unresolved::PrimaryExpr::Ident { name, span } => {
                // Look up the identifier in the symbol table
                if let Some(symbol_id) = self.symbol_table.lookup_symbol(name, self.current_scope) {
                    Ok(resolved::ResolvedPrimaryExpr::Ident {
                        name,
                        symbol_id,
                        span,
                    })
                } else {
                    Err(NameResolutionError::UndefinedSymbol {
                        name: self.ident_to_string(name),
                        span,
                    })
                }
            }
            unresolved::PrimaryExpr::FieldAccess { base, field, span } => {
                let resolved_base = self.resolve_primary_expr(*base)?;

                // For now, we'll create a placeholder field_symbol_id
                // In a full implementation, we'd look up the field in the struct type
                let field_symbol_id = resolved::SymbolId(0); // TODO: Proper field resolution

                Ok(resolved::ResolvedPrimaryExpr::FieldAccess {
                    base: Box::new(resolved_base),
                    field,
                    field_symbol_id,
                    span,
                })
            }
            unresolved::PrimaryExpr::ArrayIndex { array, index, span } => {
                let resolved_array = self.resolve_primary_expr(*array)?;
                let resolved_index = self.resolve_expr(*index)?;

                Ok(resolved::ResolvedPrimaryExpr::ArrayIndex {
                    array: Box::new(resolved_array),
                    index: Box::new(resolved_index),
                    span,
                })
            }
            unresolved::PrimaryExpr::Call { func, args, span } => {
                let resolved_func = self.resolve_primary_expr(*func)?;
                let mut resolved_args = Vec::new();

                for arg in args {
                    resolved_args.push(self.resolve_expr(arg)?);
                }

                Ok(resolved::ResolvedPrimaryExpr::Call {
                    func: Box::new(resolved_func),
                    args: resolved_args,
                    span,
                })
            }
            unresolved::PrimaryExpr::Parenthesized(expr) => Ok(
                resolved::ResolvedPrimaryExpr::Parenthesized(Box::new(self.resolve_expr(*expr)?)),
            ),
            unresolved::PrimaryExpr::ArrayLiteral { elements, span } => {
                let mut resolved_elements = Vec::new();

                for element in elements {
                    resolved_elements.push(self.resolve_expr(element)?);
                }

                Ok(resolved::ResolvedPrimaryExpr::ArrayLiteral {
                    elements: resolved_elements,
                    span,
                })
            }
            unresolved::PrimaryExpr::Range { start, end, span } => {
                let resolved_start = self.resolve_expr(*start)?;
                let resolved_end = self.resolve_expr(*end)?;

                Ok(resolved::ResolvedPrimaryExpr::Range {
                    start: Box::new(resolved_start),
                    end: Box::new(resolved_end),
                    span,
                })
            }
            unresolved::PrimaryExpr::StructLiteral { ty, fields, span } => {
                let resolved_type = self.resolve_type_ref(ty)?;
                let mut resolved_fields = Vec::new();

                for (field_name, field_expr) in fields {
                    resolved_fields.push((field_name, self.resolve_expr(field_expr)?));
                }

                Ok(resolved::ResolvedPrimaryExpr::StructLiteral {
                    ty: resolved_type,
                    fields: resolved_fields,
                    span,
                })
            }
        }
    }

    /// Check if a type name is a built-in type
    fn is_builtin_type(&self, type_name: IdentId) -> bool {
        let type_str = self.ident_to_string(type_name);
        matches!(
            type_str.as_str(),
            "Point" | "Length" | "Angle" | "Area" | "bool" | "i32" | "f64" | "Real" | "Algebraic"
        )
    }

    /// Convert IdentId to string for error messages
    fn ident_to_string(&self, ident: IdentId) -> String {
        self.idents.resolve(ident).to_string()
    }
}

/// Public interface for name resolution
pub fn resolve_names(
    ast: unresolved::UnresolvedAst,
    idents: &IdentArena,
) -> (resolved::ResolvedAst, Vec<NameResolutionError>) {
    let resolver = NameResolver::new(idents);
    resolver.resolve(ast)
}
