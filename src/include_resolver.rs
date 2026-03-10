//! Include directive resolver for CAD-DSL
//!
//! This module resolves `include "path";` directives by loading included files,
//! parsing them, and returning their statements for insertion into the main
//! program's AST.
//!
//! # Lifetime Strategy
//!
//! The AST uses `&'src str` slices that must share a consistent lifetime.
//! To support multiple source files without lifetime conflicts, this module
//! arena-allocates each included file's source string using the same `bumpalo`
//! arena used for HIR nodes. This means `'src = 'arena` for all files.
//!
//! # Deduplication
//!
//! A `HashSet<PathBuf>` of canonicalized file paths tracks which files have
//! already been processed. Files included more than once (directly or
//! transitively) are silently skipped on subsequent occurrences. This also
//! handles circular includes (e.g., A → B → A) without needing a separate
//! error: when B tries to include A, A is already in the visited set.
//!
//! # Usage
//!
//! ```ignore
//! let arena = Bump::new();
//! let mut visited = HashSet::new();
//! // Add the main file to visited so it cannot include itself
//! if let Ok(canonical) = Path::new(file).canonicalize() {
//!     visited.insert(canonical);
//! }
//! let base_dir = Path::new(file).parent().unwrap_or(Path::new("."));
//! let ast = resolve_includes(&arena, ast, base_dir, &mut visited)?;
//! ```

use crate::ast::Stmt;
use crate::lexer;
use crate::parser;
use bumpalo::Bump;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

// ============================================================================
// Error Type
// ============================================================================

/// Errors that can occur during include resolution.
#[derive(Debug)]
pub enum IncludeError {
    /// The included file could not be read from disk.
    FileNotFound { path: PathBuf },
    /// The included file contains a lexing error.
    LexError { path: PathBuf, message: String },
    /// The included file contains a parse error.
    ParseError { path: PathBuf },
}

impl std::fmt::Display for IncludeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IncludeError::FileNotFound { path } => {
                write!(f, "Cannot open include file '{}'", path.display())
            }
            IncludeError::LexError { path, message } => {
                write!(f, "Lexing error in '{}': {}", path.display(), message)
            }
            IncludeError::ParseError { path } => {
                write!(f, "Parse error in '{}'", path.display())
            }
        }
    }
}

// ============================================================================
// Include Resolution
// ============================================================================

/// Resolve all `Stmt::Include` directives in `stmts`, replacing each with the
/// flattened statements from the included file (recursively processed).
///
/// Files already present in `visited` (by canonicalized path) are silently
/// skipped. This handles both duplicate includes and circular includes.
///
/// All included file contents are arena-allocated so their string slices share
/// the `'arena` lifetime with the rest of the AST.
pub fn resolve_includes<'arena>(
    arena: &'arena Bump,
    stmts: Vec<Stmt<'arena>>,
    base_dir: &Path,
    visited: &mut HashSet<PathBuf>,
) -> Result<Vec<Stmt<'arena>>, IncludeError> {
    let mut result = Vec::new();

    for stmt in stmts {
        match stmt {
            Stmt::Include { path, .. } => {
                let include_path = base_dir.join(path);
                // Canonicalize for robust deduplication across relative paths.
                // If canonicalize fails (file doesn't exist yet), use the raw
                // path; fs::read_to_string will produce a clear error below.
                let canonical = include_path
                    .canonicalize()
                    .unwrap_or_else(|_| include_path.clone());

                if visited.contains(&canonical) {
                    // Already processed — skip silently (handles duplicates and
                    // circular includes alike).
                    continue;
                }
                visited.insert(canonical.clone());

                // Read the file and arena-allocate its content so the resulting
                // AST nodes share the 'arena lifetime.
                let raw = std::fs::read_to_string(&include_path).map_err(|_| {
                    IncludeError::FileNotFound {
                        path: include_path.clone(),
                    }
                })?;
                let content: &'arena str = arena.alloc_str(&raw);

                // Lex the included file. Arena-allocate the token slice so it
                // shares the 'arena lifetime, satisfying Chumsky's constraint
                // that the input slice and the source string share one lifetime.
                let raw_tokens = lexer::tokenize(content).map_err(|e| IncludeError::LexError {
                    path: include_path.clone(),
                    message: e.to_string(),
                })?;
                let tokens: &'arena [_] = arena.alloc_slice_clone(&raw_tokens);

                // Parse the included file.
                let included_stmts = parser::parse_program(content, tokens).map_err(|_errors| {
                    IncludeError::ParseError {
                        path: include_path.clone(),
                    }
                })?;

                // Recursively resolve any includes inside the included file.
                let inc_base = canonical.parent().unwrap_or(Path::new("."));
                let resolved = resolve_includes(arena, included_stmts, inc_base, visited)?;
                result.extend(resolved);
            }
            other => result.push(other),
        }
    }

    Ok(result)
}
