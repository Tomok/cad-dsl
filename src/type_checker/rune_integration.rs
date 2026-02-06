//! Rune Type Checking Integration
//!
//! This module provides integration with the Rune scripting language for type checking
//! rune blocks. It compiles Rune code with parameter types and infers the return type.
//!
//! # Phase 3 Implementation
//!
//! This module implements Phase 3 of the rune blocks implementation plan:
//! - Compile Rune code with parameter types
//! - Infer return types from compiled Rune code
//! - Map types between CAD-DSL and Rune type systems
//!
//! # Type Mapping
//!
//! CAD-DSL types are mapped to Rune types as follows:
//! - `i32` → `i64` (Rune uses i64 for integers)
//! - `f64` → `f64`
//! - `bool` → `bool`
//! - Structs → TODO (Phase 6)
//! - Arrays → TODO (Phase 6)

use crate::hir::expr::ResolvedRuneParam;
use crate::hir::types::ResolvedType;
use crate::lexer::Span;
use rune::{Diagnostics, Source, Sources};
use std::sync::Arc;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during Rune type checking
#[derive(Debug)]
pub enum RuneTypeCheckError {
    /// Rune code failed to compile
    CompileError { diagnostics: Diagnostics },

    /// Type is not supported in rune blocks yet
    UnsupportedType {
        type_name: String,
        _span: Span,
        message: String,
    },

    /// Failed to extract return type from compiled Rune code
    TypeExtractionError { message: String },
}

impl std::fmt::Display for RuneTypeCheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuneTypeCheckError::CompileError { diagnostics } => {
                write!(f, "Rune compile error: {:?}", diagnostics)
            }
            RuneTypeCheckError::UnsupportedType {
                type_name, message, ..
            } => {
                write!(
                    f,
                    "Unsupported type '{}' in rune block: {}",
                    type_name, message
                )
            }
            RuneTypeCheckError::TypeExtractionError { message } => {
                write!(f, "Failed to extract return type: {}", message)
            }
        }
    }
}

impl std::error::Error for RuneTypeCheckError {}

// ============================================================================
// Rune Type Checker
// ============================================================================

/// Type checker for Rune blocks
///
/// This struct provides functionality to compile Rune code and infer return types.
/// It wraps a Rune context with standard modules installed.
pub struct RuneTypeChecker {
    context: Arc<rune::Context>,
}

impl RuneTypeChecker {
    /// Create a new Rune type checker with default modules
    pub fn new() -> Result<Self, RuneTypeCheckError> {
        let mut context = rune::Context::with_default_modules().map_err(|e| {
            RuneTypeCheckError::TypeExtractionError {
                message: format!("Failed to create Rune context: {}", e),
            }
        })?;

        // Install core modules that are commonly needed
        // Note: Using rune::modules directly instead of rune_modules
        context
            .install(rune::modules::core::module().map_err(|e| {
                RuneTypeCheckError::TypeExtractionError {
                    message: format!("Failed to install core module: {}", e),
                }
            })?)
            .map_err(|e| RuneTypeCheckError::TypeExtractionError {
                message: format!("Failed to install core module: {}", e),
            })?;

        Ok(Self {
            context: Arc::new(context),
        })
    }

    /// Infer the return type of a rune block by compiling it with parameter types
    ///
    /// # Arguments
    ///
    /// * `body` - The raw Rune code body (as string from source)
    /// * `params` - The resolved parameters with their types
    /// * `span` - Source span for error reporting
    ///
    /// # Returns
    ///
    /// The inferred return type and any diagnostics (warnings), or an error if compilation fails
    pub fn infer_return_type<'src, 'arena>(
        &self,
        body: &str,
        params: &[ResolvedRuneParam<'src, 'arena>],
        span: Span,
    ) -> Result<(ResolvedType<'src, 'arena>, Diagnostics), RuneTypeCheckError> {
        // Generate a complete Rune function wrapper
        let rune_code = self.generate_rune_function(body, params)?;

        // Compile the Rune code
        let mut sources = Sources::new();
        let source = Source::new("rune_block", &rune_code).map_err(|e| {
            RuneTypeCheckError::TypeExtractionError {
                message: format!("Failed to create source: {}", e),
            }
        })?;
        sources
            .insert(source)
            .map_err(|e| RuneTypeCheckError::TypeExtractionError {
                message: format!("Failed to insert source: {}", e),
            })?;

        let mut diagnostics = Diagnostics::new();

        let result = rune::prepare(&mut sources)
            .with_context(&self.context)
            .with_diagnostics(&mut diagnostics)
            .build();

        // Check for compilation errors (not warnings)
        if diagnostics.has_error() {
            return Err(RuneTypeCheckError::CompileError { diagnostics });
        }

        let _unit = result.map_err(|e| RuneTypeCheckError::TypeExtractionError {
            message: format!("Failed to build Rune unit: {}", e),
        })?;

        // Phase 3 MVP: For now, we'll do a simple heuristic to infer return type
        // based on the Rune code structure. A more sophisticated approach would
        // use Rune's type inference API once it's stable.
        //
        // Strategy:
        // 1. If all parameters are integers, assume integer return (i32)
        // 2. If any parameter is float, assume float return (f64)
        // 3. Otherwise, default to i32
        //
        // This is a simplification. Phase 3.5 or Phase 4 should implement proper
        // type extraction from Rune's compiled unit.
        let inferred_type = self.infer_type_from_params(params, span);

        // Return both the inferred type and any diagnostics (including warnings)
        Ok((inferred_type, diagnostics))
    }

