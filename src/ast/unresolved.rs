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
    pub functions: Vec<FunctionDef>,
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
    Return {
        value: Option<Expr>,
        span: Span,
    },
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    LogicalOr(LogicalOrExpr),
}

// Type aliases for clarity
pub type PowerLevel = PowerExpr;
pub type MultiplicationLevel = MultiplicationExpr;
pub type AdditionLevel = AdditionExpr;
pub type ComparisonLevel = ComparisonExpr;
pub type LogicalAndLevel = LogicalAndExpr;

#[derive(Debug, Clone, PartialEq)]
pub enum LogicalOrExpr {
    LogicalOr {
        left: Box<LogicalAndLevel>,
        right: Box<LogicalAndLevel>,
        span: Span,
    },
    LogicalAnd(LogicalAndExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum LogicalAndExpr {
    LogicalAnd {
        left: Box<LogicalAndLevel>,
        right: Box<ComparisonLevel>,
        span: Span,
    },
    LogicalOr(Box<LogicalOrExpr>),
    Comparison(ComparisonExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComparisonExpr {
    Equal {
        left: Box<AdditionLevel>,
        right: Box<AdditionLevel>,
        span: Span,
    },
    NotEqual {
        left: Box<AdditionLevel>,
        right: Box<AdditionLevel>,
        span: Span,
    },
    LessThan {
        left: Box<AdditionLevel>,
        right: Box<AdditionLevel>,
        span: Span,
    },
    GreaterThan {
        left: Box<AdditionLevel>,
        right: Box<AdditionLevel>,
        span: Span,
    },
    LessEqual {
        left: Box<AdditionLevel>,
        right: Box<AdditionLevel>,
        span: Span,
    },
    GreaterEqual {
        left: Box<AdditionLevel>,
        right: Box<AdditionLevel>,
        span: Span,
    },
    LogicalAnd(Box<LogicalAndExpr>),
    Addition(AdditionExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AdditionExpr {
    Add {
        left: Box<AdditionLevel>,
        right: Box<MultiplicationLevel>,
        span: Span,
    },
    Subtract {
        left: Box<AdditionLevel>,
        right: Box<MultiplicationLevel>,
        span: Span,
    },
    Multiplication(Box<MultiplicationExpr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum MultiplicationExpr {
    Multiply {
        left: Box<MultiplicationLevel>,
        right: Box<PowerLevel>,
        span: Span,
    },
    Divide {
        left: Box<MultiplicationLevel>,
        right: Box<PowerLevel>,
        span: Span,
    },
    Modulo {
        left: Box<MultiplicationLevel>,
        right: Box<PowerLevel>,
        span: Span,
    },
    Addition(Box<AdditionExpr>),
    Power(PowerExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PowerExpr {
    Power {
        left: Box<UnaryExpr>,
        right: Box<PowerLevel>,
        span: Span,
    },
    Unary(UnaryExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryExpr {
    Negation { expr: Box<UnaryExpr>, span: Span },
    Not { expr: Box<UnaryExpr>, span: Span },
    Reference { expr: Box<UnaryExpr>, span: Span },
    Dereference { expr: Box<UnaryExpr>, span: Span },
    Primary(PrimaryExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PrimaryExpr {
    Literal {
        kind: LiteralKind,
        span: Span,
    },
    Ident {
        name: IdentId,
        span: Span,
    },
    Call {
        func: Box<PrimaryExpr>,
        args: Vec<Expr>,
        span: Span,
    },
    FieldAccess {
        base: Box<PrimaryExpr>,
        field: IdentId,
        span: Span,
    },
    ArrayIndex {
        array: Box<PrimaryExpr>,
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
    Parenthesized(Box<Expr>),
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
            Stmt::Return { span, .. } => *span,
            Stmt::Expr(expr) => expr.span(),
        }
    }
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::LogicalOr(expr) => expr.span(),
        }
    }
}

impl LogicalOrExpr {
    pub fn span(&self) -> Span {
        match self {
            LogicalOrExpr::LogicalOr { span, .. } => *span,
            LogicalOrExpr::LogicalAnd(expr) => expr.span(),
        }
    }
}

impl LogicalAndExpr {
    pub fn span(&self) -> Span {
        match self {
            LogicalAndExpr::LogicalAnd { span, .. } => *span,
            LogicalAndExpr::LogicalOr(expr) => expr.span(),
            LogicalAndExpr::Comparison(expr) => expr.span(),
        }
    }
}

impl ComparisonExpr {
    pub fn span(&self) -> Span {
        match self {
            ComparisonExpr::Equal { span, .. } => *span,
            ComparisonExpr::NotEqual { span, .. } => *span,
            ComparisonExpr::LessThan { span, .. } => *span,
            ComparisonExpr::GreaterThan { span, .. } => *span,
            ComparisonExpr::LessEqual { span, .. } => *span,
            ComparisonExpr::GreaterEqual { span, .. } => *span,
            ComparisonExpr::LogicalAnd(expr) => expr.span(),
            ComparisonExpr::Addition(expr) => expr.span(),
        }
    }
}

impl AdditionExpr {
    pub fn span(&self) -> Span {
        match self {
            AdditionExpr::Add { span, .. } => *span,
            AdditionExpr::Subtract { span, .. } => *span,
            AdditionExpr::Multiplication(expr) => expr.span(),
        }
    }
}

impl MultiplicationExpr {
    pub fn span(&self) -> Span {
        match self {
            MultiplicationExpr::Multiply { span, .. } => *span,
            MultiplicationExpr::Divide { span, .. } => *span,
            MultiplicationExpr::Modulo { span, .. } => *span,
            MultiplicationExpr::Addition(expr) => expr.span(),
            MultiplicationExpr::Power(expr) => expr.span(),
        }
    }
}

impl PowerExpr {
    pub fn span(&self) -> Span {
        match self {
            PowerExpr::Power { span, .. } => *span,
            PowerExpr::Unary(expr) => expr.span(),
        }
    }
}

impl UnaryExpr {
    pub fn span(&self) -> Span {
        match self {
            UnaryExpr::Negation { span, .. } => *span,
            UnaryExpr::Not { span, .. } => *span,
            UnaryExpr::Reference { span, .. } => *span,
            UnaryExpr::Dereference { span, .. } => *span,
            UnaryExpr::Primary(expr) => expr.span(),
        }
    }
}

impl PrimaryExpr {
    pub fn span(&self) -> Span {
        match self {
            PrimaryExpr::Literal { span, .. } => *span,
            PrimaryExpr::Ident { span, .. } => *span,
            PrimaryExpr::Call { span, .. } => *span,
            PrimaryExpr::FieldAccess { span, .. } => *span,
            PrimaryExpr::ArrayIndex { span, .. } => *span,
            PrimaryExpr::StructLiteral { span, .. } => *span,
            PrimaryExpr::ArrayLiteral { span, .. } => *span,
            PrimaryExpr::Range { span, .. } => *span,
            PrimaryExpr::Parenthesized(expr) => expr.span(),
        }
    }
}

// Type conversion traits for hierarchical AST
impl From<PrimaryExpr> for UnaryExpr {
    fn from(primary: PrimaryExpr) -> Self {
        UnaryExpr::Primary(primary)
    }
}

impl From<UnaryExpr> for PowerExpr {
    fn from(unary: UnaryExpr) -> Self {
        PowerExpr::Unary(unary)
    }
}

impl From<PowerExpr> for MultiplicationExpr {
    fn from(power: PowerExpr) -> Self {
        MultiplicationExpr::Power(power)
    }
}

impl From<MultiplicationExpr> for AdditionExpr {
    fn from(mult: MultiplicationExpr) -> Self {
        AdditionExpr::Multiplication(Box::new(mult))
    }
}

impl From<AdditionExpr> for ComparisonExpr {
    fn from(add: AdditionExpr) -> Self {
        ComparisonExpr::Addition(add)
    }
}

impl From<ComparisonExpr> for LogicalAndExpr {
    fn from(comp: ComparisonExpr) -> Self {
        LogicalAndExpr::Comparison(comp)
    }
}

impl From<LogicalAndExpr> for LogicalOrExpr {
    fn from(and: LogicalAndExpr) -> Self {
        LogicalOrExpr::LogicalAnd(and)
    }
}

impl From<LogicalOrExpr> for Expr {
    fn from(or: LogicalOrExpr) -> Self {
        Expr::LogicalOr(or)
    }
}

// Convenience conversions for common cases
impl From<PrimaryExpr> for PowerExpr {
    fn from(primary: PrimaryExpr) -> Self {
        UnaryExpr::Primary(primary).into()
    }
}

impl From<PrimaryExpr> for MultiplicationExpr {
    fn from(primary: PrimaryExpr) -> Self {
        MultiplicationExpr::Power(PowerExpr::from(primary))
    }
}

impl From<PrimaryExpr> for AdditionExpr {
    fn from(primary: PrimaryExpr) -> Self {
        AdditionExpr::Multiplication(Box::new(MultiplicationExpr::from(primary)))
    }
}

impl From<PrimaryExpr> for ComparisonExpr {
    fn from(primary: PrimaryExpr) -> Self {
        AdditionExpr::from(primary).into()
    }
}

impl From<PrimaryExpr> for LogicalAndExpr {
    fn from(primary: PrimaryExpr) -> Self {
        ComparisonExpr::from(primary).into()
    }
}

impl From<PrimaryExpr> for LogicalOrExpr {
    fn from(primary: PrimaryExpr) -> Self {
        LogicalAndExpr::from(primary).into()
    }
}

impl From<PrimaryExpr> for Expr {
    fn from(primary: PrimaryExpr) -> Self {
        LogicalOrExpr::from(primary).into()
    }
}

impl From<UnaryExpr> for MultiplicationExpr {
    fn from(unary: UnaryExpr) -> Self {
        PowerExpr::from(unary).into()
    }
}

impl From<UnaryExpr> for AdditionExpr {
    fn from(unary: UnaryExpr) -> Self {
        MultiplicationExpr::from(unary).into()
    }
}

impl From<UnaryExpr> for ComparisonExpr {
    fn from(unary: UnaryExpr) -> Self {
        AdditionExpr::from(unary).into()
    }
}

impl From<UnaryExpr> for LogicalAndExpr {
    fn from(unary: UnaryExpr) -> Self {
        ComparisonExpr::from(unary).into()
    }
}

impl From<UnaryExpr> for LogicalOrExpr {
    fn from(unary: UnaryExpr) -> Self {
        LogicalAndExpr::from(unary).into()
    }
}

impl From<UnaryExpr> for Expr {
    fn from(unary: UnaryExpr) -> Self {
        LogicalOrExpr::from(unary).into()
    }
}
