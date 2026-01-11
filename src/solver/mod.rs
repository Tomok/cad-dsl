//! New Trait-Based Solver Architecture
//!
//! This module implements a trait-based constraint solver architecture for the CAD-DSL language.
//! Unlike the legacy imperative solver, this version uses the `Solvable` trait pattern where
//! HIR nodes implement their own constraint generation logic.
//!
//! # Architecture Overview
//!
//! The new solver follows these design principles:
//!
//! 1. **Trait-Based**: HIR nodes implement `Solvable` trait for constraint generation
//! 2. **Modular**: Functionality split into focused modules in `impls/` subdirectory
//! 3. **Tree-Based Variables**: Variables organized in a tree structure for scoping
//! 4. **RAII Guards**: Scope management using guard types
//!
//! # Migration Status
//!
//! This solver is currently under development. Phase 1 extracts reusable components
//! from the legacy solver:
//!
//! - `struct_flattener.rs` - Flattens struct/array types to primitive fields
//! - `recursive_struct_detector.rs` - Detects cycles in struct definitions
//! - `solution_formatter.rs` - Formats Z3 solutions for display
//!
//! These modules are shared between legacy and new solver during migration.
//!
//! # Next Steps (Phase 2+)
//!
//! - Define `Solvable` trait
//! - Implement `SolverContext` with tree-based variable management
//! - Create RAII guards for scope management
//! - Implement trait for expressions and statements
//!
//! See `docs/SOLVER_ARCHITECTURE.md` and `docs/MIGRATION_STRATEGY.md` for details.

#![allow(dead_code)] // Module under development

// ============================================================================
// Reusable Components (Phase 1)
// ============================================================================

/// Struct and array field flattening for Z3 variable mapping
pub mod struct_flattener;

/// Recursive struct cycle detection
pub mod recursive_struct_detector;

/// Solution formatting for Z3 models
pub mod solution_formatter;

// ============================================================================
// Public Re-exports (Phase 1)
// ============================================================================

pub use recursive_struct_detector::detect_cycles;
pub use solution_formatter::{SolutionFormatter, SolutionFormatterError};
pub use struct_flattener::flatten_type;

// ============================================================================
// Future Modules (Phase 2+)
// ============================================================================

// TODO: Phase 2 - Core infrastructure
// pub mod context;     // SolverContext, RAII guards
// pub mod traits;      // Solvable trait definition
// pub mod types;       // VariablePath, PathComponent

// TODO: Phase 3 - Trait implementations
// pub mod impls;       // expr.rs, stmt.rs, etc.
