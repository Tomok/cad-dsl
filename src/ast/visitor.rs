use crate::ast::unresolved::*;

pub trait Visitor<R: Default = ()> {
    fn visit_ast(&mut self, ast: &UnresolvedAst) -> R {
        self.walk_ast(ast)
    }

    fn walk_ast(&mut self, ast: &UnresolvedAst) -> R {
        for sketch in &ast.sketches {
            self.visit_sketch(sketch);
        }
        for struct_def in &ast.structs {
            self.visit_struct_def(struct_def);
        }
        for import in &ast.imports {
            self.visit_import(import);
        }
        R::default()
    }

    fn visit_import(&mut self, import: &ImportDef) -> R {
        self.walk_import(import)
    }

    fn walk_import(&mut self, _import: &ImportDef) -> R {
        R::default()
    }

    fn visit_sketch(&mut self, sketch: &SketchDef) -> R {
        self.walk_sketch(sketch)
    }

    fn walk_sketch(&mut self, sketch: &SketchDef) -> R {
        for stmt in &sketch.body {
            self.visit_stmt(stmt);
        }
        R::default()
    }

    fn visit_struct_def(&mut self, struct_def: &StructDef) -> R {
        self.walk_struct_def(struct_def)
    }

    fn walk_struct_def(&mut self, struct_def: &StructDef) -> R {
        for field in &struct_def.fields {
            self.visit_field_def(field);
        }
        for method in &struct_def.methods {
            self.visit_function_def(method);
        }
        R::default()
    }

    fn visit_field_def(&mut self, field: &FieldDef) -> R {
        self.walk_field_def(field)
    }

    fn walk_field_def(&mut self, field: &FieldDef) -> R {
        self.visit_type_ref(&field.ty);
        R::default()
    }

    fn visit_function_def(&mut self, func: &FunctionDef) -> R {
        self.walk_function_def(func)
    }

    fn walk_function_def(&mut self, func: &FunctionDef) -> R {
        for param in &func.params {
            self.visit_param_def(param);
        }
        if let Some(return_type) = &func.return_type {
            self.visit_type_ref(return_type);
        }
        for stmt in &func.body {
            self.visit_stmt(stmt);
        }
        R::default()
    }

    fn visit_param_def(&mut self, param: &ParamDef) -> R {
        self.walk_param_def(param)
    }

    fn walk_param_def(&mut self, param: &ParamDef) -> R {
        self.visit_type_ref(&param.ty);
        R::default()
    }

    fn visit_stmt(&mut self, stmt: &Stmt) -> R {
        self.walk_stmt(stmt)
    }

    fn walk_stmt(&mut self, stmt: &Stmt) -> R {
        match stmt {
            Stmt::Let { ty, init, .. } => {
                if let Some(ty) = ty {
                    self.visit_type_ref(ty);
                }
                if let Some(init) = init {
                    self.visit_expr(init);
                }
            }
            Stmt::Assign { target, value, .. } => {
                self.visit_expr(target);
                self.visit_expr(value);
            }
            Stmt::For { range, body, .. } => {
                self.visit_expr(range);
                for stmt in body {
                    self.visit_stmt(stmt);
                }
            }
            Stmt::With { view, body, .. } => {
                self.visit_expr(view);
                for stmt in body {
                    self.visit_stmt(stmt);
                }
            }
            Stmt::Return { value, .. } => {
                if let Some(expr) = value {
                    self.visit_expr(expr);
                }
            }
            Stmt::Expr(expr) => {
                self.visit_expr(expr);
            }
        }
        R::default()
    }

    fn visit_expr(&mut self, expr: &Expr) -> R {
        self.walk_expr(expr)
    }

    fn walk_expr(&mut self, expr: &Expr) -> R {
        match expr {
            Expr::LogicalOr(logical_or_expr) => {
                self.visit_logical_or_expr(logical_or_expr);
            }
        }
        R::default()
    }

    fn visit_logical_or_expr(&mut self, expr: &LogicalOrExpr) -> R {
        self.walk_logical_or_expr(expr)
    }

    fn walk_logical_or_expr(&mut self, expr: &LogicalOrExpr) -> R {
        match expr {
            LogicalOrExpr::LogicalOr { left, right, .. } => {
                self.visit_logical_and_expr(left);
                self.visit_logical_and_expr(right);
            }
            LogicalOrExpr::LogicalAnd(logical_and_expr) => {
                self.visit_logical_and_expr(logical_and_expr);
            }
        }
        R::default()
    }

    fn visit_logical_and_expr(&mut self, expr: &LogicalAndExpr) -> R {
        self.walk_logical_and_expr(expr)
    }

    fn walk_logical_and_expr(&mut self, expr: &LogicalAndExpr) -> R {
        match expr {
            LogicalAndExpr::LogicalAnd { left, right, .. } => {
                self.visit_logical_and_expr(left);
                self.visit_comparison_expr(right);
            }
            LogicalAndExpr::LogicalOr(logical_or_expr) => {
                self.visit_logical_or_expr(logical_or_expr);
            }
            LogicalAndExpr::Comparison(comparison_expr) => {
                self.visit_comparison_expr(comparison_expr);
            }
        }
        R::default()
    }

