use crate::ast::expr::Expr;
use crate::ast::span::HasSpan;
use crate::lexer::Span;

// ============================================================================
// Unit Type Expressions
// ============================================================================

/// A unit type expression used as the parameter to `Real<...>`.
///
/// # Examples
/// - `Real<m>` → `UnitTypeExpr::Name { name: "m" }`
/// - `Real<m/s>` → `UnitTypeExpr::Div { ... }`
/// - `Real<m^2>` → `UnitTypeExpr::Pow { ... }`
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitTypeExpr<'src> {
    /// A named unit (base or previously declared derived unit), e.g. `m`, `mm`, `deg`
    Name { name: &'src str, span: Span },
    /// Product of two unit expressions, e.g. `m*s`
    Mul {
        lhs: Box<UnitTypeExpr<'src>>,
        rhs: Box<UnitTypeExpr<'src>>,
        span: Span,
    },
    /// Quotient of two unit expressions, e.g. `m/s`
    Div {
        lhs: Box<UnitTypeExpr<'src>>,
        rhs: Box<UnitTypeExpr<'src>>,
        span: Span,
    },
    /// Unit raised to an integer power, e.g. `m^2`
    Pow {
        base: Box<UnitTypeExpr<'src>>,
        exp: i32,
        span: Span,
    },
}

impl<'src> HasSpan for UnitTypeExpr<'src> {
    fn span(&self) -> Span {
        match self {
            UnitTypeExpr::Name { span, .. } => *span,
            UnitTypeExpr::Mul { span, .. } => *span,
            UnitTypeExpr::Div { span, .. } => *span,
            UnitTypeExpr::Pow { span, .. } => *span,
        }
    }
}

/// A unit expression used on the right-hand side of a `unit name = <expr>;` declaration.
///
/// This is a superset of `UnitTypeExpr` that also allows numeric literal factors,
/// e.g. `unit inch = 0.0254 * m;`.
#[derive(Debug, Clone, PartialEq)]
pub enum UnitExpr<'src> {
    /// Numeric literal scale factor, e.g. `0.0254`
    Literal { value: f64, span: Span },
    /// A named unit (base or previously declared), e.g. `m`, `mm`
    Name { name: &'src str, span: Span },
    /// Product, e.g. `0.0254 * m`
    Mul {
        lhs: Box<UnitExpr<'src>>,
        rhs: Box<UnitExpr<'src>>,
        span: Span,
    },
    /// Quotient, e.g. `deg / 60.0`
    Div {
        lhs: Box<UnitExpr<'src>>,
        rhs: Box<UnitExpr<'src>>,
        span: Span,
    },
    /// Power with integer exponent
    Pow {
        base: Box<UnitExpr<'src>>,
        exp: i32,
        span: Span,
    },
}

impl<'src> HasSpan for UnitExpr<'src> {
    fn span(&self) -> Span {
        match self {
            UnitExpr::Literal { span, .. } => *span,
            UnitExpr::Name { span, .. } => *span,
            UnitExpr::Mul { span, .. } => *span,
            UnitExpr::Div { span, .. } => *span,
            UnitExpr::Pow { span, .. } => *span,
        }
    }
}

// ============================================================================
// Type Annotations
// ============================================================================

/// Type annotations for variable declarations and function parameters
#[derive(Debug, Clone, PartialEq)]
pub enum Type<'src> {
    /// Boolean type
    Bool { span: Span },
    /// 32-bit integer type
    I32 { span: Span },
    /// 64-bit floating point type
    F64 { span: Span },
    /// Mathematical real number, optionally with a unit parameter.
    /// `Real` (no unit) = dimensionless real.
    /// `Real<m>` = real value in meters.
    Real {
        unit: Option<Box<UnitTypeExpr<'src>>>,
        span: Span,
    },
    /// Algebraic number (roots of polynomials with integer coefficients)
    Algebraic { span: Span },
    /// Reference type (e.g., &Point)
    Reference { inner: Box<Type<'src>>, span: Span },
    /// User-defined type (e.g., Point, Circle)
    UserDefined { name: String, span: Span },
    /// Fixed-size array type (e.g., [i32; 5], [Point; 3])
    Array {
        element_type: Box<Type<'src>>,
        size: usize,
        span: Span,
    },
}

impl<'src> HasSpan for Type<'src> {
    fn span(&self) -> Span {
        match self {
            Type::Bool { span } => *span,
            Type::I32 { span } => *span,
            Type::F64 { span } => *span,
            Type::Real { span, .. } => *span,
            Type::Algebraic { span } => *span,
            Type::Reference { span, .. } => *span,
            Type::UserDefined { span, .. } => *span,
            Type::Array { span, .. } => *span,
        }
    }
}

