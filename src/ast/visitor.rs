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
            Expr::Literal { .. } => {}
            Expr::Ident { .. } => {}
            Expr::BinaryOp { left, right, .. } => {
                self.visit_expr(left);
                self.visit_expr(right);
            }
            Expr::UnaryOp { expr, .. } => {
                self.visit_expr(expr);
            }
            Expr::Call { func, args, .. } => {
                self.visit_expr(func);
                for arg in args {
                    self.visit_expr(arg);
                }
            }
            Expr::FieldAccess { base, .. } => {
                self.visit_expr(base);
            }
            Expr::ArrayIndex { array, index, .. } => {
                self.visit_expr(array);
                self.visit_expr(index);
            }
            Expr::StructLiteral { ty, fields, .. } => {
                self.visit_type_ref(ty);
                for (_, expr) in fields {
                    self.visit_expr(expr);
                }
            }
            Expr::ArrayLiteral { elements, .. } => {
                for element in elements {
                    self.visit_expr(element);
                }
            }
            Expr::Range { start, end, .. } => {
                self.visit_expr(start);
                self.visit_expr(end);
            }
            Expr::Reference { expr, .. } => {
                self.visit_expr(expr);
            }
            Expr::Dereference { expr, .. } => {
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
