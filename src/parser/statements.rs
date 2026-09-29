//! Statement parsers for MiniC.
//!
//! # Overview
//!
//! Exposes two public functions:
//!
//! * [`statement`] — the top-level entry point; tries each statement form in
//!   order: `return`, `if`, `while`, call-statement, block, declaration,
//!   assignment.
//! * [`assignment`] — parses `lvalue = expression ;`; exported separately
//!   because the test suite uses it directly.
//!
//! # Grammar
//!
//! ```text
//! statement  := block | if_stmt | while_stmt | simple ';'
//! block      := '{' statement* '}'
//! if_stmt    := 'if' expr block ['else' block]
//! while_stmt := 'while' expr block
//! simple     := return | decl | call | assign
//! decl       := value_type ident ['=' expr]
//! assign     := lvalue '=' expr
//! lvalue     := ident ('[' expr ']' | '.' ident)*
//! ```
//!
//! Every simple statement is terminated by `;`.
//! Compound statements (`if`, `while`, block) end with `}` and need no `;`.
//!
//! # Design Decisions
//!
//! ## Declaration must be tried before assignment
//!
//! Both `int x = 0` (declaration) and `x = 0` (assignment) begin with an
//! identifier-like token, so the order of alternatives in [`statement`]
//! matters. Declaration is tried first because it starts with a type keyword
//! (`int`, `float`, …), which is unambiguous. If declaration fails, the
//! parser backtracks and tries assignment.
//!
//! ## The initializer is optional for every type
//!
//! `T x;` and `T x = e;` go through the same path and differ only in
//! `Decl.init` (`None` / `Some`). The parser never branches on the type:
//! giving `None` its zero value is the type checker's job.
//!
//! ## `lvalue` shares the postfix chain with expressions
//!
//! An assignment target can be a plain variable (`x = …`), an array element
//! (`a[i][j] = …`), a struct field (`p.campo = …`) or any mix of them
//! (`p.v[i] = …`). The private `lvalue` parser starts from an identifier and
//! reuses [`postfix`] from `expressions.rs`, so reads and writes accept
//! exactly the same suffixes.

use crate::ir::ast::{Expr, ExprD, Statement, StatementD, UncheckedExpr, UncheckedStmt};
use crate::parser::expressions::{expression, parse_call, postfix};
use crate::parser::functions::value_type_name;
use crate::parser::identifiers::identifier;
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, multispace0},
    combinator::{map, opt},
    multi::many0,
    sequence::{delimited, preceded, tuple},
    IResult,
};

fn wrap(s: Statement<()>) -> UncheckedStmt {
    StatementD { stmt: s, ty: () }
}

/// Parse any statement: block | if | while | return | decl | call | assignment.
pub fn statement(input: &str) -> IResult<&str, UncheckedStmt> {
    preceded(
        multispace0,
        alt((
            block_statement,
            if_statement,
            while_statement,
            return_statement,
            decl_statement,
            call_statement,
            assignment,
        )),
    )(input)
}

/// Parse a return statement: `return [expr] ;`.
fn return_statement(input: &str) -> IResult<&str, UncheckedStmt> {
    let (rest, _) = preceded(multispace0, tag("return"))(input)?;
    let (rest, expr) = opt(preceded(multispace0, expression))(rest)?;
    let (rest, _) = preceded(multispace0, char(';'))(rest)?;
    Ok((rest, wrap(Statement::Return(expr.map(Box::new)))))
}

/// Parse a variable declaration: `Type ident [= expr] ;`. Must come before assignment.
/// `Type` may be `struct Name`.
fn decl_statement(input: &str) -> IResult<&str, UncheckedStmt> {
    map(
        tuple((
            value_type_name,
            preceded(nom::character::complete::multispace1, identifier),
            opt(preceded(
                preceded(multispace0, tag("=")),
                preceded(multispace0, expression),
            )),
            preceded(multispace0, char(';')),
        )),
        |(ty, name, init, _)| {
            wrap(Statement::Decl {
                name: name.to_string(),
                ty,
                init: init.map(Box::new),
            })
        },
    )(input)
}

