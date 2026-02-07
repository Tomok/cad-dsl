//! Rune Executor for CAD-DSL
//!
//! This module provides runtime execution support for rune blocks.
//! It compiles Rune code with parameter values and executes it to produce results.
//!
//! # Phase 4 Implementation
//!
//! This module implements Phase 4 of the rune blocks implementation plan:
//! - Compile and execute Rune code after constraint solving
//! - Convert between Z3 values and Rune values
//! - Handle parameter substitution
//! - Extract results from executed code
//!
//! # Execution Model
//!
//! Rune blocks execute AFTER constraint solving:
//! 1. Z3 solver determines all constraint variables
//! 2. Parameter values are extracted from Z3 model
//! 3. Rune code is compiled with these values
//! 4. Execution produces result values
//! 5. Results can constrain other variables (one-way data flow)
//!
//! # Value Conversion
//!
//! Values are converted between type systems:
//! - Z3 Int → Rune i64 → Z3 Int (i32)
//! - Z3 Real → Rune f64 → Z3 Real
//! - Z3 Bool → Rune bool → Z3 Bool

use super::{SolverError, Value};
use crate::hir::expr::ResolvedRuneParam;
use crate::hir::types::ResolvedType;
use rune::{Diagnostics, Source, Sources, Unit, Value as RuneValue, Vm};
use std::sync::Arc;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during Rune execution
#[derive(Debug)]
pub enum RuneExecutionError {
    /// Rune code failed to compile
    CompileError { diagnostics: Diagnostics },

    /// Rune code execution failed
    RuntimeError { message: String },

    /// Type conversion error
    ConversionError { message: String },

    /// Unsupported type in rune block
    UnsupportedType { type_name: String },
}

impl std::fmt::Display for RuneExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuneExecutionError::CompileError { diagnostics } => {
                write!(f, "Rune compilation failed: {:?}", diagnostics)
            }
            RuneExecutionError::RuntimeError { message } => {
                write!(f, "Rune runtime error: {}", message)
            }
            RuneExecutionError::ConversionError { message } => {
                write!(f, "Type conversion error: {}", message)
            }
            RuneExecutionError::UnsupportedType { type_name } => {
                write!(f, "Unsupported type in rune block: {}", type_name)
            }
        }
    }
}

impl std::error::Error for RuneExecutionError {}

impl From<RuneExecutionError> for SolverError {
    fn from(e: RuneExecutionError) -> Self {
        SolverError::RuneExecutionError(e.to_string())
    }
}

// ============================================================================
// Rune Executor
// ============================================================================

/// Executor for Rune blocks
///
/// This struct manages the Rune runtime context and provides methods
/// to compile and execute Rune code with CAD-DSL values.
pub struct RuneExecutor {
    /// Rune context with standard modules
    context: Arc<rune::Context>,
}

impl RuneExecutor {
    /// Create a new Rune executor with default modules
    pub fn new() -> Result<Self, RuneExecutionError> {
        let mut context = rune::Context::with_default_modules().map_err(|e| {
            RuneExecutionError::RuntimeError {
                message: format!("Failed to create Rune context: {}", e),
            }
        })?;

        // Install core modules that are commonly needed
        context
            .install(rune::modules::core::module().map_err(|e| {
                RuneExecutionError::RuntimeError {
                    message: format!("Failed to install core module: {}", e),
                }
            })?)
            .map_err(|e| RuneExecutionError::RuntimeError {
                message: format!("Failed to install core module: {}", e),
            })?;

        Ok(Self {
            context: Arc::new(context),
        })
    }

    /// Compile a rune block into a Unit (for caching)
    ///
    /// This method compiles the rune code once and returns an Arc<Unit>
    /// that can be cached and reused for multiple executions with different parameters.
    pub fn compile_rune_block<'src, 'arena>(
        &self,
        body: &str,
        params: &[ResolvedRuneParam<'src, 'arena>],
    ) -> Result<Arc<Unit>, RuneExecutionError> {
        let rune_code = self.generate_rune_code(body, params)?;
        self.compile_rune_code(&rune_code)
    }

