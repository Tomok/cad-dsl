//! Statement and type annotation parsers

use crate::ast::span::HasSpan;
use crate::ast::{FunctionParam, Stmt, StructField, Type, UnitExpr, UnitTypeExpr};
use crate::lexer::{Span, Token};
use crate::parser::ParseError;
use chumsky::prelude::*;

// ============================================================================
// Type Annotation Parser
// ============================================================================

// ============================================================================
// Unit Type Expression Parser
// ============================================================================

/// Parse a unit type expression (used inside `Real<...>`)
///
/// Syntax:
///   unit_type_expr ::= unit_type_term (("*" | "/") unit_type_term)*
///   unit_type_term ::= unit_type_factor ("^" INT_LITERAL)?
///   unit_type_factor ::= IDENT | "(" unit_type_expr ")"
fn unit_type_expr<'src>()
-> impl Parser<'src, &'src [Token<'src>], UnitTypeExpr<'src>, ParseError<'src>> + Clone {
    recursive(|unit_type_expr_rec| {
        let factor = choice((
            // Parenthesised group: (unit_type_expr)
            unit_type_expr_rec.clone().delimited_by(
                select! { Token::LeftParen(_) => () },
                select! { Token::RightParen(_) => () },
            ),
            // Named unit (identifier)
            select! { Token::Identifier(t) => UnitTypeExpr::Name { name: t.name, span: t.span } },
        ));

        // term = factor ("^" int)?
        let term = factor
            .then(
                select! { Token::Power(_) => () }
                    .ignore_then(select! { Token::IntLiteral(t) => t.value })
                    .or_not(),
            )
            .map(|(base, exp_opt)| {
                if let Some(exp) = exp_opt {
                    let base_span = base.span();
                    UnitTypeExpr::Pow {
                        span: base_span,
                        base: Box::new(base),
                        exp,
                    }
                } else {
                    base
                }
            });

        // expr = term (("*" | "/") term)*
        term.clone().foldl(
            choice((
                select! { Token::Multiply(_) => true },
                select! { Token::Divide(_) => false },
            ))
            .then(term)
            .repeated(),
            |lhs, (is_mul, rhs)| {
                let lhs_span = lhs.span();
                let rhs_span = rhs.span();
                let span = Span {
                    start: lhs_span.start,
                    lines: 0,
                    end_column: rhs_span.end_column,
                };
                if is_mul {
                    UnitTypeExpr::Mul {
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                        span,
                    }
                } else {
                    UnitTypeExpr::Div {
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                        span,
                    }
                }
            },
        )
    })
    .labelled("unit type expression")
}

// ============================================================================
// Unit Expression Parser (for unit declarations)
// ============================================================================

/// Parse a unit expression (used on RHS of `unit name = <expr>`)
///
/// Same as UnitTypeExpr but also allows numeric literal scale factors.
/// Syntax:
///   unit_expr ::= unit_term (("*" | "/") unit_term)*
///   unit_term ::= unit_factor ("^" INT_LITERAL)?
///   unit_factor ::= FLOAT_LITERAL | INT_LITERAL | IDENT | "(" unit_expr ")"
fn unit_expr_parser<'src>()
-> impl Parser<'src, &'src [Token<'src>], UnitExpr<'src>, ParseError<'src>> + Clone {
    recursive(|unit_expr_rec| {
        let factor = choice((
            // Parenthesised group
            unit_expr_rec.clone().delimited_by(
                select! { Token::LeftParen(_) => () },
                select! { Token::RightParen(_) => () },
            ),
            // Float literal scale factor
            select! {
                Token::FloatLiteral(t) => UnitExpr::Literal { value: t.value, span: t.span },
            },
            // Int literal scale factor (cast to f64)
            select! {
                Token::IntLiteral(t) => UnitExpr::Literal {
                    value: t.value as f64,
                    span: t.span,
                },
            },
            // Named unit
            select! {
                Token::Identifier(t) => UnitExpr::Name { name: t.name, span: t.span },
            },
        ));

        // term = factor ("^" int)?
        let term = factor
            .then(
                select! { Token::Power(_) => () }
                    .ignore_then(select! { Token::IntLiteral(t) => t.value })
                    .or_not(),
            )
            .map(|(base, exp_opt)| {
                if let Some(exp) = exp_opt {
                    let base_span = base.span();
                    UnitExpr::Pow {
                        span: base_span,
                        base: Box::new(base),
                        exp,
                    }
                } else {
                    base
                }
            });

        // expr = term (("*" | "/") term)*
        term.clone().foldl(
            choice((
                select! { Token::Multiply(_) => true },
                select! { Token::Divide(_) => false },
            ))
            .then(term)
            .repeated(),
            |lhs, (is_mul, rhs)| {
                let lhs_span = lhs.span();
                let rhs_span = rhs.span();
                let span = Span {
                    start: lhs_span.start,
                    lines: 0,
                    end_column: rhs_span.end_column,
                };
                if is_mul {
                    UnitExpr::Mul {
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                        span,
                    }
                } else {
                    UnitExpr::Div {
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                        span,
                    }
                }
            },
        )
    })
    .labelled("unit expression")
}

// ============================================================================
// Type Annotation Parser
// ============================================================================

