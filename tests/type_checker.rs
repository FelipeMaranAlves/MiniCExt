//! Integration tests for the MiniC type checker.

use std::path::Path;

use nom::combinator::all_consuming;
use mini_c::ir::ast::{CheckedProgram, Expr, Statement, Type};
use mini_c::parser::program;
use mini_c::semantic::type_check;

fn parse_and_type_check(src: &str) -> Result<CheckedProgram, mini_c::semantic::TypeError> {
    let (_, prog) = all_consuming(program)(src).map_err(|_| {
        mini_c::semantic::TypeError {
            message: "parse failed".to_string(),
        }
    })?;
    type_check(&prog)
}

fn parse_and_type_check_fixture(name: &str) -> Result<CheckedProgram, mini_c::semantic::TypeError> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let src = std::fs::read_to_string(&path).expect("fixture file should exist");
    parse_and_type_check(src.trim())
}

#[test]
fn test_type_check_simple_assign() {
    let result = parse_and_type_check("void main() int x = 1;");
    assert!(result.is_ok());
}

#[test]
fn test_type_check_int_float_coercion() {
    let result = parse_and_type_check("void main() float x = 1 + 3.14;");
    assert!(result.is_ok());
    let prog = result.unwrap();
    let main_fn = prog.functions().find(|f| f.name == "main").unwrap();
    if let mini_c::ir::ast::Statement::Decl { init: Some(ref init), .. } = main_fn.body.stmt {
        assert_eq!(init.ty, Type::Float);
    } else {
        panic!("expected Decl");
    }
}

#[test]
fn test_type_check_undeclared_var() {
    let result = parse_and_type_check("void main() x = y;");
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("undeclared"));
}

#[test]
fn test_type_check_bool_condition() {
    let result = parse_and_type_check("void main() if true { int x = 1; }");
    assert!(result.is_ok());
}

#[test]
fn test_type_check_array_literal() {
    let result = parse_and_type_check("void main() int[] x = [1, 2, 3];");
    assert!(result.is_ok());
}

#[test]
fn test_type_check_array_index() {
    let result = parse_and_type_check("void main() { int[] arr = [1, 2]; int x = arr[0]; }");
    assert!(result.is_ok());
}

#[test]
fn test_type_check_call_arg_type_mismatch() {
    let result = parse_and_type_check("void foo(int x) x = x;\nvoid main() foo(true);");
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("argument"));
}

#[test]
fn test_type_check_missing_main() {
    let result = parse_and_type_check("void foo() int x = 1;");
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("main"));
}

#[test]
fn test_type_check_decl_then_assign() {
    let result = parse_and_type_check("void main() { int x = 1; x = 2; }");
    assert!(result.is_ok());
}

#[test]
fn test_type_check_assign_undeclared() {
    let result = parse_and_type_check("void main() x = 1;");
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("undeclared"));
}

#[test]
fn test_type_check_redeclaration() {
    let result = parse_and_type_check("void main() { int x = 1; int x = 2; }");
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("redeclaration"));
}

#[test]
fn test_type_check_relational_type_mismatch() {
    let result = parse_and_type_check("void main() bool r = \"hello\" == 42;");
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("compatible"));
}

#[test]
fn test_type_check_ordering_requires_numeric() {
    let result = parse_and_type_check("void main() bool r = true < false;");
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("numeric"));
}

#[test]
fn test_type_check_equality_same_type_ok() {
    let result = parse_and_type_check("void main() bool r = 1 == 2;");
    assert!(result.is_ok());
}

#[test]
fn test_type_check_main_non_void_return() {
    let result = parse_and_type_check("int main() int x = 1;");
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("void"));
}

#[test]
fn test_type_check_main_with_params() {
    let result = parse_and_type_check("void main(int x) int y = 1;");
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("parameters"));
}

#[test]
fn test_type_check_return_void_ok() {
    let result = parse_and_type_check("void main() { int x = 1; return; }");
    assert!(result.is_ok());
}

#[test]
fn test_type_check_return_value_in_void_fn() {
    let result = parse_and_type_check("void main() return 1;");
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("void function must not return a value"));
}

#[test]
fn test_type_check_return_correct_type() {
    let result = parse_and_type_check("int foo() return 42;\nvoid main() int x = 1;");
    assert!(result.is_ok());
}

#[test]
fn test_type_check_return_wrong_type() {
    let result = parse_and_type_check("int foo() return true;\nvoid main() int x = 1;");
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("return type mismatch"));
}

