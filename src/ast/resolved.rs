use crate::ast::unresolved::*;
use crate::ident::IdentId;
use crate::span::Span;
use std::collections::HashMap;

pub type SymbolId = usize;
pub type SymbolTableId = usize;
pub type StructId = usize;
pub type FunctionId = usize;

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedAst {
    pub sketches: Vec<ResolvedSketchDef>,
    pub symbol_tables: Vec<SymbolTable>,
    pub struct_definitions: HashMap<StructId, StructDef>,
    pub function_definitions: HashMap<FunctionId, FunctionDef>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedSketchDef {
    pub name: IdentId,
    pub body: Vec<ResolvedStmt>,
    pub scope: SymbolTableId,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SymbolTable {
    pub parent: Option<SymbolTableId>,
    pub symbols: HashMap<IdentId, Symbol>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    pub name: IdentId,
    pub kind: SymbolKind,
    pub def_span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Variable { type_ref: ResolvedTypeRef },
    Function { 
        params: Vec<ResolvedTypeRef>, 
        return_type: Option<ResolvedTypeRef>,
        function_id: FunctionId,
    },
    Struct { 
        fields: Vec<FieldDef>,
        struct_id: StructId,
    },
    Parameter { type_ref: ResolvedTypeRef },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedStmt {
    Let {
        name: IdentId,
        symbol_id: SymbolId,
        ty: Option<ResolvedTypeRef>,
        init: Option<ResolvedExpr>,
        span: Span,
    },
    Assign {
        target: ResolvedExpr,
        value: ResolvedExpr,
        span: Span,
    },
    For {
        var: IdentId,
        var_symbol_id: SymbolId,
        range: ResolvedExpr,
        body: Vec<ResolvedStmt>,
        scope: SymbolTableId,
        span: Span,
    },
    With {
        view: ResolvedExpr,
        body: Vec<ResolvedStmt>,
        scope: SymbolTableId,
        span: Span,
    },
    Expr(ResolvedExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedExpr {
    Literal {
        kind: LiteralKind,
        span: Span,
    },
    Ident {
        name: IdentId,
        symbol_id: SymbolId,
        span: Span,
    },
    BinaryOp {
        op: BinOp,
        left: Box<ResolvedExpr>,
        right: Box<ResolvedExpr>,
        span: Span,
    },
    UnaryOp {
        op: UnaryOp,
        expr: Box<ResolvedExpr>,
        span: Span,
    },
    Call {
        func: Box<ResolvedExpr>,
        func_id: Option<FunctionId>,
        args: Vec<ResolvedExpr>,
        span: Span,
    },
    FieldAccess {
        base: Box<ResolvedExpr>,
        field: IdentId,
        field_symbol_id: Option<SymbolId>,
        span: Span,
    },
    ArrayIndex {
        array: Box<ResolvedExpr>,
        index: Box<ResolvedExpr>,
        span: Span,
    },
    StructLiteral {
        ty: ResolvedTypeRef,
        fields: Vec<(IdentId, ResolvedExpr)>,
        span: Span,
    },
    ArrayLiteral {
        elements: Vec<ResolvedExpr>,
        span: Span,
    },
    Range {
        start: Box<ResolvedExpr>,
        end: Box<ResolvedExpr>,
        span: Span,
    },
    Reference {
        expr: Box<ResolvedExpr>,
        span: Span,
    },
    Dereference {
        expr: Box<ResolvedExpr>,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedTypeRef {
    pub name: IdentId,
    pub struct_id: Option<StructId>,
    pub is_reference: bool,
    pub array_size: Option<Box<ResolvedExpr>>,
    pub span: Span,
}

impl ResolvedStmt {
    pub fn span(&self) -> Span {
        match self {
            ResolvedStmt::Let { span, .. } => *span,
            ResolvedStmt::Assign { span, .. } => *span,
            ResolvedStmt::For { span, .. } => *span,
            ResolvedStmt::With { span, .. } => *span,
            ResolvedStmt::Expr(expr) => expr.span(),
        }
    }
}

impl ResolvedExpr {
    pub fn span(&self) -> Span {
        match self {
            ResolvedExpr::Literal { span, .. } => *span,
            ResolvedExpr::Ident { span, .. } => *span,
            ResolvedExpr::BinaryOp { span, .. } => *span,
            ResolvedExpr::UnaryOp { span, .. } => *span,
            ResolvedExpr::Call { span, .. } => *span,
            ResolvedExpr::FieldAccess { span, .. } => *span,
            ResolvedExpr::ArrayIndex { span, .. } => *span,
            ResolvedExpr::StructLiteral { span, .. } => *span,
            ResolvedExpr::ArrayLiteral { span, .. } => *span,
            ResolvedExpr::Range { span, .. } => *span,
            ResolvedExpr::Reference { span, .. } => *span,
            ResolvedExpr::Dereference { span, .. } => *span,
        }
    }
}