    fn visit_comparison_expr(&mut self, expr: &ComparisonExpr) -> R {
        self.walk_comparison_expr(expr)
    }

    fn walk_comparison_expr(&mut self, expr: &ComparisonExpr) -> R {
        match expr {
            ComparisonExpr::Equal { left, right, .. }
            | ComparisonExpr::NotEqual { left, right, .. }
            | ComparisonExpr::LessThan { left, right, .. }
            | ComparisonExpr::GreaterThan { left, right, .. }
            | ComparisonExpr::LessEqual { left, right, .. }
            | ComparisonExpr::GreaterEqual { left, right, .. } => {
                self.visit_addition_expr(left);
                self.visit_addition_expr(right);
            }
            ComparisonExpr::LogicalAnd(logical_and_expr) => {
                self.visit_logical_and_expr(logical_and_expr);
            }
            ComparisonExpr::Addition(addition_expr) => {
                self.visit_addition_expr(addition_expr);
            }
        }
        R::default()
    }

    fn visit_addition_expr(&mut self, expr: &AdditionExpr) -> R {
        self.walk_addition_expr(expr)
    }

    fn walk_addition_expr(&mut self, expr: &AdditionExpr) -> R {
        match expr {
            AdditionExpr::Add { left, right, .. } | AdditionExpr::Subtract { left, right, .. } => {
                self.visit_addition_expr(left);
                self.visit_multiplication_expr(right);
            }
            AdditionExpr::Multiplication(multiplication_expr) => {
                self.visit_multiplication_expr(multiplication_expr);
            }
        }
        R::default()
    }

    fn visit_multiplication_expr(&mut self, expr: &MultiplicationExpr) -> R {
        self.walk_multiplication_expr(expr)
    }

    fn walk_multiplication_expr(&mut self, expr: &MultiplicationExpr) -> R {
        match expr {
            MultiplicationExpr::Multiply { left, right, .. }
            | MultiplicationExpr::Divide { left, right, .. }
            | MultiplicationExpr::Modulo { left, right, .. } => {
                self.visit_multiplication_expr(left);
                self.visit_power_expr(right);
            }
            MultiplicationExpr::Addition(addition_expr) => {
                self.visit_addition_expr(addition_expr);
            }
            MultiplicationExpr::Power(power_expr) => {
                self.visit_power_expr(power_expr);
            }
        }
        R::default()
    }

    fn visit_power_expr(&mut self, expr: &PowerExpr) -> R {
        self.walk_power_expr(expr)
    }

    fn walk_power_expr(&mut self, expr: &PowerExpr) -> R {
        match expr {
            PowerExpr::Power { left, right, .. } => {
                self.visit_unary_expr(left);
                self.visit_power_expr(right);
            }
            PowerExpr::Unary(unary_expr) => {
                self.visit_unary_expr(unary_expr);
            }
        }
        R::default()
    }

    fn visit_unary_expr(&mut self, expr: &UnaryExpr) -> R {
        self.walk_unary_expr(expr)
    }

    fn walk_unary_expr(&mut self, expr: &UnaryExpr) -> R {
        match expr {
            UnaryExpr::Negation { expr, .. }
            | UnaryExpr::Not { expr, .. }
            | UnaryExpr::Reference { expr, .. }
            | UnaryExpr::Dereference { expr, .. } => {
                self.visit_unary_expr(expr);
            }
            UnaryExpr::Primary(primary_expr) => {
                self.visit_primary_expr(primary_expr);
            }
        }
        R::default()
    }

    fn visit_primary_expr(&mut self, expr: &PrimaryExpr) -> R {
        self.walk_primary_expr(expr)
    }

    fn walk_primary_expr(&mut self, expr: &PrimaryExpr) -> R {
        match expr {
            PrimaryExpr::Literal { .. } => {}
            PrimaryExpr::Ident { .. } => {}
            PrimaryExpr::Call { func, args, .. } => {
                self.visit_primary_expr(func);
                for arg in args {
                    self.visit_expr(arg);
                }
            }
            PrimaryExpr::FieldAccess { base, .. } => {
                self.visit_primary_expr(base);
            }
            PrimaryExpr::ArrayIndex { array, index, .. } => {
                self.visit_primary_expr(array);
                self.visit_expr(index);
            }
            PrimaryExpr::StructLiteral { ty, fields, .. } => {
                self.visit_type_ref(ty);
                for (_, expr) in fields {
                    self.visit_expr(expr);
                }
            }
            PrimaryExpr::ArrayLiteral { elements, .. } => {
                for element in elements {
                    self.visit_expr(element);
                }
            }
            PrimaryExpr::Range { start, end, .. } => {
                self.visit_expr(start);
                self.visit_expr(end);
            }
            PrimaryExpr::Parenthesized(expr) => {
                self.visit_expr(expr);
            }
        }
        R::default()
    }

    fn visit_type_ref(&mut self, type_ref: &TypeRef) -> R {
        self.walk_type_ref(type_ref)
    }

    fn walk_type_ref(&mut self, type_ref: &TypeRef) -> R {
        if let Some(array_size) = &type_ref.array_size {
            self.visit_expr(array_size);
        }
        R::default()
    }
}
