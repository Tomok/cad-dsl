# AST Parsing Status

Diese Datei dokumentiert, welche Teile des AST bereits geparsed werden können und welche noch offen sind.

## ✅ Implementiert

### Lexer (Vollständig)
- [x] **Keywords**: `struct`, `container`, `fn`, `let`, `for`, `in`, `with`, `if`, `else`, `or`, `and`, `return`, `true`, `false`, `self`
- [x] **Operatoren**: `=`, `==`, `!=`, `<`, `>`, `<=`, `>=`, `+`, `-`, `*`, `/`, `^`, `%`, `&`
- [x] **Punktuation**: `:`, `;`, `,`, `.`, `..`, `(`, `)`, `[`, `]`, `{`, `}`, `|`, `->`
- [x] **Built-in Types**: `bool`, `i32`, `f64`, `Real`, `Algebraic`
- [x] **Literals**: Integer, Float, Identifier
- [x] **Kommentare**: Einzeilig (`//`) und mehrzeilig (`/* */`)
- [x] **Position Tracking**: Zeilen- und Spaltennummern

### Parser (Teilweise)
- [x] **Atomare Ausdrücke**:
  - [x] Integer Literale
  - [x] Float Literale
  - [x] Variablen (Identifier)

- [x] **Binäre Operatoren**:
  - [x] Addition (`+`)
  - [x] Subtraktion (`-`)
  - [x] Multiplikation (`*`)
  - [x] Division (`/`)

- [x] **Präzedenz & Assoziativität**:
  - [x] Operator-Präzedenz (Multiplikation vor Addition)
  - [x] Links-Assoziativität
  - [x] Parenthesen zur Präzedenz-Änderung

- [x] **Error Reporting**:
  - [x] Ariadne Integration für schöne Fehlerausgaben
  - [x] Detaillierte Fehlerpositionierung

## ❌ Noch zu implementieren

### Parser - Fehlende Ausdrücke
- [ ] **Boolesche Literale**: `true`, `false`
- [ ] **Vergleichsoperatoren**: `==`, `!=`, `<`, `>`, `<=`, `>=`
- [ ] **Logische Operatoren**: `and`, `or`
- [ ] **Potenz-Operator**: `^`
- [ ] **Modulo-Operator**: `%`
- [ ] **Unäre Operatoren**: `-x` (negation)
- [ ] **Funktionsaufrufe**: `foo(arg1, arg2)`
- [ ] **Methodenaufrufe**: `obj.method()`
- [ ] **Feldzugriff**: `obj.field`
- [ ] **Array-Zugriff**: `arr[index]`
- [ ] **Bereichsausdrücke**: `0..10`
- [ ] **Closures**: `|x| x + 1`

### Parser - Statements
- [ ] **Variable Deklarationen**: `let x: i32 = 42;`
- [ ] **Zuweisungen**: `x = 42;`
- [ ] **Return Statements**: `return x;`
- [ ] **Ausdrucks-Statements**: `foo();`
- [ ] **Block-Statements**: `{ stmt1; stmt2; }`

### Parser - Control Flow
- [ ] **If-Else**: `if condition { ... } else { ... }`
- [ ] **For Loops**: `for i in 0..10 { ... }`
- [ ] **With Statements**: `with transform { .point = p1; }`

### Parser - Definitionen
- [ ] **Struct Definitionen**: `struct Point { x: f64, y: f64 }`
- [ ] **Container Definitionen**: `container MyShape { ... }`
- [ ] **Funktionsdefinitionen**: `fn distance(p1: &Point, p2: &Point) -> f64 { ... }`
- [ ] **Funktionsparameter**: Mit Typen und Referenzen
- [ ] **Funktions-Rückgabetypen**: `-> Type`

### Parser - Typen
- [ ] **Typ-Annotationen**: `x: i32`
- [ ] **Referenz-Typen**: `&Type`
- [ ] **Array-Typen**: `[Type; size]`
- [ ] **Custom Types**: Verwendung von user-defined types
- [ ] **Generische Typen**: Falls geplant

### Parser - Weitere Features
- [ ] **Array Literale**: `[1, 2, 3]`
- [ ] **Struct Initialisierung**: `Point { x: 1.0, y: 2.0 }`
- [ ] **Pattern Matching**: Falls geplant
- [ ] **String Literale**: Falls benötigt

### AST - Fehlende Strukturen
- [ ] **Statement AST-Typen**: Definitionen für alle Statement-Arten
- [ ] **Definition AST-Typen**: Für Structs, Functions, Containers
- [ ] **Type AST-Typen**: Typ-System-Repräsentation
- [ ] **Pattern AST-Typen**: Falls benötigt
- [ ] **Top-Level AST**: Module/Program-Struktur

## 📝 Nächste Schritte (Empfohlen)

1. **Boolesche Ausdrücke**: Erweitere den Parser um `true`, `false`, und logische Operatoren
2. **Vergleichsoperatoren**: Implementiere `==`, `!=`, `<`, `>`, etc.
3. **Funktionsaufrufe**: Parse `function(args)`
4. **Statements**: Beginne mit einfachen Statements (let, return)
5. **Struct & Function Definitionen**: Top-level Definitionen
6. **Control Flow**: If/Else und For Loops
7. **Typ-System**: Type annotations und checking

## 🔍 Code-Referenzen

- **Lexer**: `src/lexer.rs` - Vollständig implementiert
- **Parser**: `src/parser.rs` - Nur Expressions
- **AST**: `src/ast.rs` - Nur Expression-Typen
- **Main**: `src/main.rs` - CLI mit `parse` Befehl
