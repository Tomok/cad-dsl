//! Parser error reporting with Ariadne
//!
//! This module provides beautiful error reporting for parser errors
//! using the Ariadne library to create colored, contextual error messages.

use crate::lexer::Token;
use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::prelude::*;

// ============================================================================
// Error Reporting with Ariadne
// ============================================================================

/// Convert parser errors to beautiful Ariadne reports
///
/// This function converts Chumsky's Rich errors into Ariadne reports with
/// proper spans and helpful error messages. All necessary information
/// (spans, labels, expected tokens) is preserved from the parser.
pub fn report_parse_errors<'src>(
    filename: &str,
    source: &'src str,
    errors: Vec<Rich<'src, Token<'src>>>,
) {
    for error in errors {
        let span = error.span();

        // Calculate byte offset from token position
        // Note: This requires the tokens to have proper span information
        let offset = span.start;

        let mut report =
            Report::build(ReportKind::Error, filename, offset).with_message("Parse error");

        // Add the main error label
        report = report.with_label(
            Label::new((filename, offset..offset + 1))
                .with_message(format!("Unexpected token: {:?}", error.found()))
                .with_color(Color::Red),
        );

        // Add expected tokens if available
        if !error.expected().collect::<Vec<_>>().is_empty() {
            let expected = error
                .expected()
                .map(|e| format!("{:?}", e))
                .collect::<Vec<_>>()
                .join(", ");
            report = report.with_note(format!("Expected one of: {}", expected));
        }

        // Add help message based on error context
        if error.found().is_none() {
            report = report.with_help("Unexpected end of input");
        }

        report
            .finish()
            .eprint((filename, Source::from(source)))
            .unwrap();
    }
}
