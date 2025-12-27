mod ast;
mod lexer;
mod parser;

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

            // Then parse as a let statement
            match parser::let_stmt(parser::expr_inner())
                .parse(&tokens)
                .into_result()
            {
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
