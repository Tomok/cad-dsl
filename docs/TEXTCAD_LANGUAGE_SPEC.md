# TextCAD Domain-Specific Language Specification

**Version:** 2.1
**Status:** Draft

## Table of Contents

1. [Introduction](#introduction)
2. [Core Concepts](#core-concepts)
3. [Type System](#type-system)
4. [Unit System](#unit-system)
5. [Variable Declaration and Scoping](#variable-declaration-and-scoping)
6. [Assignment Semantics](#assignment-semantics)
7. [Entities vs References](#entities-vs-references)
8. [Structs](#structs)
9. [Container Structs](#container-structs)
10. [Transform Pattern](#transform-pattern)
11. [With Statements](#with-statements)
12. [Arrays](#arrays)
13. [Functions](#functions)
14. [Control Flow](#control-flow)
15. [Rune Blocks](#rune-blocks)
16. [Optimize Block](#optimize-block)
17. [Functional Operations](#functional-operations)
18. [Comments](#comments)
19. [Standard Library](#standard-library)
20. [Complete Examples](#complete-examples)

---

## Introduction

TextCAD is a declarative domain-specific language for constraint-based 2D geometric design. The language enables users to specify geometric entities and their relationships through constraints, with a solver determining concrete values that satisfy all specified constraints.

### Design Principles

The language adheres to several fundamental principles that distinguish it from imperative programming languages:

- **Declarative**: All statements within a scope are declarative, meaning their execution order does not affect the final result
- **Constraint-based**: The solver receives all entities and constraints simultaneously and determines a solution that satisfies the complete constraint system
- **Immutable bindings**: Variables cannot be mutated after initialization; instead, assignments create constraints that the solver must satisfy
- **Entity creation**: New entities are created exclusively through constructor functions called within let statements
- **Reference semantics**: All other operations work with references to existing entities

---

## Core Concepts

### Declarative Constraint System

TextCAD operates as a constraint satisfaction system rather than an imperative program. When you write statements, you are declaring relationships and properties that must hold in the final solution, not specifying a sequence of operations to execute. The order in which you write constraints does not affect the outcome.

### Entities

Entities are geometric objects such as points, lines, circles, and user-defined structures. Each entity exists globally within its scope once created, even if the name used to reference it goes out of scope.

### Constraints

Constraints are equations or inequalities that must be satisfied by the final solution. They can be specified through assignment syntax, function calls, or computed properties.

---

## Type System

### Built-in Types

The language provides several fundamental types that cannot be user-defined.

**bool** represents boolean values for logical constraints.

**i32** represents integer values used for counting and indexing.

**f64** represents floating-point numbers for scale factors and ratios.

**Real** represents mathematical real numbers with exact precision for geometric calculations and constraint solving. Real can be parameterized with units to create dimensioned quantities (see [Unit System](#unit-system)).

**Algebraic** represents algebraic numbers (roots of polynomials with integer coefficients) for exact geometric constructions involving square roots and trigonometric values.

#### Type Characteristics and Performance

**bool** constraints are efficiently handled by Z3's boolean satisfiability algorithms with minimal performance overhead.

**i32** uses exact integer arithmetic with efficient linear integer programming solvers employing cuts and branch-and-bound techniques. No automatic conversion to Real types occurs.

**f64** provides machine floating-point arithmetic for approximate calculations where exact precision is unnecessary. Should be avoided for constraint variables due to rounding error accumulation.

**Real** offers exact mathematical precision ideal for geometric measurements and constraints. Linear real arithmetic is efficiently solvable, but nonlinear real arithmetic can be very expensive and Z3 is not complete for such formulas. When parameterized with units (e.g., `Real<m>`, `Real<m/s>`), the compiler performs compile-time dimensional analysis.

**Algebraic** enables exact representation of irrational solutions from polynomial constraints. Z3 represents these numbers precisely internally while displaying decimal approximations for readability. Suitable for geometric constructions requiring exact roots and trigonometric values.

### Dimensioned Real Types

Real values can be parameterized with units to create dimensioned quantities:

```rust
let distance: Real<m> = 5m;           // Length in meters
let time: Real<s> = 10s;              // Time in seconds
let angle: Real<rad> = 1.57rad;       // Angle in radians
let speed: Real<m/s> = 50m/s;         // Derived unit
let area: Real<m²> = 100m²;           // Squared unit
```

The unit system is fully defined in the standard library, not built into the language. See [Unit System](#unit-system) for complete details.

### User-Defined Types

Users can define custom struct types to encapsulate related geometric entities and computed properties.

```rust
struct Circle {
    center: Point,
    radius: Real<m>,

    fn diameter() -> Real<m> {
        self.radius * 2.0
    }

    fn area() -> Real<m²> {
        PI * self.radius * self.radius
    }
}
```

### Reference Types

Any entity type can be referenced using the ampersand prefix. References point to existing entities rather than creating new ones.

```rust
let center_ref: &Point = circle.get_center();
```

---

## Unit System

TextCAD provides a comprehensive unit system with compile-time dimensional analysis. Unlike traditional approaches, all units are defined in the standard library rather than being built into the language.

### Design Principles

The unit system provides:
- **Compile-time checking**: Unit mismatches are detected during parsing and type checking
- **Automatic conversion**: The solver handles conversions between compatible units
- **Extensibility**: Users can define custom units without language modifications
- **Prefix system**: Standard SI prefixes automatically generate unit variants
- **Natural syntax**: Values written as `10mm`, `45deg`, `2.5h` using suffix notation

### Units as Type Parameters

Units are parameters to the `Real` type, enabling flexible composition:

```rust
struct Point2D {
    x: Real<m>,
    y: Real<m>,
}

struct Velocity2D {
    vx: Real<m/s>,
    vy: Real<m/s>,
}
```

### Unit Prefixes

Unit prefixes are multiplicative factors that combine with base units to form scaled variants.

**Syntax:**
```rust
unit_prefix <char> = <factor>;
```

**Standard SI Prefixes:**
```rust
unit_prefix m = 1e-3;   // milli
unit_prefix c = 1e-2;   // centi
unit_prefix k = 1e3;    // kilo
unit_prefix M = 1e6;    // mega
unit_prefix G = 1e9;    // giga
```

**Prefix Application:**

Prefixes combine with base units automatically:
```rust
unit m;  // Define meter as base unit

// Automatically available through prefix:
// mm = m (prefix) + m (unit) = 1e-3 * meter
// cm = c (prefix) + m (unit) = 1e-2 * meter
// km = k (prefix) + m (unit) = 1e3 * meter
```

**Parsing Rules:**

When the parser encounters a unit token:
1. Check if the complete token is a defined unit (longest match)
2. If not found, attempt to split into prefix + unit
3. Prefix match succeeds only if the remainder is a valid unit

Examples:
- `mm` → prefix `m` + unit `m` = millimeter ✓
- `min` → NOT prefix `m` + unit `in` (because `in` is not defined)
- `min` → direct unit match for "minute" ✓

**Prefix with Powers:**

When a prefixed unit is raised to a power, the prefix factor is also raised:
```rust
mm² = (1e-3 · m)² = 1e-6 · m²
km³ = (1e3 · m)³ = 1e9 · m³
cm^4 = (1e-2 · m)^4 = 1e-8 · m^4
```

### Base Units

Base units are fundamental measurement scales with no conversion factor.

**Syntax:**
```rust
unit <name>;
```

**Standard Base Units:**
```rust
unit m;      // meter (length)
unit s;      // second (time)
unit g;      // gram (mass)
unit rad;    // radian (angle)
unit K;      // kelvin (temperature)
```

Base units define the internal representation for all values of their dimension. All `Real<m>` values are stored in meters internally.

### Derived Units

Derived units are defined with explicit conversion formulas to base units.

**Syntax:**
```rust
unit <name> = <expression> * <base_unit>;
```

**Simple Scalar Conversions:**
```rust
unit inch = 0.0254 * m;
unit ft = 0.3048 * m;
unit mile = 1609.34 * m;

unit min = 60 * s;
unit h = 3600 * s;
unit day = 86400 * s;

unit deg = (PI / 180.0) * rad;
```

**Conversions Referencing Other Units:**

Derived units can reference other derived units:
```rust
unit ft = 12 * inch;        // foot = 12 inches
unit yard = 3 * ft;         // yard = 3 feet
unit mile = 5280 * ft;      // mile = 5280 feet
```

The compiler recursively resolves these to base units.

### Automatic Unit Derivation

The compiler automatically derives compound units from arithmetic operations.

**Multiplication:**
```rust
let width: Real<m> = 10m;
let height: Real<m> = 5m;
let area = width * height;  // Type: Real<m²>
```

**Division:**
```rust
let distance: Real<m> = 100m;
let time: Real<s> = 10s;
let speed = distance / time;  // Type: Real<m/s>
```

**Exponentiation:**
```rust
let side: Real<m> = 5m;
let area = side^2;      // Type: Real<m²>
let volume = side^3;    // Type: Real<m³>
```

**Unit Cancellation:**
```rust
let d1: Real<m> = 100m;
let d2: Real<m> = 50m;
let ratio = d1 / d2;    // Type: Real (dimensionless), Value: 2.0
```

### Power Notation

TextCAD supports two equivalent notations for unit powers.

**Unicode Superscripts:**
```rust
let area: Real<m²> = 100m²;
let volume: Real<m³> = 1000m³;
```

Supported: `²` (U+00B2), `³` (U+00B3)

**Caret Notation:**
```rust
let area: Real<m^2> = 100m^2;
let volume: Real<m^3> = 1000m^3;
let hyper: Real<m^4> = 10m^4;  // Arbitrary positive integer exponents
```

**Equivalence:**
```rust
Real<m²> ≡ Real<m^2>
Real<m³> ≡ Real<m^3>
100mm² ≡ 100mm^2
```

### Unit Conversion

The compiler automatically inserts conversions between compatible units.

**Implicit Conversion in Constraints:**
```rust
let d1: Real<m> = 5m;
let d2: Real<mm> = 300mm;

d1 = d2;  // Constraint with automatic conversion
          // Solver ensures: d1 = 0.3m or d2 = 5000mm
```

**Conversion in Arithmetic:**
```rust
let total: Real<m> = 5m + 300mm;
// Internally: 5m + (300 * 1e-3)m = 5.3m

let diff: Real<inch> = 10inch - 2cm;
// Conversion through base units: cm → m → inch
```

All conversions route through base units for consistency.

### Dimensionless Values

When units cancel through arithmetic, the result is dimensionless using `Real` without a unit parameter.

**Ratios:**
```rust
let d1: Real<m> = 100m;
let d2: Real<m> = 50m;
let ratio: Real = d1 / d2;  // 2.0 (dimensionless)
```

**Scale Factors:**
```rust
let scale: Real = 1.5;
let original: Real<m> = 10m;
let scaled: Real<m> = original * scale;  // 15m
```

**Trigonometric Functions:**
```rust
let angle: Real<rad> = PI / 4 * rad;
let sine_value: Real = sin(angle);  // 0.707... (dimensionless)
```

---

## Variable Declaration and Scoping

### Let Statements

New variables are introduced exclusively through let statements. The let keyword initializes a variable with a value, which may be fully specified or left unconstrained for the solver to determine.

```rust
let p1: Point = point(0mm, 0mm);  // Fully constrained
let p2: Point = point();           // Unconstrained position
let x: Real<m>;                    // Unconstrained length
```

### Container Field Declaration

Variables can be declared as fields of container structs using dot notation:

```rust
let sketch.entities.p1: Point = point(0mm, 0mm);
```

This creates a field `p1` within the `entities` container of the `sketch` object.

### Lexical Scoping

Variables follow lexical scoping rules. A variable declared in a block is visible within that block and any nested blocks, but not outside the declaring block. The language supports shadowing, where an inner scope can redeclare a variable with the same name as one in an outer scope.

```rust
let x: Real<m> = 10mm;

{
    let x: Real<m> = 20mm;  // Shadows outer x
    // Inner x is 20mm here
}

// Outer x is 10mm here
```

### Entity Lifetimes

While variable names are scoped, the entities they refer to have global lifetime within their container or scope. Once an entity is created, it continues to exist until the container completes, even if the name used to reference it goes out of scope. Entities can still be accessed through struct fields or other references.

### Forward References

The declarative nature of the language permits forward references within a scope. You can reference a variable before it is declared, as all declarations and constraints are processed together by the solver.

```rust
p2.x = p1.x + 10mm;  // p1 not yet declared

let p1: Point = point(0mm, 0mm);
let p2: Point = point();
```

---

## Assignment Semantics

### Initialization vs Constraint

The language distinguishes between initialization and constraint application. In a let statement, the equals sign performs initialization, setting a direct value. Outside of let statements, the equals sign creates a constraint that the solver must satisfy.

```rust
let x: Real<m> = 10mm;  // Initialization: x IS 10mm
x = 20mm;               // Constraint: x MUST equal 20mm (conflict!)

let y: Real<m>;   // Unconstrained
y = 30mm;         // Constraint: y MUST equal 30mm (valid)
```

### Constraint Conflicts

When multiple constraints are applied to the same variable and they conflict, the solver will report that the system is unsatisfiable. This occurs when constraints are mathematically incompatible.

```rust
let width: Real<m> = 100mm;  // width IS 100mm
width = 120mm;               // Constraint conflict: solver error
```

### Constrained Copy

When assigning one entity to another, the system creates constraints that keep them synchronized rather than copying values.

```rust
let p1: Point = point(10mm, 20mm);
let p2: Point = point();

p2 = p1;  // Creates constraints: p2.x = p1.x, p2.y = p1.y

p1.x = 15mm;  // Constraint on p1
// Solver ensures p2.x = 15mm due to constraint
```

---

## Entities vs References

### Entity Creation

Functions that return entity types without the ampersand prefix create new entities each time they are called. These functions must be invoked within a let statement or other constructor context such as struct literals or array literals.

```rust
struct Circle {
    center: Point,
    radius: Real<m>,

    // Creates NEW point each time called
    fn point_on_border() -> Point {
        let p: Point = point();
        distance(&self.center, &p) = self.radius;
        p
    }
}

let c: Circle = Circle { center: point(0mm, 0mm), radius: 50mm };
let p1: Point = c.point_on_border();  // New entity
let p2: Point = c.point_on_border();  // Different new entity
```

### Reference Return

Functions that return reference types (prefixed with ampersand) always return the same entity when called multiple times. These functions provide access to existing entities without creating new ones.

```rust
struct Line {
    start: Point,
    end: Point,

    // Returns reference to existing start point
    fn get_start() -> &Point {
        &self.start
    }
}

let line: Line = Line {
    start: point(0mm, 0mm),
    end: point(10mm, 10mm)
};

let start1: &Point = line.get_start();
let start2: &Point = line.get_start();
// start1 and start2 refer to the SAME entity
```

### Function Parameters

All function parameters for entity types must be references. This enables constraints to be applied to parameter values.

```rust
fn distance(p1: &Point, p2: &Point) -> Real<m> {
    sqrt((p2.x - p1.x)^2 + (p2.y - p1.y)^2)
}

fn create_circle(center: &Point, radius: &Real<m>) -> Circle {
    Circle {
        center: center,   // Creates constraint
        radius: *radius,  // Dereferences for value
    }
}
```

This design allows constraints on parameters to propagate correctly through function calls.

---

## Structs

### Definition

Structs group related fields and provide methods for computation and entity creation.

```rust
struct Rectangle {
    center: Point,
    width: Real<m>,
    height: Real<m>,
    rotation: Real<rad>,

    fn area() -> Real<m²> {
        self.width * self.height
    }

    fn corner(index: &i32) -> Point {
        // Implementation that creates new point at corner
    }
}
```

### Field Types

Struct fields can be owned entities, references to entities, or primitive values.

```rust
struct Line {
    start: Point,  // Owned point
    end: Point,    // Owned point
}

struct LineRef {
    start: &Point,  // Reference to external point
    end: &Point,    // Reference to external point
}
```

### Initialization

Struct initialization can specify all fields, some fields, or use computed properties as constraints.

```rust
// Full specification
let rect1: Rectangle = Rectangle {
    center: point(0mm, 0mm),
    width: 100mm,
    height: 50mm,
    rotation: 0deg,
};

// Partial specification (other fields unconstrained)
let rect2: Rectangle = Rectangle {
    width: 100mm,
};

// Using computed properties as constraints
let rect3: Rectangle = Rectangle {
    center: point(0mm, 0mm),
    area() = 5000mm²,  // width * height must equal 5000mm²
    rotation: 0deg,
};
```

### Struct Literals as Constructors

Struct literal syntax acts as an implicit constructor for all fields. Each field assignment in the literal creates a new entity (if not already existing) and applies constraints.

```rust
let p1: Point = point(0mm, 0mm);
let p2: Point = point(10mm, 10mm);

let line: Line = Line {
    start: p1,  // Creates constraint: line.start = p1
    end: p2,    // Creates constraint: line.end = p2
};
// line.start and line.end are new entities constrained to p1 and p2
```

---

## Container Structs

### Container Declaration

A struct can contain a single container field using the `container` keyword. This field acts as a namespace for dynamically added entities during `with` statements.

```rust
struct Sketch {
    container entities,  // Container for dynamically added entities
    origin: Point,       // Regular field
    scale: f64,
}
```

A struct may have at most one container field. Regular fields and the container field exist in separate namespaces.

### Container Field Access

Entities within a container are accessed using standard dot notation:

```rust
let sketch: Sketch = Sketch {
    origin: point(0mm, 0mm),
    scale: 1.0,
};

// Add entity to container directly
let sketch.entities.p1: Point = point(10mm, 10mm);

// Access from outside
sketch.entities.p1.x = 15mm;
```

### Container Semantics

The container field provides:

1. **Dynamic entity addition**: Entities can be added to the container at any point after the struct is created
2. **Namespace isolation**: Container entities are separate from regular struct fields
3. **Full entity access**: All entities in the container can be accessed and constrained from outside the container
4. **Multiple additions**: Multiple `with` blocks or direct declarations can add entities to the same container

```rust
let sketch: Sketch = Sketch { origin: point(0mm, 0mm), scale: 1.0 };

// First addition
let sketch.entities.p1: Point = point(0mm, 0mm);

// Second addition
let sketch.entities.p2: Point = point(10mm, 10mm);

// Access both
distance(&sketch.entities.p1, &sketch.entities.p2) = 20mm;
```

---

## Transform Pattern

### Overview

Structs can define transform methods that specify how to convert entities from one type to another. There are two kinds of transform methods:

- `__transform__`: For external variables (regular fields and standalone variables)
- `__transform_container__`: For container variables (dot-prefix variables in `with` blocks)

### Standard Transform Methods (`__transform__`)

The `__transform__` method applies to external variables - variables that exist outside the container or are regular struct fields:

```rust
struct Translate {
    offset_x: Real<m>,
    offset_y: Real<m>,

    // Transform Point entities for external access
    fn __transform__(p: &Point) -> Point {
        let new_p: Point = point();
        new_p.x = p.x + self.offset_x;
        new_p.y = p.y + self.offset_y;
        new_p
    }
}
```

### Container Transform Methods (`__transform_container__`)

The `__transform_container__` method applies specifically to container variables (variables declared with dot-prefix inside `with` blocks):

```rust
struct Sketch2D {
    container entities,
    origin: Point3D,
    u_axis: Vector3D,
    v_axis: Vector3D,

    // Transform for container variables (dot-prefix)
    fn __transform_container__(p3d: &Point3D) -> Point {
        let local: Vector3D = p3d - self.origin;
        let u: Real<m> = dot(&local, &self.u_axis);
        let v: Real<m> = dot(&local, &self.v_axis);
        point(u, v)
    }

    // Transform for external variables (regular fields)
    fn __transform__(p3d: &Point3D) -> Point3D {
        // No transformation for external access
        p3d
    }
}
```

### Transform Priority Rules

When a struct defines both `__transform__` and `__transform_container__`:

1. **Container variables** (declared with dot-prefix like `.p`, `.line`) use `__transform_container__`
2. **External variables** (regular fields, standalone variables) use `__transform__`

When only `__transform__` is defined:
- It applies to both container and external variables (backwards compatibility)

When only `__transform_container__` is defined:
- It applies only to container variables
- External variables are not transformed

### Use Case Example

```rust
struct Sketch2D {
    container entities,
    origin: Point3D,
    reference_point: Point,  // Regular field

    // For container variables: 3D → 2D projection
    fn __transform_container__(p3d: &Point3D) -> Point {
        point(p3d.x - self.origin.x, p3d.y - self.origin.y)
    }

    // For external variables: no transformation needed
    fn __transform__(p: &Point) -> Point {
        p
    }
}

let sketch: Sketch2D;
sketch.origin.x = 100mm;
sketch.origin.y = 200mm;

with sketch {
    // Container variable: uses __transform_container__
    // Declares a 2D Point, backed by a 3D Point3D
    let .p: Point;
    .p.x = 10mm;  // Sets 3D shadow to (110mm, 210mm, 0mm)
}

// External variable: uses __transform__
sketch.reference_point.x = 50mm;  // Direct access, no transformation
```

### Multiple Transform Methods

A struct can define multiple transform methods for different types, whether standard or container-specific:

```rust
struct Scale {
    factor: f64,
    center: Point,

    fn __transform__(p: &Point) -> Point {
        let new_p: Point = point();
        new_p.x = self.center.x + (p.x - self.center.x) * self.factor;
        new_p.y = self.center.y + (p.y - self.center.y) * self.factor;
        new_p
    }

    fn __transform__(len: &Real<m>) -> Real<m> {
        len * self.factor
    }
}
```

### Type Transformations

Transform methods can change the type of entities, enabling projections between different coordinate systems or dimensions:

```rust
struct Sketch2D {
    origin: Point3D,
    u_axis: Vector3D,  // Local x-axis
    v_axis: Vector3D,  // Local y-axis

    // Transform 3D points to 2D
    fn __transform__(p3d: &Point3D) -> Point {
        let local: Vector3D = p3d - self.origin;
        let u: Real<m> = dot(&local, &self.u_axis);
        let v: Real<m> = dot(&local, &self.v_axis);
        point(u, v)
    }
}
```

### Automatic Nested Transformation

Transformations automatically apply to nested field accesses. If a struct contains fields that have `__transform__` methods defined, accessing those fields triggers the transformation recursively:

```rust
struct Line {
    start: Point,
    end: Point,
}

let shift: Translate = Translate {
    offset_x: 5mm,
    offset_y: 3mm
};

let line: Line = Line {
    start: point(0mm, 0mm),
    end: point(10mm, 10mm)
};

with shift {
    // line.start is automatically transformed
    let p: Point = line.start;
    // p.x = 5mm, p.y = 3mm

    // Nested access also transformed
    line.end.x = 20mm;  // Sets (line.end.x + 5mm) = 20mm
                        // Therefore line.end.x = 15mm in outer context
}
```

---

## With Statements

### Basic Syntax

The `with` statement applies a transform context to all entity accesses within its block:

```rust
let transform: Translate = Translate {
    offset_x: 10mm,
    offset_y: 5mm
};

with transform {
    // All Point accesses are transformed here
    let p: Point = point(0mm, 0mm);
    // p is created at (10mm, 5mm) in global coordinates
}
```

### Container Context

When used with container structs, `with` statements enable convenient entity creation within the container using the dot prefix:

```rust
struct Sketch {
    container entities,
    origin: Point,
}

let sketch: Sketch = Sketch {
    origin: point(100mm, 50mm)
};

with sketch {
    // Local variable (not added to container)
    let temp: Real<m> = 10mm;

    // Container entity (added to sketch.entities)
    let .p1: Point = point(0mm, 0mm);
    // Equivalent to: let sketch.entities.p1: Point = point(0mm, 0mm);

    // Access existing container entity
    let .p2: Point = point(.p1.x + temp, .p1.y);
    // .p1 is equivalent to sketch.entities.p1 inside the block
}

// Access from outside
sketch.entities.p1.x = 15mm;
```

### Dot Prefix Semantics

Within a `with` block, the dot prefix (`.`) references the container field of the context struct:

- `let .name: Type = value;` creates a new field in the container
- `.name` accesses an existing field in the container
- Without the dot prefix, variables are local to the block

```rust
with sketch {
    let local: Point = point(0mm, 0mm);  // Local variable
    let .stored: Point = local;           // Stored in container

    .stored.x = 10mm;  // Constraints sketch.entities.stored
    local.x = 5mm;     // Constraints local variable only
}

// local is out of scope here
// sketch.entities.stored is accessible
```

### Nested With Statements

With statements can be nested. The innermost context takes precedence:

```rust
let outer: Sketch = Sketch { origin: point(0mm, 0mm) };
let inner: Sketch = Sketch { origin: point(50mm, 50mm) };

with outer {
    let .p1: Point = point(0mm, 0mm);  // outer.entities.p1

    with inner {
        let .p2: Point = point(0mm, 0mm);  // inner.entities.p2
        let .p3: Point = .p2;              // inner.entities.p3 = inner.entities.p2

        // Access outer context explicitly
        let .p4: Point = outer.entities.p1;
    }

    let .line: Line = Line {
        start: .p1,
        end: inner.entities.p2
    };
}
```

### Transform Application in With

If the context struct has `__transform__` methods, they are automatically applied to all matching entity accesses:

```rust
struct Sketch {
    container entities,
    origin: Point,

    fn __transform__(p: &Point) -> Point {
        let new_p: Point = point();
        new_p.x = p.x + self.origin.x;
        new_p.y = p.y + self.origin.y;
        new_p
    }
}

let base: Point = point(10mm, 20mm);

let sketch: Sketch = Sketch {
    origin: point(100mm, 50mm)
};

with sketch {
    // base is transformed when accessed
    let .p: Point = base;
    // .p = point(110mm, 70mm) in global coordinates

    // Constraints are also transformed
    base.x = 50mm;  // Actually constrains (base.x + 100mm) = 50mm
                    // Therefore base.x = -50mm in outer context
}
```

---

## Arrays

### Declaration

Arrays have fixed sizes known at parse time. The size must be a constant literal.

```rust
let points: [Point; 5] = [];  // Array of 5 unconstrained points
let values: [Real<m>; 3] = [10mm, 20mm, 30mm];  // Initialized array
```

### Array Literals

Array literals create new entities for each element, with constraints applied as specified.

```rust
let p1: Point = point(0mm, 0mm);
let p2: Point = point(10mm, 0mm);
let p3: Point = point(20mm, 0mm);

let points: [Point; 3] = [p1, p2, p3];
// Creates new entities points[0], points[1], points[2]
// with constraints to p1, p2, p3
```

### Indexing

Array elements are accessed using bracket notation with zero-based indexing.

```rust
let points: [Point; 5] = [];
points[0] = point(0mm, 0mm);
points[1].x = 10mm;
```

### Implicit Range Arrays

Range syntax creates arrays of integers for iteration and indexing.

```rust
let indices: [i32; 5] = [0..5];  // [0, 1, 2, 3, 4]
let range: [i32; 10] = [0..10];  // [0, 1, 2, ..., 9]
```

---

## Functions

### Function Definition

Functions are defined within structs or at the top level. They specify parameter types (with references for entities) and return types.

```rust
fn distance(p1: &Point, p2: &Point) -> Real<m> {
    sqrt((p2.x - p1.x)^2 + (p2.y - p1.y)^2)
}

struct Circle {
    center: Point,
    radius: Real<m>,

    fn circumference() -> Real<m> {
        2.0 * PI * self.radius
    }
}
```

### Return Values

Functions can return primitive values, references to existing entities, or new entities.

```rust
fn computed_value() -> Real<m> {
    10mm * 2.0
}

fn get_reference() -> &Point {
    &self.center
}

fn create_entity() -> Point {
    let p: Point = point(5mm, 5mm);
    p
}
```

### Method Calls

Methods are called using dot notation on struct instances.

```rust
let c: Circle = Circle { center: point(0mm, 0mm), radius: 50mm };
let circ: Real<m> = c.circumference();
let center: &Point = c.get_center();
```

---

## Control Flow

### If Statements

If statements enable conditional constraints based on boolean expressions. The solver determines values that satisfy the constraints in the active branch.

```rust
let x: i32;
let y: i32;

x > 10;

if x > 20 {
    y = x * 2;
} else {
    y = x + 5;
}
// Solver finds values where: x > 10 AND (x > 20 → y = x*2) AND (x ≤ 20 → y = x+5)
```

### If Statement Syntax

If statements consist of a condition expression and a then-branch, with an optional else-branch:

```rust
// If without else
if condition {
    // Constraints that apply when condition is true
    constraint1;
    constraint2;
}

// If with else
if condition {
    // Constraints when condition is true
    constraint1;
} else {
    // Constraints when condition is false
    constraint2;
}
```

### Conditional Constraints

If statements are translated to conditional constraints in the solver (Z3's if-then-else or ITE). Unlike imperative programming, both branches are visible to the solver, which determines which constraints apply based on the condition.

```rust
struct Point {
    x: i32,
    y: i32,
}

let p: Point;
let quadrant: i32;

// Determine quadrant based on coordinates
if p.x >= 0 {
    if p.y >= 0 {
        quadrant = 1;  // Upper-right
    } else {
        quadrant = 4;  // Lower-right
    }
} else {
    if p.y >= 0 {
        quadrant = 2;  // Upper-left
    } else {
        quadrant = 3;  // Lower-left
    }
}

// Additional constraint
quadrant = 1;
// Solver determines p.x >= 0 AND p.y >= 0
```

### If Statement Limitations

Current implementation restrictions:

- Variable declarations are not allowed inside if-statement branches (only constraints)
- Assignments inside if-statements create conditional constraints (not mutations)
- If-statements can be nested

```rust
// INVALID: Variable declaration in branch
if condition {
    let x: i32 = 5;  // Error: Not supported
}

// VALID: Constraint on existing variable
let x: i32;
if condition {
    x = 5;  // OK: Conditional constraint
}
```

### Deferred Context Restriction

The `optimize { }` block (see [Optimize Block](#optimize-block)) is restricted to the top level of a program and cannot appear inside `for` loops, `if` statements, `with` blocks, or function bodies. Placing an `optimize { }` block in any deferred context is a semantic error.

### For Loops

For loops iterate over ranges or arrays to apply constraints to multiple elements.

```rust
// Range iteration
for i in 0..5 {
    points[i] = point(i * 10mm, 0mm);
}

// Array iteration (elements are references)
for p in points {
    p.y >= 0mm;  // Constraint on each element
}
```

Loop variables are scoped to the loop body and cannot be accessed outside. The range bounds must be constant literals in the current version.

### Loop Semantics

Each iteration of a for loop creates independent constraints. Loops cannot be used for accumulation due to the constraint-based assignment semantics. Use map and reduce operations for accumulation instead.

```rust
// INVALID: Accumulation in loop
let sum: Real<m> = 0mm;
for i in 0..5 {
    sum = sum + points[i].x;  // Creates conflicting constraints!
}

// VALID: Use reduce
let sum: Real<m> = [0..5]
    .map(|i| points[i].x)
    .reduce(0mm, |acc, x| acc + x);
```

---

## Rune Blocks

### Overview

Rune blocks enable imperative computations within the declarative constraint-based language. While TextCAD excels at expressing geometric relationships through constraints, some calculations are more naturally expressed as imperative code with sequential steps, loops with accumulation, or complex conditional logic.

Rune blocks use the Rune scripting language (a Rust-like syntax) to perform computations that would be difficult or impossible to express as pure constraints.

### Motivation

Rune blocks are useful for:

- **Complex calculations**: Algorithms that are difficult to express as constraints (e.g., iterative algorithms, accumulation)
- **Mathematical functions**: Trigonometric calculations, numerical methods, series computations
- **Conditional logic**: Imperative if/else branches with side effects (not constraint-based conditionals)
- **External computations**: Calling external libraries or performing operations outside the constraint domain

### Basic Syntax

A rune block is declared using the `rune` keyword followed by a parameter list and a block of Rune code:

```rust
let result = rune(param1, param2) {
    // Rune code (Rust-like imperative syntax)
    let x = param1 * 2;
    let y = param2 + 10;
    x + y
};
```

### Parameter Syntax

Rune blocks support two forms of parameters:

**Direct parameter**: The variable is passed directly with the same name:
```rust
let x: f64 = 5.0;
let y = rune(x) {
    x * x  // x is available with the same name
};
```

**Parameter with assignment**: Allows renaming or passing expressions:
```rust
struct Point { x: f64, y: f64 }
let p: Point;
p.x == 10.0;
p.y == 20.0;

// Rename fields for clarity
let distance = rune(px=p.x, py=p.y) {
    (px * px + py * py).sqrt()
};

// Pass constants
let scaled = rune(x=p.x, factor=2.0) {
    x * factor
};
```

Parameter assignments can be:
- Variable names: `x` (direct pass-through)
- Field accesses: `x=p.x` (extract struct field)
- Expressions: `x=p.x * 2.0` (computed value)
- Constants: `x=100` (literal value)

### Execution Model

Rune blocks execute **after constraint solving** for their parameters, similar to for-loop execution:

```
1. Constraint solver determines values for all parameters
2. Once all parameters are known, rune block executes
3. Rune block returns a value
4. Returned value can be used in further constraints
```

**Important constraint direction**:
```rust
let x: f64;
x > 0.0;  // x must be determinable from constraints

let y = rune(x) { x * x };  // y is computed after x is known

let z: f64;
z == y + 10.0;  // y can constrain other variables ✅

// INVALID: y cannot backward-constrain x
y < 100.0;  // Error! x must be fully determined before rune executes
```

The parameter variables must be fully constrained **before** the rune block can execute. The rune block's result can then constrain other variables, but cannot influence its own parameters.

### Type Inference

Rune blocks use implicit type inference. The return type is inferred from the rune code:

```rust
let x: i32 = 42;
let y = rune(x) {
    x * 2  // Returns i32
};

let a: f64 = 3.14;
let b = rune(a) {
    a.sin()  // Returns f64
};
```

### Rune Language Features

Rune blocks can use standard Rune language features:

**Variables and mutations**:
```rust
let result = rune(n) {
    let mut sum = 0;
    for i in 0..n {
        sum += i;
    }
    sum
};
```

**Control flow**:
```rust
let value = rune(x) {
    if x > 10 {
        x * 2
    } else {
        x + 5
    }
};
```

**Standard library functions**:
```rust
let angle_deg: f64;
angle_deg > 0.0;
angle_deg < 90.0;

let radius = rune(angle_deg) {
    use std::f64::consts::PI;
    let rad = angle_deg * PI / 180.0;
    50.0 * rad.sin()
};
```

**Returning struct values**:
```rust
struct Point { x: f64, y: f64 }

let angle: f64;
angle == 45.0;

let p: Point = rune(angle) {
    use std::f64::consts::PI;
    let rad = angle * PI / 180.0;
    let r = 50.0;
    Point {
        x: r * rad.cos(),
        y: r * rad.sin(),
    }
};
```

### Nested Braces

Rune code can contain nested braces for blocks, structs, and control flow:

```rust
let result = rune(x) {
    let y = {
        let temp = x + 5;
        temp * 2
    };

    if y > 20 {
        Point { x: y, y: 0.0 }
    } else {
        Point { x: 0.0, y: y }
    }
};
```

The parser correctly handles nested braces by counting brace depth.

### Examples

**Fibonacci number**:
```rust
let n: i32 = 10;

let fib = rune(n) {
    let mut a = 0;
    let mut b = 1;
    for i in 0..n {
        let temp = a + b;
        a = b;
        b = temp;
    }
    b
};
```

**Polar to Cartesian conversion**:
```rust
struct Point { x: f64, y: f64 }

let radius: f64;
let angle_deg: f64;
radius == 50.0;
angle_deg == 45.0;

let cartesian: Point = rune(r=radius, a=angle_deg) {
    use std::f64::consts::PI;
    let angle_rad = a * PI / 180.0;
    Point {
        x: r * angle_rad.cos(),
        y: r * angle_rad.sin(),
    }
};
```

**Complex geometric calculation**:
```rust
struct Point { x: f64, y: f64 }

let p1: Point;
let p2: Point;
let p3: Point;
p1.x == 0.0;
p1.y == 0.0;
p2.x == 10.0;
p2.y == 0.0;
p3.x == 5.0;
p3.y == 8.66;

// Calculate triangle area using Heron's formula
let area = rune(ax=p1.x, ay=p1.y, bx=p2.x, by=p2.y, cx=p3.x, cy=p3.y) {
    let a = ((bx - cx).powi(2) + (by - cy).powi(2)).sqrt();
    let b = ((ax - cx).powi(2) + (ay - cy).powi(2)).sqrt();
    let c = ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt();
    let s = (a + b + c) / 2.0;
    (s * (s - a) * (s - b) * (s - c)).sqrt()
};
```

**Iterative numerical method**:
```rust
let initial_guess: f64;
initial_guess == 2.0;

// Newton's method for square root of 10
let sqrt_10 = rune(x0=initial_guess) {
    let target = 10.0;
    let mut x = x0;
    for i in 0..10 {
        x = (x + target / x) / 2.0;
    }
    x
};
```

### Additional Examples

For more comprehensive examples, see the `examples/rune_blocks/` directory which includes:
- `fibonacci.cad` - Fibonacci calculation with detailed comments
- `polar_to_cartesian.cad` - Coordinate transformation
- `triangle_area.cad` - Heron's formula implementation
- `newton_method.cad` - Iterative square root calculation
- `geometric_transform.cad` - Complex multi-step transformations
- `README.md` - Complete guide to using rune blocks

### Limitations

Current implementation restrictions:

- **One-way data flow**: Rune results cannot backward-constrain their parameters
- **No entity creation**: Rune blocks cannot create geometric entities (only compute values)
- **Execution timing**: Rune blocks execute after constraint solving, not during
- **Type compatibility**: Parameter and return types must be compatible with CAD-DSL type system

### Integration with Constraint Solving

Rune blocks complement the constraint-based approach:

**Constraints** → Determine parameter values → **Rune blocks** → Compute results → **More constraints** → Use results

```rust
// Step 1: Constraints determine x
let x: f64;
x > 0.0;
x < 10.0;

// Step 2: Rune computes y from x
let y = rune(x) {
    x * x + 2.0 * x + 1.0
};

// Step 3: Use y in further constraints
let z: f64;
z == y / 2.0;
```

---

## Optimize Block

### Overview

The `optimize { }` block instructs the solver to find an optimal solution rather than any satisfying solution. It uses Z3's optimization mode to minimize or maximize numeric expressions subject to the constraints declared in the rest of the program.

Without an `optimize { }` block, the solver finds any assignment of variables that satisfies all constraints. With an `optimize { }` block, the solver finds the assignment that satisfies all constraints **and** is optimal with respect to the declared objectives.

### Syntax

An `optimize { }` block contains one or more ordered `minimize` and `maximize` directives:

```rust
optimize {
    minimize <expr>;
    maximize <expr>;
}
```

Each directive consists of the keyword `minimize` or `maximize`, followed by a numeric expression, followed by a semicolon. The expression must evaluate to a numeric type (`i32` or `f64`).

```rust
let x: i32;
x > 0;
x < 100;

optimize {
    minimize x;
}
// Solver finds x = 1 (smallest value satisfying x > 0 and x < 100)
```

### Semantics

#### Lexicographic Ordering

The order of directives inside an `optimize { }` block is **intentionally significant**. This is a deliberate exception to the otherwise fully declarative, order-independent nature of the language, made explicit by the dedicated block syntax.

Directives are optimized in **lexicographic priority order**: the first directive is the primary objective and is optimized first. Subsequent directives are only optimized when the primary objective is tied (i.e., among all solutions that achieve the optimal value for the primary objective, the solver then optimizes for the secondary objective, and so on).

```rust
let x: i32;
let y: i32;
x >= 0;
y >= 0;
x + y == 10;

optimize {
    minimize x;   // Primary: minimize x first
    maximize y;   // Secondary: among ties for minimum x, maximize y
}
// Solver finds x = 0, y = 10
// (x is minimized to 0; with x fixed at 0, y is maximized to 10)
```

#### Multiple optimize Blocks

Multiple `optimize { }` blocks are valid. The optimizer processes all directives in source order across all blocks, extending the same lexicographic priority list. For clarity, using at most one `optimize { }` block per program is recommended, but multiple blocks are permitted.

```rust
optimize {
    minimize x;  // Priority 1
}

optimize {
    maximize y;  // Priority 2 (same priority list, extended)
}
```

#### Interaction with Constraints

All constraints declared outside the `optimize { }` block remain in force. The solver finds the optimal solution from among all solutions that satisfy the full constraint system. If the constraint system is unsatisfiable, optimization is not attempted and the solver reports UNSAT.

### Type Restrictions

The expression inside each `minimize` or `maximize` directive must evaluate to a numeric type:

- `i32` — integer optimization
- `f64` — floating-point optimization

The following types are **not** allowed as optimization objectives:

- `bool` — boolean values cannot be minimized or maximized
- Struct types — structs have no total numeric order
- Array types — arrays have no total numeric order

```rust
// VALID: numeric expressions
optimize {
    minimize x;           // i32 or f64 variable
    minimize x + y;       // arithmetic expression
    maximize x * 2.0;     // scaled expression
}

// INVALID: non-numeric expressions
optimize {
    minimize flag;        // Error: bool type
    minimize p;           // Error: struct type
    minimize arr;         // Error: array type
}
```

### Top-Level Restriction

`optimize { }` blocks are only allowed at the **top level** of a program. They cannot appear inside:

- `for` loops
- `if` statements
- `with` blocks
- Function bodies

Placing an `optimize { }` block in any of these deferred or nested contexts is a semantic error. This restriction exists because the optimizer must have access to the complete constraint system before solving, and deferred contexts (such as loop iterations or conditional branches) would make the set of active objectives ambiguous.

```rust
// VALID: top-level optimize block
let x: i32;
x > 0;

optimize {
    minimize x;
}

// INVALID: optimize inside a for loop
for i in 0..5 {
    optimize {        // Error: not allowed in for loop
        minimize x;
    }
}

// INVALID: optimize inside an if statement
if x > 10 {
    optimize {        // Error: not allowed in if statement
        minimize x;
    }
}

// INVALID: optimize inside a with block
with sketch {
    optimize {        // Error: not allowed in with block
        minimize x;
    }
}
```

### Implementation Status

`optimize { }` blocks are **fully implemented**. The solver activates Z3's built-in optimization mode when one or more `optimize { }` blocks are present in the program.

### Examples

#### Simple Minimization

Find the smallest positive integer satisfying a constraint:

```rust
let x: i32;
x > 0;
x < 100;

optimize {
    minimize x;
}
// Output: x = 1
```

#### Simple Maximization with Constraints

Maximize a value within a constrained region:

```rust
struct GridPoint {
    x: i32,
    y: i32,
}

let p: GridPoint;
p.x >= 0;
p.y >= 0;
p.x + p.y <= 20;

optimize {
    maximize p.x + p.y;
}
// Output: p.x + p.y = 20 (solver finds one such solution, e.g. p.x = 20, p.y = 0)
```

#### Minimizing Wire Length in a Sketch

Find a point placement that minimizes the total connection distance:

```rust
struct GridPoint {
    x: f64,
    y: f64,
}

let a: GridPoint;
let b: GridPoint;
let midpoint: GridPoint;

// Fixed anchor points
a.x == 0.0;
a.y == 0.0;
b.x == 10.0;
b.y == 0.0;

// Midpoint must lie on or above the x-axis
midpoint.y >= 0.0;

optimize {
    // Minimize total wire length from a → midpoint → b
    minimize (midpoint.x - a.x) * (midpoint.x - a.x) + (midpoint.y - a.y) * (midpoint.y - a.y)
           + (b.x - midpoint.x) * (b.x - midpoint.x) + (b.y - midpoint.y) * (b.y - midpoint.y);
}
// Solver finds midpoint.y = 0, midpoint.x = 5 (shortest path along x-axis)
```

#### Multi-Objective Lexicographic Optimization

Optimize two objectives with defined priority:

```rust
let cost: f64;
let weight: f64;

cost >= 0.0;
weight >= 0.0;
cost + weight == 100.0;

optimize {
    minimize cost;    // Primary objective: minimize cost first
    minimize weight;  // Secondary: among min-cost solutions, minimize weight
}
// Primary objective drives the solution: cost = 0, weight = 100
// (If cost had multiple solutions at 0, weight would further discriminate)
```

---

## Functional Operations

### Map

The map operation transforms each element of an array using a provided function, creating a new array of the same size.

```rust
let points: [Point; 5] = [...];

// Extract x coordinates
let x_coords: [Real<m>; 5] = points.map(|p| p.x);

// Create transformed points
let shifted: [Point; 5] = points.map(|p| {
    let new_p: Point = point();
    new_p.x = p.x + 10mm;
    new_p.y = p.y;
    new_p
});
```

The closure parameter is always a reference to the array element. Map can create new entities within the closure body.

### Reduce

The reduce operation combines all array elements into a single value using an accumulator function.

```rust
let values: [Real<m>; 4] = [10mm, 20mm, 15mm, 25mm];

// Sum all values
let total: Real<m> = values.reduce(0mm, |acc, val| acc + val);

// Find maximum
let max_val: Real<m> = values.reduce(0mm, |acc, val| {
    if val > acc { val } else { acc }
});
```

The reduce operation builds an expression rather than iteratively mutating an accumulator. The result can be constrained like any other value.

### Method Chaining

Map and reduce operations can be chained to create complex computations.

```rust
let circles: [Circle; 5] = [...];

// Total area of all circles
let total_area: Real<m²> = circles
    .map(|c| c.area())
    .reduce(0mm², |acc, a| acc + a);

// Can apply constraints to the result
total_area = 10000mm²;
```

---

## Comments

### Single-Line Comments

Single-line comments begin with two forward slashes and continue to the end of the line.

```rust
// This is a single-line comment
let p: Point = point(0mm, 0mm);  // Comment after code
```

### Multi-Line Comments

Multi-line comments are enclosed between `/*` and `*/` and can span multiple lines.

```rust
/*
 * This is a multi-line comment
 * spanning several lines
 */
let circle: Circle = Circle {
    center: point(0mm, 0mm),
    radius: 50mm
};
```

---

## Standard Library

The standard library provides commonly used structs, functions, and constraint helpers. These are not part of the core language but are expected to be available in most TextCAD environments.

### Unit System

The complete unit system is defined in the standard library:

```rust
// Prefixes
unit_prefix m = 1e-3;   // milli
unit_prefix c = 1e-2;   // centi
unit_prefix k = 1e3;    // kilo
unit_prefix M = 1e6;    // mega
unit_prefix G = 1e9;    // giga

// Length
unit m;                    // meter (base)
unit inch = 0.0254 * m;
unit ft = 0.3048 * m;
unit yard = 0.9144 * m;
unit mile = 1609.34 * m;

// Time
unit s;                    // second (base)
unit min = 60 * s;
unit h = 3600 * s;
unit day = 86400 * s;

// Mass
unit g;                    // gram (base)
unit oz = 28.3495 * g;
unit lb = 453.592 * g;

// Angle
unit rad;                  // radian (base)
unit deg = (PI / 180.0) * rad;
unit arcmin = deg / 60.0;
unit arcsec = deg / 3600.0;

// Temperature
unit K;                    // kelvin (base)

// Volume (custom)
unit liter = 1000 * cm³;
unit gallon = 3785.41 * cm³;
```

### Geometric Primitives

#### Point Constructor

```rust
struct Point {
    x: Real<m>,
    y: Real<m>,
}

fn point(x: Real<m>, y: Real<m>) -> Point  // Fully specified point
fn point() -> Point                        // Unconstrained point
```

#### Distance Function

```rust
fn distance(p1: &Point, p2: &Point) -> Real<m>
```

Calculates the Euclidean distance between two points.

### Mathematical Functions

```rust
fn abs(x: Real<m>) -> Real<m>
fn sqrt(x: f64) -> f64
fn cos(angle: Real<rad>) -> f64
fn sin(angle: Real<rad>) -> f64
fn tan(angle: Real<rad>) -> f64
fn acos(x: f64) -> Real<rad>
fn asin(x: f64) -> Real<rad>
fn atan2(y: f64, x: f64) -> Real<rad>
```

### Array Utilities

```rust
fn sum<T>(array: [T; N]) -> T
fn product<T>(array: [T; N]) -> T
fn min<T>(array: [T; N]) -> T
fn max<T>(array: [T; N]) -> T
fn average(array: [Real<m>; N]) -> Real<m>
```

These functions are implemented using map and reduce operations.

### Geometric Constraints

```rust
fn horizontal(line: &Line)               // Line is horizontal
fn vertical(line: &Line)                 // Line is vertical
fn parallel(l1: &Line, l2: &Line)        // Lines are parallel
fn perpendicular(l1: &Line, l2: &Line)   // Lines are perpendicular
fn coincident(p1: &Point, p2: &Point)    // Points at same location
```

**Note**: These constraint functions operate in the current coordinate system context (affected by `with` statements).

### View Transforms (Standard Library)

The `View` struct is a standard library component that provides coordinate system transformations:

```rust
struct View {
    origin: Point,
    rotation: Real<rad>,
    scale: f64,

    fn __transform__(p: &Point) -> Point {
        // Applies translation, rotation, and scaling
        let rotated_x: Real<m> = (p.x - self.origin.x) * cos(self.rotation) -
                                 (p.y - self.origin.y) * sin(self.rotation);
        let rotated_y: Real<m> = (p.x - self.origin.x) * sin(self.rotation) +
                                 (p.y - self.origin.y) * cos(self.rotation);

        let new_p: Point = point();
        new_p.x = self.origin.x + rotated_x * self.scale;
        new_p.y = self.origin.y + rotated_y * self.scale;
        new_p
    }
}

// Constructor
fn view(origin: Point, rotation: Real<rad>, scale: f64) -> View
fn view() -> View  // Identity view (origin at 0,0, no rotation, scale 1.0)
```

**Usage**:

```rust
let v: View = view(
    origin: point(100mm, 50mm),
    rotation: 45deg,
    scale: 2.0
);

with v {
    let p: Point = point(10mm, 0mm);
    // p is transformed according to view
}
```

### Common Transform Structs (Standard Library)

#### Translate

```rust
struct Translate {
    offset_x: Real<m>,
    offset_y: Real<m>,

    fn __transform__(p: &Point) -> Point {
        let new_p: Point = point();
        new_p.x = p.x + self.offset_x;
        new_p.y = p.y + self.offset_y;
        new_p
    }
}
```

#### Rotate

```rust
struct Rotate {
    center: Point,
    angle: Real<rad>,

    fn __transform__(p: &Point) -> Point {
        let dx: Real<m> = p.x - self.center.x;
        let dy: Real<m> = p.y - self.center.y;

        let new_p: Point = point();
        new_p.x = self.center.x + dx * cos(self.angle) - dy * sin(self.angle);
        new_p.y = self.center.y + dx * sin(self.angle) + dy * cos(self.angle);
        new_p
    }
}
```

#### Scale

```rust
struct Scale {
    center: Point,
    factor: f64,

    fn __transform__(p: &Point) -> Point {
        let new_p: Point = point();
        new_p.x = self.center.x + (p.x - self.center.x) * self.factor;
        new_p.y = self.center.y + (p.y - self.center.y) * self.factor;
        new_p
    }

    fn __transform__(len: &Real<m>) -> Real<m> {
        len * self.factor
    }
}
```

---

## Complete Examples

### Common Definitions

The following definitions are assumed to be available from the standard library in all examples below:

```rust
// Point struct
struct Point {
    x: Real<m>,
    y: Real<m>,
}

// Point constructors
fn point(x: Real<m>, y: Real<m>) -> Point {
    Point { x, y }
}

fn point() -> Point {
    Point { x: Real<m>, y: Real<m> }  // Unconstrained
}

// Distance function
fn distance(p1: &Point, p2: &Point) -> Real<m> {
    sqrt((p2.x - p1.x)^2 + (p2.y - p1.y)^2)
}

// Line struct
struct Line {
    start: Point,
    end: Point,
}

// Trigonometric functions
fn cos(angle: Real<rad>) -> f64 { /* ... */ }
fn sin(angle: Real<rad>) -> f64 { /* ... */ }
fn sqrt(x: f64) -> f64 { /* ... */ }
```

### Simple Triangle

This example demonstrates basic point creation and constraints.

```rust
let p1: Point = point(0mm, 0mm);
let p2: Point = point(30mm, 0mm);
let p3: Point = point();

distance(&p1, &p3) = 40mm;
distance(&p2, &p3) = 50mm;

// Solver determines p3 position to satisfy both constraints
// Forms a 3-4-5 right triangle
```

### Regular Hexagon

This example shows array usage and circular positioning.

```rust
let center: Point = point(50mm, 50mm);
let radius: Real<m> = 30mm;

let vertices: [Point; 6] = [];

for i in 0..6 {
    let angle: Real<rad> = (360deg / 6.0) * i;
    vertices[i] = point(
        center.x + radius * cos(angle),
        center.y + radius * sin(angle)
    );
}

// All edges have equal length
for i in 0..6 {
    let next: i32 = (i + 1) % 6;
    distance(&vertices[i], &vertices[next]) = 30mm;
}
```

### Container Struct with Sketch

This example demonstrates container structs and the dot prefix syntax.

```rust
struct Sketch {
    container entities,
    origin: Point,
    scale: f64,

    fn __transform__(p: &Point) -> Point {
        let new_p: Point = point();
        new_p.x = self.origin.x + (p.x * self.scale);
        new_p.y = self.origin.y + (p.y * self.scale);
        new_p
    }
}

let main_sketch: Sketch = Sketch {
    origin: point(100mm, 50mm),
    scale: 1.0
};

with main_sketch {
    // Create entities in the container
    let .p1: Point = point(0mm, 0mm);
    let .p2: Point = point(10mm, 0mm);
    let .p3: Point = point(5mm, 8.66mm);

    // Local variable (not in container)
    let side_length: Real<m> = 10mm;

    // Constraints
    distance(&.p1, &.p2) = side_length;
    distance(&.p2, &.p3) = side_length;
    distance(&.p3, &.p1) = side_length;
}

// Access from outside
main_sketch.entities.p1.x = 5mm;
```

### Kinematic Chain

This example demonstrates linked structures with references.

```rust
struct Link {
    start: Point,
    length: Real<m>,
    angle: Real<rad>,

    fn end() -> &Point {
        let end_point: Point = point();
        end_point.x = self.start.x + self.length * cos(self.angle);
        end_point.y = self.start.y + self.length * sin(self.angle);
        &end_point
    }
}

let link1: Link = Link {
    start: point(0mm, 0mm),
    length: 100mm,
    angle: 0deg,
};

let link2: Link = Link {
    start: link1.end(),
    length: 80mm,
    angle: 45deg,
};

let link3: Link = Link {
    start: link2.end(),
    length: 60mm,
    angle: 90deg,
};

// Constrain final position
let final_pos: &Point = link3.end();
final_pos.x = 150mm;
final_pos.y = 100mm;

// Solver determines angles to reach target
```

### Nested Transforms

This example shows how transforms compose through nesting.

```rust
let shift: Translate = Translate {
    offset_x: 50mm,
    offset_y: 30mm
};

let rotation: Rotate = Rotate {
    center: point(0mm, 0mm),
    angle: 45deg
};

let base_point: Point = point(10mm, 0mm);

with shift {
    with rotation {
        // base_point is first rotated, then translated
        let transformed: Point = base_point;
        // Result: rotated 45° then shifted by (50mm, 30mm)
    }
}
```

### Gear Pair

This example shows struct composition and constraint-based design.

```rust
struct Gear {
    center: Point,
    pitch_radius: Real<m>,
    tooth_count: i32,

    fn module() -> Real<m> {
        (self.pitch_radius * 2.0) / self.tooth_count
    }
}

let gear1: Gear = Gear {
    center: point(0mm, 0mm),
    pitch_radius: 50mm,
    tooth_count: 20,
};

let gear2: Gear = Gear {
    center: point(),
    tooth_count: 12,
};

// Gears must touch
distance(&gear1.center, &gear2.center) =
    gear1.pitch_radius + gear2.pitch_radius;

// Same module (tooth size)
gear1.module() = gear2.module();

// Solver determines gear2 radius and position
```

### Polygon with Map/Reduce

This example shows functional operations for complex calculations.

```rust
struct Polygon {
    vertices: [Point; 6],

    fn perimeter() -> Real<m> {
        [0..6]
            .map(|i| distance(
                &self.vertices[i],
                &self.vertices[(i + 1) % 6]
            ))
            .reduce(0mm, |acc, d| acc + d)
    }
}

let poly: Polygon = Polygon {
    vertices: [
        point(0mm, 0mm),
        point(10mm, 0mm),
        point(),
        point(),
        point(),
        point(),
    ],
};

// Constraint on total perimeter
poly.perimeter() = 100mm;

// Regular polygon: all edges equal
let edge_length: Real<m> = 100mm / 6.0;
for i in 0..6 {
    distance(&poly.vertices[i], &poly.vertices[(i + 1) % 6]) = edge_length;
}
```

### 3D to 2D Projection

This example demonstrates type transformation through `__transform__` and `__transform_container__`.

```rust
struct Point3D {
    x: Real<m>,
    y: Real<m>,
    z: Real<m>,
}

struct Sketch2D {
    container entities,
    origin: Point3D,
    u_axis: Vector3D,  // Local x-axis
    v_axis: Vector3D,  // Local y-axis

    // Transform 3D points to 2D for container variables
    fn __transform_container__(p3d: &Point3D) -> Point {
        let local: Vector3D = vector3d(
            p3d.x - self.origin.x,
            p3d.y - self.origin.y,
            p3d.z - self.origin.z
        );
        let u: Real<m> = dot(&local, &self.u_axis);
        let v: Real<m> = dot(&local, &self.v_axis);
        point(u, v)
    }
}

let p3d_1: Point3D = Point3D { x: 10mm, y: 20mm, z: 5mm };
let p3d_2: Point3D = Point3D { x: 15mm, y: 25mm, z: 5mm };

let sketch_plane: Sketch2D = Sketch2D {
    origin: Point3D { x: 0mm, y: 0mm, z: 5mm },
    u_axis: vector3d(1.0, 0.0, 0.0),
    v_axis: vector3d(0.0, 1.0, 0.0),
};

with sketch_plane {
    // 3D points automatically project to 2D
    let .projected_1: Point = p3d_1;  // (10mm, 20mm) in 2D
    let .projected_2: Point = p3d_2;  // (15mm, 25mm) in 2D

    // Work with 2D projections
    distance(&.projected_1, &.projected_2) = 20mm;
}
```

### Physics Simulation with Units

This example demonstrates automatic unit derivation in physics calculations.

```rust
struct Particle {
    position: Point,
    velocity: Velocity2D,
    mass: Real<kg>,
}

struct Velocity2D {
    vx: Real<m/s>,
    vy: Real<m/s>,
}

let p: Particle = Particle {
    position: Point { x: 0m, y: 0m },
    velocity: Velocity2D { vx: 10m/s, vy: 5m/s },
    mass: 2kg,
};

// Kinetic energy calculation
let speed_squared: Real<m²/s²> =
    p.velocity.vx^2 + p.velocity.vy^2;
let kinetic_energy: Real<kg·m²/s²> =
    0.5 * p.mass * speed_squared;
```

### Angular Mechanics

This example demonstrates angle units and conversions.

```rust
struct RotatingArm {
    length: Real<m>,
    angle: Real<rad>,
    angular_velocity: Real<rad/s>,

    fn tip_position() -> Point {
        Point {
            x: self.length * cos(self.angle),
            y: self.length * sin(self.angle),
        }
    }

    fn tip_velocity() -> Real<m/s> {
        self.length * self.angular_velocity
    }
}

let arm: RotatingArm = RotatingArm {
    length: 1m,
    angle: 45deg,  // Automatically converted to radians
    angular_velocity: 2rad/s,
};

let v: Real<m/s> = arm.tip_velocity();  // 2 m/s
```

### Mixed Unit System Design

This example shows working with multiple unit systems simultaneously.

```rust
struct Motorcycle {
    wheel_diameter: Real<inch>,
    engine_displacement: Real<cm³>,
    top_speed: Real<mile/h>,
    fuel_capacity: Real<liter>,
    weight: Real<kg>,

    fn wheel_circumference() -> Real<m> {
        PI * self.wheel_diameter
    }
}

let bike: Motorcycle = Motorcycle {
    wheel_diameter: 17inch,
    engine_displacement: 600cm³,
    top_speed: 120mile/h,
    fuel_capacity: 15liter,
    weight: 180kg,
};

// All conversions handled automatically
let circumference_metric: Real<m> = bike.wheel_circumference();
let speed_metric: Real<km/h> = bike.top_speed;
```

---

## Appendix: Reserved Keywords

The following keywords are reserved and cannot be used as identifiers:

`struct`, `container`, `fn`, `let`, `for`, `in`, `with`, `if`, `else`, `or`, `and`, `return`, `true`, `false`, `rune`, `optimize`, `minimize`, `maximize`, `unit`, `unit_prefix`

---

## Appendix: Language vs Standard Library

### Language Features

These are built into the language itself:

- **Core Types**: `bool`, `i32`, `f64`, `Real`, `Algebraic`
- **Keywords**: `struct`, `container`, `fn`, `let`, `for`, `in`, `with`, `if`, `else`, `or`, `and`, `return`, `true`, `false`, `rune`, `optimize`, `minimize`, `maximize`, `unit`, `unit_prefix`
- **Syntax**: Struct definitions, function definitions, with statements, for loops, dot prefix notation, unit definitions, optimize blocks, rune blocks
- **Semantics**: Constraint-based assignment, entity vs reference distinction, container semantics, transform pattern, compile-time dimensional analysis, lexicographic multi-objective optimization

### Standard Library Components

These are expected to be provided but are not part of the core language:

- **Unit System**: All units (m, s, g, rad, inch, deg, etc.) and prefixes (milli, centi, kilo, etc.)
- **Geometric Types**: `Point` struct
- **Constructors**: `point()`, `view()`
- **Math functions**: `distance()`, `abs()`, `sqrt()`, `sin()`, `cos()`, `tan()`, `asin()`, `acos()`, `atan2()`
- **Array utilities**: `sum()`, `product()`, `min()`, `max()`, `average()`
- **Constraint helpers**: `horizontal()`, `vertical()`, `parallel()`, `perpendicular()`, `coincident()`
- **Transform structs**: `View`, `Translate`, `Rotate`, `Scale`

---

**End of Specification**
