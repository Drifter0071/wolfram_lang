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

#[test]
fn test_transpile_length_operator_dot_length() {
    let result = transpile("print(products.length)", "test.wrm").unwrap();
    assert!(result.contains("#products"));
    assert!(!result.contains(".length"));
}

#[test]
fn test_transpile_length_operator_len() {
    let result = transpile("print(len(my_list))", "test.wrm").unwrap();
    assert!(result.contains("#my_list"));
    assert!(!result.contains("len("));
}

#[test]
fn test_transpile_length_in_condition() {
    let result = transpile("if (items.length > 0) {\n    return true\n}", "test.wrm").unwrap();
    assert!(result.contains("#items"));
    assert!(!result.contains(".length"));
}

#[test]
fn test_module_suffix_stripping() {
    use crate::roblox_config::{resolve_script_location, DeploymentEntry, RobloxMapping};
    let deployments = vec![
        DeploymentEntry {
            source_dir: "src/shared".to_string(),
            service: "ReplicatedStorage".to_string(),
            sub_path: vec!["Shared".to_string()],
        },
    ];
    let mappings = vec![RobloxMapping {
        source: "src/shared".to_string(),
        target: "ReplicatedStorage.Shared".to_string(),
    }];

    let loc = resolve_script_location("src/shared/logger.shared.wrm", &deployments, &mappings);
    assert!(loc.is_some());
    assert_eq!(loc.unwrap().module_name, "logger");
}

#[test]
fn test_module_suffix_stripping_server() {
    use crate::roblox_config::{resolve_script_location, DeploymentEntry, RobloxMapping};
    let deployments = vec![
        DeploymentEntry {
            source_dir: "src/server".to_string(),
            service: "ServerScriptService".to_string(),
            sub_path: vec!["Server".to_string()],
        },
    ];
    let mappings = vec![RobloxMapping {
        source: "src/server".to_string(),
        target: "ServerScriptService.Server".to_string(),
    }];

    let loc = resolve_script_location("src/server/my_module.server.wrm", &deployments, &mappings);
    assert!(loc.is_some());
    assert_eq!(loc.unwrap().module_name, "my_module");
}

#[test]
fn test_module_suffix_stripping_client() {
    use crate::roblox_config::{resolve_script_location, DeploymentEntry, RobloxMapping};
    let deployments = vec![
        DeploymentEntry {
            source_dir: "src/client".to_string(),
            service: "StarterPlayer".to_string(),
            sub_path: vec!["StarterPlayerScripts".to_string(), "Client".to_string()],
        },
    ];
    let mappings = vec![RobloxMapping {
        source: "src/client".to_string(),
        target: "StarterPlayer.StarterPlayerScripts.Client".to_string(),
    }];

    let loc = resolve_script_location("src/client/gui.client.wrm", &deployments, &mappings);
    assert!(loc.is_some());
    assert_eq!(loc.unwrap().module_name, "gui");
}

use crate::roblox_config::{resolve_script_location, DeploymentEntry, RobloxMapping};

fn shared_deployments() -> Vec<DeploymentEntry> {
    vec![DeploymentEntry {
        source_dir: "src/shared".to_string(),
        service: "ReplicatedStorage".to_string(),
        sub_path: vec!["Shared".to_string()],
    }]
}
fn server_deployments() -> Vec<DeploymentEntry> {
    vec![DeploymentEntry {
        source_dir: "src/server".to_string(),
        service: "ServerScriptService".to_string(),
        sub_path: vec!["Server".to_string()],
    }]
}
fn empty_mappings() -> Vec<RobloxMapping> { vec![] }

// ─── A: Table Length Access ───────────────────────────────────────

#[test]
fn a1_length_simple_dot() {
    let r = transpile("print(products.length)", "a1.wrm").unwrap();
    assert!(r.contains("#products"), "got: {r}");
    assert!(!r.contains(".length"));
}

#[test]
fn a2_length_len_function() {
    let r = transpile("print(len(items))", "a2.wrm").unwrap();
    assert!(r.contains("#items"), "got: {r}");
    assert!(!r.contains("len("));
}

