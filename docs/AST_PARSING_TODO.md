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

### Parser (Partial - Arithmetic Expressions Only)
- [x] **Atomic Expressions**:
  - [x] Integer literals
  - [x] Float literals
  - [x] Variables (Identifiers)

- [x] **Binary Operators** (Partial):
  - [x] Addition (`+`)
  - [x] Subtraction (`-`)
  - [x] Multiplication (`*`)
  - [x] Division (`/`)

- [x] **Operator Precedence & Associativity**:
  - [x] Correct precedence (multiplication before addition)
  - [x] Left-associativity
  - [x] Parentheses for precedence override

- [x] **Error Reporting**:
  - [x] Ariadne integration for beautiful error output
  - [x] Detailed error positioning with spans

## ❌ TODO - Core Language Features (Per Spec)

### Expressions

#### Literals
- [ ] Boolean literals: `true`, `false`
- [ ] Unit suffixes for Length: `mm`, `cm`, `m`
- [ ] Unit suffixes for Angle: `deg`, `rad`

#### Operators
- [ ] Power operator: `^`
- [ ] Modulo operator: `%`
- [ ] Comparison operators: `==`, `!=`, `<`, `>`, `<=`, `>=`
- [ ] Logical operators: `and`, `or`
- [ ] Unary minus: `-expr`
- [ ] Reference operator: `&expr`
- [ ] Dereference operator: `*expr`

#### Complex Expressions
- [ ] Function calls: `foo(arg1, arg2)`
- [ ] Method calls: `obj.method(args)`
- [ ] Field access: `obj.field`, `obj.field.subfield`
- [ ] Array indexing: `arr[index]`
- [ ] Range expressions: `0..10`, `start..end`
- [ ] Closures: `|param| expr`, `|p1, p2| { statements }`

#### Literals & Constructors
- [ ] Array literals: `[]`, `[expr1, expr2]`
- [ ] Struct literals: `StructName { field1: value1, field2: value2 }`
- [ ] Struct literals with computed properties: `Rect { area() = 5000mm² }`

### Statements

- [ ] Variable declarations: `let x: Type = value;`
- [ ] Variable declarations without initialization: `let x: Type;`
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

### Type Annotations

- [ ] Basic type annotations: `x: i32`, `p: Point`
- [ ] Reference types: `&Point`, `&Length`
- [ ] Array types: `[Type; size]`
- [ ] Function return types: `fn name() -> Type`
- [ ] Function parameter types: `fn name(param: &Type)`

### Definitions

#### Struct Definitions
- [ ] Basic struct: `struct Name { field1: Type, field2: Type }`
- [ ] Struct with container: `struct Name { container entities, field: Type }`
- [ ] Struct with methods: `struct Name { field: Type, fn method() -> Type { ... } }`
- [ ] Struct with self reference: `self.field` in methods
- [ ] Transform methods: `fn __transform__(p: &Point) -> Point { ... }`

#### Function Definitions
- [ ] Top-level functions: `fn name(param: Type) -> ReturnType { ... }`
- [ ] Functions with reference parameters: `fn name(p: &Point) -> Type`
- [ ] Functions with multiple parameters
- [ ] Functions with body blocks

### Functional Operations (Spec Required)

- [ ] Map on arrays: `array.map(|elem| expr)`
- [ ] Reduce on arrays: `array.reduce(init, |acc, elem| expr)`
- [ ] Method chaining: `array.map(...).reduce(...)`

### Top-Level Program Structure

- [ ] Multiple definitions at top level
- [ ] Mixed statements and definitions
- [ ] Module/program as list of items

## 📝 Implementation Priority (Based on Spec)

### Phase 1: Basic Expressions & Types
1. Boolean literals and logical operators
2. Unit suffixes (mm, cm, deg, rad)
3. Comparison operators
4. Power and modulo operators
5. Unary operators (-, &, *)

### Phase 2: Statements & Control Flow
1. Let statements with type annotations
2. Assignment statements
3. Return statements
4. Expression statements
5. Block statements
6. For loops
7. If-else statements

### Phase 3: Functions & Calls
1. Function calls
2. Method calls
3. Field access (single and chained)
4. Array indexing
5. Range expressions

### Phase 4: Definitions
1. Basic struct definitions
2. Struct methods
3. Top-level function definitions
4. Container structs
5. Transform methods

### Phase 5: Advanced Features
1. Struct literals
2. Array literals
3. Closures
4. Map and reduce operations
5. With statements
6. Dot prefix syntax in with blocks

## 🔍 Code References

- **Language Spec**: `docs/TEXTCAD_LANGUAGE_SPEC.md` - Complete specification
- **Lexer**: `src/lexer.rs` - Fully implemented
- **Parser**: `src/parser.rs` - Arithmetic expressions only
- **AST**: `src/ast.rs` - Expression types only
- **Main**: `src/main.rs` - CLI with `parse` command

## Notes

- This TODO list only includes features explicitly required by the TextCAD language specification
- Standard library functions (like `distance()`, `point()`, etc.) are not parser features and are not included here
- The spec explicitly mentions that if-else is part of control flow, though it's less detailed than for loops
- All geometric types (Point, Length, Angle, Area) are built-in according to the spec