// ============================================================================
// Function Parameters
// ============================================================================

/// Function parameter with name and type
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionParam<'src> {
    pub name: String,
    pub name_span: Span,
    pub type_annotation: Type<'src>,
    pub span: Span,
}

impl<'src> HasSpan for FunctionParam<'src> {
    fn span(&self) -> Span {
        self.span
    }
}

// ============================================================================
// Struct Field
// ============================================================================

/// Struct field with name and type
#[derive(Debug, Clone, PartialEq)]
pub struct StructField<'src> {
    pub name: String,
    pub name_span: Span,
    pub type_annotation: Type<'src>,
    pub span: Span,
}

impl<'src> HasSpan for StructField<'src> {
    fn span(&self) -> Span {
        self.span
    }
}

// ============================================================================
// Statements
// ============================================================================

/// Statements perform declarations and actions (not expressions)
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt<'src> {
    /// Variable declaration with optional type annotation and initialization
    /// Examples:
    ///   let x: i32 = 42;
    ///   let y: bool;
    ///   let z = 3.14;
    ///   let container.field: Point = point(0mm, 0mm);
    ///   let sketch.entities.p1: Point = point(10mm, 10mm);
    ///   let .field: Point = point(0mm, 0mm);  // Dot prefix (in with blocks)
    Let {
        /// Whether this let statement has a dot prefix (e.g., `let .field = value;`)
        /// Dot prefix indicates the entity should be stored in the container from
        /// the enclosing `with` statement.
        dot_prefix: bool,
        /// Path segments for the variable name
        /// - Simple let: `let x` -> vec![("x", span)]
        /// - Container field: `let container.field` -> vec![("container", span1), ("field", span2)]
        /// - Nested: `let a.b.c` -> vec![("a", span1), ("b", span2), ("c", span3)]
        /// - Dot prefix: `let .field` -> vec![("field", span)]
        name_path: Vec<(&'src str, Span)>,
        type_annotation: Option<Type<'src>>,
        init: Option<Expr<'src>>,
        span: Span,
    },

    /// Assignment statement (creates a constraint)
    /// Examples:
    ///   x = 42;
    ///   width = 100;
    ///   result = a + b;
    /// Note: This is for simple variable assignment only.
    /// Field assignment uses Stmt::FieldAssignment.
    Assignment {
        /// Variable name being assigned to
        name: &'src str,
        /// Span of the variable name
        name_span: Span,
        /// Value expression
        value: Expr<'src>,
        /// Overall span of the statement
        span: Span,
    },

    /// Field assignment statement (assigns to object fields)
    /// Examples:
    ///   obj.field = 42;
    ///   sketch.origin.x = 10mm;
    ///   container.entities.p1.x = 5;
    ///   .field = 42;  // Dot prefix (in with blocks)
    /// Note: Without dot prefix, the path must have at least 2 segments (object.field).
    FieldAssignment {
        /// Whether this field assignment has a dot prefix (e.g., `.field = value;`)
        /// Dot prefix indicates the field is on the container from the enclosing `with` statement.
        dot_prefix: bool,
        /// Path to the field being assigned
        /// - obj.field -> vec![("obj", span1), ("field", span2)]
        /// - obj.nested.field -> vec![("obj", span1), ("nested", span2), ("field", span3)]
        /// - .field -> vec![("field", span)]
        field_path: Vec<(&'src str, Span)>,
        /// Value expression
        value: Expr<'src>,
        /// Overall span of the statement
        span: Span,
    },

    /// For loop over ranges or arrays
    /// Examples:
    ///   for i in 0..10 { ... }
    ///   for elem in array { ... }
    For {
        loop_var: &'src str,
        loop_var_span: Span,
        iterator: Expr<'src>,
        body: Vec<Stmt<'src>>,
        span: Span,
    },

    /// Function definition with parameters, return type, and body
    /// Examples:
    ///   fn distance(p1: &Point, p2: &Point) -> Length { ... }
    ///   fn area() -> f64 { self.width * self.height }
    FunctionDef {
        name: String,
        name_span: Span,
        params: Vec<FunctionParam<'src>>,
        return_type: Type<'src>,
        body: Vec<Stmt<'src>>,
        return_expr: Option<Expr<'src>>,
        span: Span,
    },

    /// Struct definition with fields and methods
    /// Examples:
    ///   struct Point { x: f64, y: f64 }
    ///   struct Circle { center: Point, radius: f64, fn area() -> f64 { ... } }
    ///   struct Sketch { container entities, origin: Point }
    StructDef {
        name: String,
        name_span: Span,
        /// Optional container field name
        container: Option<(String, Span)>,
        fields: Vec<StructField<'src>>,
        methods: Vec<Stmt<'src>>,
        span: Span,
    },

    /// Return statement with optional expression
    /// Examples:
    ///   return;
    ///   return value;
    ///   return a + b;
    Return {
        value: Option<Expr<'src>>,
        span: Span,
    },

    /// Expression statement (expression followed by semicolon)
    /// Examples:
    ///   foo();
    ///   print(x);
    ///   obj.method();
    ///   1 + 2;
    Expression { expr: Expr<'src>, span: Span },

    /// Block statement (sequence of statements in curly braces)
    /// Examples:
    ///   { }
    ///   { let x = 1; }
    ///   { let x = 1; let y = 2; }
    ///   { { let x = 1; } { let y = 2; } }
    Block {
        statements: Vec<Stmt<'src>>,
        span: Span,
    },

    /// With statement (apply transform or container context)
    /// Examples:
    ///   with transform { ... }
    ///   with sketch { let .p1: Point = point(0mm, 0mm); }
    ///   with translate { let p: Point = point(10mm, 10mm); }
    With {
        context_expr: Expr<'src>,
        body: Vec<Stmt<'src>>,
        span: Span,
    },

    /// If statement with optional else clause
    /// Examples:
    ///   if x > 0 { ... }
    ///   if condition { ... } else { ... }
    ///   if x > 0 { ... } else { if x < 0 { ... } else { ... } }
    ///
    /// Note: Else-if chains are supported by nesting if statements in the else branch.
    If {
        condition: Expr<'src>,
        then_branch: Vec<Stmt<'src>>,
        else_branch: Option<Vec<Stmt<'src>>>,
        span: Span,
    },

    /// Optimize block with minimize/maximize directives
    /// Examples:
    ///   optimize { minimize x; }
    ///   optimize { maximize area; minimize perimeter; }
    ///
    /// Directives are applied in lexicographic priority order.
    /// Only valid at the top level of a program.
    Optimize {
        directives: Vec<OptimizeDirective<'src>>,
        span: Span,
    },

    /// Base unit declaration: `unit <name>;`
    /// Introduces a new fundamental dimension.
    UnitDecl {
        name: &'src str,
        name_span: Span,
        span: Span,
    },

    /// Derived unit declaration: `unit <name> = <expr>;`
    /// Defines a unit in terms of existing units.
    UnitDef {
        name: &'src str,
        name_span: Span,
        definition: UnitExpr<'src>,
        span: Span,
    },

    /// Unit prefix declaration: `unit_prefix <name> = <factor>;`
    /// Defines a multiplicative prefix (e.g., `unit_prefix m = 1e-3;` for milli).
    UnitPrefixDecl {
        prefix: &'src str,
        prefix_span: Span,
        factor: f64,
        span: Span,
    },

    /// Include directive: `include "<path>";`
    /// Loads and splices another .cad file into the current program.
    Include { path: &'src str, span: Span },
}

/// A single directive inside an optimize block
#[derive(Debug, Clone, PartialEq)]
pub struct OptimizeDirective<'src> {
    pub kind: OptimizeDirectiveKind,
    pub expr: Expr<'src>,
    pub span: Span,
}

/// Whether the directive minimizes or maximizes the expression
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptimizeDirectiveKind {
    Minimize,
    Maximize,
}

impl<'src> HasSpan for Stmt<'src> {
    fn span(&self) -> Span {
        match self {
            Stmt::Let { span, .. } => *span,
            Stmt::Assignment { span, .. } => *span,
            Stmt::FieldAssignment { span, .. } => *span,
            Stmt::For { span, .. } => *span,
            Stmt::FunctionDef { span, .. } => *span,
            Stmt::StructDef { span, .. } => *span,
            Stmt::Return { span, .. } => *span,
            Stmt::Expression { span, .. } => *span,
            Stmt::Block { span, .. } => *span,
            Stmt::With { span, .. } => *span,
            Stmt::If { span, .. } => *span,
            Stmt::Optimize { span, .. } => *span,
            Stmt::UnitDecl { span, .. } => *span,
            Stmt::UnitDef { span, .. } => *span,
            Stmt::UnitPrefixDecl { span, .. } => *span,
            Stmt::Include { span, .. } => *span,
        }
    }
}
