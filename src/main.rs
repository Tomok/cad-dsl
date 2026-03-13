mod ast;
mod include_resolver;
mod lexer;
mod parser;
mod units;

// HIR module
mod hir;

// Semantic analysis module
mod semantic_analyzer;

// Type checking module
mod type_checker;

// Trait-based solver implementation
mod solver;

// Public API
use solver as active_solver;

use active_solver::SolverError;
use ariadne::{Color, Label, Report, ReportKind, Source};
use bumpalo::Bump;
use clap::{Parser, Subcommand};
use lexer::TokenTrait;
use semantic_analyzer::errors::SemanticError;
use std::collections::HashSet;
use std::fs;
use std::io::{self, Read};
use std::path::Path;
use type_checker::errors::TypeCheckError;

#[derive(Parser)]
#[command(name = "cad-dsl")]
#[command(about = "A DSL for CAD operations")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Lex {
        file: String,
    },
    Parse {
        file: String,
    },
    Solve {
        file: String,
        /// Show values for under-constrained variables, marking them as [unconstrained].
        ///
        /// By default, variables that are not uniquely determined by the constraints are
        /// shown as `<underconstrained>`. With this flag, an arbitrary Z3-assigned value
        /// is displayed alongside the `[unconstrained]` marker so you can see one possible
        /// value even when many values would satisfy the constraints.
        #[arg(long)]
        show_unconstrained: bool,
        /// Path for SVG output produced by svg_begin/svg_end calls in the CAD file.
        ///
        /// When set, all SVG export functions (svg_begin, svg_line, svg_circle, etc.)
        /// write to this file instead of the default `sketch.svg`.
        /// The path is passed to rune blocks via the `CAD_DSL_SVG_OUTPUT` environment
        /// variable, which the cad2d standard library reads automatically.
        #[arg(long, value_name = "FILE")]
        svg_output: Option<String>,
    },
}

/// Read content from a file or stdin if the file is "-"
fn read_input(file: &str) -> io::Result<String> {
    if file == "-" {
        let mut content = String::new();
        io::stdin().read_to_string(&mut content)?;
        Ok(content)
    } else {
        fs::read_to_string(file)
    }
}

/// Get the filename for error reporting (use "<stdin>" for "-")
fn display_filename(file: &str) -> &str {
    if file == "-" { "<stdin>" } else { file }
}

/// Report semantic analysis errors with Ariadne formatting
fn report_semantic_errors(filename: &str, source: &str, errors: Vec<SemanticError>) {
    for error in errors {
        // Convert line/column to byte offset
        let span = match error {
            SemanticError::UndefinedVariable { span, .. }
            | SemanticError::UndefinedFunction { span, .. }
            | SemanticError::UndefinedType { span, .. }
            | SemanticError::UndefinedField { span, .. }
            | SemanticError::UndefinedMethod { span, .. }
            | SemanticError::MethodCallOnNonStruct { span, .. }
            | SemanticError::TypeMismatch { span, .. }
            | SemanticError::NotInWithContext { span }
            | SemanticError::NoContainerField { span, .. }
            | SemanticError::InvalidDotPrefix { span }
            | SemanticError::InvalidTransformSignature { span, .. }
            | SemanticError::NestedOptimizeBlock { span } => span,
            SemanticError::DuplicateDefinition { second_span, .. }
            | SemanticError::AmbiguousTransform { second_span, .. } => second_span,
        };

        let offset = calculate_byte_offset(source, span.start.line, span.start.column);

        let report = Report::build(ReportKind::Error, filename, offset)
            .with_message("Semantic error")
            .with_label(
                Label::new((filename, offset..offset + 1))
                    .with_message(error.to_string())
                    .with_color(Color::Red),
            )
            .finish();

        report.print((filename, Source::from(source))).unwrap();
    }
}

/// Report type checking errors with Ariadne formatting
fn report_type_errors(filename: &str, source: &str, errors: Vec<TypeCheckError>) {
    for error in errors {
        // Extract span from error
        let span = match &error {
            TypeCheckError::TypeMismatch { span, .. }
            | TypeCheckError::CannotInferType { span, .. }
            | TypeCheckError::IncompatibleTypes { span, .. }
            | TypeCheckError::ArgumentTypeMismatch { span, .. }
            | TypeCheckError::WrongNumberOfArguments { span, .. }
            | TypeCheckError::NonNumericOperand { span, .. }
            | TypeCheckError::NonBooleanCondition { span, .. }
            | TypeCheckError::CannotIndex { span, .. }
            | TypeCheckError::Rune { span, .. } => *span,
        };

        let offset = calculate_byte_offset(source, span.start.line, span.start.column);

        let report = Report::build(ReportKind::Error, filename, offset)
            .with_message("Type error")
            .with_label(
                Label::new((filename, offset..offset + 1))
                    .with_message(error.to_string())
                    .with_color(Color::Red),
            )
            .finish();

        report.print((filename, Source::from(source))).unwrap();
    }
}

