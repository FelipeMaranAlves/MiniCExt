//! Struct declaration parsers for MiniC.
//!
//! # Overview
//!
//! Exposes one public function:
//!
//! * [`struct_decl`] — parses a complete struct declaration:
//!   `struct Name { Type field; ... };`

use crate::ir::ast::{StructDecl, StructField, Type};
use crate::parser::functions::type_name;
use crate::parser::identifiers::identifier;
use nom::{
    bytes::complete::tag,
    character::complete::{char, multispace0, multispace1},
    combinator::{map, verify},
    multi::many1,
    sequence::{delimited, preceded, tuple},
    IResult,
};

/// Parse a single struct field: `Type name;`
/// 
/// Rejects `void` as a field type (Decision 5).
/// Nested structs are implicitly rejected because `type_name` is used
/// instead of `value_type_name` (Decision 8).
fn struct_field(input: &str) -> IResult<&str, StructField> {
    map(
        tuple((
            preceded(
                multispace0,
                // Verify that the field type is not void
                verify(type_name, |ty| *ty != Type::Unit),
            ),
            preceded(multispace1, identifier),
            preceded(multispace0, char(';')),
        )),
        |(ty, name, _)| StructField {
            name: name.to_string(),
            ty,
        },
    )(input)
}

/// Parse a complete struct declaration: `struct Name { Type field; ... };`
/// 
/// Requires at least one field (Decision 9).
pub fn struct_decl(input: &str) -> IResult<&str, StructDecl> {
    let (rest, _) = preceded(multispace0, tag("struct"))(input)?;
    let (rest, name) = preceded(multispace1, identifier)(rest)?;
    let (rest, fields) = delimited(
        preceded(multispace0, char('{')),
        // Use many1 to ensure at least one field is parsed
        many1(struct_field),
        preceded(multispace0, char('}')),
    )(rest)?;
    let (rest, _) = preceded(multispace0, char(';'))(rest)?;

    Ok((
        rest,
        StructDecl {
            name: name.to_string(),
            fields,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::ast::Type;
    use crate::parser::functions::struct_type_name;

    #[test]
    fn test_struct_type_name() {
        assert_eq!(
            struct_type_name("struct Pessoa"),
            Ok(("", Type::Struct("Pessoa".to_string())))
        );
        assert!(struct_type_name("int").is_err());
    }

    #[test]
    fn test_struct_decl_single_field() {
        let input = "struct Point { int x; };";
        let expected = StructDecl {
            name: "Point".to_string(),
            fields: vec![StructField {
                name: "x".to_string(),
                ty: Type::Int,
            }],
        };
        assert_eq!(struct_decl(input), Ok(("", expected)));
    }

    #[test]
    fn test_struct_decl_multiple_fields() {
        let input = "struct Point { int x; float y; bool z; };";
        let expected = StructDecl {
            name: "Point".to_string(),
            fields: vec![
                StructField {
                    name: "x".to_string(),
                    ty: Type::Int,
                },
                StructField {
                    name: "y".to_string(),
                    ty: Type::Float,
                },
                StructField {
                    name: "z".to_string(),
                    ty: Type::Bool,
                },
            ],
        };
        assert_eq!(struct_decl(input), Ok(("", expected)));
    }

    #[test]
    fn test_struct_decl_empty_fails() {
        // Structs must have at least one field (Decision 9)
        let input = "struct Empty {};";
        assert!(struct_decl(input).is_err());
    }

    #[test]
    fn test_struct_decl_void_field_fails() {
        // Struct fields cannot be of type void (Decision 5)
        let input = "struct Point { void x; };";
        assert!(struct_decl(input).is_err());
    }
    
    #[test]
    fn test_struct_decl_nested_struct_field_fails() {
        // Struct fields cannot be of type struct (Decision 8)
        let input = "struct Point { struct Other o; };";
        assert!(struct_decl(input).is_err());
    }
}
