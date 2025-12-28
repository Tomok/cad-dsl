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

### Parser (Near Complete for Expressions & Definitions)

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

#### Statements
- [x] Variable declarations: `let x: Type = value;`
- [x] Variable declarations without initialization: `let x: Type;`
- [x] **For loops over ranges**: `for i in 0..10 { ... }`
- [x] **For loops over arrays**: `for elem in array { ... }`

#### Definitions
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
- [x] Comprehensive test coverage with timeout protection (200+ tests)

## ❌ TODO - Remaining Language Features (Per Spec)

### Expressions

#### Literals
- [ ] Unit suffixes for Length: `mm`, `cm`, `m`
- [ ] Unit suffixes for Angle: `deg`, `rad`
- [ ] Unit suffixes for Area: `mm²`, `cm²`, `m²`

#### Operators
- [ ] Dereference operator: `*expr` (for dereferencing references)

### Statements

- [ ] Container field declarations: `let container.field: Type = value;`
- [ ] Assignment statements: `x = value;`
- [ ] Field assignment: `obj.field = value;`
- [ ] Return statements: `return expr;`
- [ ] Expression statements: `expr;`
- [ ] Block statements: `{ stmt1; stmt2; }`
- [ ] With statements: `with transform { ... }`
- [ ] With statements with dot prefix: `with container { let .field = value; }`
- [ ] If-else statements: `if condition { ... } else { ... }`

### Functional Operations (Spec Required)

- [ ] Map on arrays: `array.map(|elem| expr)`
- [ ] Reduce on arrays: `array.reduce(init, |acc, elem| expr)`
- [ ] Method chaining: `array.map(...).reduce(...)`

### Top-Level Program Structure

- [ ] Multiple definitions at top level
- [ ] Mixed statements and definitions
- [ ] Module/program as list of items

## 📊 Progress Summary

**Expressions**: ~95% complete ⬆️
- ✅ All operators implemented (arithmetic, comparison, logical, unary)
- ✅ All complex expressions (calls, field access, indexing, ranges, closures)
- ✅ All literals including computed properties in struct literals
- ⏳ Missing: unit suffixes, dereference operator

**Statements**: ~30% complete ⬆️
- ✅ Let statements fully working
- ✅ For loops (ranges and arrays)
- ⏳ Missing: assignments, returns, blocks, if/else, with statements

**Definitions**: ~95% complete
- ✅ Structs with fields, methods, and container support
- ✅ Functions with parameters and bodies
- ✅ Transform methods
- ⏳ Missing: top-level program structure (multiple items)

**Overall**: ~75% of spec features implemented ⬆️

## 📝 Next Implementation Priority

### High Priority (Core Statements)
1. **Assignment statements**: `x = value;`, `obj.field = value;`
2. **Return statements**: `return expr;`
3. **Expression statements**: `expr;`
4. **Block statements**: `{ stmt1; stmt2; }`
5. **If-else statements**: `if condition { ... } else { ... }`

### Medium Priority (Advanced Statements)
1. **With statements**: `with transform { ... }`
2. **Container field declarations**: `let container.field: Type = value;`
3. **Dot prefix in with blocks**: `with container { let .field = value; }`

### Lower Priority (Advanced Features)
1. **Unit suffixes**: `mm`, `cm`, `m`, `deg`, `rad`
2. **Dereference operator**: `*expr`
3. **Map/reduce operations**: Functional array methods
4. **Top-level program**: Parse multiple definitions/statements

## 🔍 Code References

- **Language Spec**: `docs/TEXTCAD_LANGUAGE_SPEC.md` - Complete specification
- **Lexer**: `src/lexer.rs` - Fully implemented
- **Parser**:
  - `src/parser.rs` - Main entry point, expression parser
  - `src/parser/atoms.rs` - Literals, variables, primitives (including `self` keyword)
  - `src/parser/arithmetic.rs` - Arithmetic operators with precedence
  - `src/parser/comparison.rs` - All comparison operators (==, !=, <, >, <=, >=)
  - `src/parser/logical.rs` - Logical operators (and, or)
  - `src/parser/stmt.rs` - Statements (let, for loops) and definitions (function, struct with container)
  - `src/parser/error.rs` - Error reporting
  - `src/parser/tests.rs` - Comprehensive test suite (200+ tests, 4000+ lines)
- **AST**:
  - `src/ast.rs` - Main module
  - `src/ast/expr.rs` - Expression types with type-safe precedence (includes all comparison ops)
  - `src/ast/types.rs` - Type annotations, statements (Let, For, FunctionDef, StructDef with container)
  - `src/ast/span.rs` - Span tracking trait
  - `src/ast/display.rs` - Pretty-printing
  - `src/ast/conversions.rs` - Type conversions
  - `src/ast/tests.rs` - AST tests
- **Main**: `src/main.rs` - CLI with `lex` and `parse` commands

## Test Coverage by Category

Based on `src/parser/tests.rs` (200+ tests):

### Expressions (95+ tests)
- ✅ Literals: int, float, bool, arrays, structs
- ✅ All operators: arithmetic, comparison, logical, unary
- ✅ Precedence and associativity
- ✅ Complex expressions: calls, field access, indexing, ranges, closures
- ✅ Struct literals with computed properties

### Statements (30+ tests)
- ✅ Let statements with various type annotations
- ✅ For loops over ranges and arrays

### Definitions (40+ tests)
- ✅ Function definitions with parameters, return types, bodies
- ✅ Struct definitions with fields, methods, containers, transform methods

### Error Cases (20+ tests)
- ✅ Missing operators, operands, parentheses
- ✅ Invalid syntax
- ✅ Ariadne error reporting

## Notes

- This TODO list only includes features explicitly required by the TextCAD language specification
- Standard library functions (like `distance()`, `point()`, etc.) are not parser features
- **Container structs ARE fully implemented**: `struct Name { container entities, field: Type }`
- **Transform methods ARE fully implemented**: They're just regular methods with the name `__transform__`
- **Self references in methods ARE supported**: `self` is parsed as a variable, so `self.field` works via field access
- **All comparison operators ARE implemented**: `<`, `>`, `<=`, `>=`, `==`, `!=`
- **For loops ARE implemented**: Both range (`0..10`) and array iteration
- **Computed properties in struct literals ARE implemented**: `area() = 5000mm²`
- The parser has been extensively refactored into a modular structure for maintainability
- All geometric types (Point, Length, Angle, Area) are built-in according to the spec