/// Parse type annotations (bool, i32, f64, Real, Algebraic, [T; N], &Type, UserType)
pub fn type_annotation<'src>()
-> impl Parser<'src, &'src [Token<'src>], Type<'src>, ParseError<'src>> + Clone {
    recursive(|type_annotation| {
        let base_type = choice((
            select! {
                Token::BoolType(t) => Type::Bool {
                    span: Span { start: t.position, lines: 0, end_column: t.position.column + 4 }
                },
            },
            select! {
                Token::I32Type(t) => Type::I32 {
                    span: Span { start: t.position, lines: 0, end_column: t.position.column + 3 }
                },
            },
            select! {
                Token::F64Type(t) => Type::F64 {
                    span: Span { start: t.position, lines: 0, end_column: t.position.column + 3 }
                },
            },
            // Real<unit_type_expr> or plain Real
            select! {
                Token::RealType(t) => t.position,
            }
            .then(
                select! { Token::LessThan(_) => () }
                    .ignore_then(unit_type_expr())
                    .then(select! { Token::GreaterThan(t) => t.position })
                    .or_not(),
            )
            .map(|(real_pos, unit_opt)| {
                let end_col = match &unit_opt {
                    Some((_, gt_pos)) => gt_pos.column + 1,
                    None => real_pos.column + 4,
                };
                Type::Real {
                    unit: unit_opt.map(|(u, _)| Box::new(u)),
                    span: Span {
                        start: real_pos,
                        lines: 0,
                        end_column: end_col,
                    },
                }
            }),
            select! {
                Token::AlgebraicType(t) => Type::Algebraic {
                    span: Span { start: t.position, lines: 0, end_column: t.position.column + 9 }
                },
            },
            select! {
                Token::Identifier(t) => Type::UserDefined {
                    name: t.name.to_string(),
                    span: t.span,
                },
            },
        ));

        // Array type: [T; N]
        // Supports nested arrays like [[i32; 2]; 3]
        let array_type = select! {
            Token::LeftBracket(t) => t.position,
        }
        .then(type_annotation.clone())
        .then_ignore(select! {
            Token::SemiColon(_) => (),
        })
        .then(select! {
            Token::IntLiteral(t) => (t.value, t.span),
        })
        .then(select! {
            Token::RightBracket(t) => t.position,
        })
        .map(
            |(((left_pos, element_type), (size_value, _size_span)), right_pos)| {
                // Convert size to usize (negative sizes will wrap, but that's unlikely in practice)
                // TODO: Add proper validation for negative array sizes
                let size = size_value.max(0) as usize;

                // Calculate span from opening bracket to closing bracket
                let type_span = if left_pos.line == right_pos.line {
                    Span {
                        start: left_pos,
                        lines: 0,
                        end_column: right_pos.column + 1,
                    }
                } else {
                    Span {
                        start: left_pos,
                        lines: right_pos.line - left_pos.line,
                        end_column: right_pos.column + 1,
                    }
                };

                Type::Array {
                    element_type: Box::new(element_type),
                    size,
                    span: type_span,
                }
            },
        );

        let non_ref_type = choice((array_type, base_type));

        // Reference type: &Type
        // Can reference any type including arrays (e.g., &[i32; 5])
        let reference_type = select! {
            Token::Ampersand(t) => t.position,
        }
        .then(non_ref_type.clone())
        .map(|(amp_pos, inner_type)| {
            let inner_span = inner_type.span();
            let span = if amp_pos.line == inner_span.start.line {
                Span {
                    start: amp_pos,
                    lines: 0,
                    end_column: inner_span.end_column,
                }
            } else {
                Span {
                    start: amp_pos,
                    lines: inner_span.start.line - amp_pos.line + inner_span.lines,
                    end_column: inner_span.end_column,
                }
            };
            Type::Reference {
                inner: Box::new(inner_type),
                span,
            }
        });

        choice((reference_type, non_ref_type))
    })
    .labelled("type annotation")
}

// ============================================================================
// Statement Parser
// ============================================================================

/// Parse a let statement
///
/// Syntax:
///   let <name>: <type> = <expr>;
///   let <name>: <type>;
///   let <name> = <expr>;
///   let <name>;
///   let <container>.<field>: <type> = <expr>;
///   let <container>.<subcontainer>.<field>: <type> = <expr>;
pub fn let_stmt<'src>(
    expr_parser: impl Parser<'src, &'src [Token<'src>], crate::ast::Expr<'src>, ParseError<'src>>
    + Clone,
) -> impl Parser<'src, &'src [Token<'src>], Stmt<'src>, ParseError<'src>> + Clone {
    use crate::lexer::Span;

    let colon = select! { Token::Colon(_) => () };
    let equals = select! { Token::Equals(_) => () };
    let dot = select! { Token::Dot(_) => () };
    let dot_with_pos = select! { Token::Dot(t) => t.position };

    // Parse a dotted path: [.] identifier (.identifier)*
    // Returns (has_dot_prefix, path)
    let name_path = choice((
        // Dot-prefixed path: .identifier (.identifier)*
        dot_with_pos
            .then(select! {
                Token::Identifier(t) => (t.name, t.span),
            })
            .then(
                dot.ignore_then(select! {
                    Token::Identifier(t) => (t.name, t.span),
                })
                .repeated()
                .collect::<Vec<_>>(),
            )
            .map(|((_, first), rest)| {
                let mut path = vec![first];
                path.extend(rest);
                (true, path)
            }),
        // Regular path: identifier (.identifier)*
        select! {
            Token::Identifier(t) => (t.name, t.span),
        }
        .labelled("variable name")
        .then(
            dot.ignore_then(select! {
                Token::Identifier(t) => (t.name, t.span),
            })
            .repeated()
            .collect::<Vec<_>>(),
        )
        .map(|(first, rest)| {
            let mut path = vec![first];
            path.extend(rest);
            (false, path)
        }),
    ));

    select! {
        Token::Let(t) => t.position,
    }
    .then(name_path)
    .then(
        // Optional type annotation: : <type>
        colon.ignore_then(type_annotation()).or_not(),
    )
    .then(
        // Optional initialization: = <expr>
        equals.ignore_then(expr_parser).or_not(),
    )
    .then(select! {
        Token::SemiColon(t) => t.position,
    })
    .map(
        |((((let_pos, (dot_prefix, name_path)), type_annotation), init), semi_pos)| {
            // Construct span from let keyword to semicolon
            let span = if let_pos.line == semi_pos.line {
                // Same line
                Span {
                    start: let_pos,
                    lines: 0,
                    end_column: semi_pos.column + 1,
                }
            } else {
                // Multiple lines
                Span {
                    start: let_pos,
                    lines: semi_pos.line - let_pos.line,
                    end_column: semi_pos.column + 1,
                }
            };

            Stmt::Let {
                dot_prefix,
                name_path,
                type_annotation,
                init,
                span,
            }
        },
    )
    .labelled("let statement")
}