    /// Generate a complete Rune function from a rune block body and parameters
    ///
    /// This creates a wrapper function that Rune can compile and type-check.
    fn generate_rune_function<'src, 'arena>(
        &self,
        body: &str,
        params: &[ResolvedRuneParam<'src, 'arena>],
    ) -> Result<String, RuneTypeCheckError> {
        let mut code = String::from("pub fn __rune_fn__(");

        // Add parameters with their Rune types
        for (i, param) in params.iter().enumerate() {
            if i > 0 {
                code.push_str(", ");
            }
            code.push_str(param.name);
            code.push_str(": ");
            code.push_str(&self.map_type_to_rune(param.value.ty)?);
        }

        code.push_str(") {\n");
        code.push_str(body);
        code.push_str("\n}");

        Ok(code)
    }

    /// Map a CAD-DSL type to its Rune equivalent
    ///
    /// # Type Mapping
    ///
    /// - `i32` → `i64` (Rune uses i64 for integers)
    /// - `f64` → `f64`
    /// - `bool` → `bool`
    fn map_type_to_rune<'src, 'arena>(
        &self,
        ty: &ResolvedType<'src, 'arena>,
    ) -> Result<String, RuneTypeCheckError> {
        match ty {
            ResolvedType::I32 { .. } => Ok("i64".to_string()),
            ResolvedType::F64 { .. } => Ok("f64".to_string()),
            ResolvedType::Bool { .. } => Ok("bool".to_string()),
            ResolvedType::Real { .. } => {
                // Real numbers can be approximated as f64 in Rune
                Ok("f64".to_string())
            }
            ResolvedType::Algebraic { .. } => {
                // Algebraic numbers can be approximated as f64 in Rune
                Ok("f64".to_string())
            }
            ResolvedType::UserDefined { name, span, .. } => {
                Err(RuneTypeCheckError::UnsupportedType {
                    type_name: name.to_string(),
                    _span: *span,
                    message: "Struct types in rune blocks not yet supported (planned for Phase 6)"
                        .to_string(),
                })
            }
            ResolvedType::Array { span, .. } => Err(RuneTypeCheckError::UnsupportedType {
                type_name: "Array".to_string(),
                _span: *span,
                message: "Array types in rune blocks not yet supported (planned for Phase 6)"
                    .to_string(),
            }),
            ResolvedType::Reference { span, .. } => Err(RuneTypeCheckError::UnsupportedType {
                type_name: "Reference".to_string(),
                _span: *span,
                message: "Reference types in rune blocks not yet supported".to_string(),
            }),
        }
    }

    /// Infer return type from parameter types using a simple heuristic
    ///
    /// This is a temporary solution for Phase 3 MVP. A more sophisticated
    /// approach would extract actual type information from Rune's type system.
    ///
    /// # Heuristic
    ///
    /// - If any parameter is f64, real, or algebraic → return f64
    /// - Otherwise → return i32
    fn infer_type_from_params<'src, 'arena>(
        &self,
        params: &[ResolvedRuneParam<'src, 'arena>],
        span: Span,
    ) -> ResolvedType<'src, 'arena> {
        // Check if any parameter is a floating-point type
        for param in params {
            match param.value.ty {
                ResolvedType::F64 { .. }
                | ResolvedType::Real { .. }
                | ResolvedType::Algebraic { .. } => {
                    return ResolvedType::F64 { span };
                }
                _ => {}
            }
        }

        // Default to i32 for integer-only parameters
        ResolvedType::I32 { span }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::{LineColumn, Span};

    fn make_span(line: usize, column: usize) -> Span {
        Span {
            start: LineColumn { line, column },
            lines: 0,
            end_column: column + 1,
        }
    }

    #[test]
    fn test_rune_type_checker_creation() {
        let checker = RuneTypeChecker::new();
        assert!(
            checker.is_ok(),
            "Should create RuneTypeChecker successfully"
        );
    }

    #[test]
    fn test_map_i32_to_i64() {
        let checker = RuneTypeChecker::new().unwrap();
        let ty = ResolvedType::I32 {
            span: make_span(1, 1),
        };
        let rune_ty = checker.map_type_to_rune(&ty);
        assert!(rune_ty.is_ok());
        assert_eq!(rune_ty.unwrap(), "i64");
    }

    #[test]
    fn test_map_f64_to_f64() {
        let checker = RuneTypeChecker::new().unwrap();
        let ty = ResolvedType::F64 {
            span: make_span(1, 1),
        };
        let rune_ty = checker.map_type_to_rune(&ty);
        assert!(rune_ty.is_ok());
        assert_eq!(rune_ty.unwrap(), "f64");
    }

    #[test]
    fn test_map_bool_to_bool() {
        let checker = RuneTypeChecker::new().unwrap();
        let ty = ResolvedType::Bool {
            span: make_span(1, 1),
        };
        let rune_ty = checker.map_type_to_rune(&ty);
        assert!(rune_ty.is_ok());
        assert_eq!(rune_ty.unwrap(), "bool");
    }

    #[test]
    fn test_compile_simple_rune_code() {
        let checker = RuneTypeChecker::new().unwrap();

        // Simple arithmetic: x * 2
        let body = "x * 2";
        let params = vec![];

        // This should compile successfully (even though it references undefined x)
        // The Rune compiler will handle parameter validation
        let code = checker.generate_rune_function(body, &params);
        assert!(code.is_ok());

        let code = code.unwrap();
        assert!(code.contains("pub fn __rune_fn__()"));
        assert!(code.contains("x * 2"));
    }
}