#[test]
fn a3_length_in_conditional() {
    let r = transpile("if (arr.length > 0) {\n    print(arr.length)\n}", "a3.wrm").unwrap();
    assert!(!r.contains(".length"), "got: {r}");
    assert_eq!(r.matches("#arr").count(), 2, "expected two #arr, got: {r}");
}

#[test]
fn a4_length_in_assignment() {
    let r = transpile("local count = data.length", "a4.wrm").unwrap();
    assert!(r.contains("#data"), "got: {r}");
}

#[test]
fn a5_length_as_function_arg() {
    let r = transpile("process(list.length)", "a5.wrm").unwrap();
    assert!(r.contains("#list"), "got: {r}");
    assert!(!r.contains(".length"));
}

#[test]
fn a6_len_with_expression_arg() {
    let r = transpile("print(len(getItems()))", "a6.wrm").unwrap();
    assert!(r.contains("#getItems()"), "got: {r}");
}

#[test]
fn a7_length_on_member_chain() {
    let r = transpile("print(obj.items.length)", "a7.wrm").unwrap();
    assert!(r.contains("#obj.items"), "got: {r}");
}

#[test]
fn a8_length_on_method_result() {
    let r = transpile("print(players:GetChildren().length)", "a8.wrm").unwrap();
    assert!(r.contains("#players:GetChildren()"), "got: {r}");
}

#[test]
fn a9_length_multiple_expressions() {
    let src = "local a = x.length\nlocal b = y.length\nprint(a.length + b.length)";
    let r = transpile(src, "a9.wrm").unwrap();
    assert!(!r.contains(".length"), "still contains .length: {r}");
    assert!(r.contains("#a"), "missing #a");
    assert!(r.contains("#b"), "missing #b");
    assert!(r.contains("#x"), "missing #x");
    assert!(r.contains("#y"), "missing #y");
}

// ─── B: Module Path Resolution ────────────────────────────────────

#[test]
fn b1_module_suffix_shared() {
    let loc = resolve_script_location("src/shared/logger.shared.wrm", &shared_deployments(), &empty_mappings());
    assert_eq!(loc.unwrap().module_name, "logger");
}

#[test]
fn b2_module_suffix_server() {
    let loc = resolve_script_location("src/server/handler.server.wrm", &server_deployments(), &empty_mappings());
    assert_eq!(loc.unwrap().module_name, "handler");
}

#[test]
fn b3_module_suffix_client() {
    let deps = vec![DeploymentEntry {
        source_dir: "src/client".to_string(),
        service: "StarterPlayer".to_string(),
        sub_path: vec!["StarterPlayerScripts".to_string(), "Client".to_string()],
    }];
    let loc = resolve_script_location("src/client/gui.client.wrm", &deps, &empty_mappings());
    assert_eq!(loc.unwrap().module_name, "gui");
}

#[test]
fn b4_module_no_suffix() {
    let loc = resolve_script_location("src/shared/utils.wrm", &shared_deployments(), &empty_mappings());
    assert_eq!(loc.unwrap().module_name, "utils");
}

#[test]
fn b5_module_subdir_with_suffix() {
    let loc = resolve_script_location("src/shared/helpers/validator.shared.wrm", &shared_deployments(), &empty_mappings());
    assert_eq!(loc.unwrap().module_name, "validator");
}

#[test]
fn b6_module_no_extension_input() {
    let loc = resolve_script_location("src/shared/logger.shared", &shared_deployments(), &empty_mappings());
    assert_eq!(loc.unwrap().module_name, "logger");
}

// ─── C: Complex Expressions ───────────────────────────────────────

