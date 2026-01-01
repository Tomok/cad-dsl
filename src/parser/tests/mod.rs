//! Parser tests organized by category
//!
//! This module contains comprehensive tests for the parser, split into logical
//! categories to improve compilation times and organization.

// Re-export everything from parent module for test use
use super::*;

// Helper functions shared across all test modules
mod helpers;

// Test modules organized by category
#[cfg(test)]
mod arithmetic; // 33 tests: Arithmetic operators and precedence
#[cfg(test)]
mod atoms; // 9 tests: Literal and variable parsing
#[cfg(test)]
mod comparison; // 16 tests: Comparison operators
#[cfg(test)]
mod errors;
#[cfg(test)]
mod expressions; // 47 tests: Complex expressions (calls, arrays, struct literals)
#[cfg(test)]
mod logical; // 5 tests: Logical operators (and, or)
#[cfg(test)]
mod types_and_spans; // 20 tests: Type annotations and span tracking // 9 tests: Error handling and reporting

// Statement test modules
#[cfg(test)]
mod stmt_control_flow; // 21 tests: For loops and if statements
#[cfg(test)]
mod stmt_declarations; // 37 tests: Let, assignment, field assignment
#[cfg(test)]
mod stmt_definitions; // 19 tests: Function and struct definitions
#[cfg(test)]
mod stmt_other; // 55 tests: Return, expression, block, with statements
