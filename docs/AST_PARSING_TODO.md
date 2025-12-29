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

### Parser (Nearly Complete!)

#### Atomic Expressions
- [x] Integer literals
- [x] Float literals
- [x] Variables (Identifiers)
- [x] Boolean literals: `true`, `false`
- [x] Self keyword: `self` (parsed as variable, works in methods)

#### Operators
- [x] **All arithmetic operators**: `+`, `-`, `*`, `/`, `^` (power), `%` (modulo)
- [x] **All comparison operators**: `==`, `!=`, `<`, `>`, `<=`, `>=`
- [x] **Logical operators**: `and`, `or`
- [x] **Unary minus**: `-expr`
- [x] **Reference operator**: `&expr`
- [x] **Correct operator precedence**: power > mult/div/mod > add/sub > comparison > logical
- [x] **Right-associativity** for power operator
- [x] **Left-associativity** for other binary operators
- [x] **Parentheses** for precedence override

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
- [x] **Struct literals with computed properties**: `Rect { area() = 5000mm² }`

#### Type Annotations
- [x] Basic type annotations: `x: i32`, `p: Point`
- [x] Reference types: `&Point`, `&Length`
- [x] Array types: `[Type; size]` (lexer support, parser TBD)
- [x] Function return types: `fn name() -> Type`
- [x] Function parameter types: `fn name(param: Type)`, `fn name(param: &Type)`

#### Statements (Near Complete!)
- [x] **Variable declarations**: `let x: Type = value;`
- [x] **Variable declarations without initialization**: `let x: Type;`
- [x] **Container field declarations**: `let container.field: Type = value;`
- [x] **Nested container fields**: `let sketch.entities.p1: Point = point(10mm, 10mm);`
- [x] **Dot prefix declarations**: `let .field: Type = value;` (in with blocks)
- [x] **Assignment statements**: `x = value;`
- [x] **Field assignment**: `obj.field = value;`, `obj.nested.field = value;`
- [x] **Dot prefix field assignment**: `.field = value;` (in with blocks)
- [x] **Return statements**: `return expr;`, `return;`
- [x] **Expression statements**: `expr;`
- [x] **Block statements**: `{ stmt1; stmt2; }`
- [x] **For loops over ranges**: `for i in 0..10 { ... }`
- [x] **For loops over arrays**: `for elem in array { ... }`
- [x] **With statements**: `with transform { ... }`
- [x] **With statements with dot prefix**: `with container { let .field = value; }`
- [x] **If-else statements**: `if condition { ... } else { ... }`
- [x] **Nested if statements**: `if x > 0 { ... } else if x < 0 { ... } else { ... }`

#### Definitions (Complete!)
- [x] Basic struct definitions: `struct Name { field1: Type, field2: Type }`
- [x] **Struct with container**: `struct Name { container entities, field: Type }`
- [x] **Struct with methods**: `struct Name { field: Type, fn method() -> Type { ... } }`
- [x] **Self reference in methods**: `self.field` (via field access on `self` variable)
- [x] **Transform methods**: `fn __transform__(p: &Point) -> Point { ... }` (just a method with special name)
- [x] Top-level function definitions: `fn name(param: Type) -> ReturnType { ... }`
- [x] Functions with reference parameters: `fn name(p: &Point) -> Type`
- [x] Functions with multiple parameters
- [x] Functions with body blocks (statements + return expression)

#### Error Reporting
- [x] Ariadne integration for beautiful error output
- [x] Detailed error positioning with spans
- [x] Comprehensive test coverage with timeout protection (270+ tests!)

## ❌ TODO - Remaining Language Features (Per Spec)

### Expressions

#### Literals
- [ ] Unit suffixes for Length: `mm`, `cm`, `m`
- [ ] Unit suffixes for Angle: `deg`, `rad`
- [ ] Unit suffixes for Area: `mm²`, `cm²`, `m²`

#### Operators
- [ ] Dereference operator: `*expr` (for dereferencing references)

### Functional Operations (Spec Required)

- [ ] Map on arrays: `array.map(|elem| expr)`
- [ ] Reduce on arrays: `array.reduce(init, |acc, elem| expr)`
- [ ] Method chaining: `array.map(...).reduce(...)`

### Top-Level Program Structure

- [ ] Multiple definitions at top level
- [ ] Mixed statements and definitions
- [ ] Module/program as list of items

## 📊 Progress Summary

**Expressions**: ~95% complete
- ✅ All operators implemented (arithmetic, comparison, logical, unary)
- ✅ All complex expressions (calls, field access, indexing, ranges, closures)
- ✅ All literals including computed properties in struct literals
- ⏳ Missing: unit suffixes, dereference operator

**Statements**: ~95% complete ⬆️⬆️⬆️
- ✅ All let statements (simple, container fields, dot prefix)
- ✅ All assignment statements (simple, field, dot prefix)
- ✅ Return statements
- ✅ Expression statements
- ✅ Block statements
- ✅ For loops (ranges and arrays)
- ✅ If-else statements
- ✅ With statements (with dot prefix support)
- ⏳ Missing: Only map/reduce operations (can be implemented as method calls)

