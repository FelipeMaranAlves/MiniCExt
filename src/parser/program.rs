//! Top-level program parser for MiniC.
//!
//! # Overview
//!
//! Exposes one public function:
//!
//! * [`program`] — parses a complete MiniC program as a sequence of zero or
//!   more function declarations and returns an
//!   [`UncheckedProgram`].
//!
//! A valid MiniC program contains **only** function declarations at the top
//! level — there are no top-level statements or variable declarations outside
//! of functions. This constraint is enforced here by the grammar: `program`
//! is defined as `many0(fun_decl)`, so any token that does not start a
//! function declaration causes the parse to stop. The type checker then
//! verifies that a `main` function exists.
//!
//! # Design Decisions
//!
//! ## `many0` as the top-level combinator
//!
//! `nom`'s `many0` combinator repeatedly applies a parser until it fails,
//! collecting results in a `Vec`. Using it here means the program parser
//! naturally handles empty programs (zero functions) and programs with any
//! number of functions with no extra branching logic. The existence of
//! `main` is a semantic constraint checked in the next pipeline stage, not
//! a syntactic one enforced here.

use crate::ir::ast::{Program, TopLevelItem, UncheckedProgram};
use crate::parser::functions::fun_decl;
use crate::parser::structs::struct_decl;
use nom::{branch::alt, combinator::map, multi::many0, IResult};

/// Parse a single top-level item: either a struct declaration or a function declaration.
fn top_level_item(input: &str) -> IResult<&str, TopLevelItem<()>> {
    alt((
        map(struct_decl, TopLevelItem::Struct),
        map(fun_decl, TopLevelItem::Function),
    ))(input)
}

/// Parse a complete MiniC program: zero or more top-level items (functions and structs).
pub fn program(input: &str) -> IResult<&str, UncheckedProgram> {
    map(many0(top_level_item), |items| Program { items })(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::ast::{StructDecl, StructField, Type};

    #[test]
    fn test_empty_program() {
        let (rest, prog) = program("").unwrap();
        assert_eq!(rest, "");
        assert!(prog.items.is_empty());
    }

    #[test]
    fn test_program_functions_only() {
        let input = "void f() {} int g() { return 0; }";
        let (rest, prog) = program(input).unwrap();
        assert_eq!(rest, "");
        assert_eq!(prog.items.len(), 2);
        assert!(matches!(prog.items[0], TopLevelItem::Function(_)));
        assert!(matches!(prog.items[1], TopLevelItem::Function(_)));
    }

    #[test]
    fn test_program_mixed_preserves_order() {
        let input = r#"
            struct P { int x; };
            void f() {}
            struct Q { float y; };
        "#;
        let (rest, prog) = program(input).unwrap();
        assert_eq!(rest.trim(), "");
        assert_eq!(prog.items.len(), 3);
        assert!(matches!(prog.items[0], TopLevelItem::Struct(ref s) if s.name == "P"));
        assert!(matches!(prog.items[1], TopLevelItem::Function(ref f) if f.name == "f"));
        assert!(matches!(prog.items[2], TopLevelItem::Struct(ref s) if s.name == "Q"));
    }
}