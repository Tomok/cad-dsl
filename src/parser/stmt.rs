//! Statement and type annotation parsers

use crate::ast::span::HasSpan;
use crate::ast::{FunctionParam, Stmt, StructField, Type};
use crate::lexer::Token;
use crate::parser::ParseError;
use chumsky::prelude::*;

// ============================================================================
// Type Annotation Parser
// ============================================================================

/// Parse type annotations (bool, i32, f64, Real, Algebraic, &Type, UserType)
pub fn type_annotation<'src>()
-> impl Parser<'src, &'src [Token<'src>], Type, ParseError<'src>> + Clone {
    use crate::lexer::Span;

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
        select! {
            Token::RealType(t) => Type::Real {
                span: Span { start: t.position, lines: 0, end_column: t.position.column + 4 }
            },
        },
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

    // Reference type: &Type
    // Only support single-level references (no &&Type)
    let reference_type = select! {
        Token::Ampersand(t) => t.position,
    }
    .then(base_type)
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

    choice((reference_type, base_type)).labelled("type annotation")
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

    // Parse a dotted path: identifier (.identifier)*
    let name_path = select! {
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
        path
    });

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
        |((((let_pos, name_path), type_annotation), init), semi_pos)| {
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

    // Parse a dotted path: identifier.identifier(.identifier)*
    // Must have at least 2 segments
    let field_path = select! {
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
        path
    });

    field_path
        .then_ignore(equals)
        .then(expr_parser.labelled("value expression"))
        .then(select! {
            Token::SemiColon(t) => t.position,
        })
        .map(|((field_path, value), semi_pos)| {
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
-> impl Parser<'src, &'src [Token<'src>], FunctionParam, ParseError<'src>> + Clone {
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

    // Function bodies can contain let statements, assignment statements, field assignments, return statements, for loops, blocks, and expression statements
    // Use recursive parser to support nested for loops and nested blocks
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
-> impl Parser<'src, &'src [Token<'src>], StructField, ParseError<'src>> + Clone {
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
    Field(StructField),
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
