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
    Variable {
        type_ref: ResolvedTypeRef,
    },
    Function {
        params: Vec<ResolvedTypeRef>,
        return_type: Option<ResolvedTypeRef>,
        function_id: FunctionId,
    },
    Struct {
        fields: Vec<FieldDef>,
        struct_id: StructId,
    },
    Parameter {
        type_ref: ResolvedTypeRef,
    },
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
    LogicalOr(ResolvedLogicalOrExpr),
}

// Type aliases for clarity
pub type ResolvedPowerLevel = ResolvedPowerExpr;
pub type ResolvedMultiplicationLevel = ResolvedMultiplicationExpr;
pub type ResolvedAdditionLevel = ResolvedAdditionExpr;
pub type ResolvedComparisonLevel = ResolvedComparisonExpr;
pub type ResolvedLogicalAndLevel = ResolvedLogicalAndExpr;

#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedLogicalOrExpr {
    LogicalOr {
        left: Box<ResolvedLogicalAndLevel>,
        right: Box<ResolvedLogicalAndLevel>,
        span: Span,
    },
    LogicalAnd(ResolvedLogicalAndExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedLogicalAndExpr {
    LogicalAnd {
        left: Box<ResolvedComparisonLevel>,
        right: Box<ResolvedComparisonLevel>,
        span: Span,
    },
    Comparison(ResolvedComparisonExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedComparisonExpr {
    Equal {
        left: Box<ResolvedAdditionLevel>,
        right: Box<ResolvedAdditionLevel>,
        span: Span,
    },
    NotEqual {
        left: Box<ResolvedAdditionLevel>,
        right: Box<ResolvedAdditionLevel>,
        span: Span,
    },
    LessThan {
        left: Box<ResolvedAdditionLevel>,
        right: Box<ResolvedAdditionLevel>,
        span: Span,
    },
    GreaterThan {
        left: Box<ResolvedAdditionLevel>,
        right: Box<ResolvedAdditionLevel>,
        span: Span,
    },
    LessEqual {
        left: Box<ResolvedAdditionLevel>,
        right: Box<ResolvedAdditionLevel>,
        span: Span,
    },
    GreaterEqual {
        left: Box<ResolvedAdditionLevel>,
        right: Box<ResolvedAdditionLevel>,
        span: Span,
    },
    Addition(ResolvedAdditionExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedAdditionExpr {
    Add {
        left: Box<ResolvedMultiplicationLevel>,
        right: Box<ResolvedMultiplicationLevel>,
        span: Span,
    },
    Subtract {
        left: Box<ResolvedMultiplicationLevel>,
        right: Box<ResolvedMultiplicationLevel>,
        span: Span,
    },
    Multiplication(ResolvedMultiplicationExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedMultiplicationExpr {
    Multiply {
        left: Box<ResolvedMultiplicationLevel>,
        right: Box<ResolvedPowerLevel>,
        span: Span,
    },
    Divide {
        left: Box<ResolvedMultiplicationLevel>,
        right: Box<ResolvedPowerLevel>,
        span: Span,
    },
    Modulo {
        left: Box<ResolvedMultiplicationLevel>,
        right: Box<ResolvedPowerLevel>,
        span: Span,
    },
    Power(ResolvedPowerExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedPowerExpr {
    Power {
        left: Box<ResolvedUnaryExpr>,
        right: Box<ResolvedPowerLevel>,
        span: Span,
    },
    Unary(ResolvedUnaryExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedUnaryExpr {
    Negation {
        expr: Box<ResolvedUnaryExpr>,
        span: Span,
    },
    Not {
        expr: Box<ResolvedUnaryExpr>,
        span: Span,
    },
    Reference {
        expr: Box<ResolvedUnaryExpr>,
        span: Span,
    },
    Dereference {
        expr: Box<ResolvedUnaryExpr>,
        span: Span,
    },
    Primary(ResolvedPrimaryExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedPrimaryExpr {
    Literal {
        kind: LiteralKind,
        span: Span,
    },
    Ident {
        name: IdentId,
        symbol_id: SymbolId,
        span: Span,
    },
    Call {
        func: Box<ResolvedPrimaryExpr>,
        func_id: Option<FunctionId>,
        args: Vec<ResolvedExpr>,
        span: Span,
    },
    FieldAccess {
        base: Box<ResolvedPrimaryExpr>,
        field: IdentId,
        field_symbol_id: Option<SymbolId>,
        span: Span,
    },
    ArrayIndex {
        array: Box<ResolvedPrimaryExpr>,
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
    Parenthesized(Box<ResolvedExpr>),
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
            ResolvedExpr::LogicalOr(expr) => expr.span(),
        }
    }
}

impl ResolvedLogicalOrExpr {
    pub fn span(&self) -> Span {
        match self {
            ResolvedLogicalOrExpr::LogicalOr { span, .. } => *span,
            ResolvedLogicalOrExpr::LogicalAnd(expr) => expr.span(),
        }
    }
}

impl ResolvedLogicalAndExpr {
    pub fn span(&self) -> Span {
        match self {
            ResolvedLogicalAndExpr::LogicalAnd { span, .. } => *span,
            ResolvedLogicalAndExpr::Comparison(expr) => expr.span(),
        }
    }
}

impl ResolvedComparisonExpr {
    pub fn span(&self) -> Span {
        match self {
            ResolvedComparisonExpr::Equal { span, .. } => *span,
            ResolvedComparisonExpr::NotEqual { span, .. } => *span,
            ResolvedComparisonExpr::LessThan { span, .. } => *span,
            ResolvedComparisonExpr::GreaterThan { span, .. } => *span,
            ResolvedComparisonExpr::LessEqual { span, .. } => *span,
            ResolvedComparisonExpr::GreaterEqual { span, .. } => *span,
            ResolvedComparisonExpr::Addition(expr) => expr.span(),
        }
    }
}

impl ResolvedAdditionExpr {
    pub fn span(&self) -> Span {
        match self {
            ResolvedAdditionExpr::Add { span, .. } => *span,
            ResolvedAdditionExpr::Subtract { span, .. } => *span,
            ResolvedAdditionExpr::Multiplication(expr) => expr.span(),
        }
    }
}

impl ResolvedMultiplicationExpr {
    pub fn span(&self) -> Span {
        match self {
            ResolvedMultiplicationExpr::Multiply { span, .. } => *span,
            ResolvedMultiplicationExpr::Divide { span, .. } => *span,
            ResolvedMultiplicationExpr::Modulo { span, .. } => *span,
            ResolvedMultiplicationExpr::Power(expr) => expr.span(),
        }
    }
}

impl ResolvedPowerExpr {
    pub fn span(&self) -> Span {
        match self {
            ResolvedPowerExpr::Power { span, .. } => *span,
            ResolvedPowerExpr::Unary(expr) => expr.span(),
        }
    }
}

impl ResolvedUnaryExpr {
    pub fn span(&self) -> Span {
        match self {
            ResolvedUnaryExpr::Negation { span, .. } => *span,
            ResolvedUnaryExpr::Not { span, .. } => *span,
            ResolvedUnaryExpr::Reference { span, .. } => *span,
            ResolvedUnaryExpr::Dereference { span, .. } => *span,
            ResolvedUnaryExpr::Primary(expr) => expr.span(),
        }
    }
}

impl ResolvedPrimaryExpr {
    pub fn span(&self) -> Span {
        match self {
            ResolvedPrimaryExpr::Literal { span, .. } => *span,
            ResolvedPrimaryExpr::Ident { span, .. } => *span,
            ResolvedPrimaryExpr::Call { span, .. } => *span,
            ResolvedPrimaryExpr::FieldAccess { span, .. } => *span,
            ResolvedPrimaryExpr::ArrayIndex { span, .. } => *span,
            ResolvedPrimaryExpr::StructLiteral { span, .. } => *span,
            ResolvedPrimaryExpr::ArrayLiteral { span, .. } => *span,
            ResolvedPrimaryExpr::Range { span, .. } => *span,
            ResolvedPrimaryExpr::Parenthesized(expr) => expr.span(),
        }
    }
}
