use crate::ast::resolved::*;
use crate::ast::unresolved::{BinOp, LiteralKind, UnaryOp};
use crate::ident::IdentId;
use crate::span::Span;
use std::collections::HashMap;

pub type TypeId = usize;
pub type ViewContextId = usize;

#[derive(Debug, Clone, PartialEq)]
pub struct TypedIr {
    pub sketches: Vec<TypedSketch>,
    pub type_table: TypeTable,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedSketch {
    pub name: IdentId,
    pub body: Vec<TypedStmt>,
    pub scope: SymbolTableId,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedStmt {
    Let {
        name: IdentId,
        symbol_id: SymbolId,
        ty: Type,
        init: Option<TypedExpr>,
        span: Span,
    },
    Constraint {
        target: TypedExpr,
        value: TypedExpr,
        constraint_kind: ConstraintKind,
        span: Span,
    },
    For {
        var: IdentId,
        symbol_id: SymbolId,
        var_ty: Type,
        range: TypedExpr,
        body: Vec<TypedStmt>,
        span: Span,
    },
    With {
        view: TypedExpr,
        view_ty: Type,
        body: Vec<TypedStmt>,
        context_id: ViewContextId,
        span: Span,
    },
    Expr(TypedExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedExpr {
    pub expr: ExprKind,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Literal {
        kind: LiteralKind,
    },
    Ident {
        name: IdentId,
        symbol_id: SymbolId,
    },
    BinaryOp {
        op: BinOp,
        left: Box<TypedExpr>,
        right: Box<TypedExpr>,
    },
    UnaryOp {
        op: UnaryOp,
        expr: Box<TypedExpr>,
    },
    Call {
        func: Box<TypedExpr>,
        func_id: Option<FunctionId>,
        args: Vec<TypedExpr>,
    },
    FieldAccess {
        base: Box<TypedExpr>,
        field: IdentId,
        field_symbol_id: Option<SymbolId>,
    },
    ArrayIndex {
        array: Box<TypedExpr>,
        index: Box<TypedExpr>,
    },
    StructLiteral {
        struct_id: StructId,
        fields: Vec<(IdentId, TypedExpr)>,
    },
    ArrayLiteral {
        elements: Vec<TypedExpr>,
    },
    Range {
        start: Box<TypedExpr>,
        end: Box<TypedExpr>,
    },
    Reference {
        expr: Box<TypedExpr>,
    },
    Dereference {
        expr: Box<TypedExpr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintKind {
    Equality,
    GreaterThan,
    LessThan,
    GreaterEqual,
    LessEqual,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    // Primitives
    Point,
    Length,
    Angle,
    Area,
    Bool,
    I32,
    F64,
    Real,
    Algebraic,

    // Compound
    Array {
        element_type: Box<Type>,
        size: usize,
    },
    Struct {
        struct_id: StructId,
    },
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
    },
    View,

    // Special
    Reference(Box<Type>),
    Unknown,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeTable {
    pub types: HashMap<TypeId, TypeInfo>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeInfo {
    pub name: String,
    pub kind: TypeKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    Primitive,
    Struct { fields: Vec<(String, Type)> },
    Array { element_type: Type, size: usize },
}

impl TypedStmt {
    pub fn span(&self) -> Span {
        match self {
            TypedStmt::Let { span, .. } => *span,
            TypedStmt::Constraint { span, .. } => *span,
            TypedStmt::For { span, .. } => *span,
            TypedStmt::With { span, .. } => *span,
            TypedStmt::Expr(expr) => expr.span,
        }
    }
}

impl Type {
    pub fn is_entity_type(&self) -> bool {
        matches!(
            self,
            Type::Point | Type::Length | Type::Angle | Type::Area | Type::Struct { .. }
        )
    }

    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            Type::Length
                | Type::Angle
                | Type::Area
                | Type::I32
                | Type::F64
                | Type::Real
                | Type::Algebraic
        )
    }
}