#[test]
fn test_type_check_block_scoping() {
    // variable declared inside a block should not be visible after the block
    let result = parse_and_type_check("void main() { { int x = 1; } x = 2; }");
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("undeclared"));
}

// ---------------------------------------------------------------------------
// 7.2 print accepts any type via Type::Any
// ---------------------------------------------------------------------------
#[test]
fn test_type_check_print_accepts_int() {
    assert!(parse_and_type_check("void main() { print(42); }").is_ok());
}

#[test]
fn test_type_check_print_accepts_bool() {
    assert!(parse_and_type_check("void main() { print(true); }").is_ok());
}

#[test]
fn test_type_check_print_accepts_str() {
    assert!(parse_and_type_check("void main() { print(\"hello\"); }").is_ok());
}

#[test]
fn test_type_check_print_accepts_float() {
    assert!(parse_and_type_check("void main() { print(3.14); }").is_ok());
}

#[test]
fn test_type_check_print_accepts_array() {
    assert!(parse_and_type_check("void main() { print([1, 2, 3]); }").is_ok());
}

// ---------------------------------------------------------------------------
// 7.3 Wrong arity on stdlib function reports type error
// ---------------------------------------------------------------------------
#[test]
fn test_type_check_sqrt_wrong_arity() {
    let result = parse_and_type_check("void main() { float r = sqrt(); }");
    assert!(result.is_err(), "expected arity error for sqrt()");
}

#[test]
fn test_type_check_print_wrong_arity() {
    let result = parse_and_type_check("void main() { print(1, 2); }");
    assert!(result.is_err(), "expected arity error for print(1, 2)");
}

// ---------------------------------------------------------------------------
// Pointers
// ---------------------------------------------------------------------------
#[test]
fn test_type_check_pointer_init_fixture() {
    assert!(
        parse_and_type_check_fixture("pointer_init.minic").is_ok(),
        "{}",
        parse_and_type_check_fixture("pointer_init.minic")
            .unwrap_err()
            .message
    );
}

#[test]
fn test_type_check_pointer_feature_fixture() {
    assert!(parse_and_type_check_fixture("pointer_feature.minic").is_ok());
}

#[test]
fn test_type_check_pointer_function_fixture() {
    assert!(parse_and_type_check_fixture("pointer_function.minic").is_ok());
}

#[test]
fn test_type_check_addr_of_and_deref_types() {
    let prog = parse_and_type_check(
        "void main() { int x = 1; int* p = &x; int y = *p; }",
    )
    .expect("pointer decl and deref should type-check");

    let main_fn = prog.functions().find(|f| f.name == "main").unwrap();
    let Statement::Block { seq } = &main_fn.body.stmt else {
        panic!("expected block body");
    };

    let p_init = match &seq[1].stmt {
        Statement::Decl { init: Some(init), .. } => init,
        _ => panic!("expected int* p = &x decl"),
    };
    assert_eq!(p_init.ty, Type::Pointer(Box::new(Type::Int)));
    assert!(matches!(p_init.exp, Expr::AddrOf(_)));

    let y_init = match &seq[2].stmt {
        Statement::Decl { init: Some(init), .. } => init,
        _ => panic!("expected int y = *p decl"),
    };
    assert_eq!(y_init.ty, Type::Int);
    assert!(matches!(y_init.exp, Expr::Deref(_)));
}

#[test]
fn test_type_check_pointer_param_and_deref_assign() {
    let prog = parse_and_type_check(
        "void bump(int* p) { *p = *p + 1; }\nvoid main() { int x = 0; bump(&x); }",
    )
    .expect("pointer param and assignment through deref");

    let bump = prog.functions().find(|f| f.name == "bump").unwrap();
    assert_eq!(
        bump.params,
        vec![("p".to_string(), Type::Pointer(Box::new(Type::Int)))]
    );
}

#[test]
fn test_type_check_pointer_return_type() {
    let prog = parse_and_type_check(
        "int* id(int* p) { return p; }\nvoid main() { int x = 1; int* q = id(&x); }",
    )
    .expect("pointer return");

    let id_fn = prog.functions().find(|f| f.name == "id").unwrap();
    assert_eq!(id_fn.return_type, Type::Pointer(Box::new(Type::Int)));
}

#[test]
fn test_type_check_pointer_rebind() {
    assert!(parse_and_type_check(
        "void main() { int x = 1; int y = 2; int* a = &x; int* b = &y; a = b; }",
    )
    .is_ok());
}

