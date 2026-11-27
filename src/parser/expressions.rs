//! Expression parsers for MiniC.
//!
//! # Overview
//!
//! Exposes three public functions:
//!
//! * [`expression`] — the top-level entry point; parses any MiniC expression.
//! * [`parse_call`] — parses a function call `name(arg, …)`; re-used by the
//!   statement parser to handle call-statements.
//! * [`postfix`] — parses the chain of `[ expr ]` and `.field` suffixes;
//!   re-used by the statement parser for assignment targets.
//!
//! # Design Decisions
//!
//! ## Precedence via a parser chain
//!
//! Operator precedence is encoded by the order in which parsers call each
//! other, from lowest to highest:
//!
//! ```text
//! expression → logical_or → logical_and → logical_not
//!           → relational → additive → multiplicative
//!           → unary → primary → atom
//! ```
//!
//! Each level calls the level above it for its operands, which naturally
//! gives higher-precedence operators tighter binding — the same technique
//! used in hand-written recursive-descent parsers. No separate precedence
//! table or Pratt parser is needed for the small MiniC operator set.
//!
//! ## Left-associativity via an accumulator loop
//!
//! Operators at the same precedence level (e.g., `+` and `-`) are
//! left-associative: `1 - 2 - 3` means `(1 - 2) - 3`. This is implemented
//! with an explicit `loop` that accumulates results into `acc` rather than
//! recursing on the right-hand side, which would accidentally produce
//! right-associative trees.

use crate::ir::ast::{Expr, ExprD, UncheckedExpr};
use crate::parser::identifiers::identifier;
use crate::parser::literals::literal;
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, multispace0},
    combinator::{map, opt},
    multi::{separated_list0, separated_list1},
    sequence::{delimited, pair, preceded, terminated, tuple},
    IResult,
};

fn wrap(e: Expr<()>) -> UncheckedExpr {
    ExprD { exp: e, ty: () }
}

/// Parse a function call: `identifier ( expr_list )`. Returns (name, args).
pub fn parse_call(input: &str) -> IResult<&str, (String, Vec<UncheckedExpr>)> {
    let (rest, name) = preceded(multispace0, identifier)(input)?;
    let (rest, args) = delimited(
        preceded(multispace0, tag("(")),
        separated_list0(
            preceded(multispace0, tag(",")),
            preceded(multispace0, expression),
        ),
        preceded(multispace0, tag(")")),
    )(rest)?;
    Ok((rest, (name.to_string(), args)))
}

/// Parse a single field initializer in a struct literal: `field_name: expr`
fn struct_field_init(input: &str) -> IResult<&str, (String, UncheckedExpr)> {
    let (rest, field_name) = preceded(multispace0, identifier)(input)?;
    let (rest, _) = preceded(multispace0, char(':'))(rest)?;
    let (rest, val) = preceded(multispace0, expression)(rest)?;
    Ok((rest, (field_name.to_string(), val)))
}

/// Parse a struct literal: `Name { field: expr, ... }`.
/// Requires at least one field (Decision 9). Trailing comma is optional.
pub fn struct_literal(input: &str) -> IResult<&str, UncheckedExpr> {
    let (rest, name) = preceded(multispace0, identifier)(input)?;
    let (rest, fields) = delimited(
        preceded(multispace0, char('{')),
        terminated(
            separated_list1(preceded(multispace0, tag(",")), struct_field_init),
            opt(preceded(multispace0, tag(","))),
        ),
        preceded(multispace0, char('}')),
    )(rest)?;
    Ok((
        rest,
        wrap(Expr::StructLit {
            name: name.to_string(),
            fields,
        }),
    ))
}

/// Atom: literal, call, struct literal, array literal, identifier, or parenthesized expression.
fn atom(input: &str) -> IResult<&str, UncheckedExpr> {
    alt((
        map(literal, |l| wrap(Expr::Literal(l.into()))),
        map(parse_call, |(name, args)| wrap(Expr::Call { name, args })),
        struct_literal,
        map(
            delimited(
                preceded(multispace0, char('[')),
                separated_list0(
                    preceded(multispace0, tag(",")),
                    preceded(multispace0, expression),
                ),
                preceded(multispace0, char(']')),
            ),
            |elems| wrap(Expr::ArrayLit(elems)),
        ),
        map(identifier, |s: &str| wrap(Expr::Ident(s.to_string()))),
        delimited(
            preceded(multispace0, char('(')),
            preceded(multispace0, expression),
            preceded(multispace0, char(')')),
        ),
    ))(input)
}

/// Primary: atom followed by its postfix chain.
fn primary(input: &str) -> IResult<&str, UncheckedExpr> {
    let (rest, base) = atom(input)?;
    postfix(rest, base)
}