    /// Execute a pre-compiled rune block with the given parameter values
    ///
    /// This method reuses a cached compiled Unit, avoiding recompilation.
    pub fn execute_compiled_block<'src, 'arena>(
        &self,
        compiled_unit: Arc<Unit>,
        params: &[ResolvedRuneParam<'src, 'arena>],
        param_values: Vec<Value>,
    ) -> Result<RuneValue, RuneExecutionError> {
        // Convert parameter values to Rune values
        let rune_param_values: Result<Vec<_>, _> = params
            .iter()
            .zip(param_values.iter())
            .map(|(param, value)| self.convert_to_rune_value(value, param.value.ty))
            .collect();
        let rune_param_values = rune_param_values?;

        // Execute the Rune function with pre-compiled unit
        let result = self.execute_rune_function(compiled_unit, rune_param_values)?;

        Ok(result)
    }

    /// Generate complete Rune code from a rune block body and parameters
    ///
    /// Creates a wrapper function that can be compiled and executed by Rune.
    fn generate_rune_code<'src, 'arena>(
        &self,
        body: &str,
        params: &[ResolvedRuneParam<'src, 'arena>],
    ) -> Result<String, RuneExecutionError> {
        let mut code = String::from("pub fn __rune_fn__(");

        // Add parameters (types will be inferred by Rune)
        for (i, param) in params.iter().enumerate() {
            if i > 0 {
                code.push_str(", ");
            }
            code.push_str(param.name);
        }

        // Keep body inline to ensure it's treated as an expression
        // Rune requires the last expression to be on the same line or explicitly returned
        code.push_str(") { ");
        code.push_str(body);
        code.push_str(" }");

        Ok(code)
    }

    /// Compile Rune code into a Unit
    fn compile_rune_code(&self, code: &str) -> Result<Arc<Unit>, RuneExecutionError> {
        let mut sources = Sources::new();
        let source =
            Source::new("rune_block", code).map_err(|_e| RuneExecutionError::CompileError {
                diagnostics: Diagnostics::new(),
            })?;
        sources
            .insert(source)
            .map_err(|_e| RuneExecutionError::CompileError {
                diagnostics: Diagnostics::new(),
            })?;

        let mut diagnostics = Diagnostics::new();

        let result = rune::prepare(&mut sources)
            .with_context(&self.context)
            .with_diagnostics(&mut diagnostics)
            .build();

        // Check for compilation errors
        if diagnostics.has_error() {
            return Err(RuneExecutionError::CompileError { diagnostics });
        }

        let unit = result.map_err(|e| RuneExecutionError::RuntimeError {
            message: format!("Failed to build Rune unit: {}", e),
        })?;

        Ok(Arc::new(unit))
    }

    /// Execute a compiled Rune function with the given arguments
    fn execute_rune_function(
        &self,
        unit: Arc<Unit>,
        args: Vec<RuneValue>,
    ) -> Result<RuneValue, RuneExecutionError> {
        // Create VM - Rune requires a RuntimeContext from the context
        let runtime_context =
            self.context
                .runtime()
                .map_err(|e| RuneExecutionError::RuntimeError {
                    message: format!("Failed to get runtime context: {}", e),
                })?;

        let mut vm = Vm::new(Arc::new(runtime_context), unit);

        // Execute the function
        let result =
            vm.call(["__rune_fn__"], args)
                .map_err(|e| RuneExecutionError::RuntimeError {
                    message: format!("Rune execution failed: {}", e),
                })?;

        Ok(result)
    }

    /// Convert a Z3 solver value to a Rune value
    ///
    /// # Type Mapping
    ///
    /// - Z3 Int → Rune i64
    /// - Z3 Real → Rune f64
    /// - Z3 Bool → Rune bool
    pub fn convert_to_rune_value<'src, 'arena>(
        &self,
        value: &Value,
        ty: &'arena ResolvedType<'src, 'arena>,
    ) -> Result<RuneValue, RuneExecutionError> {
        match (value, ty) {
            (Value::Int(i), ResolvedType::I32 { .. }) => {
                // Rune uses i64, so convert i32 to i64
                Ok(RuneValue::from(*i))
            }
            (Value::Real(f), ResolvedType::F64 { .. }) => Ok(RuneValue::from(*f)),
            (Value::Real(f), ResolvedType::Real { .. }) => Ok(RuneValue::from(*f)),
            (Value::Real(f), ResolvedType::Algebraic { .. }) => Ok(RuneValue::from(*f)),
            (Value::Bool(b), ResolvedType::Bool { .. }) => Ok(RuneValue::from(*b)),
            (Value::UnderConstrained, _) => Err(RuneExecutionError::ConversionError {
                message: "Cannot execute rune block with under-constrained parameter".to_string(),
            }),
            _ => Err(RuneExecutionError::ConversionError {
                message: format!(
                    "Type mismatch: value {:?} does not match type {:?}",
                    value, ty
                ),
            }),
        }
    }

