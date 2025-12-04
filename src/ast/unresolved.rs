use crate::ident::IdentId;
use crate::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct UnresolvedAst {
    pub sketches: Vec<SketchDef>,
    pub structs: Vec<StructDef>,
    pub imports: Vec<ImportDef>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportDef {
    pub path: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SketchDef {
    pub name: IdentId,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDef {
    pub name: IdentId,
    pub fields: Vec<FieldDef>,
    pub methods: Vec<FunctionDef>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldDef {
    pub name: IdentId,
    pub ty: TypeRef,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDef {
    pub name: IdentId,
    pub params: Vec<ParamDef>,
    pub return_type: Option<TypeRef>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParamDef {
    pub name: IdentId,
    pub ty: TypeRef,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let {
        name: IdentId,
        ty: Option<TypeRef>,
        init: Option<Expr>,
        span: Span,
    },
    Assign {
        target: Expr,
        value: Expr,
        span: Span,
    },
    For {
        var: IdentId,
        range: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    With {
        view: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal {
        kind: LiteralKind,
        span: Span,
    },
    Ident {
        name: IdentId,
        span: Span,
    },
    BinaryOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    UnaryOp {
        op: UnaryOp,
        expr: Box<Expr>,
        span: Span,
    },
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    FieldAccess {
        base: Box<Expr>,
        field: IdentId,
        span: Span,
    },
    ArrayIndex {
        array: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    StructLiteral {
        ty: TypeRef,
        fields: Vec<(IdentId, Expr)>,
        span: Span,
    },
    ArrayLiteral {
        elements: Vec<Expr>,
        span: Span,
    },
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        span: Span,
    },
    Reference {
        expr: Box<Expr>,
        span: Span,
    },
    Dereference {
        expr: Box<Expr>,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum LiteralKind {
    Int(i64),
    Float(f64),
    Bool(bool),
    Length { value: f64, unit: LengthUnit },
    Angle { value: f64, unit: AngleUnit },
}

#[derive(Debug, Clone, PartialEq)]
pub enum LengthUnit {
    Millimeter,
    Centimeter,
    Meter,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AngleUnit {
    Degree,
    Radian,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Mod,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeRef {
    pub name: IdentId,
    pub is_reference: bool,
    pub array_size: Option<Box<Expr>>,
    pub span: Span,
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::Let { span, .. } => *span,
            Stmt::Assign { span, .. } => *span,
            Stmt::For { span, .. } => *span,
            Stmt::With { span, .. } => *span,
            Stmt::Expr(expr) => expr.span(),
        }
    }
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Literal { span, .. } => *span,
            Expr::Ident { span, .. } => *span,
            Expr::BinaryOp { span, .. } => *span,
            Expr::UnaryOp { span, .. } => *span,
            Expr::Call { span, .. } => *span,
            Expr::FieldAccess { span, .. } => *span,
            Expr::ArrayIndex { span, .. } => *span,
            Expr::StructLiteral { span, .. } => *span,
            Expr::ArrayLiteral { span, .. } => *span,
            Expr::Range { span, .. } => *span,
            Expr::Reference { span, .. } => *span,
            Expr::Dereference { span, .. } => *span,
        }
    }
}
