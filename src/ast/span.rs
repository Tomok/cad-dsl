use crate::lexer::Span;

// ============================================================================
// Span Access Trait
// ============================================================================

/// Trait for AST nodes that have span information
pub trait HasSpan {
    /// Returns the span of this AST node
    fn span(&self) -> Span;
}
