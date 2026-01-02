mod ast;
mod lexer;
mod parser;

// HIR module
mod hir;

// Semantic analysis modules
mod semantic_analyzer;
mod semantic_analyzer_context;
mod semantic_analyzer_errors;
mod semantic_analyzer_pass1;
mod semantic_analyzer_pass2;

// Type checking modules
mod type_checker;
mod type_checker_context;
mod type_checker_errors;
mod type_checker_inference;
mod type_checker_validation;

// Constraint extraction module
mod constraint_extractor;

// Z3 constraint solver bridge
mod z3_bridge;

// Solution formatter
mod solution_formatter;

// Solver pipeline
mod solver;

use ariadne::{Color, Label, Report, ReportKind, Source};
use bumpalo::Bump;
use chumsky::Parser as _;
use clap::{Parser, Subcommand};
use lexer::TokenTrait;
use semantic_analyzer_errors::SemanticError;
use solver::SolverError;
use std::fs;
use type_checker_errors::TypeCheckError;

#[derive(Parser)]
#[command(name = "cad-dsl")]
#[command(about = "A DSL for CAD operations")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Lex { file: String },
    Parse { file: String },
    Solve { file: String },
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
            | SemanticError::TypeMismatch { span, .. }
            | SemanticError::NotInWithContext { span }
            | SemanticError::NoContainerField { span, .. }
            | SemanticError::InvalidDotPrefix { span } => span,
            SemanticError::DuplicateDefinition { second_span, .. } => second_span,
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
            | TypeCheckError::CannotIndex { span, .. } => *span,
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

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Lex { file } => {
            let content = fs::read_to_string(file).expect("Failed to read file");

            match lexer::tokenize(&content) {
                Ok(tokens) => {
                    for token in tokens {
                        println!(
                            "{:?} at {:?} - value: {}",
                            token,
                            token.position(),
                            token.value_str()
                        );
                    }
                }
                Err(error) => eprintln!("Lexing error: {}", error),
            }
        }
        Commands::Parse { file } => {
            let content = fs::read_to_string(file).expect("Failed to read file");

            // Step 1: Tokenize
            let tokens = match lexer::tokenize(&content) {
                Ok(tokens) => tokens,
                Err(error) => {
                    eprintln!("Lexing error: {}", error);
                    std::process::exit(1);
                }
            };

            // Step 2: Parse as either a let statement, function definition, or struct definition
            use chumsky::primitive::choice;
            let stmt_parser = choice((
                parser::struct_def(parser::expr_inner()),
                parser::function_def(parser::expr_inner()),
                parser::let_stmt(parser::expr_inner()),
            ));

            let ast = match stmt_parser.parse(&tokens).into_result() {
                Ok(stmt) => {
                    println!("✓ Parsing successful");
                    vec![stmt]
                }
                Err(errors) => {
                    eprintln!("Parse errors:");
                    parser::report_parse_errors(file, &content, errors);
                    std::process::exit(1);
                }
            };

            // Step 3: Semantic Analysis
            let arena = Bump::new();
            let hir = match semantic_analyzer::analyze(&arena, &content, &ast) {
                Ok(hir) => {
                    println!("✓ Semantic analysis successful");
                    hir
                }
                Err(errors) => {
                    eprintln!("\nSemantic errors:");
                    report_semantic_errors(file, &content, errors);
                    std::process::exit(1);
                }
            };

            // Step 4: Type Checking
            match type_checker::type_check(&arena, &content, &hir[..]) {
                Ok(()) => {
                    println!("✓ Type checking successful");
                    println!("\nAll checks passed! Program is well-typed.");
                }
                Err(errors) => {
                    eprintln!("\nType errors:");
                    report_type_errors(file, &content, errors);
                    std::process::exit(1);
                }
            }
        }
        Commands::Solve { file } => {
            let content = fs::read_to_string(file).expect("Failed to read file");

            // Step 1: Tokenize
            let tokens = match lexer::tokenize(&content) {
                Ok(tokens) => tokens,
                Err(error) => {
                    eprintln!("Lexing error: {}", error);
                    std::process::exit(1);
                }
            };

            // Step 2: Parse the program (may have multiple statements)
            use chumsky::IterParser;
            use chumsky::primitive::choice;
            let stmt_parser = choice((
                parser::struct_def(parser::expr_inner()),
                parser::function_def(parser::expr_inner()),
                parser::let_stmt(parser::expr_inner()),
                parser::assignment_stmt(parser::expr_inner()),
                parser::expression_stmt(parser::expr_inner()),
            ))
            .repeated()
            .collect::<Vec<_>>();

            let ast = match stmt_parser.parse(&tokens).into_result() {
                Ok(stmts) => stmts,
                Err(errors) => {
                    eprintln!("Parse errors:");
                    parser::report_parse_errors(file, &content, errors);
                    std::process::exit(1);
                }
            };

            // Step 3: Semantic Analysis
            let arena = Bump::new();
            let hir = match semantic_analyzer::analyze(&arena, &content, &ast) {
                Ok(hir) => hir,
                Err(errors) => {
                    eprintln!("Semantic errors:");
                    report_semantic_errors(file, &content, errors);
                    std::process::exit(1);
                }
            };

            // Step 4: Type Checking
            if let Err(errors) = type_checker::type_check(&arena, &content, &hir[..]) {
                eprintln!("Type errors:");
                report_type_errors(file, &content, errors);
                std::process::exit(1);
            }

            // Step 5: Constraint Solving
            match solver::solve(&hir[..]) {
                Ok(solution) => {
                    print!("{}", solution);
                }
                Err(error) => {
                    report_solver_errors(file, &content, error);
                    std::process::exit(1);
                }
            }
        }
    }
}
