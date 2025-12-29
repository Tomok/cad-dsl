mod ast;
mod lexer;
mod parser;

// HIR modules
mod hir;
mod hir_context;
mod hir_definitions;
mod hir_expr;
mod hir_scope;
mod hir_types;

// Semantic analysis modules
mod semantic_analyzer;
mod semantic_analyzer_context;
mod semantic_analyzer_errors;
mod semantic_analyzer_pass1;
mod semantic_analyzer_pass2;

// Type checking modules
mod type_checker_context;
mod type_checker_errors;
mod type_checker_inference;
mod type_checker_validation;

use chumsky::Parser as _;
use clap::{Parser, Subcommand};
use lexer::TokenTrait;
use std::fs;

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

            // First tokenize
            let tokens = match lexer::tokenize(&content) {
                Ok(tokens) => tokens,
                Err(error) => {
                    eprintln!("Lexing error: {}", error);
                    std::process::exit(1);
                }
            };

            // Parse as either a let statement, function definition, or struct definition
            use chumsky::primitive::choice;
            let stmt_parser = choice((
                parser::struct_def(parser::expr_inner()),
                parser::function_def(parser::expr_inner()),
                parser::let_stmt(parser::expr_inner()),
            ));

            match stmt_parser.parse(&tokens).into_result() {
                Ok(stmt) => {
                    println!("Successfully parsed!");
                    println!("Statement: {:?}", stmt);
                }
                Err(errors) => {
                    eprintln!("Parse errors:");
                    parser::report_parse_errors(file, &content, errors);
                    std::process::exit(1);
                }
            }
        }
    }
}