/// Parse a block statement: `{ stmt* }`.
/// Each statement inside the block carries its own terminator (`;` or `}`).
fn block_statement(input: &str) -> IResult<&str, UncheckedStmt> {
    map(
        delimited(
            preceded(multispace0, char('{')),
            many0(preceded(multispace0, statement)),
            preceded(multispace0, char('}')),
        ),
        |seq| wrap(Statement::Block { seq }),
    )(input)
}

/// Parse a function call as a statement: `identifier ( expr_list ) ;`.
fn call_statement(input: &str) -> IResult<&str, UncheckedStmt> {
    let (rest, (name, args)) = parse_call(input)?;
    let (rest, _) = preceded(multispace0, char(';'))(rest)?;
    Ok((rest, wrap(Statement::Call { name, args })))
}

/// Parse an if statement: `if expr block ['else' block]`.
/// Both branches must be blocks — bare statements are not allowed.
fn if_statement(input: &str) -> IResult<&str, UncheckedStmt> {
    let (rest, _) = preceded(multispace0, tag("if"))(input)?;
    let (rest, cond) = preceded(multispace0, expression)(rest)?;
    let (rest, then_branch) = preceded(multispace0, block_statement)(rest)?;
    let (rest, else_branch) = opt(map(
        tuple((
            preceded(multispace0, tag("else")),
            preceded(multispace0, block_statement),
        )),
        |(_, stmt)| stmt,
    ))(rest)?;
    Ok((
        rest,
        wrap(Statement::If {
            cond: Box::new(cond),
            then_branch: Box::new(then_branch),
            else_branch: else_branch.map(Box::new),
        }),
    ))
}

/// Parse a while statement: `while expr block`.
/// The body must be a block — bare statements are not allowed.
fn while_statement(input: &str) -> IResult<&str, UncheckedStmt> {
    let (rest, _) = preceded(multispace0, tag("while"))(input)?;
    let (rest, cond) = preceded(multispace0, expression)(rest)?;
    let (rest, body) = preceded(multispace0, block_statement)(rest)?;
    Ok((
        rest,
        wrap(Statement::While {
            cond: Box::new(cond),
            body: Box::new(body),
        }),
    ))
}

/// Parse an lvalue: identifier followed by zero or more `[ expr ]` / `.field` suffixes.
/// Starts from an identifier, not an atom, so literals and calls are never targets.
fn lvalue(input: &str) -> IResult<&str, UncheckedExpr> {
    let (rest, id) = preceded(multispace0, identifier)(input)?;
    let base = ExprD {
        exp: Expr::Ident(id.to_string()),
        ty: (),
    };
    postfix(rest, base)
}

