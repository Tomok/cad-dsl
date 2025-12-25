mod ast;
mod lexer;

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
    }
}
