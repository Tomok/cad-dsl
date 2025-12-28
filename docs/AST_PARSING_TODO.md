# AST Parsing Status

This file tracks which parts of the TextCAD language specification can be parsed and which are still pending implementation.

## ✅ Implemented

### Lexer (Complete)
- [x] **Keywords**: `struct`, `container`, `fn`, `let`, `for`, `in`, `with`, `if`, `else`, `or`, `and`, `return`, `true`, `false`, `self`
- [x] **Operators**: `=`, `==`, `!=`, `<`, `>`, `<=`, `>=`, `+`, `-`, `*`, `/`, `^`, `%`, `&`
- [x] **Punctuation**: `:`, `;`, `,`, `.`, `..`, `(`, `)`, `[`, `]`, `{`, `}`, `|`, `->`
- [x] **Built-in Types**: `bool`, `i32`, `f64`, `Real`, `Algebraic`
- [x] **Literals**: Integer, Float, Identifier
- [x] **Comments**: Single-line (`//`) and multi-line (`/* */`)
- [x] **Position Tracking**: Line and column numbers
- [x] **Span Tracking**: Full span information for all tokens and AST nodes

### Parser (Substantial Progress)

#### Atomic Expressions
- [x] Integer literals
- [x] Float literals
- [x] Variables (Identifiers)
- [x] Boolean literals: `true`, `false`

#### Operators
- [x] Arithmetic operators: `+`, `-`, `*`, `/`, `^` (power), `%` (modulo)
- [x] Comparison operators: `==`, `!=`
- [x] Logical operators: `and`, `or`
- [x] Unary minus: `-expr`
- [x] Reference operator: `&expr`
- [x] Correct operator precedence (power > mult/div/mod > add/sub > comparison > logical)
- [x] Right-associativity for power operator
- [x] Left-associativity for other binary operators
- [x] Parentheses for precedence override

#### Complex Expressions
- [x] Function calls: `foo(arg1, arg2)`
- [x] Method calls: `obj.method(args)`
- [x] Field access: `obj.field`, `obj.field.subfield`
- [x] Array indexing: `arr[index]`
- [x] Range expressions: `0..10`, `start..end`
- [x] Closures: `|param| expr`, `|p1, p2| expr`

#### Literals & Constructors
- [x] Array literals: `[]`, `[expr1, expr2]`
- [x] Struct literals: `StructName { field1: value1, field2: value2 }`

#### Type Annotations
- [x] Basic type annotations: `x: i32`, `p: Point`
- [x] Reference types: `&Point`, `&Length`
- [x] Array types: `[Type; size]`
- [x] Function return types: `fn name() -> Type`
- [x] Function parameter types: `fn name(param: Type)`, `fn name(param: &Type)`

#### Statements
- [x] Variable declarations: `let x: Type = value;`
- [x] Variable declarations without initialization: `let x: Type;`

#### Definitions
- [x] Basic struct definitions: `struct Name { field1: Type, field2: Type }`
- [x] Struct with methods: `struct Name { field: Type, fn method() -> Type { ... } }`
- [x] Top-level function definitions: `fn name(param: Type) -> ReturnType { ... }`
- [x] Functions with reference parameters: `fn name(p: &Point) -> Type`
- [x] Functions with multiple parameters
- [x] Functions with body blocks

#### Error Reporting
- [x] Ariadne integration for beautiful error output
- [x] Detailed error positioning with spans
- [x] Comprehensive test coverage with timeout protection

## ❌ TODO - Remaining Language Features (Per Spec)

### Expressions

#### Literals
- [ ] Unit suffixes for Length: `mm`, `cm`, `m`
- [ ] Unit suffixes for Angle: `deg`, `rad`
- [ ] Unit suffixes for Area: `mm²`, `cm²`, `m²`

#### Operators
- [ ] Comparison operators: `<`, `>`, `<=`, `>=` (only `==`, `!=` implemented)
- [ ] Dereference operator: `*expr`

#### Literals & Constructors
- [ ] Struct literals with computed properties: `Rect { area() = 5000mm² }`

### Statements

