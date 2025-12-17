# TextCAD Domain-Specific Language Specification

## Table of Contents

1. [Introduction](#introduction)
2. [Core Concepts](#core-concepts)
3. [Type System](#type-system)
4. [Variable Declaration and Scoping](#variable-declaration-and-scoping)
5. [Assignment Semantics](#assignment-semantics)
6. [Entities vs References](#entities-vs-references)
7. [Structs](#structs)
8. [Container Structs](#container-structs)
9. [Transform Pattern](#transform-pattern)
10. [With Statements](#with-statements)
11. [Arrays](#arrays)
12. [Functions](#functions)
13. [Control Flow](#control-flow)
14. [Functional Operations](#functional-operations)
15. [Units](#units)
16. [Comments](#comments)
17. [Standard Library](#standard-library)
18. [Complete Examples](#complete-examples)

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

**Point** represents a position in 2D space with x and y coordinates expressed as lengths.

**Length** represents a distance measurement in SI base units (meters), with support for convenient constructors for millimeters and centimeters.

**Angle** represents a rotation measurement in SI base units (radians), with support for degree-based constructors.

**Area** represents a derived unit from length multiplication.

**bool** represents boolean values for logical constraints.

**i32** represents integer values used for counting and indexing.

**f64** represents floating-point numbers for scale factors and ratios.

**Real** represents mathematical real numbers with exact precision for geometric calculations and constraint solving.

**Algebraic** represents algebraic numbers (roots of polynomials with integer coefficients) for exact geometric constructions involving square roots and trigonometric values.

#### Type Characteristics and Performance

**Length, Angle, Area** are built on the Real type, providing exact arithmetic without rounding errors. Linear operations are efficiently solved using dual simplex algorithms. However, nonlinear operations (multiplication between unknowns) are computationally expensive and may result in undecidable constraint systems.

**bool** constraints are efficiently handled by Z3's boolean satisfiability algorithms with minimal performance overhead.

**i32** uses exact integer arithmetic with efficient linear integer programming solvers employing cuts and branch-and-bound techniques. No automatic conversion to Real types occurs.

**f64** provides machine floating-point arithmetic for approximate calculations where exact precision is unnecessary. Should be avoided for constraint variables due to rounding error accumulation.

**Real** offers exact mathematical precision ideal for geometric measurements and constraints. Linear real arithmetic is efficiently solvable, but nonlinear real arithmetic can be very expensive and Z3 is not complete for such formulas.

**Algebraic** enables exact representation of irrational solutions from polynomial constraints. Z3 represents these numbers precisely internally while displaying decimal approximations for readability. Suitable for geometric constructions requiring exact roots and trigonometric values.

### User-Defined Types

Users can define custom struct types to encapsulate related geometric entities and computed properties.

```rust
struct Circle {
    center: Point,
    radius: Length,

    fn diameter() -> Length {
        self.radius * 2.0
    }

    fn area() -> Area {
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

## Variable Declaration and Scoping

### Let Statements

New variables are introduced exclusively through let statements. The let keyword initializes a variable with a value, which may be fully specified or left unconstrained for the solver to determine.

```rust
let p1: Point = point(0mm, 0mm);  // Fully constrained
let p2: Point = point();           // Unconstrained position
let x: Length;                     // Unconstrained length
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
let x: Length = 10mm;

{
    let x: Length = 20mm;  // Shadows outer x
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
let x: Length = 10mm;  // Initialization: x IS 10mm
x = 20mm;              // Constraint: x MUST equal 20mm (conflict!)

let y: Length;   // Unconstrained
y = 30mm;        // Constraint: y MUST equal 30mm (valid)
```

### Constraint Conflicts

When multiple constraints are applied to the same variable and they conflict, the solver will report that the system is unsatisfiable. This occurs when constraints are mathematically incompatible.

```rust
let width: Length = 100mm;  // width IS 100mm
width = 120mm;              // Constraint conflict: solver error
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
    radius: Length,

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
fn distance(p1: &Point, p2: &Point) -> Length {
    sqrt((p2.x - p1.x)^2 + (p2.y - p1.y)^2)
}

fn create_circle(center: &Point, radius: &Length) -> Circle {
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
    width: Length,
    height: Length,
    rotation: Angle,

    fn area() -> Area {
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

### Transform Methods

Structs can define `__transform__` methods that specify how to transform entities of specific types. These methods are automatically invoked when accessing entities within `with` statements.

```rust
struct Translate {
    offset_x: Length,
    offset_y: Length,
    
    // Transform Point entities
    fn __transform__(p: &Point) -> Point {
        let new_p: Point = point();
        new_p.x = p.x + self.offset_x;
        new_p.y = p.y + self.offset_y;
        new_p
    }
}
```

### Multiple Transform Methods

A struct can define multiple `__transform__` methods for different types:

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
    
    fn __transform__(len: &Length) -> Length {
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
        let u: Length = dot(&local, &self.u_axis);
        let v: Length = dot(&local, &self.v_axis);
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
    let temp: Length = 10mm;
    
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
let values: [Length; 3] = [10mm, 20mm, 30mm];  // Initialized array
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
fn distance(p1: &Point, p2: &Point) -> Length {
    sqrt((p2.x - p1.x)^2 + (p2.y - p1.y)^2)
}

struct Circle {
    center: Point,
    radius: Length,

    fn circumference() -> Length {
        2.0 * PI * self.radius
    }
}
```

### Return Values

Functions can return primitive values, references to existing entities, or new entities.

```rust
fn computed_value() -> Length {
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
let circ: Length = c.circumference();
let center: &Point = c.get_center();
```

---

## Control Flow

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
let sum: Length = 0mm;
for i in 0..5 {
    sum = sum + points[i].x;  // Creates conflicting constraints!
}

// VALID: Use reduce
let sum: Length = [0..5]
    .map(|i| points[i].x)
    .reduce(0mm, |acc, x| acc + x);
```

---

## Functional Operations

### Map

The map operation transforms each element of an array using a provided function, creating a new array of the same size.

```rust
let points: [Point; 5] = [...];

// Extract x coordinates
let x_coords: [Length; 5] = points.map(|p| p.x);

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
let values: [Length; 4] = [10mm, 20mm, 15mm, 25mm];

// Sum all values
let total: Length = values.reduce(0mm, |acc, val| acc + val);

// Find maximum
let max_val: Length = values.reduce(0mm, |acc, val| {
    if val > acc { val } else { acc }
});
```

The reduce operation builds an expression rather than iteratively mutating an accumulator. The result can be constrained like any other value.

### Method Chaining

Map and reduce operations can be chained to create complex computations.

```rust
let circles: [Circle; 5] = [...];

// Total area of all circles
let total_area: Area = circles
    .map(|c| c.area())
    .reduce(0mm², |acc, a| acc + a);

// Can apply constraints to the result
total_area = 10000mm²;
```

---

## Units

### Length Units

Length values support multiple unit constructors that convert to the SI base unit (meters).

```rust
let a: Length = 1000mm;  // Millimeters
let b: Length = 100cm;   // Centimeters
let c: Length = 1m;      // Meters

// All internally stored as meters
a = b;  // Valid: both equal 1m
```

### Angle Units

Angle values support degree and radian constructors.

```rust
let a: Angle = 180deg;     // Degrees
let b: Angle = PI rad;     // Radians
let c: Angle = 3.14159rad; // Radians

a = b;  // Valid: both equal π radians
```

### Derived Units

Area is derived from length multiplication.

```rust
let width: Length = 10mm;
let height: Length = 20mm;
let area: Area = width * height;  // 200mm²
```

### Unit Enforcement

All geometric coordinates and dimensions must have explicit units. The type system prevents mixing incompatible units.

```rust
let p: Point = point(10mm, 20mm);   // Valid
let x: Length = 10mm;
let y: Length = 20mm;
let p2: Point = point(x, y);        // Valid

// Invalid: missing units
// let p3: Point = point(10, 20);   // Error
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

### Geometric Primitives

#### Point Constructor

```rust
fn point(x: Length, y: Length) -> Point  // Fully specified point
fn point() -> Point                       // Unconstrained point
```

**Language feature**: Point type is built-in
**Standard library**: Constructor functions

#### Distance Function

```rust
fn distance(p1: &Point, p2: &Point) -> Length
```

Calculates the Euclidean distance between two points.

### Mathematical Functions

```rust
fn abs(x: Length) -> Length
fn sqrt(x: f64) -> f64
fn cos(angle: Angle) -> f64
fn sin(angle: Angle) -> f64
fn tan(angle: Angle) -> f64
fn acos(x: f64) -> Angle
fn asin(x: f64) -> Angle
fn atan2(y: f64, x: f64) -> Angle
```

### Array Utilities

```rust
fn sum<T>(array: [T; N]) -> T
fn product<T>(array: [T; N]) -> T
fn min<T>(array: [T; N]) -> T
fn max<T>(array: [T; N]) -> T
fn average(array: [Length; N]) -> Length
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
    rotation: Angle,
    scale: f64,
    
    fn __transform__(p: &Point) -> Point {
        // Applies translation, rotation, and scaling
        let rotated_x: Length = (p.x - self.origin.x) * cos(self.rotation) - 
                                 (p.y - self.origin.y) * sin(self.rotation);
        let rotated_y: Length = (p.x - self.origin.x) * sin(self.rotation) + 
                                 (p.y - self.origin.y) * cos(self.rotation);
        
        let new_p: Point = point();
        new_p.x = self.origin.x + rotated_x * self.scale;
        new_p.y = self.origin.y + rotated_y * self.scale;
        new_p
    }
}

// Constructor
fn view(origin: Point, rotation: Angle, scale: f64) -> View
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
    offset_x: Length,
    offset_y: Length,
    
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
    angle: Angle,
    
    fn __transform__(p: &Point) -> Point {
        let dx: Length = p.x - self.center.x;
        let dy: Length = p.y - self.center.y;
        
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
    
    fn __transform__(len: &Length) -> Length {
        len * self.factor
    }
}
```

---

## Complete Examples

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
let radius: Length = 30mm;

let vertices: [Point; 6] = [];

for i in 0..6 {
    let angle: Angle = (360deg / 6.0) * i;
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
    let side_length: Length = 10mm;
    
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
    length: Length,
    angle: Angle,
    
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
    pitch_radius: Length,
    tooth_count: i32,

    fn module() -> Length {
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

    fn perimeter() -> Length {
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
let edge_length: Length = 100mm / 6.0;
for i in 0..6 {
    distance(&poly.vertices[i], &poly.vertices[(i + 1) % 6]) = edge_length;
}
```

### 3D to 2D Projection

This example demonstrates type transformation through `__transform__`.

```rust
struct Point3D {
    x: Length,
    y: Length,
    z: Length,
}

struct Sketch2D {
    container entities,
    origin: Point3D,
    u_axis: Vector3D,  // Local x-axis
    v_axis: Vector3D,  // Local y-axis
    
    // Transform 3D points to 2D
    fn __transform__(p3d: &Point3D) -> Point {
        let local: Vector3D = vector3d(
            p3d.x - self.origin.x,
            p3d.y - self.origin.y,
            p3d.z - self.origin.z
        );
        let u: Length = dot(&local, &self.u_axis);
        let v: Length = dot(&local, &self.v_axis);
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

---

## Appendix: Reserved Keywords

The following keywords are reserved and cannot be used as identifiers:

`struct`, `container`, `fn`, `let`, `for`, `in`, `with`, `if`, `else`, `or`, `and`, `return`, `true`, `false`

---

## Appendix: Language vs Standard Library

### Language Features

These are built into the language itself:

- **Types**: `Point`, `Length`, `Angle`, `Area`, `bool`, `i32`, `f64`, `Real`, `Algebraic`
- **Keywords**: `struct`, `container`, `fn`, `let`, `for`, `in`, `with`, `if`, `else`, `or`, `and`, `return`, `true`, `false`
- **Syntax**: Struct definitions, function definitions, with statements, for loops, dot prefix notation
- **Semantics**: Constraint-based assignment, entity vs reference distinction, container semantics, transform pattern

### Standard Library Components

These are expected to be provided but are not part of the core language:

- **Constructors**: `point()`, `view()`
- **Math functions**: `distance()`, `abs()`, `sqrt()`, `sin()`, `cos()`, `tan()`, `asin()`, `acos()`, `atan2()`
- **Array utilities**: `sum()`, `product()`, `min()`, `max()`, `average()`
- **Constraint helpers**: `horizontal()`, `vertical()`, `parallel()`, `perpendicular()`, `coincident()`
- **Transform structs**: `View`, `Translate`, `Rotate`, `Scale`

---

**End of Specification**