// ============================================================================
// Assignment Statement Parser
// ============================================================================

/// Parse an assignment statement
///
/// Syntax:
///   <name> = <expr>;
///
/// Examples:
///   x = 42;
///   width = 100;
///   result = a + b;
///
/// Note: This parser handles simple variable assignment only.
/// Field assignment (obj.field = value) is not yet implemented.
pub fn assignment_stmt<'src>(
    expr_parser: impl Parser<'src, &'src [Token<'src>], crate::ast::Expr<'src>, ParseError<'src>>
    + Clone,
) -> impl Parser<'src, &'src [Token<'src>], Stmt<'src>, ParseError<'src>> + Clone {
    use crate::lexer::Span;

    let equals = select! { Token::Equals(_) => () };

    select! {
        Token::Identifier(t) => (t.name, t.span),
    }
    .labelled("variable name")
    .then_ignore(equals)
    .then(expr_parser.labelled("value expression"))
    .then(select! {
        Token::SemiColon(t) => t.position,
    })
    .map(|(((name, name_span), value), semi_pos)| {
        // Construct span from variable name to semicolon
        let span = if name_span.start.line == semi_pos.line {
            // Same line
            Span {
                start: name_span.start,
                lines: 0,
                end_column: semi_pos.column + 1,
            }
        } else {
            // Multiple lines
            Span {
                start: name_span.start,
                lines: semi_pos.line - name_span.start.line,
                end_column: semi_pos.column + 1,
            }
        };

        Stmt::Assignment {
            name,
            name_span,
            value,
            span,
        }
    })
    .labelled("assignment statement")
}

// ============================================================================
// Field Assignment Statement Parser
// ============================================================================

/// Parse a field assignment statement
///
/// Syntax:
///   <obj>.<field> = <expr>;
///   <obj>.<nested>.<field> = <expr>;
///
/// Examples:
///   obj.field = 42;
///   sketch.origin.x = 10mm;
///   container.entities.p1.x = 5;
///
/// Note: The field path must have at least 2 segments (object.field).
/// This parser handles field assignment (obj.field = value), distinct from
/// simple assignment (x = value) which uses assignment_stmt.
pub fn field_assignment_stmt<'src>(
    expr_parser: impl Parser<'src, &'src [Token<'src>], crate::ast::Expr<'src>, ParseError<'src>>
    + Clone,
) -> impl Parser<'src, &'src [Token<'src>], Stmt<'src>, ParseError<'src>> + Clone {
    use crate::lexer::Span;

    let equals = select! { Token::Equals(_) => () };
    let dot = select! { Token::Dot(_) => () };
    let dot_with_pos = select! { Token::Dot(t) => t.position };

    // Parse a dotted path: [.] identifier(.identifier)*
    // Regular form requires at least 2 segments (obj.field)
    // Dot-prefixed form requires at least 1 segment (.field)
    // Returns (has_dot_prefix, path)
    let field_path = choice((
        // Dot-prefixed path: .identifier(.identifier)*
        dot_with_pos
            .then(
                select! {
                    Token::Identifier(t) => (t.name, t.span),
                }
                .labelled("field name")
                .separated_by(dot)
                .at_least(1)
                .collect::<Vec<_>>(),
            )
            .map(|(_, path)| (true, path)),
        // Regular path: identifier.identifier(.identifier)*
        // Must have at least 2 segments
        select! {
            Token::Identifier(t) => (t.name, t.span),
        }
        .labelled("object name")
        .then_ignore(dot)
        .then(
            select! {
                Token::Identifier(t) => (t.name, t.span),
            }
            .labelled("field name")
            .separated_by(dot)
            .at_least(1)
            .collect::<Vec<_>>(),
        )
        .map(|(first, rest)| {
            let mut path = vec![first];
            path.extend(rest);
            (false, path)
        }),
    ));

    field_path
        .then_ignore(equals)
        .then(expr_parser.labelled("value expression"))
        .then(select! {
            Token::SemiColon(t) => t.position,
        })
        .map(|(((dot_prefix, field_path), value), semi_pos)| {
            // Construct span from first identifier to semicolon
            let first_span = field_path[0].1;
            let span = if first_span.start.line == semi_pos.line {
                // Same line
                Span {
                    start: first_span.start,
                    lines: 0,
                    end_column: semi_pos.column + 1,
                }
            } else {
                // Multiple lines
                Span {
                    start: first_span.start,
                    lines: semi_pos.line - first_span.start.line,
                    end_column: semi_pos.column + 1,
                }
            };

            Stmt::FieldAssignment {
                dot_prefix,
                field_path,
                value,
                span,
            }
        })
        .labelled("field assignment statement")
}