/// Postfix chain applied to `base`: zero or more `[ expr ]` or `.field`, in any order.
/// Accumulating left to right makes `p.v[i]` an `Index` over a `Field`.
pub fn postfix(input: &str, base: UncheckedExpr) -> IResult<&str, UncheckedExpr> {
    let (mut rest, mut acc) = (input, base);
    loop {
        let index = delimited(
            preceded(multispace0, char('[')),
            preceded(multispace0, expression),
            preceded(multispace0, char(']')),
        )(rest);
        if let Ok((r, index)) = index {
            acc = wrap(Expr::Index {
                base: Box::new(acc),
                index: Box::new(index),
            });
            rest = r;
            continue;
        }
        let field = preceded(
            preceded(multispace0, char('.')),
            preceded(multispace0, identifier),
        )(rest);
        if let Ok((r, field)) = field {
            acc = wrap(Expr::Field {
                base: Box::new(acc),
                field: field.to_string(),
            });
            rest = r;
            continue;
        }
        break;
    }
    Ok((rest, acc))
}

/// Unary: optional unary `-` applied to primary.
fn unary(input: &str) -> IResult<&str, UncheckedExpr> {
    alt((
        map(pair(preceded(multispace0, tag("-")), unary), |(_, e)| {
            wrap(Expr::Neg(Box::new(e)))
        }),
        map(pair(preceded(multispace0, tag("&")), unary), |(_, e)| {
            wrap(Expr::AddrOf(Box::new(e)))
        }),
        map(pair(preceded(multispace0, tag("*")), unary), |(_, e)| {
            wrap(Expr::Deref(Box::new(e)))
        }),
        primary,
    ))(input)
}

/// Multiplicative: unary with `*` and `/` (left-associative).
fn multiplicative(input: &str) -> IResult<&str, UncheckedExpr> {
    let (mut rest, mut acc) = unary(input)?;
    loop {
        let mul = tuple((multispace0, tag("*"), multispace0, unary))(rest);
        if let Ok((r, (_, _, _, e))) = mul {
            acc = wrap(Expr::Mul(Box::new(acc), Box::new(e)));
            rest = r;
            continue;
        }
        let div = tuple((multispace0, tag("/"), multispace0, unary))(rest);
        if let Ok((r, (_, _, _, e))) = div {
            acc = wrap(Expr::Div(Box::new(acc), Box::new(e)));
            rest = r;
            continue;
        }
        break;
    }
    Ok((rest, acc))
}

/// Additive: multiplicative with `+` and `-` (left-associative).
fn additive(input: &str) -> IResult<&str, UncheckedExpr> {
    let (mut rest, mut acc) = multiplicative(input)?;
    loop {
        let add = tuple((multispace0, tag("+"), multispace0, multiplicative))(rest);
        if let Ok((r, (_, _, _, e))) = add {
            acc = wrap(Expr::Add(Box::new(acc), Box::new(e)));
            rest = r;
            continue;
        }
        let sub = tuple((multispace0, tag("-"), multispace0, multiplicative))(rest);
        if let Ok((r, (_, _, _, e))) = sub {
            acc = wrap(Expr::Sub(Box::new(acc), Box::new(e)));
            rest = r;
            continue;
        }
        break;
    }
    Ok((rest, acc))
}

/// Relational: additive with ==, !=, <, <=, >, >=
fn relational(input: &str) -> IResult<&str, UncheckedExpr> {
    let (mut rest, mut acc) = additive(input)?;
    loop {
        let ops = alt((
            tuple((multispace0, tag("=="), multispace0, additive)),
            tuple((multispace0, tag("!="), multispace0, additive)),
            tuple((multispace0, tag("<="), multispace0, additive)),
            tuple((multispace0, tag(">="), multispace0, additive)),
            tuple((multispace0, tag("<"), multispace0, additive)),
            tuple((multispace0, tag(">"), multispace0, additive)),
        ))(rest);
        match ops {
            Ok((r, (_, op, _, e))) => {
                acc = wrap(match op {
                    "==" => Expr::Eq(Box::new(acc), Box::new(e)),
                    "!=" => Expr::Ne(Box::new(acc), Box::new(e)),
                    "<" => Expr::Lt(Box::new(acc), Box::new(e)),
                    "<=" => Expr::Le(Box::new(acc), Box::new(e)),
                    ">" => Expr::Gt(Box::new(acc), Box::new(e)),
                    ">=" => Expr::Ge(Box::new(acc), Box::new(e)),
                    _ => unreachable!(),
                });
                rest = r;
            }
            Err(_) => break,
        }
    }
    Ok((rest, acc))
}

/// Logical not: optional `!` applied to relational.
fn logical_not(input: &str) -> IResult<&str, UncheckedExpr> {
    alt((
        map(
            pair(
                preceded(multispace0, char('!')),
                preceded(multispace0, logical_not),
            ),
            |(_, e)| wrap(Expr::Not(Box::new(e))),
        ),
        relational,
    ))(input)
}

