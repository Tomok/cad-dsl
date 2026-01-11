//! Recursive Struct Detection for Constraint Solving
//!
//! This module detects recursive struct definitions that would cause infinite
//! expansion when flattening structs into primitive Z3 variables.
//!
//! # Purpose
//!
//! The constraint solver flattens struct types into their primitive fields
//! (e.g., `Point { x: i32, y: i32 }` becomes two variables `point.x` and `point.y`).
//! However, recursive structs would cause infinite expansion:
//!
//! ```text
//! struct Node { value: i32, next: Node }  // Direct recursion - infinite!
//! struct A { field: B }
//! struct B { field: A }  // Indirect recursion - also infinite!
//! ```
//!
//! # Algorithm
//!
//! Uses depth-first search (DFS) with a "currently visiting" set to detect cycles:
//! 1. Start DFS from a struct definition
//! 2. Mark struct as "visiting" (gray)
//! 3. Recursively visit all field types
//! 4. If we encounter a "visiting" struct, we found a cycle
//! 5. Mark struct as "visited" (black) when done
//!
//! # Handling Reference Types
//!
//! Reference types (`&Point`) are transparent for cycle detection - we look through
//! the reference to the underlying type. This is because flattening dereferences
//! the type anyway.

use crate::hir::definitions::StructDefinition;
use crate::hir::types::ResolvedType;
use crate::lexer::Span;
use std::collections::HashSet;
use std::fmt;

// ============================================================================
// Error Types
// ============================================================================

/// Error indicating a recursive struct type that cannot be flattened
#[derive(Debug, Clone, PartialEq)]
pub struct RecursiveStructError<'src> {
    /// Path of struct names forming the cycle (e.g., ["A", "B", "C", "A"])
    pub cycle_path: Vec<&'src str>,

    /// Span of the struct that closes the cycle
    pub span: Span,
}

impl<'src> fmt::Display for RecursiveStructError<'src> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Recursive struct detected at line {}, column {}: {}. Recursive structs cannot be solved.",
            self.span.start.line,
            self.span.start.column,
            self.cycle_path.join(" → ")
        )
    }
}

impl<'src> std::error::Error for RecursiveStructError<'src> {}

// ============================================================================
// Cycle Detection
// ============================================================================

/// Detects if a struct definition contains recursive references
///
/// Returns Ok(()) if the struct is acyclic, or Err with the cycle path if recursive.
///
/// # Examples
///
/// ```text
/// struct Point { x: i32, y: i32 }  // Ok - no recursion
/// struct Line { start: Point, end: Point }  // Ok - nested but not recursive
/// struct Node { value: i32, next: Node }  // Err - direct recursion
/// ```
pub fn detect_cycles<'src, 'arena>(
    struct_def: &'arena StructDefinition<'src, 'arena>,
) -> Result<(), RecursiveStructError<'src>> {
    // Set of struct definitions currently being visited (gray nodes in DFS)
    // Use pointer equality since structs are arena-allocated
    let mut visiting: HashSet<*const StructDefinition<'src, 'arena>> = HashSet::new();

    // Stack tracking the current path (for error reporting)
    let mut path: Vec<&'src str> = Vec::new();

    // Start DFS from the root struct
    visit_struct(struct_def, &mut visiting, &mut path)
}

/// Recursively visit a struct and its field types to detect cycles
fn visit_struct<'src, 'arena>(
    struct_def: &'arena StructDefinition<'src, 'arena>,
    visiting: &mut HashSet<*const StructDefinition<'src, 'arena>>,
    path: &mut Vec<&'src str>,
) -> Result<(), RecursiveStructError<'src>> {
    let struct_ptr = struct_def as *const StructDefinition<'src, 'arena>;

    // Check if we're already visiting this struct (cycle detected!)
    if visiting.contains(&struct_ptr) {
        // Build the cycle path for error message
        let mut cycle_path = path.clone();
        cycle_path.push(struct_def.name); // Close the cycle

        return Err(RecursiveStructError {
            cycle_path,
            span: struct_def.span,
        });
    }

    // Mark this struct as currently visiting (gray)
    visiting.insert(struct_ptr);
    path.push(struct_def.name);

    // Visit all field types
    for &field in &struct_def.fields {
        visit_type(field.field_type, visiting, path)?;
    }

    // Done visiting this struct (mark as black)
    visiting.remove(&struct_ptr);
    path.pop();

    Ok(())
}

