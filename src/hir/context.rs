//! With-context tracking for the High-level Intermediate Representation (HIR).
//!
//! This module provides types for tracking context information in CAD-DSL's `with` statements,
//! which are used for both transform contexts and container field initialization contexts.

#![allow(dead_code)] // Planned for constraint generation phase
//!
//! # With-Context Semantics
//!
//! CAD-DSL supports two primary uses of `with` statements:
//!
//! ## Transform Contexts
//!
//! Transform contexts allow implicit transformation of coordinates through a chain of transforms:
//!
//! ```cad
//! with transform {
//!     // Inside this block, coordinates are implicitly transformed
//!     point(10, 20)  // Coordinates transformed by the transform object
//! }
//! ```
//!
//! Transform objects must implement a `__transform__` method that converts coordinates.
//! Multiple transforms can be chained, with each transform's `__transform__` method called
//! in sequence.
//!
//! ## Container Contexts
//!
//! Container contexts provide shorthand syntax for initializing fields of container objects:
//!
//! ```cad
//! with sketch {
//!     let .field1 = value1;
//!     let .field2 = value2;
//! }
//! ```
//!
//! The dot-prefix syntax (`.field`) is only valid inside a `with` statement and refers to
//! fields of the container expression.
//!
//! ## Nested With Statements
//!
//! With statements can be nested, creating a stack of contexts:
//!
//! ```cad
//! with transform1 {
//!     with transform2 {
//!         // Both transforms apply here, transform1 then transform2
//!         point(10, 20)
//!     }
//! }
//! ```
//!
//! The HIR resolver maintains a stack of `WithContext` objects to track the current
//! nesting level and available transforms.

use super::definitions::{ContainerField, FunctionDefinition};
use super::expr::ResolvedExpr;
use super::types::ResolvedType;

/// A with-context tracks the state of a `with` statement during HIR resolution.
///
/// Each `with` statement creates a new context that affects name resolution and
/// implicit operations within its scope. Contexts can be either:
/// - Transform contexts: providing implicit coordinate transformation
/// - Container contexts: enabling dot-prefix field initialization
///
/// # Lifetimes
///
/// - `'src`: The lifetime of the source code string
/// - `'arena`: The lifetime of the arena allocator used for HIR nodes
#[derive(Debug, Clone, PartialEq)]
pub struct WithContext<'src, 'arena> {
    /// The expression that follows the `with` keyword.
    ///
    /// This is the value that provides the context. For transform contexts,
    /// this expression must evaluate to a type with transform methods.
    /// For container contexts, this expression is the container being initialized.
    pub context_expr: &'arena ResolvedExpr<'src, 'arena>,

    /// If this is a container context, this field holds the container's field information.
    ///
    /// When `Some`, dot-prefix field access (`.fieldname`) is allowed within the
    /// with block, referring to fields of this container.
    pub container_field: Option<&'arena ContainerField<'src, 'arena>>,

    /// The chain of transform methods available in this context.
    ///
    /// Transform methods are collected from the context expression's type.
    /// When multiple `with` statements are nested, transforms are applied
    /// in order from outermost to innermost.
    ///
    /// Each transform defines an input type and output type, and the chain
    /// must be compatible (output of one transform matches input of the next).
    pub transforms: Vec<TransformMethod<'src, 'arena>>,
}