#[test]
fn test_type_check_deref_non_pointer() {
    let result = parse_and_type_check("void main() { int x = 1; int y = *x; }");
    assert!(result.is_err());
    assert!(
        result.unwrap_err().message.contains("cannot dereference non-pointer")
    );
}

#[test]
fn test_type_check_addr_of_non_variable() {
    let result = parse_and_type_check("void main() { int x = 1; int* p = &(x + 1); }");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .message
            .contains("can only take address of variables")
    );
}

#[test]
fn test_type_check_deref_assign_type_mismatch() {
    let result = parse_and_type_check(
        "void main() { int x = 1; int* p = &x; *p = true; }",
    );
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .message
            .contains("assignment through pointer")
    );
}

#[test]
fn test_type_check_call_expects_pointer_arg() {
    let result = parse_and_type_check(
        "void take(int* p) { *p = 1; }\nvoid main() { int x = 0; take(x); }",
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("argument"));
}

#[test]
fn test_type_check_assign_int_from_pointer() {
    let result = parse_and_type_check(
        "void main() { int x = 1; int* p = &x; int y = p; }",
    );
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.message.contains("declaration"),
        "unexpected error: {}",
        err.message
    );
}

// --- Declaration without initializer ---

#[test]
fn test_type_check_decl_without_init_any_type() {
    for decl in ["int x;", "float f;", "bool b;", "str s;", "int[] a;", "int* p;"] {
        let src = format!("void main() {{ {decl} }}");
        assert!(parse_and_type_check(&src).is_ok(), "{decl}");
    }
}

#[test]
fn test_type_check_decl_without_init_keeps_declared_type() {
    // `x` is still an int: later uses are checked against `int`.
    assert!(parse_and_type_check("void main() { int x; x = 1; int y = x + 1; }").is_ok());
    let err = parse_and_type_check("void main() { int x; x = \"a\"; }").unwrap_err();
    assert!(err.message.contains("assignment to x"), "{}", err.message);
}

#[test]
fn test_type_check_decl_without_init_rules_still_apply() {
    let err = parse_and_type_check("void main() { void v; }").unwrap_err();
    assert!(err.message.contains("void"), "{}", err.message);
    let err = parse_and_type_check("void main() { int x; int x; }").unwrap_err();
    assert!(err.message.contains("redeclaration"), "{}", err.message);
}

#[test]
fn test_type_check_decl_without_init_keeps_none() {
    let prog = parse_and_type_check("void main() int x;").unwrap();
    let main_fn = prog.main_function().unwrap();
    assert!(matches!(main_fn.body.stmt, Statement::Decl { init: None, .. }));
}

// --- Struct ---

#[test]
fn test_type_check_struct_valid_definition() {
    let src = "struct Point { int x; int y; }; void main() {}";
    assert!(parse_and_type_check(src).is_ok());
}

#[test]
fn test_type_check_struct_duplicate_definition_fails() {
    let src = "struct Point { int x; }; struct Point { float y; }; void main() {}";
    let err = parse_and_type_check(src).unwrap_err();
    assert!(err.message.contains("duplicate struct definition"), "{}", err.message);
}

#[test]
fn test_type_check_struct_name_collision_with_function() {
    let src = "void Point() {} struct Point { int x; }; void main() {}";
    let err = parse_and_type_check(src).unwrap_err();
    assert!(err.message.contains("name collision"), "{}", err.message);
}

#[test]
fn test_type_check_function_name_collision_with_struct() {
    let src = "struct Point { int x; }; void Point() {} void main() {}";
    let err = parse_and_type_check(src).unwrap_err();
    assert!(err.message.contains("name collision"), "{}", err.message);
}

#[test]
fn test_type_check_struct_duplicate_fields_fails() {
    let src = "struct Point { int x; float x; }; void main() {}";
    let err = parse_and_type_check(src).unwrap_err();
    assert!(err.message.contains("duplicate field 'x'"), "{}", err.message);
}

#[test]
fn test_type_check_function_with_struct_param_fails() {
    // Decision 15: functions cannot have struct parameters
    let src = "struct Point { int x; }; void f(Point p) {} void main() {}";
    // Note: the parser itself rejects UserDefined in params, returning a parse error
    let err = parse_and_type_check(src).unwrap_err();
    assert!(err.message.contains("parse failed") || err.message.contains("cannot have a struct type"));
}

// --- Local Variable Declarations with Structs (Task B - Step 2) ---

