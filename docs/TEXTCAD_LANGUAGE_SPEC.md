# TextCAD Domain-Specific Language Specification

**Version:** 1.0 (MVP)  
**Date:** 2025-01-20  
**Status:** Draft

## Table of Contents

1. [Introduction](#introduction)
2. [Core Concepts](#core-concepts)
3. [Type System](#type-system)
4. [Variable Declaration and Scoping](#variable-declaration-and-scoping)
5. [Assignment Semantics](#assignment-semantics)
6. [Entities vs References](#entities-vs-references)
7. [Structs](#structs)
8. [Arrays](#arrays)
9. [Functions](#functions)
10. [Control Flow](#control-flow)
11. [Functional Operations](#functional-operations)
12. [Views and Coordinate Systems](#views-and-coordinate-systems)
13. [Units](#units)
14. [Comments](#comments)
15. [Import System](#import-system)
16. [Complete Examples](#complete-examples)

---

## Introduction

TextCAD is a declarative domain-specific language for constraint-based 2D geometric design. The language enables users to specify geometric entities and their relationships through constraints, with a solver determining concrete values that satisfy all specified constraints.

### Design Principles

The language adheres to several fundamental principles that distinguish it from imperative programming languages. All statements within a scope are declarative, meaning their execution order does not affect the final result. The solver receives all entities and constraints simultaneously and determines a solution that satisfies the complete constraint system. Variables cannot be mutated after initialization; instead, assignments create constraints that the solver must satisfy. New entities are created exclusively through constructor functions called within let statements, while all other operations work with references to existing entities.

---

## Core Concepts

### Declarative Constraint System

TextCAD operates as a constraint satisfaction system rather than an imperative program. When you write statements, you are declaring relationships and properties that must hold in the final solution, not specifying a sequence of operations to execute. The order in which you write constraints does not affect the outcome.

### Sketches

A sketch represents a complete geometric design. All entity definitions and constraints exist within a sketch scope.

```rust
sketch example_name {
    // Entity definitions and constraints
}
```

### Entities

Entities are geometric objects such as points, lines, circles, and user-defined structures. Each entity exists globally within the sketch once created, even if the name used to reference it goes out of scope.

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

### Lexical Scoping

Variables follow lexical scoping rules. A variable declared in a block is visible within that block and any nested blocks, but not outside the declaring block. The language supports shadowing, where an inner scope can redeclare a variable with the same name as one in an outer scope.

```rust
sketch scoping_example {
    let x: Length = 10mm;
    
    with view1 {
        let x: Length = 20mm;  // Shadows outer x
        // Inner x is 20mm here
    }
    
    // Outer x is 10mm here
}
```

### Entity Lifetimes

While variable names are scoped, the entities they refer to have global lifetime within the sketch. Once an entity is created, it continues to exist until the sketch completes, even if the name used to reference it goes out of scope. Entities can still be accessed through struct fields or other references.

```rust
sketch entity_lifetime {
    let line: Line;
    
    with view1 {
        let p1: Point = point(0mm, 0mm);
        let p2: Point = point(10mm, 10mm);
        
        line = Line { start: p1, end: p2 };
    }
    
    // p1 and p2 names are out of scope
    // But the entities still exist and can be accessed via line.start and line.end
    line.start.x = 5mm;  // Valid constraint
}
```

### Forward References

The declarative nature of the language permits forward references within a scope. You can reference a variable before it is declared, as all declarations and constraints are processed together by the solver.

```rust
sketch forward_reference {
    p2.x = p1.x + 10mm;  // p1 not yet declared
    
    let p1: Point = point(0mm, 0mm);
    let p2: Point = point();
}
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

### Computed References

When an entity is fully determined by other properties and always returns the same logical entity, functions should return references even though the entity might be computed.

```rust
struct Link {
    start: Point,
    length: Length,
    angle: Angle,
    
    // end is fully determined by start, length, angle
    // Returns reference to the (computed) end point
    fn end() -> &Point {
        // Implementation creates constrained end point
        &self._computed_end
    }
}

let link: Link = Link { 
    start: point(0mm, 0mm), 
    length: 100mm, 
    angle: 0deg 
};

let end1: &Point = link.end();
let end2: &Point = link.end();
// end1 and end2 refer to the SAME entity
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

### Automatic Temporaries

When entity-creating functions are called in nested expressions, the compiler automatically generates temporary let statements to hold intermediate results.

```rust
let result: Point = transform2.apply(&transform1.apply(&p));

// Compiler expands to:
// let _temp1: Point = transform1.apply(&p);
// let result: Point = transform2.apply(&_temp1);
```

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

Functions are defined within structs or at the sketch level. They specify parameter types (with references for entities) and return types.

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

Loop variables are scoped to the loop body and cannot be accessed outside. The range bounds must be constant literals in the MVP, though extension to const expressions is planned.

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

### Standard Library Helpers

The standard library provides common operations built on map and reduce.

```rust
fn sum<T>(array: [T; N]) -> T {
    array.reduce(T::zero(), |acc, val| acc + val)
}

fn average(array: [Length; N]) -> Length {
    sum(array) / N
}

fn min<T>(array: [T; N]) -> T {
    array.reduce(T::max_value(), |acc, val| {
        if val < acc { val } else { acc }
    })
}

fn max<T>(array: [T; N]) -> T {
    array.reduce(T::min_value(), |acc, val| {
        if val > acc { val } else { acc }
    })
}
```

---

## Views and Coordinate Systems

### View Definition

Views represent coordinate systems with specified origins, rotations, and scales. They are first-class objects created with let statements.

```rust
let native_view: View = view();  // Identity view
let rotated: View = view(rotation: 45deg);
let shifted: View = view(origin: point(100mm, 50mm));
let complex: View = view(
    origin: point(50mm, 50mm),
    rotation: 30deg,
    scale: 2.0
);
```

### View Parameters

View parameters can be dynamic, referencing points or angles determined by the solver.

```rust
let center: Point = point();  // Unconstrained
let angle: Angle;             // Unconstrained

let dynamic_view: View = view(
    origin: center,
    rotation: angle
);

// Later constraints determine center and angle
center.x = 50mm;
angle = 45deg;
```

### With Blocks

The with statement applies a view context to all entities and constraints within its block.

```rust
let v1: View = view(rotation: 45deg);

with v1 {
    // Entities defined here use v1's coordinate system
    let p: Point = point(10mm, 0mm);
    // p is 10mm "right" in v1 (45° rotated in global)
    
    // Constraints defined here are interpreted in v1
    horizontal(line);  // line is horizontal in v1
}
```

### View Coordinate Transformation

When entities are created within a view block, their coordinates are transformed according to the view's origin, rotation, and scale.

```rust
let v: View = view(
    origin: point(50mm, 50mm),
    rotation: 90deg,
    scale: 2.0
);

with v {
    let p: Point = point(10mm, 0mm);
    // In v: p is at (10mm, 0mm)
    // In global: p is at (50mm, 70mm) approximately
    // (50mm origin.x + 0mm*cos(90°)*2.0 - 10mm*sin(90°)*2.0,
    //  50mm origin.y + 0mm*sin(90°)*2.0 + 10mm*cos(90°)*2.0)
}
```

### View-Based Constraints

Constraints defined within a view block are interpreted relative to that view's coordinate system.

```rust
let line: Line = Line { 
    start: point(0mm, 0mm), 
    end: point(10mm, 10mm) 
};

with view(rotation: 45deg) {
    horizontal(line);  
    // line is horizontal in this rotated view
    // → line is at 45° in global coordinates
}
```

### View Nesting

View blocks cannot be nested. Attempting to use a with block inside another with block results in a compilation error.

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

## Import System

### Import Statement

The import statement includes definitions from other files. Imports are planned for future implementation but not included in the MVP.

```rust
import "standard_library.tcad";
import "bolt_pattern.tcad";
```

### Future Scope

The import system will enable code reuse and modular design. Imported definitions will be available at the sketch scope level, allowing reuse of struct definitions, helper functions, and parameterized designs.

---

## Complete Examples

### Simple Triangle

This example demonstrates basic point creation, constraints, and the 3-4-5 right triangle.

```rust
sketch simple_triangle {
    let p1: Point = point(0mm, 0mm);
    let p2: Point = point(30mm, 0mm);
    let p3: Point = point();
    
    distance(&p1, &p3) = 40mm;
    distance(&p2, &p3) = 50mm;
    
    // Solver determines p3 position to satisfy both constraints
}
```

### Regular Hexagon

This example shows array usage, loops, and circular positioning.

```rust
sketch regular_hexagon {
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
}
```

### Kinematic Chain

This example demonstrates linked structures with computed endpoints.

```rust
struct Link {
    start: Point,
    length: Length,
    angle: Angle,
    
    fn end() -> &Point {
        // Returns reference to computed end point
        &self._computed_end
    }
}

sketch kinematic_chain {
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

sketch gear_pair {
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
}
```

### Rotated Rectangle with View

This example demonstrates view-based coordinate systems.

```rust
struct Rectangle {
    center: Point,
    width: Length,
    height: Length,
    rotation: Angle,
    
    fn area() -> Area {
        self.width * self.height
    }
}

sketch rotated_rectangle {
    let rect: Rectangle = Rectangle {
        center: point(0mm, 0mm),
        rotation: 45deg,
    };
    
    // Define constraints in rotated view
    let rect_view: View = view(
        origin: rect.center,
        rotation: rect.rotation
    );
    
    with rect_view {
        // In this view, rectangle appears axis-aligned
        let corner: Point = point(50mm, 30mm);
        // This point is at (50mm, 30mm) in rect_view
        // Which is rotated 45° in global coordinates
    }
    
    rect.area() = 6000mm²;
}
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

sketch polygon_design {
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
}
```

---

## Appendix: Reserved Keywords

The following keywords are reserved and cannot be used as identifiers:

`sketch`, `struct`, `fn`, `let`, `for`, `in`, `with`, `view`, `if`, `else`, `or`, `and`, `import`, `return`, `true`, `false`

---

## Appendix: Built-in Functions

### Geometric Functions

```rust
fn point(x: Length, y: Length) -> Point
fn point() -> Point  // Unconstrained

fn distance(p1: &Point, p2: &Point) -> Length
fn abs(x: Length) -> Length
fn sqrt(x: f64) -> f64

fn cos(angle: Angle) -> f64
fn sin(angle: Angle) -> f64
fn atan2(y: f64, x: f64) -> Angle
```

### Array Functions

```rust
fn sum<T>(array: [T; N]) -> T
fn product<T>(array: [T; N]) -> T
fn min<T>(array: [T; N]) -> T
fn max<T>(array: [T; N]) -> T
fn average(array: [Length; N]) -> Length
```

### Constraint Functions

```rust
fn horizontal(line: &Line)      // Line is horizontal in current view
fn vertical(line: &Line)        // Line is vertical in current view
fn parallel(l1: &Line, l2: &Line)
fn perpendicular(l1: &Line, l2: &Line)
fn coincident(p1: &Point, p2: &Point)  // Points at same location
```

---

**End of Specification**
