# TextCAD Parser Architecture

**Version:** 1.1  
**Date:** 2025-12-05  
**Status:** Implementation Status Review

## Current Implementation Status

✅ **Fully Implemented**
- Phase 1: Lexical Analysis (Lexer) - Complete with string interning
- Phase 2: Syntax Analysis (Parser) - All expression precedence and associativity working correctly
- Phase 2.1: Extended Parser Features - Reference expressions (`&variable` syntax) and function definitions in sketch context
- Identifier Arena (string interning)
- Span tracking and source location preservation
- Core AST data structures (unresolved, resolved, typed)

✅ **Fully Implemented**
- Phase 3: Name Resolution - Complete with symbol tables, scoping, and error reporting

✅ **Fully Implemented**
- Phase 4: Type Checking - Complete type checker with inference, validation, and error reporting

❌ **Not Yet Implemented**
- Phase 5: Constraint Collection
- Ariadne error reporting

**Test Status**: 
- Unit tests: 52/52 passing
- Integration tests: 47/47 passing  
- Total tests: 99 tests passing
- Total codebase: ~6,900+ lines of Rust code

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture Phases](#architecture-phases)
3. [Component Details](#component-details)
4. [Intermediate Representations](#intermediate-representations)
5. [Error Handling Strategy](#error-handling-strategy)
6. [Source Location Tracking](#source-location-tracking)
7. [Implementation Technologies](#implementation-technologies)
8. [Data Flow](#data-flow)

---

## Overview

The TextCAD parser transforms source text into a validated, semantically-analyzed intermediate representation (IR) ready for constraint solving. The architecture is structured as a pipeline of distinct phases, each operating on progressively higher-level representations while maintaining complete source location information for error reporting.

### Design Goals

1. **Comprehensive Error Reporting**: Collect and report multiple errors in a single pass
2. **Source Location Preservation**: Track precise file positions (line, column) throughout all phases
3. **Declarative Semantics**: Preserve the declarative nature of the language in the IR
4. **Extensibility**: Support future language features with minimal architectural changes
5. **Clear Separation of Concerns**: Distinct phases for lexing, parsing, name resolution, and type checking

### High-Level Pipeline

```
Source Text
    ↓
[Lexer] ✅ → Token Stream
    ↓
[Parser] ✅ → Unresolved AST
    ↓
[Name Resolution] ✅ → Resolved AST
    ↓
[Type Checking] ✅ → Typed IR
    ↓
[Constraint Collection] ❌ → Constraint System

Legend: ✅ Complete, ⚠️ Partial, ❌ Not implemented
```

---

## Architecture Phases

### Phase 1: Lexical Analysis (Lexer) ✅

**Purpose**: Transform source text into a stream of tokens with location information.

**Implementation**: `logos` crate for efficient lexing

**Input**: UTF-8 source text  
**Output**: `Vec<Token>` with span information

**Responsibilities**:
- ✅ Tokenize keywords, identifiers, operators, literals
- ✅ Handle numeric literals with units (e.g., `10mm`, `45deg`)
- ✅ Strip comments (single-line `//` and multi-line `/* */`)
- ✅ Track line and column numbers for each token
- ✅ Report lexical errors (invalid characters, malformed numbers)

**Key Data Structures**:
```rust
struct Token {
    kind: TokenKind,
    span: Span,
}

enum TokenKind {
    // Keywords
    Sketch, Struct, Fn, Let, For, In, With, View,
    If, Else, Return,
    
    // Identifiers and Literals
    Ident(IdentId),  // Interned identifier
    IntLiteral(i64),
    FloatLiteral(f64),
    BoolLiteral(bool),
    
    // Operators
    Plus, Minus, Star, Slash, Caret,
    Eq, NotEq, Lt, Gt, LtEq, GtEq,
    Assign, Arrow,
    
    // Delimiters
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,
    Comma, Semicolon, Colon, Dot, Ampersand,
    
    // Units
    Millimeter, Centimeter, Meter,
    Degree, Radian,
    
    // Special
    Eof,
}
```

### Phase 2: Syntax Analysis (Parser) ✅

**Purpose**: Build an Abstract Syntax Tree (AST) from tokens, validating syntactic structure.

**Implementation**: `chumsky` parser combinator library

**Input**: Token stream  
**Output**: Unresolved AST with syntax validation errors

**Responsibilities**:
- ✅ Parse sketch definitions, struct declarations, function definitions
- ✅ Build expression trees with proper precedence and associativity (all left-associative chains working)
- ✅ Parse control flow (for loops, with blocks)
- ✅ Handle nested structures (struct fields, array indexing, method calls)
- ✅ Collect syntax errors without stopping (error recovery)
- ✅ Preserve source spans for all AST nodes

**Parser Strategy**:
- Recursive descent parsing via combinators
- Pratt parsing for expressions with operator precedence
- Error recovery using chumsky's recovery strategies
- Backtracking for ambiguous constructs

**Key Combinators**:
```
sketch_parser := "sketch" >> ident >> "{" >> stmt* >> "}"

stmt_parser := 
    | let_stmt
    | assign_stmt
    | for_stmt
    | with_stmt
    | expr_stmt

let_stmt := "let" >> ident >> ":" >> type >> ("=" >> expr)? >> ";"

expr_parser := pratt_parser([
    infix("+", Assoc::Left),
    infix("*", Assoc::Left),
    postfix("()", call),
    postfix(".", field_access),
    ...
])
```

### Phase 3: Name Resolution ✅

**Purpose**: Resolve all identifiers to their declarations, building a symbol table.

**Input**: Unresolved AST  
**Output**: Resolved AST with symbol references

**Responsibilities**:
- ✅ Build symbol tables for each scope (sketch, struct, function, block)
- ✅ Resolve variable references to their `let` declarations
- ✅ Resolve type names to struct definitions
- ✅ Resolve function calls to function declarations
- ✅ Handle shadowing correctly
- ✅ Support forward references within scopes
- ✅ Report undefined name errors
- ✅ Report duplicate definition errors

**Algorithm**:

1. **Symbol Table Construction**:
   - Walk AST top-down
   - For each scope, create a new symbol table
   - Register all declarations (`let`, `struct`, `fn`) in current scope
   - Handle lexical scoping with parent scope chain

2. **Reference Resolution**:
   - Walk AST again
   - For each identifier reference:
     - Search current scope and parent scopes
     - Link reference to declaration via `SymbolId`
     - Report error if not found
   - For each type reference:
     - Search type namespace
     - Link to struct definition
   - For each function call:
     - Search function namespace (including methods)
     - Link to function definition

3. **Forward Reference Handling**:
   - Within a single scope, allow references before declarations
   - Collect all declarations in first pass
   - Resolve references in second pass

**Key Data Structures**:
```rust
struct SymbolTable {
    symbols: HashMap<String, SymbolId>,
    parent: Option<SymbolTableId>,
}

struct SymbolId(usize);

struct Symbol {
    name: String,
    kind: SymbolKind,
    def_span: Span,
}

enum SymbolKind {
    Variable { type_ref: TypeId },
    Function { params: Vec<TypeId>, return_type: TypeId },
    Struct { fields: Vec<FieldDef> },
}
```

**Resolved AST Enhancement**:
Each identifier node gets annotated with its `SymbolId`:
```rust
struct ResolvedIdent {
    name: String,
    span: Span,
    symbol_id: SymbolId,  // Added during resolution
}
```

### Phase 4: Type Checking ✅

**Purpose**: Validate type correctness and annotate all expressions with types.

**Input**: Resolved AST  
**Output**: Typed IR with type annotations

**Responsibilities**:
- ✅ Infer types for expressions
- ✅ Check type compatibility for assignments and constraints
- ✅ Validate function argument types and signatures
- ✅ Check struct field initialization
- ✅ Validate array index types
- ✅ Handle reference vs. value types
- ✅ Report type mismatch errors
- ✅ Annotate all expressions with resolved types

**Type Checking Algorithm**:

1. **Type Inference**:
   - Bottom-up traversal of expression trees
   - Infer types from literals, variable declarations
   - Propagate types through operations
   - Handle generic types (e.g., arrays with size parameter)

2. **Type Unification**:
   - For constrained variables, unify declared type with inferred type
   - Check assignment compatibility
   - Ensure function arguments match parameter types

3. **Reference Checking**:
   - Validate that entity types are passed as references to functions
   - Check that reference returns are consistent
   - Ensure dereferencing is valid

**Type System Representation**:
```rust
enum Type {
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
    Array { element_type: Box<Type>, size: usize },
    Struct { struct_id: StructId },
    Function { params: Vec<Type>, return_type: Box<Type> },
    
    // Special
    Reference(Box<Type>),
    Unknown,  // For inference
    Error,    // For error recovery
}

struct TypedExpr {
    expr: Expr,
    ty: Type,
    span: Span,
}
```

### Phase 5: Constraint Collection ❌ (Future)

**Purpose**: Extract constraint equations from the typed IR for the solver.

**Note**: This phase is mentioned for completeness but may be implemented separately.

**Responsibilities**:
- ❌ Convert assignment statements to constraint equations
- ❌ Extract geometric constraints from function calls
- ❌ Handle view-based transformations
- ❌ Build constraint system for solver

---

## Component Details

### Identifier Arena ✅

**Module**: `ident.rs`

**Purpose**: Centralized string interning for all identifiers, reducing memory usage and enabling fast equality comparisons.

**Implementation**:
```rust
use string_interner::{StringInterner, Symbol};

pub type IdentId = Symbol;

pub struct IdentArena {
    interner: StringInterner,
}

impl IdentArena {
    pub fn new() -> Self {
        IdentArena {
            interner: StringInterner::default(),
        }
    }
    
    pub fn intern(&mut self, name: &str) -> IdentId {
        self.interner.get_or_intern(name)
    }
    
    pub fn resolve(&self, id: IdentId) -> &str {
        self.interner.resolve(id).unwrap()
    }
    
    pub fn get(&self, name: &str) -> Option<IdentId> {
        self.interner.get(name)
    }
}
```

**Usage Pattern**:
- Create a single `IdentArena` at the start of compilation
- Pass `&mut IdentArena` to lexer/parser for interning
- Store `IdentId` in AST nodes instead of `String`
- Pass `&IdentArena` to later phases for resolving names

**Benefits**:
- O(1) identifier equality comparison (just compare IDs)
- Reduced memory: Each unique identifier stored once
- Cache-friendly: IDs are small integers
- Thread-safe option available via `Arc<RwLock<IdentArena>>`

### Lexer Component ✅

**Module**: `lexer/scanner.rs`

**Key Functions**:
```rust
pub fn tokenize(
    source: &str,
    idents: &mut IdentArena
) -> Result<Vec<Token>, Vec<LexError>>

struct LexError {
    message: String,
    span: Span,
}
```

**Implementation Notes**:
- Use `logos` derive macro for token definitions
- Custom lexer for numeric literals with units
- Intern all identifiers via `idents.intern()` during tokenization
- Track newlines for line/column calculation
- Maintain a `Span` for each token

**Error Recovery**:
- Skip invalid characters and continue
- Collect all lexical errors
- Emit error tokens for recovery

### Parser Component ✅

**Module**: `parser/mod.rs`, `parser/recursive.rs`, `parser/unified.rs`, etc.

**Key Functions**:
```rust
pub fn parse(
    tokens: Vec<Token>,
    idents: &IdentArena
) -> (Option<UnresolvedAst>, Vec<ParseError>)

struct ParseError {
    message: String,
    span: Span,
    label: Option<String>,
}
```

**Implementation Strategy**:
- Use `chumsky::Parser` trait extensively
- Pass `idents` reference for resolving identifier names when needed
- Define parser combinators for each grammar rule
- Use `recover()` for error recovery
- Use `delimited_by()` for paired delimiters
- Use `separated_by()` for lists

**Error Recovery Strategies**:
- Skip to next statement boundary (`;`, `}`)
- Insert missing tokens when obvious
- Use recovery markers in grammar
- Continue parsing after errors to find more issues

**Example Parser Combinator** (pseudocode):
```rust
fn let_stmt_parser() -> impl Parser<Token, LetStmt> {
    keyword("let")
        .ignore_then(ident())
        .then(just(Token::Colon).ignore_then(type_expr()))
        .then(just(Token::Assign).ignore_then(expr()).or_not())
        .then_ignore(just(Token::Semicolon))
        .map(|((name, ty), init)| LetStmt { name, ty, init, span })
        .recover_with(skip_until([Token::Semicolon, Token::RBrace]))
}
```

### Name Resolution Component ✅

**Module**: `src/name_resolution.rs` (fully implemented)

**Key Functions**:
```rust
pub fn resolve_names(
    ast: UnresolvedAst,
    idents: &IdentArena
) -> (ResolvedAst, Vec<ResolutionError>)

struct ResolutionError {
    kind: ResolutionErrorKind,
    span: Span,
}

enum ResolutionErrorKind {
    UndefinedName(IdentId),
    DuplicateDefinition {
        name: IdentId,
        first_def: Span,
    },
    InvalidReference(IdentId),
}
```

**Algorithm**:

1. **First Pass - Declaration Collection**:
```pseudocode
for each sketch:
    create sketch_scope
    for each struct:
        register struct in sketch_scope (by IdentId)
        create struct_scope for fields/methods
        for each field:
            register field in struct_scope (by IdentId)
        for each method:
            register method in struct_scope (by IdentId)
    for each top-level function:
        register function in sketch_scope (by IdentId)
    for each statement in sketch body:
        collect_declarations(statement, sketch_scope)

collect_declarations(stmt, scope):
    if stmt is let_stmt:
        register variable in scope (by IdentId)
    if stmt is for_loop:
        create loop_scope as child of scope
        collect_declarations(loop.body, loop_scope)
    if stmt is with_block:
        create block_scope as child of scope
        collect_declarations(block.body, block_scope)
```

2. **Second Pass - Reference Resolution**:
```pseudocode
resolve_references(ast, scope_tree, idents):
    for each identifier reference (IdentId):
        symbol = lookup_symbol(identifier, current_scope)
        if symbol is None:
            report UndefinedName error with idents.resolve(identifier)
        else:
            identifier.symbol_id = symbol.id
    
    for each type reference (IdentId):
        type_def = lookup_type(type_ref, current_scope)
        if type_def is None:
            report UndefinedType error
        else:
            type_ref.struct_id = type_def.id
    
    recurse into child expressions/statements

lookup_symbol(ident_id: IdentId, scope):
    while scope is not None:
        if scope.symbols.contains_key(ident_id):
            return scope.symbols[ident_id]
        scope = scope.parent
    return None
```

**Data Structures**:
```rust
struct Resolver {
    symbol_tables: Arena<SymbolTable>,
    current_scope: SymbolTableId,
    errors: Vec<ResolutionError>,
    idents: &IdentArena,  // For error messages
}

struct SymbolTable {
    parent: Option<SymbolTableId>,
    symbols: HashMap<IdentId, Symbol>,  // Key by IdentId
}

struct Symbol {
    name: IdentId,  // Interned name
    kind: SymbolKind,
    def_span: Span,
}
```

### Type Checker Component ✅

**Module**: `src/type_checker/mod.rs` (complete implementation)

**Key Functions**:
```rust
pub fn check_types(
    ast: ResolvedAst,
    idents: &IdentArena
) -> (TypedIr, Vec<TypeError>)

struct TypeError {
    kind: TypeErrorKind,
    span: Span,
}

enum TypeErrorKind {
    TypeMismatch { expected: Type, found: Type },
    UnknownType(IdentId),
    InvalidOperation { op: String, operand_types: Vec<Type> },
    ArgumentCountMismatch { expected: usize, found: usize },
    InvalidReference,
}
```

**Algorithm**:
```pseudocode
type_check_expr(expr, context):
    match expr:
        Literal:
            return infer_literal_type(literal)
        
        Ident:
            symbol = lookup_symbol(expr.symbol_id)
            return symbol.type
        
        BinaryOp:
            left_ty = type_check_expr(expr.left, context)
            right_ty = type_check_expr(expr.right, context)
            result_ty = check_binary_op(expr.op, left_ty, right_ty)
            return result_ty
        
        FunctionCall:
            func = lookup_function(expr.func_id)
            arg_types = [type_check_expr(arg, context) for arg in expr.args]
            check_argument_types(func.param_types, arg_types)
            return func.return_type
        
        FieldAccess:
            base_ty = type_check_expr(expr.base, context)
            field = lookup_field(base_ty, expr.field_name)
            return field.type
        
        ArrayIndex:
            array_ty = type_check_expr(expr.array, context)
            index_ty = type_check_expr(expr.index, context)
            check_type(index_ty, I32)
            return array_ty.element_type

check_binary_op(op, left_ty, right_ty):
    match (op, left_ty, right_ty):
        (Plus, Length, Length) -> Length
        (Mult, Length, F64) -> Length
        (Mult, Length, Length) -> Area
        ...
        _ -> report TypeError and return Error type
```

**Type Inference**:
- Bidirectional type checking
- Constraint-based inference for complex cases
- Instantiate generic types (arrays)
- Handle reference/dereference automatically when needed

---

## Intermediate Representations

### Unresolved AST ✅

**Purpose**: Direct representation of source syntax after parsing.

**Characteristics**:
- Identifiers are interned as `IdentId` (efficient integer handles)
- Types are represented by `IdentId` references
- No semantic validation
- Preserves all source structure including whitespace spans

**Key Structures**:
```rust
struct UnresolvedAst {
    sketches: Vec<SketchDef>,
    structs: Vec<StructDef>,
    source_file: PathBuf,
}

struct SketchDef {
    name: IdentId,  // Interned identifier
    body: Vec<Stmt>,
    span: Span,
}

struct StructDef {
    name: IdentId,  // Interned identifier
    fields: Vec<FieldDef>,
    methods: Vec<FunctionDef>,
    span: Span,
}

struct FieldDef {
    name: IdentId,  // Interned identifier
    ty: TypeRef,
    span: Span,
}

enum Stmt {
    Let {
        name: IdentId,  // Interned identifier
        ty: TypeRef,
        init: Option<Expr>,
        span: Span,
    },
    Assign {
        target: Expr,
        value: Expr,
        span: Span,
    },
    For {
        var: IdentId,  // Interned identifier
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

enum Expr {
    Literal {
        kind: LiteralKind,
        span: Span,
    },
    Ident {
        name: IdentId,  // Interned identifier
        span: Span,
    },
    BinaryOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    FieldAccess {
        base: Box<Expr>,
        field: IdentId,  // Interned identifier
        span: Span,
    },
    ArrayIndex {
        array: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    StructLiteral {
        ty: TypeRef,
        fields: Vec<(IdentId, Expr)>,  // Field names interned
        span: Span,
    },
    ArrayLiteral {
        elements: Vec<Expr>,
        span: Span,
    },
}

struct TypeRef {
    name: IdentId,  // Interned type name
    is_reference: bool,
    array_size: Option<usize>,
    span: Span,
}
```

### Resolved AST ✅

**Purpose**: AST with all identifiers resolved to their definitions.

**Characteristics**:
- All identifier references link to `SymbolId`
- Symbol tables attached to scopes
- Ready for type checking
- Still preserves source locations

**Key Additions**:
```rust
struct ResolvedAst {
    sketches: Vec<ResolvedSketchDef>,
    symbol_tables: Arena<SymbolTable>,
    struct_definitions: HashMap<StructId, StructDef>,
    source_file: PathBuf,
}

// Enhanced structures with symbol IDs
enum ResolvedExpr {
    Ident {
        name: IdentId,  // Interned identifier
        symbol_id: SymbolId,  // NEW
        span: Span,
    },
    Call {
        func_id: FunctionId,  // NEW: resolved function
        args: Vec<ResolvedExpr>,
        span: Span,
    },
    // ... other variants similarly enhanced
}

struct ResolvedTypeRef {
    name: IdentId,  // Interned type name
    struct_id: Option<StructId>,  // NEW: for user-defined types
    is_reference: bool,
    array_size: Option<usize>,
    span: Span,
}
```

### Typed IR ✅

**Purpose**: Fully type-checked representation ready for constraint generation.

**Characteristics**:
- ✅ Complete type structures defined in `ast/typed.rs`
- ✅ Type checking algorithm implemented
- ✅ Expression type annotations complete
- ✅ Type conversions and compatibility checking
- ✅ Source locations preserved in all structures

**Key Structures**:
```rust
struct TypedIr {
    sketches: Vec<TypedSketch>,
    type_table: TypeTable,
    source_file: PathBuf,
}

struct TypedSketch {
    name: IdentId,  // Interned identifier
    body: Vec<TypedStmt>,
    scope: SymbolTableId,
    span: Span,
}

enum TypedStmt {
    Let {
        name: IdentId,  // Interned identifier
        symbol_id: SymbolId,
        ty: Type,
        init: Option<TypedExpr>,
        span: Span,
    },
    Constraint {
        target: TypedExpr,
        value: TypedExpr,
        constraint_kind: ConstraintKind,  // NEW
        span: Span,
    },
    For {
        var: IdentId,  // Interned identifier
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
        context_id: ViewContextId,  // NEW: track view scope
        span: Span,
    },
    Expr(TypedExpr),
}

struct TypedExpr {
    expr: ExprKind,
    ty: Type,
    span: Span,
}

enum ConstraintKind {
    Equality,      // =
    GreaterThan,   // >
    LessThan,      // <
    GreaterEqual,  // >=
    LessEqual,     // <=
}

struct TypeTable {
    types: HashMap<TypeId, TypeInfo>,
}
```

---

## Error Handling Strategy

### Error Collection

All phases collect errors without stopping execution, allowing multiple errors to be reported in one pass.

**Error Collection Pattern**:
```rust
struct ErrorCollector<E> {
    errors: Vec<E>,
}

impl<E> ErrorCollector<E> {
    fn report(&mut self, error: E) {
        self.errors.push(error);
    }
    
    fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
    
    fn into_errors(self) -> Vec<E> {
        self.errors
    }
}
```

### Error Types

Using `thiserror` for error definitions:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LexError {
    #[error("Invalid character: {0}")]
    InvalidChar(char),
    
    #[error("Malformed number literal")]
    MalformedNumber,
    
    #[error("Unterminated string")]
    UnterminatedString,
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Unexpected token: expected {expected}, found {found}")]
    UnexpectedToken {
        expected: String,
        found: String,
    },
    
    #[error("Missing {0}")]
    MissingToken(String),
    
    #[error("Invalid expression")]
    InvalidExpression,
}

#[derive(Error, Debug)]
pub enum ResolutionError {
    #[error("Undefined name: {0}")]
    UndefinedName(String),
    
    #[error("Duplicate definition of '{name}'")]
    DuplicateDefinition {
        name: String,
        first_def: Span,
    },
    
    #[error("Cannot reference {0} in this context")]
    InvalidReference(String),
}

#[derive(Error, Debug)]
pub enum TypeError {
    #[error("Type mismatch: expected {expected}, found {found}")]
    TypeMismatch {
        expected: String,
        found: String,
    },
    
    #[error("Cannot apply operator {op} to types {left} and {right}")]
    InvalidOperation {
        op: String,
        left: String,
        right: String,
    },
    
    #[error("Function expects {expected} arguments, found {found}")]
    ArgumentCountMismatch {
        expected: usize,
        found: usize,
    },
}
```

### Error Reporting with Ariadne

**Module**: `diagnostics.rs`

**Purpose**: Pretty-print errors with source context using Ariadne.

**Implementation**:
```rust
use ariadne::{Report, ReportKind, Label, Source, Color};

pub fn report_errors<E: ToDiagnostic>(
    errors: Vec<E>,
    source: &str,
    filename: &str
) {
    for error in errors {
        let diagnostic = error.to_diagnostic();
        
        Report::build(ReportKind::Error, filename, diagnostic.span.start)
            .with_message(diagnostic.message)
            .with_label(
                Label::new((filename, diagnostic.span.into_range()))
                    .with_message(diagnostic.label)
                    .with_color(Color::Red)
            )
            .finish()
            .eprint((filename, Source::from(source)))
            .unwrap();
    }
}

pub trait ToDiagnostic {
    fn to_diagnostic(&self) -> Diagnostic;
}

pub struct Diagnostic {
    message: String,
    span: Span,
    label: String,
    notes: Vec<String>,
}
```

**Example Error Output**:
```
Error: Type mismatch: expected Length, found Angle
  ┌─ examples/simple.tcad:15:9
  │
15│     x = 45deg;
  │         ^^^^^ expected Length, found Angle
  │
  = note: x was declared as Length on line 10
```

### Error Recovery

Each phase implements recovery strategies:

**Lexer Recovery**:
- Skip invalid character
- Continue tokenizing
- Report error for skipped content

**Parser Recovery**:
- Skip to next statement boundary
- Insert placeholder nodes with error markers
- Continue parsing sibling nodes

**Name Resolution Recovery**:
- Insert placeholder symbol for undefined names
- Continue checking other references
- Avoid cascading errors from same undefined name

**Type Checker Recovery**:
- Assign `Error` type to failed expressions
- Continue checking other expressions
- Suppress cascading errors from error-typed expressions

---

## Source Location Tracking

### Span Representation

**Module**: `span.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,  // Byte offset in source
    pub end: usize,    // Byte offset in source
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }
    
    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
    
    pub fn to_location(&self, source: &str) -> Location {
        // Convert byte offset to line/column
        let mut line = 1;
        let mut col = 1;
        
        for (pos, ch) in source.chars().enumerate() {
            if pos >= self.start {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        
        Location { line, col }
    }
    
    pub fn into_range(self) -> std::ops::Range<usize> {
        self.start..self.end
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Location {
    pub line: usize,
    pub col: usize,
}
```

### Span Propagation

**Strategy**: Every AST node carries its source span.

**Implementation Pattern**:
- Tokens from lexer include spans
- Parser combinators automatically merge spans
- AST constructors accept spans as parameters
- Transformations preserve original spans

**Example**:
```rust
// In parser
let_stmt_parser()
    .map_with_span(|(name, ty, init), span| LetStmt {
        name,
        ty,
        init,
        span,  // Automatically provided by chumsky
    })
```

### Source File Management

```rust
pub struct SourceFile {
    pub path: PathBuf,
    pub content: String,
    pub line_starts: Vec<usize>,  // Byte offsets of line starts
}

impl SourceFile {
    pub fn new(path: PathBuf, content: String) -> Self {
        let line_starts = compute_line_starts(&content);
        SourceFile { path, content, line_starts }
    }
    
    pub fn span_to_location(&self, span: Span) -> (Location, Location) {
        let start = self.offset_to_location(span.start);
        let end = self.offset_to_location(span.end);
        (start, end)
    }
    
    fn offset_to_location(&self, offset: usize) -> Location {
        let line = self.line_starts.binary_search(&offset)
            .unwrap_or_else(|i| i.saturating_sub(1));
        let col = offset - self.line_starts[line];
        Location { line: line + 1, col: col + 1 }
    }
}
```

---

## Implementation Technologies

### String Interner ✅

**Why string-interner?**
- Efficient string interning with minimal overhead
- Thread-safe options available
- Symbol type provides type-safe integer handles
- Optimized for common use cases like identifier storage

**Key Features**:
- `StringInterner` for single-threaded use
- O(1) lookups and interning (amortized)
- Small memory footprint for symbol handles
- Compatible with serialization if needed

**Usage**:
```rust
use string_interner::{StringInterner, Symbol};

let mut interner = StringInterner::default();
let hello = interner.get_or_intern("hello");
let world = interner.get_or_intern("world");

assert_eq!(interner.resolve(hello), Some("hello"));
assert_ne!(hello, world);
```

### Chumsky Parser Combinators ✅

**Why Chumsky?**
- Zero-copy parsing with spans
- Excellent error recovery
- Clear combinator API
- Built-in support for error collection

**Key Features Used**:
- `Parser` trait for composability
- `map_with_span()` for automatic span tracking
- `recover_with()` for error recovery
- `separated_by()` for lists
- `delimited_by()` for paired delimiters
- `choice()` for alternatives

**Example Structure**:
```rust
use chumsky::prelude::*;

pub fn parser() -> impl Parser<Token, UnresolvedAst, Error = ParseError> {
    sketch_parser()
        .repeated()
        .map(|sketches| UnresolvedAst { sketches })
}

fn sketch_parser() -> impl Parser<Token, SketchDef, Error = ParseError> {
    keyword(Token::Sketch)
        .ignore_then(ident())
        .then(stmt_parser().repeated().delimited_by(
            just(Token::LBrace),
            just(Token::RBrace)
        ))
        .map_with_span(|(name, body), span| SketchDef {
            name,
            body,
            span,
        })
        .recover_with(skip_until([Token::RBrace]))
}
```

### Logos Lexer ✅

**Why Logos?**
- Fast DFA-based lexing
- Macro-based token definitions
- Built-in span tracking
- Custom callbacks for complex tokens

**Example**:
```rust
use logos::Logos;

#[derive(Logos, Debug, PartialEq)]
pub enum Token {
    #[token("sketch")]
    Sketch,
    
    #[token("struct")]
    Struct,
    
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Ident,  // Will be post-processed to intern
    
    #[regex(r"[0-9]+", |lex| lex.slice().parse())]
    IntLiteral(i64),
    
    #[regex(r"[0-9]+(\.[0-9]+)?mm", parse_length_mm)]
    Millimeter(f64),
    
    #[regex(r"//[^\n]*", logos::skip)]
    Comment,
    
    #[error]
    Error,
}

fn parse_length_mm(lex: &mut logos::Lexer<Token>) -> Option<f64> {
    let slice = lex.slice();
    let num_part = &slice[..slice.len()-2];
    num_part.parse().ok()
}

// Post-processing to intern identifiers
pub fn tokenize_with_interning(
    source: &str,
    idents: &mut IdentArena
) -> Result<Vec<TokenWithSpan>, Vec<LexError>> {
    let mut tokens = Vec::new();
    let mut lexer = Token::lexer(source);
    
    while let Some(token) = lexer.next() {
        let span = Span::new(lexer.span().start, lexer.span().end);
        
        let token_kind = match token {
            Ok(Token::Ident) => {
                let name = lexer.slice();
                let id = idents.intern(name);
                TokenKind::Ident(id)
            }
            Ok(other) => other.into(),
            Err(_) => return Err(vec![LexError::InvalidToken(span)]),
        };
        
        tokens.push(TokenWithSpan { kind: token_kind, span });
    }
    
    Ok(tokens)
}
```

### Ariadne Error Reporting ❌

**Why Ariadne?** (Not yet implemented)
- Beautiful terminal output
- Source code snippets
- Multiple label support
- Color-coded messages
- Compatible with any span type

**Usage Pattern**:
```rust
use ariadne::{Report, ReportKind, Label, Source, Color};

fn report_type_error(error: &TypeError, source: &SourceFile) {
    let span = error.span();
    let (start_loc, end_loc) = source.span_to_location(span);
    
    Report::build(ReportKind::Error, &source.path, span.start)
        .with_message(error.to_string())
        .with_label(
            Label::new((&source.path, span.into_range()))
                .with_message("type error here")
                .with_color(Color::Red)
        )
        .finish()
        .eprint((&source.path, Source::from(&source.content)))
        .unwrap();
}
```

---

## Data Flow

### Complete Pipeline

```
┌─────────────┐
│ Source Text │
└──────┬──────┘
       │
       ▼
┌──────────────────┐
│ Create           │
│ IdentArena       │
└──────┬───────────┘
       │
       ▼
┌─────────────────────┐
│ Lexer (Logos)       │
│ - Tokenize          │
│ - Intern identifiers│
│ - Track spans       │
│ - Skip comments     │
└──────┬──────────────┘
       │
       │ Vec<Token> (with IdentId)
       │ + IdentArena
       ▼
┌────────────────────┐
│ Parser (Chumsky)   │
│ - Build AST        │
│ - Use IdentId      │
│ - Error recovery   │
│ - Preserve spans   │
└──────┬─────────────┘
       │
       │ UnresolvedAst (IdentId) + Errors
       │ + IdentArena
       ▼
┌────────────────────┐
│ Name Resolution    │
│ - Build sym tables │
│ - Map IdentId→ID   │
│ - Resolve refs     │
│ - Check scopes     │
└──────┬─────────────┘
       │
       │ ResolvedAst (IdentId→SymbolId) + Errors
       │ + IdentArena
       ▼
┌────────────────────┐
│ Type Checker       │
│ - Infer types      │
│ - Check compat     │
│ - Annotate exprs   │
└──────┬─────────────┘
       │
       │ TypedIr (IdentId, typed) + Errors
       │ + IdentArena
       ▼
┌────────────────────┐
│ Constraint Collect │
│ (Future phase)     │
└────────────────────┘
```

### Error Flow

Errors are collected at each phase and can be reported independently:

```
Phase 1: Lexer
    ↓ LexErrors
Phase 2: Parser
    ↓ ParseErrors
Phase 3: Name Resolution
    ↓ ResolutionErrors
Phase 4: Type Checker
    ↓ TypeErrors

All errors → Ariadne → Terminal Output
```

### Information Preservation

Throughout all phases:
- **Spans**: Every node retains original source location
- **Names**: Identifier strings preserved alongside IDs
- **Structure**: AST structure maintained through transformations
- **Comments**: Stripped in lexer but could be preserved if needed

---

## Module Structure

Recommended Rust module organization:

```
src/
├── main.rs           # CLI entry point
├── lib.rs            # Library exports
│
├── ident.rs          # Identifier arena and interning
├── span.rs           # Span and location types
├── error.rs          # Error type definitions (thiserror)
├── diagnostics.rs    # Error reporting (ariadne)
│
├── lexer/
│   ├── mod.rs        # Lexer interface
│   ├── token.rs      # Token definitions (logos)
│   └── tests.rs      # Lexer tests
│
├── parser/
│   ├── mod.rs        # Parser interface
│   ├── grammar.rs    # Grammar combinators (chumsky)
│   ├── expr.rs       # Expression parsing
│   ├── stmt.rs       # Statement parsing
│   └── tests.rs      # Parser tests
│
├── ast/
│   ├── mod.rs        # AST exports
│   ├── unresolved.rs # Unresolved AST types
│   ├── resolved.rs   # Resolved AST types
│   ├── typed.rs      # Typed IR types
│   └── visitor.rs    # AST visitor pattern
│
├── resolve/
│   ├── mod.rs        # Name resolution interface
│   ├── scope.rs      # Symbol tables and scopes
│   ├── resolver.rs   # Resolution algorithm
│   └── tests.rs      # Resolution tests
│
├── typecheck/
│   ├── mod.rs        # Type checker interface
│   ├── types.rs      # Type system definitions
│   ├── infer.rs      # Type inference
│   ├── checker.rs    # Type checking algorithm
│   └── tests.rs      # Type checker tests
│
└── constraints/
    ├── mod.rs        # Constraint system (future)
    └── collect.rs    # Constraint collection (future)
```

---

## Testing Strategy

### Unit Tests

Each module should have comprehensive unit tests:

**Lexer Tests**:
```rust
#[test]
fn test_tokenize_numeric_literals() {
    let source = "10mm 5.5cm 45deg";
    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Millimeter(10.0));
    assert_eq!(tokens[1].kind, TokenKind::Centimeter(5.5));
    assert_eq!(tokens[2].kind, TokenKind::Degree(45.0));
}

#[test]
fn test_identifier_interning() {
    let source = "let x: Length = y;";
    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).unwrap();
    
    // Extract identifier tokens
    let x_id = match tokens[1].kind {
        TokenKind::Ident(id) => id,
        _ => panic!("Expected identifier"),
    };
    let y_id = match tokens[7].kind {
        TokenKind::Ident(id) => id,
        _ => panic!("Expected identifier"),
    };
    
    // Verify interning
    assert_eq!(idents.resolve(x_id), "x");
    assert_eq!(idents.resolve(y_id), "y");
    assert_ne!(x_id, y_id);
}
```

**Parser Tests**:
```rust
#[test]
fn test_parse_let_statement() {
    let source = "let x: Length = 10mm;";
    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).unwrap();
    let (ast, errors) = parse(tokens, &idents);
    assert!(errors.is_empty());
    assert!(matches!(ast.stmts[0], Stmt::Let { .. }));
}
```

**Resolution Tests**:
```rust
#[test]
fn test_resolve_forward_reference() {
    let source = r#"
        sketch test {
            x = y + 10mm;
            let y: Length = 5mm;
            let x: Length;
        }
    "#;
    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).unwrap();
    let (ast, _) = parse(tokens, &idents);
    let (resolved, errors) = resolve_names(ast.unwrap(), &idents);
    assert!(errors.is_empty());
}
```

**Type Checker Tests**:
```rust
#[test]
fn test_type_mismatch_error() {
    let source = r#"
        sketch test {
            let x: Length = 45deg;
        }
    "#;
    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).unwrap();
    let (ast, _) = parse(tokens, &idents);
    let (resolved, _) = resolve_names(ast.unwrap(), &idents);
    let (typed, errors) = check_types(resolved, &idents);
    assert_eq!(errors.len(), 1);
    assert!(matches!(errors[0].kind, TypeErrorKind::TypeMismatch { .. }));
}
```

### Integration Tests

Full pipeline tests with example programs:

```rust
#[test]
fn test_simple_triangle() {
    let source = include_str!("../../examples/simple_triangle.tcad");
    
    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).unwrap();
    let (ast, parse_errors) = parse(tokens, &idents);
    let (resolved, resolve_errors) = resolve_names(ast.unwrap(), &idents);
    let (typed, type_errors) = check_types(resolved, &idents);
    
    assert!(parse_errors.is_empty());
    assert!(resolve_errors.is_empty());
    assert!(type_errors.is_empty());
}
```

### Error Recovery Tests

Verify that multiple errors are collected:

```rust
#[test]
fn test_multiple_undefined_names() {
    let source = r#"
        sketch test {
            a = b + c;
            d = e + f;
        }
    "#;
    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).unwrap();
    let (ast, _) = parse(tokens, &idents);
    let (_, errors) = resolve_names(ast.unwrap(), &idents);
    assert_eq!(errors.len(), 6); // a, b, c, d, e, f all undefined
}
```

---

## Performance Considerations

### Parsing Performance

- **Logos**: Generates efficient DFA-based lexer
- **Chumsky**: Zero-copy parsing where possible
- **Avoid**: Deep recursion limits for expressions

### Memory Management

- **Arena allocation**: Use `typed-arena` for AST nodes
- **Identifier interning**: Use `string-interner` or custom arena for identifier strings
  - All identifier strings are interned on first use
  - Identifiers reference interned strings via `IdentId`
  - Reduces memory usage and enables O(1) equality comparison
- **Sharing**: Share immutable type information

### Parallel Processing

Future optimization:
- Parse multiple files in parallel
- Type check independent sketches in parallel
- Requires thread-safe error collection

---

## Future Extensions

### Import System

When imports are implemented:
- Module resolution algorithm
- Cyclic import detection
- Incremental compilation

### Macro System

If macro system is added:
- Macro expansion phase between parsing and resolution
- Hygiene for generated identifiers
- Span tracking through expansions

### Incremental Compilation

- Cache parsed ASTs
- Invalidate only changed files
- Reuse symbol tables for unchanged scopes

### IDE Support

- Language Server Protocol (LSP) implementation
- Hover information from type annotations
- Go-to-definition from resolved symbol IDs
- Real-time error reporting

---

## Conclusion

This architecture provides a clear separation of concerns with well-defined interfaces between phases. The use of proven libraries (string-interner, logos, chumsky, ariadne, thiserror) reduces implementation complexity while providing robust error handling and excellent developer experience.

Key benefits:
- **Comprehensive error reporting** through error collection
- **Precise error messages** via source location tracking
- **Efficient identifier handling** through string interning with O(1) comparison
- **Extensible design** for future language features
- **Clear separation** between syntax and semantics
- **Type safety** through Rust's type system
- **Memory efficiency** via arena allocation and identifier interning

The architecture is ready for implementation and can be incrementally developed by implementing each phase independently with clear test boundaries.

---

**End of Architecture Document**