/// A transform method provides coordinate transformation within a with-context.
///
/// Transform methods are special functions (typically named `__transform__`) that
/// convert values from one coordinate space to another. They enable implicit
/// transformation of coordinates within `with` blocks.
///
/// # Type Compatibility
///
/// For a chain of transforms to be valid:
/// - Each transform's `output_type` must be compatible with the next transform's `input_type`
/// - The final transform's `output_type` determines what coordinate types can be used
///   in the with block
///
/// # Lifetimes
///
/// - `'src`: The lifetime of the source code string
/// - `'arena`: The lifetime of the arena allocator used for HIR nodes
///   Kind of transform method.
///
/// Transforms can be either standard (for external variables) or
/// container-specific (for dot-prefix variables in with blocks).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformMethodKind {
    /// Standard transform (__transform__) for external variables.
    ///
    /// This transform applies to:
    /// - Regular struct fields accessed from outside
    /// - Standalone variables in transform contexts
    Standard,

    /// Container-specific transform (__transform_container__) for dot-prefix variables.
    ///
    /// This transform applies specifically to variables declared with
    /// dot-prefix syntax inside with blocks (e.g., `.p`, `.line`).
    Container,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransformMethod<'src, 'arena> {
    /// The function definition for this transform method.
    ///
    /// This is typically a `__transform__` method defined on the transform type.
    /// The function takes input coordinates and returns transformed coordinates.
    pub function: &'arena FunctionDefinition<'src, 'arena>,

    /// The input type accepted by this transform.
    ///
    /// Coordinates or values of this type can be passed to the transform function.
    /// In a transform chain, this must match the output type of the previous transform.
    pub input_type: &'arena ResolvedType<'src, 'arena>,

    /// The output type produced by this transform.
    ///
    /// The transform function returns values of this type.
    /// In a transform chain, this must match the input type of the next transform.
    pub output_type: &'arena ResolvedType<'src, 'arena>,

    /// The kind of transform method.
    ///
    /// Determines whether this transform applies to container variables
    /// (dot-prefix in with blocks) or external variables (regular fields).
    pub kind: TransformMethodKind,
}

impl<'src, 'arena> WithContext<'src, 'arena> {
    /// Creates a new transform context with the given expression and transforms.
    ///
    /// # Parameters
    ///
    /// - `context_expr`: The expression providing the transform context
    /// - `transforms`: The chain of transform methods available in this context
    pub fn new_transform(
        context_expr: &'arena ResolvedExpr<'src, 'arena>,
        transforms: Vec<TransformMethod<'src, 'arena>>,
    ) -> Self {
        Self {
            context_expr,
            container_field: None,
            transforms,
        }
    }

    /// Creates a new container context with the given expression and container field.
    ///
    /// # Parameters
    ///
    /// - `context_expr`: The expression providing the container context
    /// - `container_field`: The container field information for dot-prefix access
    pub fn new_container(
        context_expr: &'arena ResolvedExpr<'src, 'arena>,
        container_field: &'arena ContainerField<'src, 'arena>,
    ) -> Self {
        Self {
            context_expr,
            container_field: Some(container_field),
            transforms: Vec::new(),
        }
    }

    /// Returns `true` if this is a transform context (has transforms).
    pub fn is_transform_context(&self) -> bool {
        !self.transforms.is_empty()
    }

    /// Returns `true` if this is a container context (has a container field).
    pub fn is_container_context(&self) -> bool {
        self.container_field.is_some()
    }
}

impl<'src, 'arena> TransformMethod<'src, 'arena> {
    /// Creates a new transform method.
    ///
    /// # Parameters
    ///
    /// - `function`: The transform function definition
    /// - `input_type`: The type accepted by this transform
    /// - `output_type`: The type produced by this transform
    /// - `kind`: The kind of transform (Standard or Container)
    pub fn new(
        function: &'arena FunctionDefinition<'src, 'arena>,
        input_type: &'arena ResolvedType<'src, 'arena>,
        output_type: &'arena ResolvedType<'src, 'arena>,
        kind: TransformMethodKind,
    ) -> Self {
        Self {
            function,
            input_type,
            output_type,
            kind,
        }
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_with_context_is_transform() {
        // This test would require setting up actual HIR types,
        // which will be implemented when the HIR resolver is built.
        // For now, we just verify the module compiles.
    }

    #[test]
    fn test_with_context_is_container() {
        // This test would require setting up actual HIR types,
        // which will be implemented when the HIR resolver is built.
        // For now, we just verify the module compiles.
    }
}
