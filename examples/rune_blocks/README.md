# Rune Blocks Examples

This directory contains comprehensive examples demonstrating the capabilities of rune blocks in CAD-DSL.

## What are Rune Blocks?

Rune blocks enable imperative computations within the declarative constraint-based language. They are useful for:
- Complex iterative algorithms (Fibonacci, Newton's method)
- Mathematical formulas that are easier to express imperatively
- Coordinate transformations
- Any computation that's difficult to express as pure constraints

## Syntax

```
let result = rune(param1, param2) {
    // Imperative Rune code goes here
    // Can use control flow, loops, mutable variables
    // Last expression is the return value
};
```

## Parameter Syntax

Rune blocks support flexible parameter binding:

```
rune(x)              // Direct parameter: x = x
rune(x=p.x)          // Rename parameter: x = p.x
rune(x=p.x, y, z=100) // Mixed: rename, direct, constant
```

## Examples

### 1. fibonacci.cad
Computes the nth Fibonacci number using an iterative algorithm.
- Demonstrates: loops, mutable variables, conditional logic
- Run: `cargo run -- solve examples/rune_blocks/fibonacci.cad`

### 2. polar_to_cartesian.cad
Converts polar coordinates (r, theta) to Cartesian coordinates (x, y).
- Demonstrates: trigonometric functions, multiple rune blocks
- Run: `cargo run -- solve examples/rune_blocks/polar_to_cartesian.cad`

### 3. triangle_area.cad
Calculates triangle area using Heron's formula given three side lengths.
- Demonstrates: complex mathematical formulas, multi-step calculations
- Run: `cargo run -- solve examples/rune_blocks/triangle_area.cad`

### 4. newton_method.cad
Uses Newton's method to compute square roots iteratively.
- Demonstrates: numerical methods, iterative refinement
- Run: `cargo run -- solve examples/rune_blocks/newton_method.cad`

### 5. geometric_transform.cad
Applies a series of geometric transformations (rotation, scaling, translation).
- Demonstrates: chaining multiple rune blocks, parameter assignments
- Run: `cargo run -- solve examples/rune_blocks/geometric_transform.cad`

## Execution Model

1. **One-way data flow**: Parameters must be fully determined before execution
2. **Post-constraint solving**: Rune blocks execute after Z3 solves constraints for parameters
3. **Results as constraints**: Rune block results can constrain other variables
4. **No backward constraints**: Results cannot backward-constrain parameters

## Key Features

- **Imperative code**: Use loops, conditionals, mutable variables
- **Type inference**: Return types inferred automatically
- **Parameter flexibility**: Direct binding or expression assignment
- **Standard library**: Access to Rune's standard library (math functions, etc.)
- **Compilation caching**: Rune code is compiled once, executed multiple times

## Limitations

- Parameters must be fully determined before execution
- No bidirectional constraints (one-way only)
- Currently supports primitive types (i32, f64, bool)
- Struct and array support coming in future phases

## Best Practices

1. **Use rune blocks for algorithms difficult to express as constraints**
   - Iterative methods (Newton's method, bisection)
   - Accumulation (sum, product)
   - Complex conditionals

2. **Keep rune blocks focused**
   - Each block should do one thing
   - Chain multiple blocks rather than creating complex monoliths

3. **Document your rune code**
   - Add comments explaining the algorithm
   - Note any mathematical formulas or references

4. **Test with different inputs**
   - Verify behavior with edge cases
   - Check numerical stability for iterative methods

## See Also

- Language specification: `docs/TEXTCAD_LANGUAGE_SPEC.md`
- Implementation plan: `docs/RUNE_BLOCKS_IMPLEMENTATION.md`
- Main CLAUDE.md: Development guidelines and status
