use crate::ast;
use crate::tokenize_and_parse;
use crate::transpile;

#[test]
fn test_parse_empty() {
    let ast = tokenize_and_parse("").unwrap();
    assert!(ast.is_empty());
}

#[test]
fn test_parse_variable_declaration() {
    let ast = tokenize_and_parse("local x = 42").unwrap();
    assert_eq!(ast.len(), 1);
    match &ast[0] {
        ast::Stmt::Local { name, .. } => assert_eq!(name, "x"),
        _ => panic!("Expected Local statement"),
    }
}

#[test]
fn test_parse_string_variable() {
    let ast = tokenize_and_parse(r#"local msg = "hello""#).unwrap();
    assert_eq!(ast.len(), 1);
}

#[test]
fn test_parse_function() {
    let ast = tokenize_and_parse("function greet() {\n    return 42\n}").unwrap();
    assert_eq!(ast.len(), 1);
    match &ast[0] {
        ast::Stmt::FuncDef { name, params, .. } => {
            assert_eq!(name, "greet");
            assert!(params.is_empty());
        }
        _ => panic!("Expected FuncDef"),
    }
}

#[test]
fn test_parse_if_statement() {
    let ast = tokenize_and_parse("if (x > 0) {\n    return 1\n}").unwrap();
    assert_eq!(ast.len(), 1);
    match &ast[0] {
        ast::Stmt::If { .. } => {}
        _ => panic!("Expected If statement"),
    }
}

#[test]
fn test_parse_class() {
    let source = "class Player {\n    local name = \"\"\n    public function init(n) {\n        name = n\n    }\n}";
    let ast = tokenize_and_parse(source).unwrap();
    assert_eq!(ast.len(), 1);
    match &ast[0] {
        ast::Stmt::ClassDef { name, .. } => assert_eq!(name, "Player"),
        _ => panic!("Expected ClassDef"),
    }
}

#[test]
fn test_parse_enum() {
    let ast = tokenize_and_parse("enum State {\n    Active, Inactive\n}").unwrap();
    assert_eq!(ast.len(), 1);
    match &ast[0] {
        ast::Stmt::EnumDef { name, variants, .. } => {
            assert_eq!(name, "State");
            assert_eq!(variants.len(), 2);
        }
        _ => panic!("Expected EnumDef"),
    }
}

#[test]
fn test_parse_struct() {
    let ast = tokenize_and_parse("struct Vec3 {\n    x, y, z\n}").unwrap();
    assert_eq!(ast.len(), 1);
    match &ast[0] {
        ast::Stmt::StructDef { name, fields, .. } => {
            assert_eq!(name, "Vec3");
            assert_eq!(fields.len(), 3);
        }
        _ => panic!("Expected StructDef"),
    }
}

#[test]
fn test_parse_import() {
    let ast = tokenize_and_parse(r#"import "./lib" as lib"#).unwrap();
    assert_eq!(ast.len(), 1);
}

#[test]
fn test_transpile_variable() {
    let result = transpile("local x = 42", "test.wrm").unwrap();
    assert!(result.contains("local x = 42"));
}

#[test]
fn test_transpile_function() {
    let result = transpile(
        "function add(a, b) {\n    return a + b\n}",
        "test.wrm",
    )
    .unwrap();
    assert!(result.contains("local function add(a, b)"));
    assert!(result.contains("return a + b"));
}

#[test]
fn test_transpile_if() {
    let source = "if (x > 0) {\n    return 1\n} else {\n    return 0\n}";
    let result = transpile(source, "test.wrm").unwrap();
    assert!(result.contains("if x > 0 then"));
    assert!(result.contains("end"));
}

#[test]
fn test_transpile_while() {
    let source = "while (x > 0) {\n    x = x - 1\n}";
    let result = transpile(source, "test.wrm").unwrap();
    assert!(result.contains("while x > 0 do"));
    assert!(result.contains("end"));
}

#[test]
fn test_transpile_enum() {
    let source = "enum State {\n    Lobby, Playing, Ended\n}";
    let result = transpile(source, "test.wrm").unwrap();
    assert!(result.contains("table.freeze"));
    assert!(result.contains("State"));
}

#[test]
fn test_parse_error_recovery() {
    let result = tokenize_and_parse("local = 42");
    assert!(result.is_err());
}

#[test]
fn test_span_populated() {
    let source = "local x = 42";
    let ast = tokenize_and_parse(source).unwrap();
    if let ast::Stmt::Local { span, .. } = &ast[0] {
        assert!(
            span.end >= span.start,
            "Span end ({}) must be >= start ({})",
            span.end,
            span.start
        );
    }
}

#[test]
fn test_span_later_statement() {
    let source = "local a = 1\nlocal b = 2";
    let ast = tokenize_and_parse(source).unwrap();
    assert_eq!(ast.len(), 2);
    if let ast::Stmt::Local { span, .. } = &ast[1] {
        assert!(span.start > 0, "Second statement's span should not be 0");
    }
}

#[test]
fn test_type_annotation_preservation() {
    let source = "function greet(name: string, age: number, flag: bool) {\n    return name\n}";
    let ast = tokenize_and_parse(source).unwrap();
    assert_eq!(ast.len(), 1);
    match &ast[0] {
        ast::Stmt::FuncDef { params, param_types, .. } => {
            assert_eq!(params.len(), 3);
            assert_eq!(param_types.len(), 3);
            assert_eq!(params[0], "name");
            assert_eq!(param_types[0], Some("string".into()));
            assert_eq!(params[1], "age");
            assert_eq!(param_types[1], Some("number".into()));
            assert_eq!(params[2], "flag");
            assert_eq!(param_types[2], Some("bool".into()));
        }
        _ => panic!("Expected FuncDef"),
    }
}