// ============================================================================
// For Loop Parser
// ============================================================================

/// Parse a for loop
///
/// Syntax:
///   for <var> in <expr> { <statements> }
///
/// Examples:
///   for i in 0..10 { ... }
///   for elem in array { ... }
///
/// Note: Pass a recursive statement parser for nested for loops.
/// Use `recursive(|stmt| choice((let_stmt(...), for_stmt(..., stmt))))` for full statement support.
pub fn for_stmt<'src>(
    expr_parser: impl Parser<'src, &'src [Token<'src>], crate::ast::Expr<'src>, ParseError<'src>>
    + Clone
    + 'src,
    stmt_parser: impl Parser<'src, &'src [Token<'src>], Stmt<'src>, ParseError<'src>> + Clone + 'src,
) -> impl Parser<'src, &'src [Token<'src>], Stmt<'src>, ParseError<'src>> + Clone {
    use crate::lexer::Span;

    let left_brace = select! { Token::LeftBrace(_) => () };
    let right_brace = select! { Token::RightBrace(t) => t.position };

    select! {
        Token::For(t) => t.position,
    }
    .then(
        select! {
            Token::Identifier(t) => (t.name, t.span),
        }
        .labelled("loop variable"),
    )
    .then_ignore(select! {
        Token::In(_) => (),
    })
    .then(expr_parser.labelled("iterator expression"))
    .then_ignore(left_brace)
    .then(stmt_parser.repeated().collect::<Vec<_>>())
    .then(right_brace)
    .map(
        |((((for_pos, (loop_var, loop_var_span)), iterator), body), brace_pos)| {
            // Construct span from for keyword to closing brace
            let span = if for_pos.line == brace_pos.line {
                Span {
                    start: for_pos,
                    lines: 0,
                    end_column: brace_pos.column + 1,
                }
            } else {
                Span {
                    start: for_pos,
                    lines: brace_pos.line - for_pos.line,
                    end_column: brace_pos.column + 1,
                }
            };

            Stmt::For {
                loop_var,
                loop_var_span,
                iterator,
                body,
                span,
            }
        },
    )
    .labelled("for loop")
}

// ============================================================================
// Return Statement Parser
// ============================================================================

/// Parse a return statement
///
/// Syntax:
///   return;
///   return <expr>;
///
/// Examples:
///   return;
///   return value;
///   return a + b;
///
/// Note: Return without a value is allowed for functions with no return type.
pub fn return_stmt<'src>(
    expr_parser: impl Parser<'src, &'src [Token<'src>], crate::ast::Expr<'src>, ParseError<'src>>
    + Clone,
) -> impl Parser<'src, &'src [Token<'src>], Stmt<'src>, ParseError<'src>> + Clone {
    use crate::lexer::Span;

    select! {
        Token::Return(t) => t.position,
    }
    .then(
        // Optional expression before semicolon
        expr_parser.or_not(),
    )
    .then(select! {
        Token::SemiColon(t) => t.position,
    })
    .map(|((return_pos, value), semi_pos)| {
        // Construct span from return keyword to semicolon
        let span = if return_pos.line == semi_pos.line {
            // Same line
            Span {
                start: return_pos,
                lines: 0,
                end_column: semi_pos.column + 1,
            }
        } else {
            // Multiple lines
            Span {
                start: return_pos,
                lines: semi_pos.line - return_pos.line,
                end_column: semi_pos.column + 1,
            }
        };

        Stmt::Return { value, span }
    })
    .labelled("return statement")
}

// ============================================================================
// Expression Statement Parser
// ============================================================================

/// Parse an expression statement
///
/// Syntax:
///   <expr>;
///
/// Examples:
///   foo();
///   print(x);
///   obj.method();
///   1 + 2;
///
/// Note: This parser should be used LAST in the statement choice combinator
/// to avoid consuming parts of other statement types.
pub fn expression_stmt<'src>(
    expr_parser: impl Parser<'src, &'src [Token<'src>], crate::ast::Expr<'src>, ParseError<'src>>
    + Clone,
) -> impl Parser<'src, &'src [Token<'src>], Stmt<'src>, ParseError<'src>> + Clone {
    use crate::lexer::Span;

    expr_parser
        .labelled("expression")
        .then(select! {
            Token::SemiColon(t) => t.position,
        })
        .map(|(expr, semi_pos)| {
            use crate::ast::span::HasSpan;
            let expr_span = expr.span();

            // Construct span from expression start to semicolon
            let span = if expr_span.start.line == semi_pos.line {
                // Same line
                Span {
                    start: expr_span.start,
                    lines: 0,
                    end_column: semi_pos.column + 1,
                }
            } else {
                // Multiple lines
                Span {
                    start: expr_span.start,
                    lines: semi_pos.line - expr_span.start.line,
                    end_column: semi_pos.column + 1,
                }
            };

            Stmt::Expression { expr, span }
        })
        .labelled("expression statement")
}

// ============================================================================
// Function Definition Parser
// ============================================================================

/// Parse a function parameter: name: Type
pub fn function_param<'src>()
-> impl Parser<'src, &'src [Token<'src>], FunctionParam<'src>, ParseError<'src>> + Clone {
    use crate::lexer::Span;

    let colon = select! { Token::Colon(_) => () };

    select! {
        Token::Identifier(t) => (t.name.to_string(), t.span),
    }
    .labelled("parameter name")
    .then_ignore(colon)
    .then(type_annotation())
    .map(|((name, name_span), type_annotation)| {
        let type_span = type_annotation.span();
        let span = if name_span.start.line == type_span.start.line {
            Span {
                start: name_span.start,
                lines: 0,
                end_column: type_span.end_column,
            }
        } else {
            Span {
                start: name_span.start,
                lines: type_span.start.line - name_span.start.line + type_span.lines,
                end_column: type_span.end_column,
            }
        };
        FunctionParam {
            name,
            name_span,
            type_annotation,
            span,
        }
    })
    .labelled("function parameter")
}

/// Parse a function definition
///
/// Syntax:
///   fn name(param1: Type1, param2: Type2) -> ReturnType { body }
///   fn name() -> ReturnType { body }
pub fn function_def<'src>(
    expr_parser: impl Parser<'src, &'src [Token<'src>], crate::ast::Expr<'src>, ParseError<'src>>
    + Clone
    + 'src,
) -> impl Parser<'src, &'src [Token<'src>], Stmt<'src>, ParseError<'src>> + Clone {
    use crate::lexer::Span;

    let left_paren = select! { Token::LeftParen(_) => () };
    let right_paren = select! { Token::RightParen(_) => () };
    let comma = select! { Token::Comma(_) => () };
    let arrow = select! { Token::Arrow(_) => () };
    let left_brace = select! { Token::LeftBrace(_) => () };
    let right_brace = select! { Token::RightBrace(t) => t.position };

    // Function bodies can contain let statements, assignment statements, field assignments, return statements, for loops, with statements, if statements, blocks, and expression statements
    // Use recursive parser to support nested for loops, with statements, if statements, and nested blocks
    // Note: field_assignment_stmt must come before assignment_stmt to avoid ambiguity
    // (obj.field = value should parse as field assignment, not fail on obj.field)
    // Note: expression_stmt must come LAST to avoid consuming parts of other statements
    let stmt_parser = recursive(|stmt_rec| {
        choice((
            let_stmt(expr_parser.clone()),
            field_assignment_stmt(expr_parser.clone()),
            assignment_stmt(expr_parser.clone()),
            return_stmt(expr_parser.clone()),
            for_stmt(expr_parser.clone(), stmt_rec.clone()),
            with_stmt(expr_parser.clone(), stmt_rec.clone()),
            if_stmt(expr_parser.clone(), stmt_rec.clone()),
            block_stmt(stmt_rec),
            expression_stmt(expr_parser.clone()),
        ))
    });

    select! {
        Token::Fn(t) => t.position,
    }
    .then(
        select! {
            Token::Identifier(t) => (t.name.to_string(), t.span),
        }
        .labelled("function name"),
    )
    .then_ignore(left_paren)
    .then(
        // Parameter list: param1: Type1, param2: Type2, ...
        function_param()
            .separated_by(comma)
            .allow_trailing()
            .collect::<Vec<_>>(),
    )
    .then_ignore(right_paren)
    .then_ignore(arrow)
    .then(type_annotation().labelled("return type"))
    .then_ignore(left_brace)
    .then(
        // Function body: statements followed by optional return expression
        stmt_parser
            .repeated()
            .collect::<Vec<_>>()
            .then(expr_parser.or_not()),
    )
    .then(right_brace)
    .map(
        |(
            ((((fn_pos, (name, name_span)), params), return_type), (body, return_expr)),
            brace_pos,
        )| {
            // Construct span from fn keyword to closing brace
            let span = if fn_pos.line == brace_pos.line {
                Span {
                    start: fn_pos,
                    lines: 0,
                    end_column: brace_pos.column + 1,
                }
            } else {
                Span {
                    start: fn_pos,
                    lines: brace_pos.line - fn_pos.line,
                    end_column: brace_pos.column + 1,
                }
            };

            Stmt::FunctionDef {
                name,
                name_span,
                params,
                return_type,
                body,
                return_expr,
                span,
            }
        },
    )
    .labelled("function definition")
}

// ============================================================================
// Struct Definition Parser
// ============================================================================

/// Parse a struct field: name: Type
fn struct_field<'src>()
-> impl Parser<'src, &'src [Token<'src>], StructField<'src>, ParseError<'src>> + Clone {
    use crate::lexer::Span;

    let colon = select! { Token::Colon(_) => () };

    select! {
        Token::Identifier(t) => (t.name.to_string(), t.span),
    }
    .labelled("field name")
    .then_ignore(colon)
    .then(type_annotation())
    .map(|((name, name_span), type_annotation)| {
        let type_span = type_annotation.span();
        let span = if name_span.start.line == type_span.start.line {
            Span {
                start: name_span.start,
                lines: 0,
                end_column: type_span.end_column,
            }
        } else {
            Span {
                start: name_span.start,
                lines: type_span.start.line - name_span.start.line + type_span.lines,
                end_column: type_span.end_column,
            }
        };
        StructField {
            name,
            name_span,
            type_annotation,
            span,
        }
    })
    .labelled("struct field")
}

/// Parse a container field declaration: container name
fn container_field<'src>()
-> impl Parser<'src, &'src [Token<'src>], (String, crate::lexer::Span), ParseError<'src>> + Clone {
    select! {
        Token::Container(t) => t.position,
    }
    .then(select! {
        Token::Identifier(t) => (t.name.to_string(), t.span),
    })
    .map(|(_, (name, span))| (name, span))
    .labelled("container field")
}

/// Parse a struct definition
///
/// Syntax:
///   struct Name { field1: Type, field2: Type }
///   struct Name { container entities, field: Type }
///   struct Name { field: Type, fn method() -> Type { ... } }
pub fn struct_def<'src>(
    expr_parser: impl Parser<'src, &'src [Token<'src>], crate::ast::Expr<'src>, ParseError<'src>>
    + Clone
    + 'src,
) -> impl Parser<'src, &'src [Token<'src>], Stmt<'src>, ParseError<'src>> + Clone {
    use crate::lexer::Span;

    let left_brace = select! { Token::LeftBrace(_) => () };
    let right_brace = select! { Token::RightBrace(t) => t.position };
    let comma = select! { Token::Comma(_) => () };

    // Parse struct members: either container field, regular field, or method
    let member_parser = choice((
        // Container field
        container_field().map(MemberItem::Container),
        // Function definition
        function_def(expr_parser.clone()).map(MemberItem::Method),
        // Regular field
        struct_field().map(MemberItem::Field),
    ));

    select! {
        Token::Struct(t) => t.position,
    }
    .then(
        select! {
            Token::Identifier(t) => (t.name.to_string(), t.span),
        }
        .labelled("struct name"),
    )
    .then_ignore(left_brace)
    .then(
        // Parse members separated by commas (with optional trailing comma)
        member_parser
            .separated_by(comma)
            .allow_trailing()
            .collect::<Vec<_>>(),
    )
    .then(right_brace)
    .try_map(
        |(((struct_pos, (name, name_span)), members), brace_pos), span| {
            // Separate members into container, fields, and methods
            let mut container = None;
            let mut fields = Vec::new();
            let mut methods = Vec::new();

            for member in members {
                match member {
                    MemberItem::Container(c) => {
                        if container.is_some() {
                            return Err(Rich::custom(
                                span,
                                "struct can have at most one container field",
                            ));
                        }
                        container = Some(c);
                    }
                    MemberItem::Field(f) => fields.push(f),
                    MemberItem::Method(m) => methods.push(m),
                }
            }

            // Construct span from struct keyword to closing brace
            let struct_span = if struct_pos.line == brace_pos.line {
                Span {
                    start: struct_pos,
                    lines: 0,
                    end_column: brace_pos.column + 1,
                }
            } else {
                Span {
                    start: struct_pos,
                    lines: brace_pos.line - struct_pos.line,
                    end_column: brace_pos.column + 1,
                }
            };

            Ok(Stmt::StructDef {
                name,
                name_span,
                container,
                fields,
                methods,
                span: struct_span,
            })
        },
    )
    .labelled("struct definition")
}

// Helper enum for parsing struct members
enum MemberItem<'src> {
    Container((String, crate::lexer::Span)),
    Field(StructField<'src>),
    Method(Stmt<'src>),
}

// ============================================================================
// Block Statement Parser
// ============================================================================

/// Parse a block statement
///
/// Syntax:
///   { <statements> }
///
/// Examples:
///   { }
///   { let x = 1; }
///   { let x = 1; let y = 2; }
///   { { let x = 1; } { let y = 2; } }
///
/// Note: Pass a recursive statement parser for nested blocks.
/// Blocks can contain any statement type, including nested blocks.
pub fn block_stmt<'src>(
    stmt_parser: impl Parser<'src, &'src [Token<'src>], Stmt<'src>, ParseError<'src>> + Clone + 'src,
) -> impl Parser<'src, &'src [Token<'src>], Stmt<'src>, ParseError<'src>> + Clone {
    use crate::lexer::Span;

    let left_brace = select! { Token::LeftBrace(t) => t.position };
    let right_brace = select! { Token::RightBrace(t) => t.position };

    left_brace
        .then(stmt_parser.repeated().collect::<Vec<_>>())
        .then(right_brace)
        .map(|((left_pos, statements), right_pos)| {
            // Construct span from left brace to right brace
            let span = if left_pos.line == right_pos.line {
                Span {
                    start: left_pos,
                    lines: 0,
                    end_column: right_pos.column + 1,
                }
            } else {
                Span {
                    start: left_pos,
                    lines: right_pos.line - left_pos.line,
                    end_column: right_pos.column + 1,
                }
            };

            Stmt::Block { statements, span }
        })
        .labelled("block statement")
}

// ============================================================================
// With Statement Parser
// ============================================================================

/// Parse a with statement
///
/// Syntax:
///   with <expr> { <statements> }
///
/// Examples:
///   with transform { ... }
///   with sketch { let .p1: Point = point(0mm, 0mm); }
///   with translate { let p: Point = point(10mm, 10mm); }
///
/// The with statement applies a transform or container context to all
/// entity accesses within its block. When used with container structs,
/// it enables the dot prefix (.) to reference the container field.
///
/// Note: Pass a recursive statement parser for nested with statements.
pub fn with_stmt<'src>(
    expr_parser: impl Parser<'src, &'src [Token<'src>], crate::ast::Expr<'src>, ParseError<'src>>
    + Clone
    + 'src,
    stmt_parser: impl Parser<'src, &'src [Token<'src>], Stmt<'src>, ParseError<'src>> + Clone + 'src,
) -> impl Parser<'src, &'src [Token<'src>], Stmt<'src>, ParseError<'src>> + Clone {
    use crate::lexer::Span;

    let left_brace = select! { Token::LeftBrace(_) => () };
    let right_brace = select! { Token::RightBrace(t) => t.position };

    select! {
        Token::With(t) => t.position,
    }
    .then(expr_parser.labelled("context expression"))
    .then_ignore(left_brace)
    .then(stmt_parser.repeated().collect::<Vec<_>>())
    .then(right_brace)
    .map(|(((with_pos, context_expr), body), brace_pos)| {
        // Construct span from with keyword to closing brace
        let span = if with_pos.line == brace_pos.line {
            Span {
                start: with_pos,
                lines: 0,
                end_column: brace_pos.column + 1,
            }
        } else {
            Span {
                start: with_pos,
                lines: brace_pos.line - with_pos.line,
                end_column: brace_pos.column + 1,
            }
        };

        Stmt::With {
            context_expr,
            body,
            span,
        }
    })
    .labelled("with statement")
}

// ============================================================================
// If Statement Parser
// ============================================================================

/// Parse an if statement
///
/// Syntax:
///   if <expr> { <statements> }
///   if <expr> { <statements> } else { <statements> }
///
/// Examples:
///   if x > 0 { return x; }
///   if condition { doSomething(); } else { doSomethingElse(); }
///   if x > 0 { pos(); } else { if x < 0 { neg(); } else { zero(); } }
///
/// The else clause is optional. Else-if chains are supported by nesting
/// if statements in the else branch (requires braces around the nested if).
///
/// Note: Pass a recursive statement parser for nested if statements.
pub fn if_stmt<'src>(
    expr_parser: impl Parser<'src, &'src [Token<'src>], crate::ast::Expr<'src>, ParseError<'src>>
    + Clone
    + 'src,
    stmt_parser: impl Parser<'src, &'src [Token<'src>], Stmt<'src>, ParseError<'src>> + Clone + 'src,
) -> impl Parser<'src, &'src [Token<'src>], Stmt<'src>, ParseError<'src>> + Clone {
    use crate::lexer::Span;

    let left_brace = select! { Token::LeftBrace(_) => () };
    let right_brace = select! { Token::RightBrace(t) => t.position };

    select! {
        Token::If(t) => t.position,
    }
    .then(expr_parser.labelled("condition expression"))
    .then_ignore(left_brace)
    .then(stmt_parser.clone().repeated().collect::<Vec<_>>())
    .then(right_brace)
    .then(
        // Optional else clause: else { <statements> }
        // The else clause consists of braces with statements inside
        select! {
            Token::Else(_) => (),
        }
        .ignore_then(left_brace)
        .ignore_then(stmt_parser.repeated().collect::<Vec<_>>())
        .then(right_brace)
        .map(|(stmts, end_pos)| (stmts, end_pos))
        .or_not(),
    )
    .map(
        |((((if_pos, condition), then_branch), then_end_pos), else_branch)| {
            // Construct span from if keyword to end of else branch (if present) or end of then branch
            let end_pos = if let Some((_, else_end_pos)) = &else_branch {
                *else_end_pos
            } else {
                then_end_pos
            };

            let span = if if_pos.line == end_pos.line {
                Span {
                    start: if_pos,
                    lines: 0,
                    end_column: end_pos.column + 1,
                }
            } else {
                Span {
                    start: if_pos,
                    lines: end_pos.line - if_pos.line,
                    end_column: end_pos.column + 1,
                }
            };

            Stmt::If {
                condition,
                then_branch,
                else_branch: else_branch.map(|(stmts, _)| stmts),
                span,
            }
        },
    )
    .labelled("if statement")
}

// ============================================================================
// Optimize Block Parser
// ============================================================================

/// Parse an optimize block
///
/// Syntax:
///   optimize { minimize <expr>; }
///   optimize { maximize <expr>; minimize <expr>; }
///
/// Examples:
///   optimize { minimize x; }
///   optimize { minimize perimeter; maximize area; }
///
/// Directives are evaluated in lexicographic priority order.
/// Only valid at the top level of a program.
pub fn optimize_stmt<'src>(
    expr_parser: impl Parser<'src, &'src [Token<'src>], crate::ast::Expr<'src>, ParseError<'src>>
    + Clone,
) -> impl Parser<'src, &'src [Token<'src>], Stmt<'src>, ParseError<'src>> + Clone {
    use crate::ast::{OptimizeDirective, OptimizeDirectiveKind};
    use crate::lexer::Span;

    let left_brace = select! { Token::LeftBrace(_) => () };
    let right_brace = select! { Token::RightBrace(t) => t.position };

    // Parse a single directive: (minimize | maximize) <expr>;
    let directive = choice((
        select! { Token::Minimize(t) => (OptimizeDirectiveKind::Minimize, t.position) },
        select! { Token::Maximize(t) => (OptimizeDirectiveKind::Maximize, t.position) },
    ))
    .then(expr_parser.labelled("objective expression"))
    .then(select! { Token::SemiColon(t) => t.position })
    .map(|(((kind, kw_pos), expr), semi_pos)| {
        use crate::ast::span::HasSpan;
        let expr_span = expr.span();
        let span = if kw_pos.line == semi_pos.line {
            Span {
                start: kw_pos,
                lines: 0,
                end_column: semi_pos.column + 1,
            }
        } else {
            Span {
                start: kw_pos,
                lines: semi_pos.line - kw_pos.line,
                end_column: semi_pos.column + 1,
            }
        };
        let _ = expr_span;
        OptimizeDirective { kind, expr, span }
    });

    select! { Token::Optimize(t) => t.position }
        .then_ignore(left_brace)
        .then(directive.repeated().at_least(1).collect::<Vec<_>>())
        .then(right_brace)
        .map(|((opt_pos, directives), brace_pos)| {
            let span = if opt_pos.line == brace_pos.line {
                Span {
                    start: opt_pos,
                    lines: 0,
                    end_column: brace_pos.column + 1,
                }
            } else {
                Span {
                    start: opt_pos,
                    lines: brace_pos.line - opt_pos.line,
                    end_column: brace_pos.column + 1,
                }
            };
            Stmt::Optimize { directives, span }
        })
        .labelled("optimize block")
}

// ============================================================================
// Unit Declaration Parsers
// ============================================================================

/// Parse a base unit declaration: `unit <name>;`
/// or a derived unit definition: `unit <name> = <unit_expr>;`
///
/// Examples:
///   unit m;
///   unit inch = 0.0254 * m;
pub fn unit_stmt<'src>()
-> impl Parser<'src, &'src [Token<'src>], Stmt<'src>, ParseError<'src>> + Clone {
    select! { Token::Unit(t) => t.position }
        .then(select! { Token::Identifier(t) => (t.name, t.span) }.labelled("unit name"))
        .then(
            select! { Token::Equals(_) => () }
                .ignore_then(unit_expr_parser())
                .or_not(),
        )
        .then(select! { Token::SemiColon(t) => t.position })
        .map(|(((unit_pos, (name, name_span)), def_opt), semi_pos)| {
            let span = Span {
                start: unit_pos,
                lines: if unit_pos.line == semi_pos.line {
                    0
                } else {
                    semi_pos.line - unit_pos.line
                },
                end_column: semi_pos.column + 1,
            };
            match def_opt {
                None => Stmt::UnitDecl {
                    name,
                    name_span,
                    span,
                },
                Some(definition) => Stmt::UnitDef {
                    name,
                    name_span,
                    definition,
                    span,
                },
            }
        })
        .labelled("unit declaration")
}

/// Parse a unit prefix declaration: `unit_prefix <name> = <factor>;`
///
/// Examples:
///   unit_prefix m = 1e-3;
///   unit_prefix k = 1e3;
pub fn unit_prefix_stmt<'src>()
-> impl Parser<'src, &'src [Token<'src>], Stmt<'src>, ParseError<'src>> + Clone {
    select! { Token::UnitPrefix(t) => t.position }
        .then(select! { Token::Identifier(t) => (t.name, t.span) }.labelled("prefix name"))
        .then_ignore(select! { Token::Equals(_) => () })
        .then(
            // Accept either a float literal or integer literal as the factor
            choice((
                select! { Token::FloatLiteral(t) => t.value },
                select! { Token::IntLiteral(t) => t.value as f64 },
            ))
            .labelled("prefix factor"),
        )
        .then(select! { Token::SemiColon(t) => t.position })
        .map(
            |(((unit_prefix_pos, (prefix, prefix_span)), factor), semi_pos)| {
                let span = Span {
                    start: unit_prefix_pos,
                    lines: if unit_prefix_pos.line == semi_pos.line {
                        0
                    } else {
                        semi_pos.line - unit_prefix_pos.line
                    },
                    end_column: semi_pos.column + 1,
                };
                Stmt::UnitPrefixDecl {
                    prefix,
                    prefix_span,
                    factor,
                    span,
                }
            },
        )
        .labelled("unit prefix declaration")
}

// ============================================================================
// Include Directive Parser
// ============================================================================

/// Parse an include directive: `include "path";`
///
/// Examples:
///   include "lib/si_units.cad";
pub fn include_stmt<'src>()
-> impl Parser<'src, &'src [Token<'src>], Stmt<'src>, ParseError<'src>> + Clone {
    select! { Token::Include(t) => t.position }
        .then(select! { Token::StringLiteral(t) => (t.value, t.span) }.labelled("include path"))
        .then(select! { Token::SemiColon(t) => t.position })
        .map(|((include_pos, (path, _path_span)), semi_pos)| {
            let span = Span {
                start: include_pos,
                lines: if include_pos.line == semi_pos.line {
                    0
                } else {
                    semi_pos.line - include_pos.line
                },
                end_column: semi_pos.column + 1,
            };
            Stmt::Include { path, span }
        })
        .labelled("include directive")
}