/// Visit a type and recursively explore if it's a struct or reference
fn visit_type<'src, 'arena>(
    resolved_type: ResolvedType<'src, 'arena>,
    visiting: &mut HashSet<*const StructDefinition<'src, 'arena>>,
    path: &mut Vec<&'src str>,
) -> Result<(), RecursiveStructError<'src>> {
    match resolved_type {
        // Primitive types - no recursion needed
        ResolvedType::Bool { .. }
        | ResolvedType::I32 { .. }
        | ResolvedType::F64 { .. }
        | ResolvedType::Real { .. }
        | ResolvedType::Algebraic { .. } => Ok(()),

        // Reference type - look through to inner type
        ResolvedType::Reference { inner, .. } => visit_type(*inner, visiting, path),

        // Array type - check element type for recursion
        ResolvedType::Array { element_type, .. } => visit_type(*element_type, visiting, path),

        // User-defined type - recursively visit the struct
        ResolvedType::UserDefined { definition, .. } => visit_struct(definition, visiting, path),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::definitions::FieldDefinition;
    use crate::lexer::{LineColumn, Span};
    use bumpalo::Bump;

    fn test_span() -> Span {
        Span {
            start: LineColumn { line: 1, column: 1 },
            lines: 0,
            end_column: 10,
        }
    }

    #[test]
    fn test_non_recursive_simple_struct() {
        let arena = Bump::new();

        // struct Point { x: i32, y: i32 }
        let x_field = arena.alloc(FieldDefinition::new(
            "x",
            test_span(),
            ResolvedType::I32 { span: test_span() },
            test_span(),
        ));
        let y_field = arena.alloc(FieldDefinition::new(
            "y",
            test_span(),
            ResolvedType::I32 { span: test_span() },
            test_span(),
        ));

        let point = StructDefinition::new(
            "Point",
            test_span(),
            vec![x_field, y_field],
            vec![],
            None,
            test_span(),
        );

        assert!(detect_cycles(&point).is_ok());
    }

    #[test]
    fn test_direct_recursion() {
        let arena = Bump::new();

        // struct Node { value: i32, next: Node }
        // We need to create a placeholder and then update it (normally done in semantic analysis)
        // For this test, we'll create it with the recursive reference

        let node = arena.alloc(StructDefinition::new(
            "Node",
            test_span(),
            vec![],
            vec![],
            None,
            test_span(),
        ));

        let value_field = arena.alloc(FieldDefinition::new(
            "value",
            test_span(),
            ResolvedType::I32 { span: test_span() },
            test_span(),
        ));

        let next_field = arena.alloc(FieldDefinition::new(
            "next",
            test_span(),
            ResolvedType::UserDefined {
                name: "Node",
                definition: node,
                span: test_span(),
            },
            test_span(),
        ));

        // Update node with fields (unsafe mutation for test setup)
        let node_mut = unsafe { &mut *(node as *const StructDefinition as *mut StructDefinition) };
        node_mut.fields = vec![value_field, next_field];

        let result = detect_cycles(node);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.cycle_path, vec!["Node", "Node"]);
    }

    #[test]
    fn test_indirect_recursion() {
        let arena = Bump::new();

        // struct A { field: B }
        // struct B { field: A }

        let a = arena.alloc(StructDefinition::new(
            "A",
            test_span(),
            vec![],
            vec![],
            None,
            test_span(),
        ));

        let b = arena.alloc(StructDefinition::new(
            "B",
            test_span(),
            vec![],
            vec![],
            None,
            test_span(),
        ));

        let a_field = arena.alloc(FieldDefinition::new(
            "field",
            test_span(),
            ResolvedType::UserDefined {
                name: "B",
                definition: b,
                span: test_span(),
            },
            test_span(),
        ));

        let b_field = arena.alloc(FieldDefinition::new(
            "field",
            test_span(),
            ResolvedType::UserDefined {
                name: "A",
                definition: a,
                span: test_span(),
            },
            test_span(),
        ));

        // Update structs with fields
        let a_mut = unsafe { &mut *(a as *const StructDefinition as *mut StructDefinition) };
        a_mut.fields = vec![a_field];

        let b_mut = unsafe { &mut *(b as *const StructDefinition as *mut StructDefinition) };
        b_mut.fields = vec![b_field];

        let result = detect_cycles(a);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.cycle_path, vec!["A", "B", "A"]);
    }

    #[test]
    fn test_nested_non_recursive() {
        let arena = Bump::new();

        // struct Point { x: i32, y: i32 }
        // struct Line { start: Point, end: Point }

        let point = arena.alloc(StructDefinition::new(
            "Point",
            test_span(),
            vec![
                arena.alloc(FieldDefinition::new(
                    "x",
                    test_span(),
                    ResolvedType::I32 { span: test_span() },
                    test_span(),
                )),
                arena.alloc(FieldDefinition::new(
                    "y",
                    test_span(),
                    ResolvedType::I32 { span: test_span() },
                    test_span(),
                )),
            ],
            vec![],
            None,
            test_span(),
        ));

        let line = StructDefinition::new(
            "Line",
            test_span(),
            vec![
                arena.alloc(FieldDefinition::new(
                    "start",
                    test_span(),
                    ResolvedType::UserDefined {
                        name: "Point",
                        definition: point,
                        span: test_span(),
                    },
                    test_span(),
                )),
                arena.alloc(FieldDefinition::new(
                    "end",
                    test_span(),
                    ResolvedType::UserDefined {
                        name: "Point",
                        definition: point,
                        span: test_span(),
                    },
                    test_span(),
                )),
            ],
            vec![],
            None,
            test_span(),
        );

        assert!(detect_cycles(&line).is_ok());
    }

    #[test]
    fn test_reference_type_non_recursive() {
        let arena = Bump::new();

        // struct Node { value: i32, next: &Node }
        // References don't create cycles for flattening purposes

        let node = arena.alloc(StructDefinition::new(
            "Node",
            test_span(),
            vec![],
            vec![],
            None,
            test_span(),
        ));

        let next_type = arena.alloc(ResolvedType::UserDefined {
            name: "Node",
            definition: node,
            span: test_span(),
        });

        let value_field = arena.alloc(FieldDefinition::new(
            "value",
            test_span(),
            ResolvedType::I32 { span: test_span() },
            test_span(),
        ));

        let next_field = arena.alloc(FieldDefinition::new(
            "next",
            test_span(),
            ResolvedType::Reference {
                inner: next_type,
                span: test_span(),
            },
            test_span(),
        ));

        let node_mut = unsafe { &mut *(node as *const StructDefinition as *mut StructDefinition) };
        node_mut.fields = vec![value_field, next_field];

        let result = detect_cycles(node);
        // References still create cycles - they're just transparent wrappers
        assert!(result.is_err());
    }
}