/// Report solver errors with Ariadne formatting
fn report_solver_errors(filename: &str, source: &str, error: SolverError) {
    // For solver errors, we don't have specific span information,
    // so we report the error at the beginning of the file
    let report = Report::build(ReportKind::Error, filename, 0)
        .with_message("Solver error")
        .with_label(
            Label::new((filename, 0..1))
                .with_message(error.to_string())
                .with_color(Color::Red),
        )
        .finish();

    report.print((filename, Source::from(source))).unwrap();
}

/// Calculate byte offset from line and column numbers
fn calculate_byte_offset(source: &str, target_line: usize, target_column: usize) -> usize {
    let mut current_line = 1;
    let mut current_column = 1;

    for (byte_offset, ch) in source.char_indices() {
        if current_line == target_line && current_column == target_column {
            return byte_offset;
        }

        if ch == '\n' {
            current_line += 1;
            current_column = 1;
        } else {
            current_column += 1;
        }
    }

    // If we didn't find the exact position, return the end of the source
    source.len()
}

/// Recursively tokenize a source file, expanding `include "path";` directives
/// by inserting the tokens from the referenced file in-place.
///
/// `visited` tracks canonicalized paths already processed to prevent duplicate
/// or circular includes. Tokens from included files are printed with the same
/// format as the main file's tokens.
fn lex_with_includes(content: &str, base_dir: &Path, visited: &mut HashSet<std::path::PathBuf>) {
    let tokens = match lexer::tokenize(content) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Lexing error: {}", e);
            std::process::exit(1);
        }
    };

    let mut i = 0;
    while i < tokens.len() {
        // Check for the pattern: Include StringLiteral SemiColon
        if let lexer::Token::Include(_) = &tokens[i]
            && i + 2 < tokens.len()
            && let (lexer::Token::StringLiteral(path_tok), lexer::Token::SemiColon(_)) =
                (&tokens[i + 1], &tokens[i + 2])
        {
            let include_path = base_dir.join(path_tok.value);
            let canonical = include_path
                .canonicalize()
                .unwrap_or_else(|_| include_path.clone());

            if !visited.contains(&canonical) {
                visited.insert(canonical.clone());
                match std::fs::read_to_string(&include_path) {
                    Ok(inc_content) => {
                        let inc_base = canonical.parent().unwrap_or(Path::new(".")).to_path_buf();
                        lex_with_includes(&inc_content, &inc_base, visited);
                    }
                    Err(e) => {
                        eprintln!(
                            "Cannot open include file '{}': {}",
                            include_path.display(),
                            e
                        );
                        std::process::exit(1);
                    }
                }
            }
            i += 3; // skip Include + StringLiteral + SemiColon
            continue;
        }

        println!(
            "{:?} at {:?} - value: {}",
            tokens[i],
            tokens[i].position(),
            tokens[i].value_str()
        );
        i += 1;
    }
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Lex { file } => {
            let content = read_input(file).expect("Failed to read input");

            let base_dir = if file.as_str() == "-" {
                std::env::current_dir().unwrap_or_default()
            } else {
                Path::new(file.as_str())
                    .canonicalize()
                    .ok()
                    .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                    .unwrap_or_default()
            };
            let mut visited = HashSet::new();
            if file.as_str() != "-"
                && let Ok(canonical) = Path::new(file.as_str()).canonicalize()
            {
                visited.insert(canonical);
            }
            lex_with_includes(&content, &base_dir, &mut visited);
        }
        Commands::Parse { file } => {
            // Create the arena early so included-file source strings share
            // the same 'arena lifetime as the main source and all AST nodes.
            let arena = Bump::new();

            let raw_content = read_input(file).expect("Failed to read input");
            let filename = display_filename(file);

            // Arena-allocate source and tokens for consistent 'arena lifetime.
            let content: &str = arena.alloc_str(&raw_content);

            // Step 1: Tokenize
            let raw_tokens = match lexer::tokenize(content) {
                Ok(tokens) => tokens,
                Err(error) => {
                    eprintln!("Lexing error: {}", error);
                    std::process::exit(1);
                }
            };
            let tokens: &[_] = arena.alloc_slice_clone(&raw_tokens);

            // Step 2: Parse the full program
            let ast = match parser::parse_program(content, tokens) {
                Ok(stmts) => {
                    println!("✓ Parsing successful");
                    stmts
                }
                Err(errors) => {
                    eprintln!("Parse errors:");
                    parser::report_parse_errors(filename, content, errors);
                    std::process::exit(1);
                }
            };

            // Step 2b: Resolve include directives
            let base_dir = if file.as_str() == "-" {
                std::env::current_dir().unwrap_or_default()
            } else {
                Path::new(file.as_str())
                    .canonicalize()
                    .ok()
                    .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                    .unwrap_or_default()
            };
            let mut visited = HashSet::new();
            if file.as_str() != "-"
                && let Ok(canonical) = Path::new(file.as_str()).canonicalize()
            {
                visited.insert(canonical);
            }
            let ast = match include_resolver::resolve_includes(&arena, ast, &base_dir, &mut visited)
            {
                Ok(stmts) => stmts,
                Err(e) => {
                    eprintln!("Include error: {}", e);
                    std::process::exit(1);
                }
            };

            // Step 3: Semantic Analysis
            let hir = match semantic_analyzer::analyze(&arena, content, &ast) {
                Ok(hir) => {
                    println!("✓ Semantic analysis successful");
                    hir
                }
                Err(errors) => {
                    eprintln!("\nSemantic errors:");
                    report_semantic_errors(filename, content, errors);
                    std::process::exit(1);
                }
            };

            // Step 4: Type Checking
            match type_checker::type_check(&arena, content, &hir[..]) {
                Ok(warnings) => {
                    println!("✓ Type checking successful");
                    if !warnings.is_empty() {
                        eprintln!("\nType checking warnings:");
                        for warning in warnings {
                            eprintln!("  warning: {}", warning);
                        }
                    }
                    println!("\nAll checks passed! Program is well-typed.");
                }
                Err(errors) => {
                    eprintln!("\nType errors:");
                    report_type_errors(filename, content, errors);
                    std::process::exit(1);
                }
            }
        }
        Commands::Solve {
            file,
            show_unconstrained,
            svg_output,
        } => {
            // Set the SVG output path env var so rune blocks in cad2d.cad can read it.
            // This must be done before solving so the rune blocks pick it up at runtime.
            // SAFETY: single-threaded at this point; no other threads have been spawned yet.
            if let Some(svg_path) = svg_output {
                unsafe { std::env::set_var("CAD_DSL_SVG_OUTPUT", svg_path) };
            }
            // Create the arena early so included-file source strings can share
            // the same 'arena lifetime as the main source and all AST nodes.
            let arena = Bump::new();

            let raw_content = read_input(file).expect("Failed to read input");
            let filename = display_filename(file);

            // Arena-allocate the source so its lifetime matches 'arena.
            let content: &str = arena.alloc_str(&raw_content);

            // Step 1: Tokenize
            // Arena-allocate the token slice so it shares the 'arena lifetime
            // with the source string, satisfying Chumsky's lifetime requirements.
            let raw_tokens = match lexer::tokenize(content) {
                Ok(tokens) => tokens,
                Err(error) => {
                    eprintln!("Lexing error: {}", error);
                    std::process::exit(1);
                }
            };
            let tokens: &[_] = arena.alloc_slice_clone(&raw_tokens);

            // Step 2: Parse the program (may have multiple statements)
            let ast = match parser::parse_program(content, tokens) {
                Ok(stmts) => stmts,
                Err(errors) => {
                    eprintln!("Parse errors:");
                    parser::report_parse_errors(filename, content, errors);
                    std::process::exit(1);
                }
            };

            // Step 2b: Resolve include directives.
            // The main file is inserted into `visited` first so the program
            // cannot (accidentally) include itself.
            let base_dir = if file.as_str() == "-" {
                std::env::current_dir().unwrap_or_default()
            } else {
                Path::new(file.as_str())
                    .canonicalize()
                    .ok()
                    .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                    .unwrap_or_default()
            };
            let mut visited = HashSet::new();
            if file.as_str() != "-"
                && let Ok(canonical) = Path::new(file.as_str()).canonicalize()
            {
                visited.insert(canonical);
            }
            let ast = match include_resolver::resolve_includes(&arena, ast, &base_dir, &mut visited)
            {
                Ok(stmts) => stmts,
                Err(e) => {
                    eprintln!("Include error: {}", e);
                    std::process::exit(1);
                }
            };

            // Step 3: Semantic Analysis
            let hir = match semantic_analyzer::analyze(&arena, content, &ast) {
                Ok(hir) => hir,
                Err(errors) => {
                    eprintln!("Semantic errors:");
                    report_semantic_errors(filename, content, errors);
                    std::process::exit(1);
                }
            };

            // Step 4: Type Checking
            match type_checker::type_check(&arena, content, &hir[..]) {
                Ok(warnings) => {
                    // Print warnings if any, but continue execution
                    if !warnings.is_empty() {
                        eprintln!("Type checking warnings:");
                        for warning in warnings {
                            eprintln!("  warning: {}", warning);
                        }
                        eprintln!();
                    }
                }
                Err(errors) => {
                    eprintln!("Type errors:");
                    report_type_errors(filename, content, errors);
                    std::process::exit(1);
                }
            }

            // Step 5: Constraint Solving
            match active_solver::solve(&hir[..], &arena, *show_unconstrained) {
                Ok(solution) => {
                    print!("{}", solution);
                }
                Err(error) => {
                    report_solver_errors(filename, content, error);
                    std::process::exit(1);
                }
            }
        }
    }
}