/// Parse an assignment statement: `lvalue = expression ;`.
pub fn assignment(input: &str) -> IResult<&str, UncheckedStmt> {
    map(
        tuple((
            lvalue,
            preceded(multispace0, nom::bytes::complete::tag("=")),
            preceded(multispace0, expression),
            preceded(multispace0, char(';')),
        )),
        |(target, _, value, _)| {
            wrap(Statement::Assign {
                target: Box::new(target),
                value: Box::new(value),
            })
        },
    )(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::ast::{Literal, Type};

    fn e(exp: Expr<()>) -> Box<UncheckedExpr> {
        Box::new(ExprD { exp, ty: () })
    }

    fn ident(name: &str) -> Box<UncheckedExpr> {
        e(Expr::Ident(name.to_string()))
    }

    fn field(base: Box<UncheckedExpr>, name: &str) -> Box<UncheckedExpr> {
        e(Expr::Field {
            base,
            field: name.to_string(),
        })
    }

    fn decl(name: &str, ty: Type, init: Option<Box<UncheckedExpr>>) -> UncheckedStmt {
        wrap(Statement::Decl {
            name: name.to_string(),
            ty,
            init,
        })
    }

    fn assign(target: Box<UncheckedExpr>, value: Box<UncheckedExpr>) -> UncheckedStmt {
        wrap(Statement::Assign { target, value })
    }

    #[test]
    fn test_struct_decl_without_init() {
        assert_eq!(
            statement("struct Pessoa p;"),
            Ok(("", decl("p", Type::Struct("Pessoa".to_string()), None)))
        );
    }

    #[test]
    fn test_scalar_and_array_decl_without_init() {
        let cases = [
            ("int x;", "x", Type::Int),
            ("float f;", "f", Type::Float),
            ("bool b;", "b", Type::Bool),
            ("str s;", "s", Type::Str),
            ("int[] a;", "a", Type::Array(Box::new(Type::Int))),
        ];
        for (src, name, ty) in cases {
            assert_eq!(statement(src), Ok(("", decl(name, ty, None))), "{src}");
        }
    }

    #[test]
    fn test_decl_with_init_unchanged() {
        assert_eq!(
            statement("int x = 1;"),
            Ok((
                "",
                decl("x", Type::Int, Some(e(Expr::Literal(Literal::Int(1)))))
            ))
        );
    }

    #[test]
    fn test_struct_decl_with_init_is_accepted() {
        // Decision 18 (no struct-to-struct copy) is enforced by the checker, not here.
        assert_eq!(
            statement("struct Pessoa p = q;"),
            Ok((
                "",
                decl("p", Type::Struct("Pessoa".to_string()), Some(ident("q")))
            ))
        );
    }

    #[test]
    fn test_field_read() {
        let (rest, expr) = expression("p.campo").unwrap();
        assert_eq!(rest, "");
        assert_eq!(Box::new(expr), field(ident("p"), "campo"));
    }

    #[test]
    fn test_field_in_arithmetic() {
        let (rest, expr) = expression("p.campo + 1").unwrap();
        assert_eq!(rest, "");
        assert_eq!(
            Box::new(expr),
            e(Expr::Add(
                field(ident("p"), "campo"),
                e(Expr::Literal(Literal::Int(1)))
            ))
        );
    }

    #[test]
    fn test_field_and_index_chain() {
        let (_, expr) = expression("p.valores[i]").unwrap();
        assert_eq!(
            Box::new(expr),
            e(Expr::Index {
                base: field(ident("p"), "valores"),
                index: ident("i"),
            })
        );
        let (_, expr) = expression("a[i].x").unwrap();
        assert_eq!(
            Box::new(expr),
            field(
                e(Expr::Index {
                    base: ident("a"),
                    index: ident("i"),
                }),
                "x"
            )
        );
    }

    #[test]
    fn test_field_assignment() {
        assert_eq!(
            statement("p.campo = 42;"),
            Ok((
                "",
                assign(
                    field(ident("p"), "campo"),
                    e(Expr::Literal(Literal::Int(42)))
                )
            ))
        );
        assert_eq!(
            statement("p.v[0] = 1;"),
            Ok((
                "",
                assign(
                    e(Expr::Index {
                        base: field(ident("p"), "v"),
                        index: e(Expr::Literal(Literal::Int(0))),
                    }),
                    e(Expr::Literal(Literal::Int(1)))
                )
            ))
        );
    }

    #[test]
    fn test_rejected_forms() {
        for src in ["struct Pessoa;", "int x", "p. = 1;", "p.1 = 2;"] {
            assert!(statement(src).is_err(), "{src}");
        }
    }

    #[test]
    fn test_keyword_prefixed_names_are_assignments() {
        // `int`/`struct` are matched as prefixes; the mandatory space after the type
        // keeps `integer` and `structure` as plain identifiers.
        let one = || e(Expr::Literal(Literal::Int(1)));
        assert_eq!(
            statement("integer = 1;"),
            Ok(("", assign(ident("integer"), one())))
        );
        assert_eq!(
            statement("structure = 1;"),
            Ok(("", assign(ident("structure"), one())))
        );
    }
}