/// Logical and: logical_not with `and` (left-associative).
fn logical_and(input: &str) -> IResult<&str, UncheckedExpr> {
    let (mut rest, mut acc) = logical_not(input)?;
    loop {
        let and = tuple((multispace0, tag("and"), multispace0, logical_not))(rest);
        if let Ok((r, (_, _, _, e))) = and {
            acc = wrap(Expr::And(Box::new(acc), Box::new(e)));
            rest = r;
            continue;
        }
        break;
    }
    Ok((rest, acc))
}

/// Logical or: logical_and with `or` (left-associative).
fn logical_or(input: &str) -> IResult<&str, UncheckedExpr> {
    let (mut rest, mut acc) = logical_and(input)?;
    loop {
        let or = tuple((multispace0, tag("or"), multispace0, logical_and))(rest);
        if let Ok((r, (_, _, _, e))) = or {
            acc = wrap(Expr::Or(Box::new(acc), Box::new(e)));
            rest = r;
            continue;
        }
        break;
    }
    Ok((rest, acc))
}

/// Top-level expression parser. Returns UncheckedExpr with ty: () at each node.
pub fn expression(input: &str) -> IResult<&str, UncheckedExpr> {
    preceded(multispace0, logical_or)(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::ast::Literal;
    use crate::parser::statements::statement;

    #[test]
    fn test_struct_literal_single_field() {
        let (rest, expr) = expression("Point { x: 10 }").unwrap();
        assert_eq!(rest, "");
        if let Expr::StructLit { name, fields } = expr.exp {
            assert_eq!(name, "Point");
            assert_eq!(fields.len(), 1);
            assert_eq!(fields[0].0, "x");
            assert_eq!(fields[0].1.exp, Expr::Literal(Literal::Int(10)));
        } else {
            panic!("expected StructLit, got {:?}", expr);
        }
    }

    #[test]
    fn test_struct_literal_multiple_fields() {
        let (rest, expr) = expression("Pessoa { idade: 30, nome: \"Ana\" }").unwrap();
        assert_eq!(rest, "");
        if let Expr::StructLit { name, fields } = expr.exp {
            assert_eq!(name, "Pessoa");
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].0, "idade");
            assert_eq!(fields[0].1.exp, Expr::Literal(Literal::Int(30)));
            assert_eq!(fields[1].0, "nome");
            assert_eq!(
                fields[1].1.exp,
                Expr::Literal(Literal::Str("Ana".to_string()))
            );
        } else {
            panic!("expected StructLit, got {:?}", expr);
        }
    }

    #[test]
    fn test_struct_literal_trailing_comma() {
        let (rest, expr) = expression("Point { x: 1, y: 2, }").unwrap();
        assert_eq!(rest, "");
        if let Expr::StructLit { name, fields } = expr.exp {
            assert_eq!(name, "Point");
            assert_eq!(fields.len(), 2);
        } else {
            panic!("expected StructLit, got {:?}", expr);
        }
    }

    #[test]
    fn test_struct_literal_empty_fails() {
        // Empty struct literal should not parse as StructLit (Decision 9 requires >= 1 field)
        let (rest, expr) = expression("Point {}").unwrap();
        // Since Point {} fails struct_literal, it falls back to identifier "Point" leaving "{}"
        assert_eq!(rest, " {}");
        assert_eq!(expr.exp, Expr::Ident("Point".to_string()));
    }

    #[test]
    fn test_struct_literal_field_access_postfix() {
        let (rest, expr) = expression("Pessoa { idade: 30, nome: \"Ana\" }.nome").unwrap();
        assert_eq!(rest, "");
        if let Expr::Field { base, field } = expr.exp {
            assert_eq!(field, "nome");
            assert!(matches!(base.exp, Expr::StructLit { .. }));
        } else {
            panic!("expected Field access on StructLit, got {:?}", expr);
        }
    }

    #[test]
    fn test_if_with_empty_block_not_confused_with_struct_literal() {
        let (rest, stmt) = statement("if x {}").unwrap();
        assert_eq!(rest, "");
        assert!(matches!(stmt.stmt, crate::ir::ast::Statement::If { .. }));
    }

    #[test]
    fn test_if_with_assignment_block_not_confused_with_struct_literal() {
        let (rest, stmt) = statement("if x { y = 1; }").unwrap();
        assert_eq!(rest, "");
        assert!(matches!(stmt.stmt, crate::ir::ast::Statement::If { .. }));
    }

    #[test]
    fn test_while_not_confused_with_struct_literal() {
        let (rest, stmt) = statement("while ok { print(ok); }").unwrap();
        assert_eq!(rest, "");
        assert!(matches!(stmt.stmt, crate::ir::ast::Statement::While { .. }));
    }
}