#[test]
fn test_type_check_struct_local_decl_valid() {
    let src = "struct Point { int x; }; void main() { Point p; }";
    assert!(parse_and_type_check(src).is_ok());
}

#[test]
fn test_type_check_struct_undefined_type_fails() {
    let src = "void main() { Fantasma f; }";
    let err = parse_and_type_check(src).unwrap_err();
    assert!(err.message.contains("undefined type: 'Fantasma'"), "{}", err.message);
}

#[test]
fn test_type_check_struct_use_before_decl_fails() {
    // Decision 11: declaration before use
    let src = "void main() { Point p; } struct Point { int x; };";
    let err = parse_and_type_check(src).unwrap_err();
    assert!(err.message.contains("undefined type: 'Point'"), "{}", err.message);
}

#[test]
fn test_type_check_local_var_name_collision_with_struct() {
    // Decision 12: local variable cannot share name with a struct
    let src = "struct Point { int x; }; void main() { int Point = 10; }";
    let err = parse_and_type_check(src).unwrap_err();
    assert!(err.message.contains("name collision"), "{}", err.message);
}

#[test]
fn test_type_check_struct_copy_assignment_in_decl_fails() {
    // Decision 24: structs cannot be copied from another variable
    let src = "struct Point { int x; }; void main() { Point a; Point b = a; }";
    let err = parse_and_type_check(src).unwrap_err();
    assert!(err.message.contains("cannot copy struct"), "{}", err.message);
}

#[test]
fn test_type_check_array_of_structs_fails() {
    // Decision 24: array of structs is not supported
    let src = "struct Point { int x; }; void main() { Point[] lista; }";
    let err = parse_and_type_check(src).unwrap_err();
    assert!(err.message.contains("arrays of structs are not supported"), "{}", err.message);
}

// --- Nominal Typing and Compatibility (Task B - Step 3) ---

#[test]
fn test_type_check_nominal_typing_compatibility() {
    use mini_c::semantic::types_compatible;

    let pessoa = Type::UserDefined("Pessoa".to_string());
    let aluno = Type::UserDefined("Aluno".to_string());
    let pessoa_clone = Type::UserDefined("Pessoa".to_string());

    // Same struct name is compatible
    assert!(types_compatible(&pessoa, &pessoa_clone));

    // Different struct names are never compatible, even if they have identical shape (TAPL §19.3)
    assert!(!types_compatible(&pessoa, &aluno));

    // Structs are not compatible with primitive types
    assert!(!types_compatible(&pessoa, &Type::Int));

    // Structs are not compatible with Type::Any (protects print and generic functions)
    assert!(!types_compatible(&pessoa, &Type::Any));
}

#[test]
fn test_type_check_struct_cannot_be_printed() {
    // Decisions 17 & 24: print cannot accept whole structs
    let src = "struct Point { int x; }; void main() { Point p; print(p); }";
    let err = parse_and_type_check(src).unwrap_err();
    assert!(err.message.contains("passing structs to functions is not supported"), "{}", err.message);
}

// --- Prohibited Operations on Whole Structs (Task B - Step 4 / Decision 24) ---

#[test]
fn test_type_check_struct_whole_assignment_fails() {
    // Decision 18 & 24: a = b is prohibited
    let src = "struct Point { int x; }; void main() { Point a; Point b; a = b; }";
    let err = parse_and_type_check(src).unwrap_err();
    assert!(err.message.contains("cannot assign whole struct"), "{}", err.message);
}

#[test]
fn test_type_check_struct_equality_comparison_fails() {
    // Decision 16 & 24: a == b is prohibited
    let src = "struct Point { int x; }; void main() { Point a; Point b; if a == b {} }";
    let err = parse_and_type_check(src).unwrap_err();
    assert!(err.message.contains("equality operators are not supported for structs"), "{}", err.message);
}

#[test]
fn test_type_check_struct_inequality_comparison_fails() {
    // Decision 16 & 24: a != b is prohibited
    let src = "struct Point { int x; }; void main() { Point a; Point b; if a != b {} }";
    let err = parse_and_type_check(src).unwrap_err();
    assert!(err.message.contains("equality operators are not supported for structs"), "{}", err.message);
}

#[test]
fn test_type_check_struct_in_array_literal_fails() {
    // Decision 24: [a] is prohibited
    let src = "struct Point { int x; }; void main() { Point a; int x = [a][0]; }";
    let err = parse_and_type_check(src).unwrap_err();
    assert!(err.message.contains("arrays of structs are not supported"), "{}", err.message);
}