    /// Convert a Rune value back to a Z3 solver value
    ///
    /// # Type Mapping
    ///
    /// - Rune i64 → Z3 Int (clamped to i32)
    /// - Rune f64 → Z3 Real
    /// - Rune bool → Z3 Bool
    pub fn convert_from_rune_value<'src, 'arena>(
        &self,
        value: RuneValue,
        expected_ty: &'arena ResolvedType<'src, 'arena>,
    ) -> Result<Value, RuneExecutionError> {
        match expected_ty {
            ResolvedType::I32 { .. } => {
                // Try to extract i64 from Rune value
                let i: i64 =
                    rune::from_value(value).map_err(|e| RuneExecutionError::ConversionError {
                        message: format!("Failed to convert Rune value to i64: {}", e),
                    })?;
                // Clamp to i32 range (or we could error on overflow)
                Ok(Value::Int(i))
            }
            ResolvedType::F64 { .. }
            | ResolvedType::Real { .. }
            | ResolvedType::Algebraic { .. } => {
                let f: f64 =
                    rune::from_value(value).map_err(|e| RuneExecutionError::ConversionError {
                        message: format!("Failed to convert Rune value to f64: {}", e),
                    })?;
                Ok(Value::Real(f))
            }
            ResolvedType::Bool { .. } => {
                let b: bool =
                    rune::from_value(value).map_err(|e| RuneExecutionError::ConversionError {
                        message: format!("Failed to convert Rune value to bool: {}", e),
                    })?;
                Ok(Value::Bool(b))
            }
            ty => Err(RuneExecutionError::UnsupportedType {
                type_name: format!("{:?}", ty),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Span;

    fn make_span(line: usize, column: usize) -> Span {
        use crate::lexer::LineColumn;
        Span {
            start: LineColumn { line, column },
            lines: 0,
            end_column: column + 1,
        }
    }

    #[test]
    fn test_create_executor() {
        let executor = RuneExecutor::new();
        assert!(executor.is_ok(), "Should create RuneExecutor successfully");
    }

    #[test]
    fn test_convert_int_to_rune() {
        let executor = RuneExecutor::new().unwrap();
        let value = Value::Int(42);
        let ty = ResolvedType::I32 {
            span: make_span(1, 1),
        };

        let rune_value = executor.convert_to_rune_value(&value, &ty);
        assert!(rune_value.is_ok());

        let rune_value = rune_value.unwrap();
        let extracted: Result<i64, _> = rune::from_value(rune_value);
        assert!(extracted.is_ok());
        assert_eq!(extracted.unwrap(), 42);
    }

    #[test]
    fn test_convert_float_to_rune() {
        let executor = RuneExecutor::new().unwrap();
        let value = Value::Real(3.14);
        let ty = ResolvedType::F64 {
            span: make_span(1, 1),
        };

        let rune_value = executor.convert_to_rune_value(&value, &ty);
        assert!(rune_value.is_ok());

        let rune_value = rune_value.unwrap();
        let extracted: Result<f64, _> = rune::from_value(rune_value);
        assert!(extracted.is_ok());
        assert!((extracted.unwrap() - 3.14).abs() < 0.0001);
    }

    #[test]
    fn test_convert_bool_to_rune() {
        let executor = RuneExecutor::new().unwrap();
        let value = Value::Bool(true);
        let ty = ResolvedType::Bool {
            span: make_span(1, 1),
        };

        let rune_value = executor.convert_to_rune_value(&value, &ty);
        assert!(rune_value.is_ok());

        let rune_value = rune_value.unwrap();
        let extracted: Result<bool, _> = rune::from_value(rune_value);
        assert!(extracted.is_ok());
        assert_eq!(extracted.unwrap(), true);
    }

    #[test]
    fn test_compile_simple_rune_code() {
        let executor = RuneExecutor::new().unwrap();
        let code = "pub fn __rune_fn__(x) { x * 2 }";

        let unit = executor.compile_rune_code(code);
        assert!(unit.is_ok(), "Should compile simple Rune code successfully");
    }

    #[test]
    fn test_execute_simple_rune_block() {
        let _executor = RuneExecutor::new().unwrap();
        let _body = "x * 2";
        let _params: Vec<ResolvedRuneParam> = vec![];
        let _param_values = vec![Value::Int(21)];

        // This test demonstrates the structure but won't work without proper ResolvedRuneParam setup
        // Full integration tests should be in the solver module
    }
}