- [ ] Container field declarations: `let container.field: Type = value;`
- [ ] Assignment statements: `x = value;`
- [ ] Field assignment: `obj.field = value;`
- [ ] Return statements: `return expr;`
- [ ] Expression statements: `expr;`
- [ ] Block statements: `{ stmt1; stmt2; }`
- [ ] For loops over ranges: `for i in 0..10 { ... }`
- [ ] For loops over arrays: `for elem in array { ... }`
- [ ] With statements: `with transform { ... }`
- [ ] With statements with dot prefix: `with container { let .field = value; }`
- [ ] If-else statements: `if condition { ... } else { ... }`

### Definitions

#### Struct Definitions
- [ ] Struct with container: `struct Name { container entities, field: Type }`
- [ ] Transform methods: `fn __transform__(p: &Point) -> Point { ... }`

### Functional Operations (Spec Required)

- [ ] Map on arrays: `array.map(|elem| expr)`
- [ ] Reduce on arrays: `array.reduce(init, |acc, elem| expr)`
- [ ] Method chaining: `array.map(...).reduce(...)`

### Top-Level Program Structure

- [ ] Multiple definitions at top level
- [ ] Mixed statements and definitions
- [ ] Module/program as list of items

## 📊 Progress Summary

**Expressions**: ~85% complete
- All basic operators implemented
- Function/method calls, field access, indexing all working
- Closures and ranges implemented
- Missing: unit suffixes, remaining comparison operators, dereference

**Statements**: ~15% complete
- Let statements fully working
- Missing: assignments, returns, blocks, control flow

**Definitions**: ~70% complete
- Structs and functions implemented
- Missing: container structs, transform methods, top-level program structure

**Overall**: ~60% of spec features implemented

## 📝 Next Implementation Priority

### High Priority (Core Functionality)
1. **Remaining comparison operators**: `<`, `>`, `<=`, `>=`
2. **Assignment statements**: `x = value;`, `obj.field = value;`
3. **Return statements**: `return expr;`
4. **Expression statements**: `expr;`
5. **Block statements**: `{ stmt1; stmt2; }`

### Medium Priority (Control Flow)
1. **If-else statements**: `if condition { ... } else { ... }`
2. **For loops**: `for i in 0..10 { ... }`, `for elem in array { ... }`
3. **With statements**: `with transform { ... }`

### Lower Priority (Advanced Features)
1. **Unit suffixes**: `mm`, `cm`, `m`, `deg`, `rad`
2. **Container structs**: `container entities` field
3. **Transform methods**: `fn __transform__(...)`
4. **Map/reduce operations**: Functional array methods
5. **Top-level program**: Parse multiple definitions/statements

## 🔍 Code References

- **Language Spec**: `docs/TEXTCAD_LANGUAGE_SPEC.md` - Complete specification
- **Lexer**: `src/lexer.rs` - Fully implemented
- **Parser**:
  - `src/parser.rs` - Main entry point, expression parser
  - `src/parser/atoms.rs` - Literals, variables, primitives
  - `src/parser/arithmetic.rs` - Arithmetic operators with precedence
  - `src/parser/comparison.rs` - Equality operators
  - `src/parser/logical.rs` - Logical operators (and, or)
  - `src/parser/stmt.rs` - Statements and definitions
  - `src/parser/error.rs` - Error reporting
  - `src/parser/tests.rs` - Comprehensive test suite
- **AST**:
  - `src/ast.rs` - Main module
  - `src/ast/expr.rs` - Expression types with type-safe precedence
  - `src/ast/types.rs` - Type annotations and statements
  - `src/ast/span.rs` - Span tracking trait
  - `src/ast/display.rs` - Pretty-printing
  - `src/ast/conversions.rs` - Type conversions
  - `src/ast/tests.rs` - AST tests
- **Main**: `src/main.rs` - CLI with `lex` and `parse` commands

## Notes

- This TODO list only includes features explicitly required by the TextCAD language specification
- Standard library functions (like `distance()`, `point()`, etc.) are not parser features and are not included here
- The spec explicitly mentions if-else as part of control flow
- All geometric types (Point, Length, Angle, Area) are built-in according to the spec
- The parser has been extensively refactored into a modular structure for maintainability