#[test]
fn c1_ternary_expression() {
    let r = transpile(r#"local result = x > 0 ? "positive" : "negative""#, "c1.wrm").unwrap();
    assert!(r.contains("(if x > 0 then \"positive\" else \"negative\")"), "got: {r}");
}

#[test]
fn c2_nested_method_calls() {
    let r = transpile("part:SetPrimaryPartCFrame(CFrame.new(0, 10, 0))", "c2.wrm").unwrap();
    assert!(r.contains("part:SetPrimaryPartCFrame(CFrame.new(0, 10, 0))"));
}

#[test]
fn c3_deep_member_chain() {
    let r = transpile("local health = player.Character.Humanoid.Health", "c3.wrm").unwrap();
    assert!(r.contains("player.Character"), "got: {r}");
    assert!(r.contains("Humanoid"), "got: {r}");
    assert!(r.contains("Health"), "got: {r}");
    assert!(!r.contains(".length"), "should not contain .length: {r}");
}

#[test]
fn c4_method_colon_syntax() {
    let r = transpile(r#"game:GetService("Players")"#, "c4.wrm").unwrap();
    assert!(r.contains("game:GetService(\"Players\")"));
}

#[test]
fn c5_method_dot_syntax() {
    let r = transpile("Vector3.new(1, 2, 3)", "c5.wrm").unwrap();
    assert!(r.contains("Vector3.new(1, 2, 3)"));
}

#[test]
fn c6_logical_operators() {
    let r = transpile("local ok = x and y or z", "c6.wrm").unwrap();
    assert!(r.contains("local ok = x and y or z"));
}

#[test]
fn c7_unary_operations() {
    let r = transpile("local neg = -value\nlocal flag = not done", "c7.wrm").unwrap();
    assert!(r.contains("-value"));
    assert!(r.contains("not done"));
}

// ─── D: Class Definitions ─────────────────────────────────────────

#[test]
fn d1_basic_class_with_constructor() {
    let src = "class Player {\n    local name = \"\"\n    public function init(n) {\n        name = n\n    }\n    public function getName() {\n        return name\n    }\n}";
    let r = transpile(src, "d1.wrm").unwrap();
    assert!(r.contains("local Player = {}"));
    assert!(r.contains("Player.__index = Player"));
    assert!(r.contains("function Player.new"));
    assert!(r.contains("setmetatable"));
    assert!(r.contains("function Player:getName()"));
    assert!(r.contains("self:init(...)"));
}

#[test]
fn d2_class_with_private_fields() {
    let src = "class Counter {\n    private local count = 0\n    public function init() {}\n    private function increment() {\n        count = count + 1\n    }\n    public function next() {\n        self:increment()\n        return count\n    }\n}";
    let r = transpile(src, "d2.wrm").unwrap();
    assert!(r.contains("__private_Counter"));
    assert!(r.contains("function Counter:next()"));
}

// ─── E: Control Flow ──────────────────────────────────────────────

#[test]
fn e1_for_range_loop() {
    let r = transpile("for i in range(0, 10) {\n    print(i)\n}", "e1.wrm").unwrap();
    assert!(r.contains("for i = 0, 10 - 1 do"));
}

#[test]
fn e2_for_array_loop() {
    let r = transpile("for item in items {\n    print(item)\n}", "e2.wrm").unwrap();
    assert!(r.contains("for "), "got: {r}");
    assert!(r.contains(" in "), "got: {r}");
}

#[test]
fn e3_while_loop() {
    let r = transpile("while (x > 0) {\n    x = x - 1\n}", "e3.wrm").unwrap();
    assert!(r.contains("while x > 0 do"));
}

#[test]
fn e4_if_elif_else() {
    let src = "if (x > 10) {\n    print(\"high\")\n} else if (x > 5) {\n    print(\"mid\")\n} else {\n    print(\"low\")\n}";
    let r = transpile(src, "e4.wrm").unwrap();
    assert!(r.contains("elseif x > 5 then"));
    assert!(r.contains("else\n"));
}

#[test]
fn e5_try_catch() {
    let src = "try {\n    riskyCall()\n} catch {\n    print(\"failed\")\n}";
    let r = transpile(src, "e5.wrm").unwrap();
    assert!(r.contains("pcall("));
    assert!(r.contains("if not ok then"));
}

#[test]
fn e6_break_and_continue() {
    let src = "while (true) {\n    if (x > 0) {\n        break\n    } else {\n        continue\n    }\n}";
    let r = transpile(src, "e6.wrm").unwrap();
    assert!(r.contains("break"));
    assert!(r.contains("continue"));
}

// ─── F: Data Structures ───────────────────────────────────────────

#[test]
fn f1_array_literal() {
    let r = transpile("local nums = [1, 2, 3, 4]", "f1.wrm").unwrap();
    assert!(r.contains("{1, 2, 3, 4}"));
}

#[test]
fn f2_table_named_keys() {
    let r = transpile(r#"local cfg = {name: "test", value: 42}"#, "f2.wrm").unwrap();
    assert!(r.contains("name = \"test\""));
    assert!(r.contains("value = 42"));
}

#[test]
fn f3_mixed_table() {
    let r = transpile(r#"local mixed = {"one", "two", key: "val"}"#, "f3.wrm").unwrap();
    assert!(r.contains("\"one\", \"two\", key = \"val\""));
}

#[test]
fn f4_empty_structures() {
    let r = transpile("local empty = []\nlocal empty_table = {}", "f4.wrm").unwrap();
    assert_eq!(r.matches("{}").count(), 2);
}

#[test]
fn f5_index_access() {
    let r = transpile("local val = data[idx + 1]", "f5.wrm").unwrap();
    assert!(r.contains("data[idx + 1]"));
}

// ─── G: Enum and Struct ───────────────────────────────────────────

#[test]
fn g1_enum_definition() {
    let r = transpile("enum State {\n    Lobby, Playing, Ended\n}", "g1.wrm").unwrap();
    assert!(r.contains("table.freeze"));
    assert!(r.contains("Lobby = \"Lobby\""));
    assert!(r.contains("Playing = \"Playing\""));
    assert!(r.contains("Ended = \"Ended\""));
}

#[test]
fn g2_struct_definition() {
    let r = transpile("struct Vec3 {\n    x, y, z\n}", "g2.wrm").unwrap();
    assert!(r.contains("local Vec3 = {}"));
    assert!(r.contains("function Vec3.new(x, y, z)"));
    assert!(r.contains("return {x = x, y = y, z = z}"));
}

// ─── H: Edge Cases ────────────────────────────────────────────────

#[test]
fn h1_parse_empty_file() {
    match transpile("", "h1.wrm") {
        Ok(r) => {
            let non_comment = r.lines().filter(|l| !l.starts_with("--")).collect::<Vec<_>>().join("\n");
            assert!(non_comment.trim().is_empty(), "expected only comments/warnings, got: {r}");
        }
        Err(_) => {}
    }
}

#[test]
fn h2_only_comments() {
    match transpile("// comment\n// another line", "h2.wrm") {
        Ok(r) => assert!(r.trim().is_empty() || r.contains("//"), "got: {r:?}"),
        Err(_) => {}
    }
}

#[test]
fn h3_roblox_service_access() {
    let r = transpile(r#"local ps = game:GetService("Players")\nlocal list = ps:GetPlayers()"#, "h3.wrm").unwrap();
    assert!(r.contains("game:GetService(\"Players\")"));
}

#[test]
fn h4_self_expr() {
    let r = transpile("function test() {\n    return self\n}", "h4.wrm").unwrap();
    assert!(r.contains("return self"));
}

#[test]
fn h5_equality_check() {
    let r = transpile("if (x == nil) {\n    print(\"null\")\n}", "h5.wrm").unwrap();
    assert!(r.contains("x == nil"));
}

// ─── I: Type Constructors ─────────────────────────────────────────

#[test]
fn i1_vector3_constructor() {
    let r = transpile("local pos = Vector3.new(10, 0, 5)", "i1.wrm").unwrap();
    assert!(r.contains("Vector3.new(10, 0, 5)"));
}

#[test]
fn i2_cframe_arithmetic() {
    let r = transpile("local cf = CFrame.new(0, 5, 0) * CFrame.Angles(0, math.rad(90), 0)", "i2.wrm").unwrap();
    assert!(r.contains("CFrame.new(0, 5, 0)"));
    assert!(r.contains("CFrame.Angles"));
    assert!(r.contains("math.rad(90)"));
}

#[test]
fn i3_color3_from_rgb() {
    let r = transpile("local col = Color3.fromRGB(255, 0, 0)", "i3.wrm").unwrap();
    assert!(r.contains("Color3.fromRGB(255, 0, 0)"));
}

#[test]
fn i4_instance_new() {
    let r = transpile(r#"local part = Instance.new("Part", workspace)"#, "i4.wrm").unwrap();
    assert!(r.contains("Instance.new(\"Part\", workspace)"));
}

// ─── J: Nil/Boolean Patterns ──────────────────────────────────────

#[test]
fn j1_nil_var() {
    let r = transpile("local x = nil", "j1.wrm").unwrap();
    assert!(r.contains("local x = nil"));
}

#[test]
fn j2_boolean_literals() {
    let r = transpile("local flag = true\nlocal off = false", "j2.wrm").unwrap();
    assert!(r.contains("true"));
    assert!(r.contains("false"));
}

#[test]
fn j3_equality_with_true() {
    let r = transpile("if (done == true) {\n    print(\"done\")\n}", "j3.wrm").unwrap();
    assert!(r.contains("if done then"), "equality with true should simplify: {r}");
}

// ─── K: Transpiler Edge Cases ─────────────────────────────────────

#[test]
fn k1_length_mid_chain() {
    let r = transpile("print(someTable.someField.length.something)", "k1.wrm").unwrap();
    assert!(!r.contains(".length"), "should not have .length: {r}");
    assert!(r.contains("#someTable"), "should contain length operator: {r}");
    assert!(r.contains("someField"), "should preserve chain: {r}");
}

#[test]
fn k2_same_name_as_global() {
    let r = transpile(r#"local game = "hello"\nprint(game)"#, "k2.wrm").unwrap();
    assert!(r.contains("local game = \"hello\""));
}

#[test]
fn k3_public_export_no_roblox() {
    let src = "public function greet() {\n    return \"hello\"\n}";
    let r = transpile(src, "k3.wrm").unwrap();
    assert!(r.contains("local function greet()"));
    assert!(r.contains("return {greet = greet}"));
}

#[test]
fn k4_comment_inside_code() {
    let r = transpile("local x = 1 // inline comment\nlocal y = 2", "k4.wrm").unwrap();
    assert!(r.contains("local x = 1"));
    assert!(r.contains("local y = 2"));
}

// ─── Regression: Meta test ────────────────────────────────────────

#[test]
fn regression_no_dot_length_in_output() {
    let sources = &[
        "print(x.length)",
        "print(len(x))",
        "local c = arr.length",
        "if (d.length > 0) {}",
        "process(items.length, more.length)",
        "print(x.y.z.length)",
    ];
    for src in sources {
        match transpile(src, "reg.wrm") {
            Ok(r) => assert!(!r.contains(".length"), ".length found in output for '{src}': {r}"),
            Err(_) => {} // parse errors ok for edge cases
        }
    }
}

#[test]
fn regression_no_len_call_in_output() {
    let sources = &[
        "print(len(x))",
        "print(len(getItems()))",
    ];
    for src in sources {
        match transpile(src, "reg2.wrm") {
            Ok(r) => assert!(!r.contains("len("), "len( found in output for '{src}': {r}"),
            Err(_) => {}
        }
    }
}

#[test]
fn regression_braces_balanced_in_output() {
    let sources = &[
        "",
        "local x = 42",
        "print(products.length)",
        "if (x > 0) { return 1 } else { return 0 }",
        "class Foo { local x = 1\n public function bar() { return x } }",
        "local arr = [1, 2, 3]",
        "local tbl = {a: 1, b: 2}",
        "while (true) { break }",
        "for i in range(0, 10) { print(i) }",
        "try { f() } catch { print(err) }",
    ];
    for src in sources {
        match transpile(src, "reg3.wrm") {
            Ok(r) => {
                let open = r.matches('{').count();
                let close = r.matches('}').count();
                assert_eq!(open, close, "Unbalanced braces in output for '{src}': open={open} close={close}\nOutput:\n{r}");
            }
            Err(_) => {}
        }
    }
}
