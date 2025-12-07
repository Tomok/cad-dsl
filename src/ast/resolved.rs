use crate::ast::unresolved::*;
use crate::ident::IdentId;
use crate::span::Span;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeId(pub usize);

pub type StructId = usize;
pub type FunctionId = usize;

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedAst {
    pub imports: Vec<ImportDef>,
    pub sketches: Vec<ResolvedSketchDef>,
    pub structs: Vec<ResolvedStructDef>,
    pub symbol_table: SymbolTable,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedSketchDef {
    pub name: IdentId,
    pub body: Vec<ResolvedStmt>,
    pub functions: Vec<ResolvedFunctionDef>,
    pub scope_id: ScopeId,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedStructDef {
    pub name: IdentId,
    pub fields: Vec<ResolvedFieldDef>,
    pub methods: Vec<ResolvedFunctionDef>,
    pub scope_id: ScopeId,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedFieldDef {
    pub name: IdentId,
    pub ty: ResolvedTypeRef,
    pub symbol_id: SymbolId,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedFunctionDef {
    pub name: IdentId,
    pub params: Vec<ResolvedParamDef>,
    pub return_type: Option<ResolvedTypeRef>,
    pub body: Vec<ResolvedStmt>,
    pub symbol_id: SymbolId,
    pub scope_id: ScopeId,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedParamDef {
    pub name: IdentId,
    pub ty: ResolvedTypeRef,
    pub symbol_id: SymbolId,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct SymbolTable {
    scopes: Vec<Scope>,
    symbols: Vec<Symbol>,
    next_symbol_id: usize,
    next_scope_id: usize,
}

#[derive(Debug, Clone)]
pub struct Scope {
    pub id: ScopeId,
    pub parent: Option<ScopeId>,
    pub symbols: HashMap<IdentId, SymbolId>,
    pub kind: ScopeKind,
}

#[derive(Debug, Clone)]
pub enum ScopeKind {
    Global,
    Sketch,
    Struct,
    Function,
    Block,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub id: SymbolId,
    pub name: IdentId,
    pub kind: SymbolKind,
    pub def_span: Span,
    pub scope_id: ScopeId,
}

#[derive(Debug, Clone)]
pub enum SymbolKind {
    Variable {
        type_ref: Option<ResolvedTypeRef>,
        is_mutable: bool,
    },
    Function {
        params: Vec<SymbolId>,
        return_type: Option<ResolvedTypeRef>,
    },
    Struct {
        fields: Vec<SymbolId>,
        methods: Vec<SymbolId>,
    },
    Field {
        type_ref: ResolvedTypeRef,
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
        scope_id: ScopeId,
        span: Span,
    },
    With {
        view: ResolvedExpr,
        body: Vec<ResolvedStmt>,
        scope_id: ScopeId,
        span: Span,
    },
    Return {
        value: Option<ResolvedExpr>,
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
        args: Vec<ResolvedExpr>,
        span: Span,
    },
    FieldAccess {
        base: Box<ResolvedPrimaryExpr>,
        field: IdentId,
        field_symbol_id: SymbolId,
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
    pub symbol_id: Option<SymbolId>, // For user-defined types
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
            ResolvedStmt::Return { span, .. } => *span,
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

impl SymbolTable {
    pub fn new() -> Self {
        let mut table = Self {
            scopes: Vec::new(),
            symbols: Vec::new(),
            next_symbol_id: 0,
            next_scope_id: 0,
        };

        // Create global scope
        table.create_scope(None, ScopeKind::Global);
        table
    }

    pub fn create_scope(&mut self, parent: Option<ScopeId>, kind: ScopeKind) -> ScopeId {
        let id = ScopeId(self.next_scope_id);
        self.next_scope_id += 1;

        let scope = Scope {
            id,
            parent,
            symbols: HashMap::new(),
            kind,
        };

        self.scopes.push(scope);
        id
    }

    pub fn create_symbol(
        &mut self,
        name: IdentId,
        kind: SymbolKind,
        def_span: Span,
        scope_id: ScopeId,
    ) -> Result<SymbolId, String> {
        // Check for duplicate in current scope
        // Note: Clippy suggests collapsing this if statement using `let ... && condition` syntax,
        // but that requires unstable Rust features (RFC #53667). We use nested ifs for stable compatibility.
        #[allow(clippy::collapsible_if)]
        if let Some(scope) = self.scopes.iter().find(|s| s.id == scope_id) {
            if scope.symbols.contains_key(&name) {
                return Err(format!("Symbol {:?} already defined in scope", name));
            }
        }

        let symbol_id = SymbolId(self.next_symbol_id);
        self.next_symbol_id += 1;

        let symbol = Symbol {
            id: symbol_id,
            name,
            kind,
            def_span,
            scope_id,
        };

        self.symbols.push(symbol);

        // Add to scope's symbol map
        if let Some(scope) = self.scopes.iter_mut().find(|s| s.id == scope_id) {
            scope.symbols.insert(name, symbol_id);
        }

        Ok(symbol_id)
    }

    pub fn lookup_symbol(&self, name: IdentId, scope_id: ScopeId) -> Option<SymbolId> {
        let mut current_scope = Some(scope_id);

        while let Some(scope_id) = current_scope {
            if let Some(scope) = self.scopes.iter().find(|s| s.id == scope_id) {
                if let Some(&symbol_id) = scope.symbols.get(&name) {
                    return Some(symbol_id);
                }
                current_scope = scope.parent;
            } else {
                break;
            }
        }

        None
    }

    pub fn get_symbol(&self, symbol_id: SymbolId) -> Option<&Symbol> {
        self.symbols.iter().find(|s| s.id == symbol_id)
    }

    pub fn get_scope(&self, scope_id: ScopeId) -> Option<&Scope> {
        self.scopes.iter().find(|s| s.id == scope_id)
    }

    pub fn global_scope_id(&self) -> ScopeId {
        ScopeId(0)
    }

    pub fn get_symbol_mut(&mut self, symbol_id: SymbolId) -> Option<&mut Symbol> {
        self.symbols.iter_mut().find(|s| s.id == symbol_id)
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for SymbolTable {
    fn eq(&self, other: &Self) -> bool {
        // For testing purposes, compare basic structure
        self.symbols.len() == other.symbols.len() && self.scopes.len() == other.scopes.len()
    }
}
