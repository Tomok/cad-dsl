use crate::ProcessedTokenKind;
use crate::ast::unresolved::*;
use crate::span::Span;
use chumsky::prelude::*;
use std::ops::Range;

use super::unified::unified_expr_parser;

pub type TokenStream = ProcessedTokenKind;

fn range_to_span(range: Range<usize>) -> Span {
    Span::new(range.start, range.end)
}

#[derive(Debug, Clone)]
enum StructMember {
    Field(FieldDef),
    Method(FunctionDef),
}

pub fn recursive_parser()
-> impl Parser<TokenStream, UnresolvedAst, Error = Simple<ProcessedTokenKind>> + Clone {
    let ident = select! { ProcessedTokenKind::Ident(id) => id };

    // Use the new unified expression parser that handles recursion properly
    let expr = unified_expr_parser();

    // Type parser with recursive array and reference support
    let type_ref = recursive(|type_ref| {
        choice((
            // Array type: [Type; size] (where Type can be a reference)
            just(ProcessedTokenKind::LBracket)
                .ignore_then(type_ref.clone())
                .then_ignore(just(ProcessedTokenKind::Semicolon))
                .then(expr.clone())
                .then_ignore(just(ProcessedTokenKind::RBracket))
                .map_with_span(|(inner_type, size): (TypeRef, Expr), span| TypeRef {
                    name: inner_type.name,
                    is_reference: inner_type.is_reference,
                    array_size: Some(Box::new(size)),
                    span: range_to_span(span),
                }),
            // Reference type: &Type
            just(ProcessedTokenKind::Ampersand)
                .ignore_then(ident)
                .map_with_span(|name, span| TypeRef {
                    name,
                    is_reference: true,
                    array_size: None,
                    span: range_to_span(span),
                }),
            // Simple type: Type
            ident.map_with_span(|name, span| TypeRef {
                name,
                is_reference: false,
                array_size: None,
                span: range_to_span(span),
            }),
        ))
    });

    // Statement parser with control flow support
    let stmt = recursive(|stmt| {
        choice((
            // Let statement
            just(ProcessedTokenKind::Let)
                .ignore_then(ident)
                .then(
                    just(ProcessedTokenKind::Colon)
                        .ignore_then(type_ref.clone())
                        .or_not(),
                )
                .then(
                    just(ProcessedTokenKind::Assign)
                        .ignore_then(expr.clone())
                        .or_not(),
                )
                .then_ignore(just(ProcessedTokenKind::Semicolon))
                .map_with_span(|((name, ty), init), span| Stmt::Let {
                    name,
                    ty,
                    init,
                    span: range_to_span(span),
                }),
            // For statement: for var in range { body }
            just(ProcessedTokenKind::For)
                .ignore_then(ident)
                .then_ignore(just(ProcessedTokenKind::In))
                .then(expr.clone())
                .then(stmt.clone().repeated().delimited_by(
                    just(ProcessedTokenKind::LBrace),
                    just(ProcessedTokenKind::RBrace),
                ))
                .map_with_span(|((var, range), body), span| Stmt::For {
                    var,
                    range,
                    body,
                    span: range_to_span(span),
                }),
            // With statement: with view { body }
            just(ProcessedTokenKind::With)
                .ignore_then(expr.clone())
                .then(stmt.clone().repeated().delimited_by(
                    just(ProcessedTokenKind::LBrace),
                    just(ProcessedTokenKind::RBrace),
                ))
                .map_with_span(|(view, body), span| Stmt::With {
                    view,
                    body,
                    span: range_to_span(span),
                }),
            // Assignment statement: target = value;
            expr.clone()
                .then_ignore(just(ProcessedTokenKind::Assign))
                .then(expr.clone())
                .then_ignore(just(ProcessedTokenKind::Semicolon))
                .map_with_span(|(target, value), span| Stmt::Assign {
                    target,
                    value,
                    span: range_to_span(span),
                }),
            // Expression statement
            expr.clone()
                .then_ignore(just(ProcessedTokenKind::Semicolon))
                .map(Stmt::Expr),
        ))
    });

    // Field definition parser
    let field_def = ident
        .then_ignore(just(ProcessedTokenKind::Colon))
        .then(type_ref.clone())
        .then_ignore(just(ProcessedTokenKind::Comma).or_not())
        .map_with_span(|(name, ty), span| FieldDef {
            name,
            ty,
            span: range_to_span(span),
        });

    // Parameter definition parser for functions
    let param_def = ident
        .then_ignore(just(ProcessedTokenKind::Colon))
        .then(type_ref.clone())
        .map_with_span(|(name, ty), span| ParamDef {
            name,
            ty,
            span: range_to_span(span),
        });

    // Function definition parser
    let function_def = just(ProcessedTokenKind::Fn)
        .ignore_then(ident)
        .then(
            param_def
                .separated_by(just(ProcessedTokenKind::Comma))
                .delimited_by(
                    just(ProcessedTokenKind::LParen),
                    just(ProcessedTokenKind::RParen),
                ),
        )
        .then(
            just(ProcessedTokenKind::Arrow)
                .ignore_then(type_ref.clone())
                .or_not(),
        )
        .then(
            // Simple function body parser - parse statements like in sketches
            stmt.clone().repeated().delimited_by(
                just(ProcessedTokenKind::LBrace),
                just(ProcessedTokenKind::RBrace),
            ),
        )
        .map_with_span(|(((name, params), return_type), body), span| FunctionDef {
            name,
            params,
            return_type,
            body,
            span: range_to_span(span),
        });

    // Struct member (field or method) - try function first since it has a clear keyword
    let struct_member = choice((
        function_def.map(StructMember::Method),
        field_def.map(StructMember::Field),
    ));

    // Struct parser with methods support
    let struct_def = just(ProcessedTokenKind::Struct)
        .ignore_then(ident)
        .then(struct_member.repeated().delimited_by(
            just(ProcessedTokenKind::LBrace),
            just(ProcessedTokenKind::RBrace),
        ))
        .map_with_span(|(name, members), span| {
            let mut fields = Vec::new();
            let mut methods = Vec::new();

            for member in members {
                match member {
                    StructMember::Field(field) => fields.push(field),
                    StructMember::Method(method) => methods.push(method),
                }
            }

            StructDef {
                name,
                fields,
                methods,
                span: range_to_span(span),
            }
        });

    // Simple sketch parser
    let sketch = just(ProcessedTokenKind::Sketch)
        .ignore_then(ident)
        .then(stmt.repeated().delimited_by(
            just(ProcessedTokenKind::LBrace),
            just(ProcessedTokenKind::RBrace),
        ))
        .map_with_span(|(name, body), span| SketchDef {
            name,
            body,
            span: range_to_span(span),
        });

    // Import statement parser
    let import_def = just(ProcessedTokenKind::Import)
        .ignore_then(ident) // For now, we'll use identifiers as import paths
        .then_ignore(just(ProcessedTokenKind::Semicolon))
        .map_with_span(|path_id, span| ImportDef {
            path: format!("{:?}", path_id), // Convert IdentId to string representation
            span: range_to_span(span),
        });

    // Top level - support imports, sketches and structs
    let top_level_item = choice((
        import_def.map(TopLevelItem::Import),
        struct_def.map(TopLevelItem::Struct),
        sketch.map(TopLevelItem::Sketch),
    ));

    top_level_item
        .repeated()
        .then_ignore(just(ProcessedTokenKind::Eof))
        .map(|items| {
            let mut imports = Vec::new();
            let mut sketches = Vec::new();
            let mut structs = Vec::new();

            for item in items {
                match item {
                    TopLevelItem::Import(i) => imports.push(i),
                    TopLevelItem::Struct(s) => structs.push(s),
                    TopLevelItem::Sketch(s) => sketches.push(s),
                }
            }

            UnresolvedAst {
                imports,
                structs,
                sketches,
            }
        })
}

#[derive(Debug, Clone)]
enum TopLevelItem {
    Import(ImportDef),
    Struct(StructDef),
    Sketch(SketchDef),
}
