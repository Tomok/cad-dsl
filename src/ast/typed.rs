use crate::ast::resolved::*;
use crate::ast::unresolved::LiteralKind;
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
    LogicalOr(TypedLogicalOrExpr),
}

// Type aliases for clarity
pub type TypedPowerLevel = TypedPowerExpr;
pub type TypedMultiplicationLevel = TypedMultiplicationExpr;
pub type TypedAdditionLevel = TypedAdditionExpr;
pub type TypedComparisonLevel = TypedComparisonExpr;
pub type TypedLogicalAndLevel = TypedLogicalAndExpr;

#[derive(Debug, Clone, PartialEq)]
pub enum TypedLogicalOrExpr {
    LogicalOr {
        left: Box<TypedLogicalAndLevel>,
        right: Box<TypedLogicalAndLevel>,
    },
    LogicalAnd(TypedLogicalAndExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedLogicalAndExpr {
    LogicalAnd {
        left: Box<TypedComparisonLevel>,
        right: Box<TypedComparisonLevel>,
    },
    Comparison(TypedComparisonExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedComparisonExpr {
    Equal {
        left: Box<TypedAdditionLevel>,
        right: Box<TypedAdditionLevel>,
    },
    NotEqual {
        left: Box<TypedAdditionLevel>,
        right: Box<TypedAdditionLevel>,
    },
    LessThan {
        left: Box<TypedAdditionLevel>,
        right: Box<TypedAdditionLevel>,
    },
    GreaterThan {
        left: Box<TypedAdditionLevel>,
        right: Box<TypedAdditionLevel>,
    },
    LessEqual {
        left: Box<TypedAdditionLevel>,
        right: Box<TypedAdditionLevel>,
    },
    GreaterEqual {
        left: Box<TypedAdditionLevel>,
        right: Box<TypedAdditionLevel>,
    },
    Addition(TypedAdditionExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedAdditionExpr {
    Add {
        left: Box<TypedMultiplicationLevel>,
        right: Box<TypedMultiplicationLevel>,
    },
    Subtract {
        left: Box<TypedMultiplicationLevel>,
        right: Box<TypedMultiplicationLevel>,
    },
    Multiplication(TypedMultiplicationExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedMultiplicationExpr {
    Multiply {
        left: Box<TypedMultiplicationLevel>,
        right: Box<TypedPowerLevel>,
    },
    Divide {
        left: Box<TypedMultiplicationLevel>,
        right: Box<TypedPowerLevel>,
    },
    Modulo {
        left: Box<TypedMultiplicationLevel>,
        right: Box<TypedPowerLevel>,
    },
    Power(TypedPowerExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedPowerExpr {
    Power {
        left: Box<TypedUnaryExpr>,
        right: Box<TypedPowerLevel>,
    },
    Unary(TypedUnaryExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedUnaryExpr {
    Negation { expr: Box<TypedUnaryExpr> },
    Not { expr: Box<TypedUnaryExpr> },
    Reference { expr: Box<TypedUnaryExpr> },
    Dereference { expr: Box<TypedUnaryExpr> },
    Primary(TypedPrimaryExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedPrimaryExpr {
    Literal {
        kind: LiteralKind,
    },
    Ident {
        name: IdentId,
        symbol_id: SymbolId,
    },
    Call {
        func: Box<TypedPrimaryExpr>,
        func_id: Option<FunctionId>,
        args: Vec<TypedExpr>,
    },
    FieldAccess {
        base: Box<TypedPrimaryExpr>,
        field: IdentId,
        field_symbol_id: Option<SymbolId>,
    },
    ArrayIndex {
        array: Box<TypedPrimaryExpr>,
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
    Parenthesized(Box<TypedExpr>),
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