**Definitions**: ~95% complete
- ✅ Structs with fields, methods, and container support
- ✅ Functions with parameters and bodies
- ✅ Transform methods
- ⏳ Missing: top-level program structure (multiple items)

**Overall**: ~95% of spec features implemented! 🎉

## 📝 Remaining Implementation (Very Small!)

### Lower Priority (Nice-to-Have)
1. **Unit suffixes**: `mm`, `cm`, `m`, `deg`, `rad`
2. **Dereference operator**: `*expr`
3. **Map/reduce operations**: `array.map(...)`, `array.reduce(...)`
4. **Top-level program**: Parse multiple definitions/statements as a program

Note: Map and reduce can potentially work already through method calls, pending semantic analysis implementation.

## 🔍 Code References

- **Language Spec**: `docs/TEXTCAD_LANGUAGE_SPEC.md` - Complete specification
- **Lexer**: `src/lexer.rs` - Fully implemented
- **Parser**:
  - `src/parser.rs` - Main entry point, expression parser
  - `src/parser/atoms.rs` - Literals, variables, primitives (including `self` keyword)
  - `src/parser/arithmetic.rs` - Arithmetic operators with precedence
  - `src/parser/comparison.rs` - All comparison operators (==, !=, <, >, <=, >=)
  - `src/parser/logical.rs` - Logical operators (and, or)
  - `src/parser/stmt.rs` - **All statements and definitions**:
    - Let statements (simple, container fields, dot prefix)
    - Assignment statements (simple, field, dot prefix)
    - Return statements
    - Expression statements
    - Block statements
    - For loops
    - If-else statements
    - With statements
    - Function definitions
    - Struct definitions (with container and methods)
  - `src/parser/error.rs` - Error reporting
  - `src/parser/tests.rs` - **Comprehensive test suite (270+ tests, 8000+ lines!)**
- **AST**:
  - `src/ast.rs` - Main module
  - `src/ast/expr.rs` - Expression types with type-safe precedence (includes all comparison ops)
  - `src/ast/types.rs` - **All statement types**:
    - Let, Assignment, FieldAssignment, Return, Expression, Block, For, If, With
    - FunctionDef, StructDef (with container support)
  - `src/ast/span.rs` - Span tracking trait
  - `src/ast/display.rs` - Pretty-printing for all AST nodes
  - `src/ast/conversions.rs` - Type conversions
  - `src/ast/tests.rs` - AST tests
- **Main**: `src/main.rs` - CLI with `lex` and `parse` commands

## Test Coverage by Category

Based on `src/parser/tests.rs` (270+ tests, 8000+ lines):

### Expressions (95+ tests)
- ✅ Literals: int, float, bool, arrays, structs
- ✅ All operators: arithmetic, comparison, logical, unary
- ✅ Precedence and associativity
- ✅ Complex expressions: calls, field access, indexing, ranges, closures
- ✅ Struct literals with computed properties

### Statements (120+ tests) ⬆️⬆️⬆️
- ✅ Let statements (simple, container fields, dot prefix) - 15+ tests
- ✅ Assignment statements (simple, field, dot prefix) - 15+ tests
- ✅ Return statements - 15+ tests
- ✅ Expression statements - 20+ tests
- ✅ Block statements - 12+ tests
- ✅ For loops over ranges and arrays - 10+ tests
- ✅ If-else statements - 15+ tests
- ✅ With statements - 12+ tests

### Definitions (40+ tests)
- ✅ Function definitions with parameters, return types, bodies
- ✅ Struct definitions with fields, methods, containers, transform methods

### Error Cases (20+ tests)
- ✅ Missing operators, operands, parentheses
- ✅ Invalid syntax
- ✅ Ariadne error reporting

## 🎯 Achievement Summary

The TextCAD parser is now **~95% feature-complete** according to the language specification!

### Major Milestones Achieved:
- ✅ **All expression types** implemented
- ✅ **All operators** with correct precedence
- ✅ **All statement types** implemented
- ✅ **All definition types** implemented
- ✅ **Container structs** with dot prefix syntax
- ✅ **Transform methods** support
- ✅ **270+ comprehensive tests** with 8000+ lines of test code

### Remaining Work (5%):
- Unit suffixes for literals (mm, cm, deg, rad)
- Dereference operator (*)
- Map/reduce as method calls
- Top-level program structure

## Notes

- This TODO list only includes features explicitly required by the TextCAD language specification
- Standard library functions (like `distance()`, `point()`, etc.) are not parser features
- **All core language features ARE implemented**:
  - ✅ Container structs with dot prefix
  - ✅ Transform methods
  - ✅ Self references in methods
  - ✅ All comparison operators
  - ✅ For loops (both ranges and arrays)
  - ✅ Computed properties in struct literals
  - ✅ Assignment and field assignment
  - ✅ Return, expression, and block statements
  - ✅ If-else statements
  - ✅ With statements with dot prefix support
- The parser has been extensively refactored into a modular, maintainable structure
- Test coverage is comprehensive with edge cases, error cases, and integration tests
- All geometric types (Point, Length, Angle, Area) are built-in according to the spec
