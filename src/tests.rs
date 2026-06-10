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
        Ok(r) => {
            let non_comment = r.lines().filter(|l| !l.starts_with("--")).collect::<Vec<_>>().join("\n");
            assert!(non_comment.trim().is_empty(), "expected only warnings, got: {r}");
        }
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
    assert!(r.contains("module.greet = function()"));
    assert!(r.contains("return module"));
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

#[test]
fn regression_fstring_length_interpolation() {
    let src = "local products = [{name = \"a\"}, {name = \"b\"}]\nprint(f\"Total: {products.length} items\")\nprint(f\"Hello {user.name.length}\")\nprint(f\"Long: {deeply.nested.table.length}\")";
    let result = transpile(src, "fstr.wrm").unwrap();
    assert!(!result.contains(".length"), "f-string still has .length: {result}");
    assert!(result.contains("{#products}"), "missing #products in f-string: {result}");
    assert!(result.contains("{#user.name}"), "missing #user.name in f-string: {result}");
    assert!(result.contains("{#deeply.nested.table}"), "missing #deeply.nested.table: {result}");
}

// ─── L: Data Persistence / DataStore ───────────────────────────────
// Tests: realistic save/load patterns used in Roblox games

#[test]
fn l1_datastore_get_async() {
    let src = r#"
local DSS = game:GetService("DataStoreService")
local store = DSS:GetDataStore("PlayerData")
local function loadData(player) {
    local key = "player_" + tostring(player.UserId)
    local data = store:GetAsync(key)
    if (data == nil) {
        data = {coins: 0, level: 1}
    }
    return data
}
"#;
    let r = transpile(src, "l1.wrm").unwrap();
    assert!(r.contains("DataStoreService"));
    assert!(r.contains("GetDataStore"));
    assert!(r.contains("GetAsync"));
    assert!(r.contains("local function loadData(player)"));
    assert!(r.contains("tostring(player.UserId)"));
}

#[test]
fn l2_datastore_set_async_with_pcall() {
    let src = r#"
try {
    store:SetAsync(key, playerData)
    print("Saved!")
} catch {
    warn("Failed to save data for " + player.Name)
}
"#;
    let r = transpile(src, "l2.wrm").unwrap();
    assert!(r.contains("pcall("));
    assert!(r.contains("if not ok then"));
    assert!(r.contains("SetAsync(key, playerData)"));
}

#[test]
fn l3_player_data_autosave() {
    let src = r#"
local function autoSave(player) {
    local data = {
        coins: leaderstats.coins.Value,
        xp: leaderstats.xp.Value,
        lastSave: os.time(),
    }
    local ok, err = pcall(function() {
        store:SetAsync(player.UserId, data)
    })
    if (not ok) {
        warn("Autosave failed")
    }
}
Players.PlayerRemoving:Connect(autoSave)
"#;
    let r = transpile(src, "l3.wrm").unwrap();
    assert!(r.contains("local function autoSave(player)"));
    assert!(r.contains("os.time()"));
    assert!(r.contains("pcall("));
}

#[test]
fn l4_datastore_update_async() {
    let src = r#"
local function addCoins(player, amount) {
    local success = store:UpdateAsync(player.UserId, function(oldData) {
        if (oldData == nil) {
            oldData = {coins: 0}
        }
        oldData.coins = oldData.coins + amount
        return oldData
    })
    return success
}
"#;
    let r = transpile(src, "l4.wrm").unwrap();
    assert!(r.contains("UpdateAsync"));
    assert!(r.contains("local success = store:UpdateAsync("));
    assert!(r.contains("oldData.coins = oldData.coins + amount"));
}

// ─── M: Remote Events/Functions ────────────────────────────────────

#[test]
fn m1_remote_event_fire_all_clients() {
    let src = r#"
local ReplicatedStorage = game:GetService("ReplicatedStorage")
local updateGoldEvent = ReplicatedStorage:FindFirstChild("UpdateGold")
local function updateGold(player, amount) {
    local stats = leaderstats:FindFirstChild(player.Name)
    if (stats) {
        stats.Gold.Value = stats.Gold.Value + amount
        updateGoldEvent:FireAllClients(player, amount)
    }
}
"#;
    let r = transpile(src, "m1.wrm").unwrap();
    assert!(r.contains("ReplicatedStorage"));
    assert!(r.contains("FindFirstChild"));
    assert!(r.contains("FireAllClients"));
}

#[test]
fn m2_remote_function_callback() {
    let src = r#"
local checkPass = ReplicatedStorage:FindFirstChild("CheckGamepass")
checkPass.OnServerInvoke = function(player, passId) {
    local hasPass = game:GetService("MarketplaceService"):UserOwnsGamePassAsync(player.UserId, passId)
    return hasPass
}
"#;
    let r = transpile(src, "m2.wrm").unwrap();
    assert!(r.contains("OnServerInvoke"));
    assert!(r.contains("MarketplaceService"));
    assert!(r.contains("UserOwnsGamePassAsync"));
}

#[test]
fn m3_predicted_client_event() {
    let src = r#"
local shootEvent = ReplicatedStorage:FindFirstChild("Shoot")
local function fireWeapon() {
    local mousePos = player:GetMouse().Hit.Position
    shootEvent:FireServer(mousePos, weapon.Damage)
}
Tool.Activated:Connect(fireWeapon)
"#;
    let r = transpile(src, "m3.wrm").unwrap();
    assert!(r.contains("FireServer"));
    assert!(r.contains("GetMouse"));
    assert!(r.contains("Tool.Activated:Connect(fireWeapon)"));
}

// ─── N: Game Mechanics / Tycoon ────────────────────────────────────

#[test]
fn n1_tycoon_income_loop() {
    let src = r#"
while (true) {
    for tycoon in allTycoons {
        if (tycoon.owner) {
            tycoon.cash = tycoon.cash + tycoon.income
            tycoon:updateGui()
        }
    }
    task.wait(1)
}
"#;
    let r = transpile(src, "n1.wrm").unwrap();
    assert!(r.contains("for tycoon, _ in pairs"));
    assert!(r.contains("tycoon.cash = tycoon.cash + tycoon.income"));
    assert!(r.contains("task.wait(1)"));
}

#[test]
fn n2_touch_debounce_pattern() {
    let src = r#"
local db = false
local debounceTime = 2
part.Touched:Connect(function(hit) {
    if (db) { return }
    db = true
    local char = hit.Parent:FindFirstChild("Humanoid")
    if (char) {
        char.Health = char.Health - 10
    }
    task.wait(debounceTime)
    db = false
})
"#;
    let r = transpile(src, "n2.wrm").unwrap();
    assert!(r.contains("local debounceTime = 2"));
    assert!(r.contains("if db then"));
    assert!(r.contains("db = false"));
}

#[test]
fn n3_obby_checkpoint_system() {
    let src = r#"
local checkpoints = [stage1, stage2, stage3, stage4, stage5]
for cp in checkpoints {
    cp.Touched:Connect(function(hit) {
        local player = game.Players:GetPlayerFromCharacter(hit.Parent)
        if (player) {
            local stage = cp:GetAttribute("Stage")
            player:SetAttribute("Checkpoint", stage)
        }
    })
}
"#;
    let r = transpile(src, "n3.wrm").unwrap();
    assert!(r.contains("GetPlayerFromCharacter"));
    assert!(r.contains("GetAttribute"));
    assert!(r.contains("SetAttribute"));
}

// ─── O: Module Scripts / Shared Code ───────────────────────────────

#[test]
fn o1_shared_utility_module() {
    let src = r#"
public function deepCopy(tbl) {
    local result = {}
    for key, val in tbl {
        if (typeof(val) == "table") {
            result[key] = deepCopy(val)
        } else {
            result[key] = val
        }
    }
    return result
}
"#;
    let r = transpile(src, "o1.wrm").unwrap();
    assert!(r.contains("return module"));
    assert!(r.contains("module.deepCopy = function"));
    assert!(r.contains("typeof(val)"));
}

#[test]
fn o2_class_exported_as_module() {
    let src = r#"
public struct PlayerData {
    gold, level, xp
}
public class ShopManager {
    public function buyItem(player, itemId) {
        local data = DataStore.load(player)
        local item = items[itemId]
        if (data.gold >= item.price) {
            data.gold = data.gold - item.price
            DataStore.save(player, data)
            return true
        }
        return false
    }
}
"#;
    let r = transpile(src, "o2.wrm").unwrap();
    assert!(r.contains("local module = {}"));
    assert!(r.contains("module.PlayerData = {}"));
    assert!(r.contains("module.ShopManager"));
    assert!(r.contains("return module"));
    assert!(r.contains("function module.ShopManager:buyItem("));
}

#[test]
fn o3_multiple_public_exports() {
    let src = r#"
public enum GameState { Lobby, Intermission, Playing, Ended }
public struct MatchInfo { map, players, timeLimit }
public function calculateElo(winner, loser) {
    local k = 32
    local expected = 1.0 / (1.0 + 10.0 ^ ((loser - winner) / 400.0))
    return round(k * (1.0 - expected))
}
"#;
    let r = transpile(src, "o3.wrm").unwrap();
    assert!(r.contains("return module"));
    assert!(r.contains("module.GameState"));
    assert!(r.contains("module.MatchInfo"));
    assert!(r.contains("module.calculateElo ="));
}

#[test]
fn o4_import_and_use_shared_module() {
    let src = r#"
import "../shared/config" as Config
import "../shared/utils" as Utils
local function setupGame() {
    local maxPlayers = Config.maxPlayers
    local seed = Config.mapSeed
    Utils.shuffleDeck()
    return maxPlayers
}
"#;
    let r = transpile(src, "o4.wrm").unwrap();
    assert!(r.contains("Config = require("));
    assert!(r.contains("Utils = require("));
    assert!(r.contains("Config.maxPlayers"));
}

// ─── P: Leaderboard / Leaderstats ──────────────────────────────────

#[test]
fn p1_create_leaderstats() {
    let src = r#"
local function setupLeaderstats(player) {
    local leaderstats = Instance.new("Folder", player)
    leaderstats.Name = "leaderstats"
    local coins = Instance.new("IntValue", leaderstats)
    coins.Name = "Coins"
    coins.Value = 0
    local level = Instance.new("IntValue", leaderstats)
    level.Name = "Level"
    level.Value = 1
}
"#;
    let r = transpile(src, "p1.wrm").unwrap();
    assert!(r.contains("local leaderstats = Instance.new(\"Folder\", player)"));
    assert!(r.contains("leaderstats.Name = \"leaderstats\""));
    assert!(r.contains("local coins = Instance.new(\"IntValue\", leaderstats)"));
}

#[test]
fn p2_leaderboard_sort_and_display() {
    let src = r#"
local function updateLeaderboard() {
    local sorted = {}
    for player in Players:GetPlayers() {
        local entry = {
            name: player.Name,
            coins: player.leaderstats.Coins.Value,
        }
        sorted[sorted.length + 1] = entry
    }
    -- Sort by coins descending
    for i in range(0, sorted.length) {
        for j in range(i + 1, sorted.length) {
            if (sorted[i].coins < sorted[j].coins) {
                local temp = sorted[i]
                sorted[i] = sorted[j]
                sorted[j] = temp
            }
        }
    }
}
"#;
    let r = transpile(src, "p2.wrm").unwrap();
    assert!(r.contains("#sorted"));
    assert!(r.contains("leaderstats.Coins.Value"));
}

#[test]
fn p3_killstreak_tracker() {
    let src = r#"
local function onPlayerDeath(victim, killer) {
    if (killer and killer ~= victim) {
        local ks = killer:FindFirstChild("Killstreak")
        if (ks == nil) {
            ks = Instance.new("IntValue", killer)
            ks.Name = "Killstreak"
        }
        ks.Value = ks.Value + 1
        local msg = f"{killer.Name} 🔥 {ks.Value} kill streak!"
        for player in Players:GetPlayers() {
            player:SendNotification(msg)
        }
    }
}
"#;
    let r = transpile(src, "p3.wrm").unwrap();
    assert!(r.contains("killer ~= victim"));
    assert!(r.contains("killer:FindFirstChild"));
    assert!(r.contains("`"));  // f-string → backtick template
    assert!(r.contains("killer.Name"));
}

// ─── Q: NPC / AI Patterns ──────────────────────────────────────────

#[test]
fn q1_npc_roam_behavior() {
    let src = r#"
class NPC {
    local waypoints = []
    local currentIndex = 0
    local humanoid = nil
    public function init(model, points) {
        humanoid = model:FindFirstChild("Humanoid")
        waypoints = points
    }
    public function patrol() {
        while (true) {
            local target = waypoints[currentIndex % waypoints.length]
            humanoid:MoveTo(target.Position)
            humanoid.MoveToFinished:Wait()
            task.wait(1)
            currentIndex = currentIndex + 1
        }
    }
}
"#;
    let r = transpile(src, "q1.wrm").unwrap();
    assert!(r.contains("humanoid:MoveTo("));
    assert!(r.contains("MoveToFinished:Wait()"));
    assert!(r.contains("__private_NPC[self].waypoints"));
}

#[test]
fn q2_enemy_detection_cone() {
    let src = r#"
local function isInCone(origin, lookDirection, target, angle) {
    local toTarget = (target - origin).Unit
    local dot = lookDirection:Dot(toTarget)
    local threshold = math.cos(math.rad(angle / 2))
    return dot >= threshold
}
local function detectPlayers(npc) {
    local detected = []
    for player in Players:GetPlayers() {
        local char = player.Character
        if (char and isInCone(npc.Head.Position, npc.Head.CFrame.LookVector, char.Head.Position, 60)) {
            detected[detected.length + 1] = player
        }
    }
    return detected
}
"#;
    let r = transpile(src, "q2.wrm").unwrap();
    assert!(r.contains("math.cos(math.rad("));
    assert!(r.contains("LookVector"));
    assert!(r.contains("Dot("));
    assert!(r.contains("#detected"));
}

#[test]
fn q3_npc_dialogue_tree() {
    let src = r#"
public class DialogueNode {
    public function init(text) {
        self.text = text
        self.options = []
    }
    public function addOption(label, nextNode) {
        local option = {label: label, next: nextNode}
        self.options[self.options.length + 1] = option
        return self
    }
    public function show(player) {
        local gui = player.PlayerGui:FindFirstChild("DialogueGui")
        if (gui == nil) {
            gui = Instance.new("ScreenGui", player.PlayerGui)
            gui.Name = "DialogueGui"
        }
        return gui
    }
}
"#;
    let r = transpile(src, "q3.wrm").unwrap();
    assert!(r.contains("self.text = text"));
    assert!(r.contains("self.options = {}"));
    assert!(r.contains("player.PlayerGui:FindFirstChild"));
}

// ─── R: UI / GUI Patterns ──────────────────────────────────────────

#[test]
fn r1_shop_gui_builder() {
    let src = r#"
local function createShopButton(parent, item, position) {
    local btn = Instance.new("TextButton", parent)
    btn.Name = item.name
    btn.Text = f"{item.name} — 💰{item.price}"
    btn.Position = position
    btn.Size = UDim2.new(0, 200, 0, 50)
    btn.MouseButton1Click:Connect(function() {
        buyItem(item)
    })
    return btn
}
"#;
    let r = transpile(src, "r1.wrm").unwrap();
    assert!(r.contains("TextButton"));
    assert!(r.contains("MouseButton1Click:Connect"));
    assert!(r.contains("UDim2.new(0, 200, 0, 50)"));
    assert!(r.contains("`"));  // f-string → backtick template
}

#[test]
fn r2_inventory_grid_layout() {
    let src = r#"
local function buildInventoryGrid(player, items) {
    local gui = Instance.new("ScreenGui", player.PlayerGui)
    local frame = Instance.new("Frame", gui)
    frame.Size = UDim2.new(0, 400, 0, 300)
    local slotSize = UDim2.new(0, 80, 0, 80)
    local cols = 5
    for i in range(0, items.length) {
        local item = items[i]
        local slot = Instance.new("ImageButton", frame)
        local row = i / cols
        local col = i % cols
        slot.Position = UDim2.new(0, col * 80, 0, row * 80)
        slot.Size = slotSize
        if (item.icon) {
            slot.Image = item.icon
        }
    }
}
"#;
    let r = transpile(src, "r2.wrm").unwrap();
    assert!(r.contains("#items"));
    assert!(r.contains("row = i / cols"));
    assert!(r.contains("col = i % cols"));
    assert!(r.contains("ScreenGui"));
}

#[test]
fn r3_notification_system() {
    let src = r#"
local function sendNotification(player, title, message, duration) {
    local gui = Instance.new("ScreenGui", player.PlayerGui)
    local frame = Instance.new("Frame", gui)
    frame.BackgroundColor3 = Color3.fromRGB(30, 30, 30)
    frame.BorderSizePixel = 0
    local titleLabel = Instance.new("TextLabel", frame)
    titleLabel.Text = title
    titleLabel.Font = Enum.Font.GothamBold
    titleLabel.TextSize = 18
    local msgLabel = Instance.new("TextLabel", frame)
    msgLabel.Text = message
    msgLabel.TextWrapped = true
    task.wait(duration)
    gui:Destroy()
}
"#;
    let r = transpile(src, "r3.wrm").unwrap();
    assert!(r.contains("Color3.fromRGB"));
    assert!(r.contains("Enum.Font.GothamBold"));
    assert!(r.contains("Destroy()"));
}

// ─── S: Advanced OOP Patterns ──────────────────────────────────────

#[test]
fn s1_class_inheritance_chain() {
    let src = r#"
class Weapon {
    local damage = 10
    local durability = 100
    public function init(dmg) {
        damage = dmg
    }
    public function attack(target) {
        target:TakeDamage(damage)
        durability = durability - 1
    }
    public function isBroken() {
        return durability <= 0
    }
}
class Sword {
    local weapon = Weapon.new(25)
    public function slash(target) {
        weapon:attack(target)
        print(f"Slashed for {weapon.damage}!")
    }
}
"#;
    let r = transpile(src, "s1.wrm").unwrap();
    assert!(r.contains("target:TakeDamage("));
    assert!(r.contains("__private_Weapon[self].damage"));
    assert!(r.contains("durability <= 0"));
}

#[test]
fn s2_builder_pattern() {
    let src = r#"
class House {
    local walls = 0
    local roof = false
    local color = "white"
    public function init() {}
    public function setWalls(count) {
        walls = count
        return self
    }
    public function setRoof() {
        roof = true
        return self
    }
    public function setColor(c) {
        color = c
        return self
    }
    public function build() {
        local part = Instance.new("Part", workspace)
        part.BrickColor = BrickColor.new(color)
        part.Size = Vector3.new(walls * 4, 3, walls * 4)
    }
}
local mansion = House.new():setWalls(10):setRoof():setColor("Really red")
"#;
    let r = transpile(src, "s2.wrm").unwrap();
    assert!(r.contains("return self"));
    assert!(r.contains("BrickColor.new("));
    assert!(r.contains("__private_House[self].color"));
    assert!(r.contains("House.new():setWalls(10):setRoof():setColor(\"Really red\")"));
}

#[test]
fn s3_state_machine_pattern() {
    let src = r#"
enum States { Idle, Walking, Running, Jumping, Dead }
class StateMachine {
    local current = States.Idle
    public function init() {}
    public function transition(newState) {
        if (current == States.Dead) { return }
        self:exit(current)
        current = newState
        self:enter(current)
    }
    private function enter(state) {
        print(f"Entering {state}")
    }
    private function exit(state) {
        print(f"Exiting {state}")
    }
}
"#;
    let r = transpile(src, "s3.wrm").unwrap();
    assert!(r.contains("current = States.Idle"));
    assert!(r.contains("current == States.Dead"));
    assert!(r.contains("__private_StateMachine[self].exit("));
    assert!(r.contains("__private_StateMachine[self].enter("));
    assert!(r.contains("__private_StateMachine[self].current"));
}

#[test]
fn s4_singleton_service_class() {
    let src = r#"
public class AudioManager {
    public function init() {}
    public function playSound(soundId, volume) {
        local sound = Instance.new("Sound", workspace)
        sound.SoundId = f"rbxassetid://{soundId}"
        sound.Volume = volume
        sound:Play()
    }
    public function playBackgroundMusic(soundId) {
        self:playSound(soundId, 0.5)
    }
}
"#;
    let r = transpile(src, "s4.wrm").unwrap();
    assert!(r.contains("return module"));
    assert!(r.contains("module.AudioManager = {}"));
    assert!(r.contains("function module.AudioManager:playSound("));
}

// ─── T: Tool / Weapon Systems ──────────────────────────────────────

#[test]
fn t1_raycast_gun() {
    let src = r#"
local function shootGun(player, origin, direction) {
    local params = RaycastParams.new()
    params.FilterType = Enum.RaycastFilterType.Blacklist
    params.FilterDescendantsInstances = [player.Character]
    local result = workspace:Raycast(origin, direction * 500, params)
    if (result) {
        local hit = result.Instance
        local humanoid = hit.Parent:FindFirstChild("Humanoid")
        if (humanoid) {
            humanoid:TakeDamage(25)
        }
        local hitMarker = Instance.new("Part", workspace)
        hitMarker.Position = result.Position
        hitMarker.BrickColor = BrickColor.new("Really red")
        task.wait(2)
        hitMarker:Destroy()
    }
}
"#;
    let r = transpile(src, "t1.wrm").unwrap();
    assert!(r.contains("RaycastParams.new()"));
    assert!(r.contains("RaycastFilterType.Blacklist"));
    assert!(r.contains("workspace:Raycast("));
    assert!(r.contains("result.Instance"));
}

#[test]
fn t2_projectile_motion() {
    let src = r#"
local function launchProjectile(start, target, speed) {
    local direction = (target - start).Unit
    local projectile = Instance.new("Part", workspace)
    projectile.Position = start
    projectile.Velocity = direction * speed
    projectile.Touched:Connect(function(hit) {
        if (hit.Parent:FindFirstChild("Humanoid")) {
            projectile:Destroy()
        }
    })
}
"#;
    let r = transpile(src, "t2.wrm").unwrap();
    assert!(r.contains("(target - start).Unit"));
    assert!(r.contains("direction * speed"));
    assert!(r.contains("Touched:Connect"));
}

#[test]
fn t3_aoe_damage_calculator() {
    let src = r#"
local function applyAoeDamage(center, radius, damage, source) {
    local parts = workspace:GetPartBoundsInRadius(center, radius)
    local hit = {}
    for obj in parts {
        local char = obj.Parent
        if (char:FindFirstChild("Humanoid") and not hit[char]) {
            hit[char] = true
            local dist = (char:GetPivot().Position - center).Magnitude
            local falloff = 1.0 - (dist / radius)
            local dmg = damage * falloff
            char.Humanoid:TakeDamage(dmg)
        }
    }
}
"#;
    let r = transpile(src, "t3.wrm").unwrap();
    assert!(r.contains("GetPartBoundsInRadius"));
    assert!(r.contains("Magnitude"));
    assert!(r.contains("falloff = "));
    assert!(r.contains("not hit[char]"));
}

// ─── U: Realistic Error Handling ───────────────────────────────────

#[test]
fn u1_http_request_with_retry() {
    let src = r#"
local function fetchWithRetry(url, maxRetries) {
    for attempt in range(0, maxRetries) {
        try {
            local response = HttpService:GetAsync(url)
            return response
        } catch {
            warn(f"Attempt {attempt + 1} failed, retrying...")
            task.wait(2 ^ attempt)
        }
    }
    return nil
}
"#;
    let r = transpile(src, "u1.wrm").unwrap();
    assert!(r.contains("pcall("));
    assert!(r.contains("if not ok then"));
    assert!(r.contains("task.wait(2 ^ attempt)"));
}

#[test]
fn u2_pcall_with_custom_error() {
    let src = r#"
local function safeDivide(a, b) {
    if (b == 0) {
        error("Division by zero")
    }
    return a / b
}
local function calculateRatio(x, y) {
    try {
        return safeDivide(x, y)
    } catch e {
        warn(f"Calculation failed: {e}")
        return 0
    }
}
"#;
    let r = transpile(src, "u2.wrm").unwrap();
    assert!(r.contains("error(\"Division by zero\")"));
    assert!(r.contains("pcall("));
}

#[test]
fn u3_require_with_fallback() {
    let src = r#"
local Logger
try {
    Logger = require(script.Parent.Shared.logger)
} catch {
    warn("Logger module not found, using default")
    Logger = {log: function(msg) { print(msg) }}
}
Logger.log("System initialized")
"#;
    let r = transpile(src, "u3.wrm").unwrap();
    assert!(r.contains("pcall("));
    assert!(r.contains("Logger = require("));
}

// ─── V: List Comprehensions / Advanced Features ────────────────────

#[test]
fn v1_list_comp_filter_roblox_players() {
    let src = r#"
local function getAlivePlayers() {
    return [p for p in Players:GetPlayers() if (p.Character and p.Character:FindFirstChild("Humanoid"))]
}
"#;
    let r = transpile(src, "v1.wrm").unwrap();
    assert!(r.contains("for _"));
    assert!(r.contains("ipairs("));
    assert!(r.contains("table.insert"));
}

#[test]
fn v2_list_comp_map_items() {
    let src = r#"
local function getPlayerNames() {
    return [p.Name for p in Players:GetPlayers() if (p.Name.length > 0)]
}
"#;
    let r = transpile(src, "v2.wrm").unwrap();
    assert!(r.contains("table.insert"));
    assert!(r.contains("#p.Name"));
}

#[test]
fn v3_ternary_in_game_logic() {
    let src = r#"
local function getPlayerTeamColor(team) {
    return team == "Red" ? Color3.fromRGB(255, 50, 50) :
           team == "Blue" ? Color3.fromRGB(50, 50, 255) :
           Color3.fromRGB(150, 150, 150)
}
"#;
    let r = transpile(src, "v3.wrm").unwrap();
    assert!(r.contains("(if team == \"Red\" then Color3.fromRGB(255, 50, 50)"));
    assert!(r.contains("(if team == \"Blue\" then Color3.fromRGB(50, 50, 255)"));
}

// ─── W: Async / Task Patterns ──────────────────────────────────────

#[test]
fn w1_parallel_task_loading() {
    let src = r#"
local function loadAssets() {
    task.spawn(function() {
        loadCharacterAssets()
    })
    task.spawn(function() {
        loadMapAssets()
    })
    task.spawn(function() {
        loadGuiAssets()
    })
    task.wait(5)
    print("All assets loaded")
}
"#;
    let r = transpile(src, "w1.wrm").unwrap();
    assert!(r.contains("task.spawn(function()"));
    assert!(r.contains("task.wait(5)"));
}

#[test]
fn w2_countdown_timer_with_callback() {
    let src = r#"
local function startCountdown(seconds, onTick, onComplete) {
    local remaining = seconds
    while (remaining > 0) {
        onTick(remaining)
        task.wait(1)
        remaining = remaining - 1
    }
    onComplete()
}
startCountdown(10,
    function(s) { print(f"⏱️ {s}...") },
    function() { print("GO! 🚀") }
)
"#;
    let r = transpile(src, "w2.wrm").unwrap();
    assert!(r.contains("local function startCountdown(seconds, onTick, onComplete)"));
    assert!(r.contains("onTick(remaining)"));
    assert!(r.contains("onComplete()"));
}

#[test]
fn w3_coroutine_scheduler_sim() {
    let src = r#"
local function heartbeat(dt) {
    for entity in activeEntities {
        if (entity:isAlive()) {
            entity:update(dt)
        } else {
            entity:destroy()
        }
    }
}
RunService.Heartbeat:Connect(heartbeat)
"#;
    let r = transpile(src, "w3.wrm").unwrap();
    assert!(r.contains("RunService.Heartbeat:Connect(heartbeat)"));
    assert!(r.contains("entity:update(dt)"));
    assert!(r.contains("entity:destroy()"));
}

// ─── U: Private Member Scope & Forwarding Stubs ─────────────────────
// Tests for shadow-table resolution, method forwarding stubs,
// safe-chain suppression on self-rooted chains, and scope shadowing.

#[test]
fn u1_private_bare_access_resolves_to_shadow_table() {
    let src = r#"
private class Data {
    private local value = 10
    private function get() {
        return value
    }
}
"#;
    let r = transpile(src, "u1.wrm").unwrap();
    assert!(r.contains("__private_Data[self].value"));
    assert!(r.contains("__private_Data[self].get = function("));
}

#[test]
fn u2_private_member_chain_resolves_correctly() {
    let src = r#"
private class Holder {
    private local config = {radius = 5}
    private function read() {
        return config.radius
    }
}
"#;
    let r = transpile(src, "u2.wrm").unwrap();
    assert!(r.contains("__private_Holder[self].config.radius"));
}

#[test]
fn u3_self_rooted_chain_skips_safe_wrapping() {
    let src = r#"
private class Box {
    private local data = {x = 1, y = 2}
    private function show() {
        return self.data.x
    }
}
"#;
    let r = transpile(src, "u3.wrm").unwrap();
    assert!(!r.contains("(self and self.data and self.data.x)"));
    assert!(r.contains("__private_Box[self].data.x"));
}

#[test]
fn u4_non_self_chain_no_safe_wrapping() {
    let src = r#"
local function read(obj) {
    return obj.nested.value
}
"#;
    let r = transpile(src, "u4.wrm").unwrap();
    assert!(!r.contains("(obj and obj.nested and obj.nested.value)"));
    assert!(r.contains("return obj.nested.value"));
}

#[test]
fn u5_private_method_forwarding_stub_generated() {
    let src = r#"
private class Calc {
    private function add(a, b) {
        return a + b
    }
}
local c = Calc.new()
c:add(3, 4)
"#;
    let r = transpile(src, "u5.wrm").unwrap();
    assert!(r.contains("function Calc:add(a, b)\n"));
    assert!(r.contains("__private_Calc[self].add(a, b)"));
    assert!(r.contains("c:add(3, 4)"));
}

#[test]
fn u6_private_method_internal_call_uses_direct_shadow_access() {
    let src = r#"
private class Chain {
    private function inner() { return 42 }
    private function outer() { return self:inner() }
}
"#;
    let r = transpile(src, "u6.wrm").unwrap();
    assert!(r.contains("__private_Chain[self].inner()"));
}

#[test]
fn u7_private_member_assign_no_local_declaration() {
    let src = r#"
private class Counter {
    private local count = 0
    private function increment() {
        count = count + 1
    }
}
"#;
    let r = transpile(src, "u7.wrm").unwrap();
    assert!(r.contains("__private_Counter[self].count = __private_Counter[self].count + 1"));
}

#[test]
fn u8_private_init_called_from_constructor() {
    let src = r#"
private class Widget {
    private local ready = false
    private function init() {
        ready = true
    }
}
"#;
    let r = transpile(src, "u8.wrm").unwrap();
    assert!(r.contains("__private_Widget[self].init(...)"));
    assert!(!r.contains("self:init(...)"));
}

#[test]
fn u9_private_init_sets_public_property() {
    let src = r#"
private class Configurable {
    private function init(cfg) {
        self.cfg = cfg
    }
}
local obj = Configurable.new({a = 1})
obj:init({b = 2})
"#;
    let r = transpile(src, "u9.wrm").unwrap();
    assert!(r.contains("self.cfg = cfg"));
    assert!(r.contains("function Configurable:init(cfg)"));
}

#[test]
fn u10_forwarding_stub_no_params() {
    let src = r#"
private class Empty {
    private function ping() { }
}
"#;
    let r = transpile(src, "u10.wrm").unwrap();
    assert!(r.contains("function Empty:ping()\n"));
    assert!(r.contains("__private_Empty[self].ping()"));
}

#[test]
fn u11_forwarding_stub_multiple_params() {
    let src = r#"
private class Mover {
    private function move(a, b, c) { }
}
"#;
    let r = transpile(src, "u11.wrm").unwrap();
    assert!(r.contains("function Mover:move(a, b, c)\n"));
    assert!(r.contains("__private_Mover[self].move(a, b, c)"));
}

#[test]
fn u12_lexical_local_shadows_private_member() {
    let src = r#"
private class Shadow {
    private local name = "class"
    private function test() {
        local name = "local"
        return name
    }
}
"#;
    let r = transpile(src, "u12.wrm").unwrap();
    assert!(r.contains("local name = \"local\""));
    assert!(r.contains("return name"));
}

#[test]
fn u13_multiple_classes_separate_private_tables() {
    let src = r#"
private class A {
    private local x = 1
}
private class B {
    private local x = 2
}
"#;
    let r = transpile(src, "u13.wrm").unwrap();
    assert!(r.contains("local __private_A = setmetatable"));
    assert!(r.contains("local __private_B = setmetatable"));
    assert!(r.contains("__private_A[self].x = 1"));
    assert!(r.contains("__private_B[self].x = 2"));
}

#[test]
fn u14_public_method_still_works_normally() {
    let src = r#"
private class Mixed {
    private local secret = "hidden"
    public function expose() {
        return self:readSecret()
    }
    private function readSecret() {
        return secret
    }
}
"#;
    let r = transpile(src, "u14.wrm").unwrap();
    assert!(r.contains("function Mixed:expose()"));
    assert!(r.contains("__private_Mixed[self].readSecret()"));
}

#[test]
fn u15_range_for_loop_generated_correctly() {
    let src = r#"
local function loop() {
    for i in range(0, 5) {
        print(i)
    }
}
"#;
    let r = transpile(src, "u15.wrm").unwrap();
    assert!(r.contains("for i = 0, 5 - 1 do"));
}

#[test]
fn u16_deep_self_chain_no_safe_wrapping() {
    let src = r#"
private class Deep {
    private local nested = {inner = {value = 42}}
    private function get() {
        return self.data.deep.field
    }
}
"#;
    let r = transpile(src, "u16.wrm").unwrap();
    assert!(!r.contains("(self and self.data and self.data.deep and self.data.deep.field)"));
}

#[test]
fn u17_private_method_call_with_args_from_other_private_method() {
    let src = r#"
private class Helper {
    private function double(x) { return x * 2 }
    private function compute() {
        return self:double(5)
    }
}
"#;
    let r = transpile(src, "u17.wrm").unwrap();
    assert!(r.contains("__private_Helper[self].double(5)"));
}

#[test]
fn u18_private_member_used_in_binary_expression() {
    let src = r#"
private class Calc {
    private local offset = 10
    private function add(v) {
        return v + offset
    }
}
"#;
    let r = transpile(src, "u18.wrm").unwrap();
    assert!(r.contains("v + __private_Calc[self].offset"));
}

#[test]
fn u19_private_table_field_initialized_in_constructor() {
    let src = r#"
private class Store {
    private local items = []
}
"#;
    let r = transpile(src, "u19.wrm").unwrap();
    assert!(r.contains("__private_Store[self].items = {}"));
}

#[test]
fn u20_forwarding_stub_no_self_arg_in_call() {
    let src = r#"
private class Greeter {
    private function greet(name) {
        print(f"Hello {name}")
    }
}
local g = Greeter.new()
g:greet("World")
"#;
    let r = transpile(src, "u20.wrm").unwrap();
    assert!(r.contains("__private_Greeter[self].greet(name)"));
    assert!(!r.contains("__private_Greeter[self].greet(self, name)"));
}

#[test]
fn u21_constructor_private_var_init_with_expr() {
    let src = r#"
private class Calc {
    private local factor = 2 * 3
    private local multiplier = factor + 1
}
"#;
    let r = transpile(src, "u21.wrm").unwrap();
    assert!(r.contains("__private_Calc[self].factor = 2 * 3"));
    assert!(r.contains("__private_Calc[self].multiplier"));
}

#[test]
fn u22_private_method_used_as_callback() {
    let src = r#"
private class Timer {
    private function onTick() {
        spawnedCount = spawnedCount + 1
    }
    private local spawnedCount = 0
}
"#;
    let r = transpile(src, "u22.wrm").unwrap();
    assert!(r.contains("__private_Timer[self].spawnedCount + 1"));
}

#[test]
fn u23_enum_var_used_as_private_member_value() {
    let src = r#"
private enum Mode { A, B }
private class Switcher {
    private local current = Mode.A
    private function read() { return current }
}
"#;
    let r = transpile(src, "u23.wrm").unwrap();
    assert!(r.contains("__private_Switcher[self].current"));
    assert!(r.contains("Mode.A"));
}

#[test]
fn u24_struct_default_as_private_member() {
    let src = r#"
private struct Vec { x: number, y: number }
private class Point {
    private local pos = Vec.new(1, 2)
}
"#;
    let r = transpile(src, "u24.wrm").unwrap();
    assert!(r.contains("__private_Point[self].pos = Vec.new(1, 2)"));
}

#[test]
fn u25_private_vars_not_exported_in_module() {
    let src = r#"
private class Internal {
    private local val = 42
}
"#;
    let r = transpile(src, "u25.wrm").unwrap();
    assert!(r.contains("local Internal = {}"));
    assert!(!r.contains("module."));
}

#[test]
fn u26_array_length_via_length_property() {
    let src = r#"
private class List {
    private local items = [1, 2, 3]
    private function count() {
        return items.length
    }
}
"#;
    let r = transpile(src, "u26.wrm").unwrap();
    assert!(r.contains("#__private_List[self].items"));
}

#[test]
fn u27_private_method_accesses_self_public_property() {
    let src = r#"
private class Foo {
    private function init(name) {
        self.name = name
    }
}
"#;
    let r = transpile(src, "u27.wrm").unwrap();
    assert!(r.contains("self.name = name"));
}

#[test]
fn u28_multiple_private_methods_call_chain() {
    let src = r#"
private class Pipeline {
    private function step1(x) { return x + 1 }
    private function step2(x) { return x * 2 }
    private function run() {
        return self:step2(self:step1(5))
    }
}
"#;
    let r = transpile(src, "u28.wrm").unwrap();
    assert!(r.contains("__private_Pipeline[self].step2(__private_Pipeline[self].step1(5))"));
}

#[test]
fn u29_private_method_void_no_return() {
    let src = r#"
private class Logger {
    private function log(msg) {
        print(msg)
    }
}
"#;
    let r = transpile(src, "u29.wrm").unwrap();
    assert!(r.contains("__private_Logger[self].log = function(msg)"));
    assert!(r.contains("function Logger:log(msg)"));
}

#[test]
fn u30_for_loop_with_range_and_private_member() {
    let src = r#"
private class Looper {
    private local max = 5
    private function loop() {
        for i in range(0, max) {
            print(i)
        }
    }
}
"#;
    let r = transpile(src, "u30.wrm").unwrap();
    assert!(r.contains("for i = 0, __private_Looper[self].max - 1 do"));
}

#[test]
fn u31_fstring_with_private_member_interpolation() {
    let src = r#"
private class Info {
    private local version = "1.0"
    private function show() {
        return f"v{version}"
    }
}
"#;
    let r = transpile(src, "u31.wrm").unwrap();
    assert!(r.contains("__private_Info[self].version"));
}

#[test]
fn u32_ternary_with_private_member() {
    let src = r#"
private class Check {
    private local flag = true
    private function status() {
        return flag ? "on" : "off"
    }
}
"#;
    let r = transpile(src, "u32.wrm").unwrap();
    assert!(r.contains("if __private_Check[self].flag"));
}

#[test]
fn u33_list_comprehension_over_workspace() {
    let src = r#"
private class Finder {
    private function find() {
        return [p for p in workspace:GetChildren() if (p:IsA("Part"))]
    }
}
"#;
    let r = transpile(src, "u33.wrm").unwrap();
    assert!(r.contains("__private_Finder[self].find()"));
}

#[test]
fn u34_private_and_public_mixed_in_same_class() {
    let src = r#"
private class Hybrid {
    private local counter = 0
    public function bump() { counter = counter + 1; return counter }
    private function reset() { counter = 0 }
}
"#;
    let r = transpile(src, "u34.wrm").unwrap();
    assert!(r.contains("function Hybrid:bump()"));
    assert!(r.contains("function Hybrid:reset()"));
    assert!(r.contains("__private_Hybrid[self].reset()"));
}

#[test]
fn u35_init_with_params_stored() {
    let src = r#"
private class Data {
    private local items = []
    private function init(defaults) {
        items = defaults
    }
}
"#;
    let r = transpile(src, "u35.wrm").unwrap();
    assert!(r.contains("__private_Data[self].items = defaults"));
}

#[test]
fn u36_private_class_in_module_context() {
    let src = r#"
public class Shared {
    private local key = "secret"
}
"#;
    let r = transpile(src, "u36.wrm").unwrap();
    assert!(r.contains("module.Shared = {}"));
    assert!(r.contains("__private_Shared[self].key"));
}

#[test]
fn u37_method_with_default_params() {
    let src = r#"
private class Defs {
    private function greet(name = "World") {
        print(f"Hello {name}")
    }
}
"#;
    let r = transpile(src, "u37.wrm").unwrap();
    assert!(r.contains("function Defs:greet(name)"));
    assert!(r.contains("__private_Defs[self].greet(name)"));
}

#[test]
fn u38_private_method_accessed_externally_via_stub() {
    let src = r#"
private class Calc {
    private function square(x) { return x * x }
}
local c = Calc.new()
local r = c:square(3)
"#;
    let r = transpile(src, "u38.wrm").unwrap();
    assert!(r.contains("c:square(3)"));
    assert!(r.contains("function Calc:square(x)"));
}

#[test]
fn u39_nested_if_with_private_member() {
    let src = r#"
private class Guard {
    private local active = true
    private function check() {
        if (active) {
            if (active) {
                return 1
            }
        }
        return 0
    }
}
"#;
    let r = transpile(src, "u39.wrm").unwrap();
    assert!(r.contains("if __private_Guard[self].active then"));
}

#[test]
fn u40_try_catch_in_private_method() {
    let src = r#"
private class Safe {
    private function run() {
        try {
            risky()
        } catch e {
            print(f"Caught {e}")
        }
    }
}
"#;
    let r = transpile(src, "u40.wrm").unwrap();
    assert!(r.contains("__private_Safe[self].run = function("));
    assert!(r.contains("pcall("));
    assert!(r.contains("function Safe:run()"));
}

// ═══════════════════════════════════════════════════════════════════════
// U-SECTION — Type Annotations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn u41_local_type_annotation_simple() {
    let src = r#"local player: Player = Players.LocalPlayer
print(player.Name)"#;
    let r = transpile(src, "u41.wrm").unwrap();
    assert!(r.contains("local player = Players.LocalPlayer"));
    assert!(r.contains("print(player.Name)"));
    assert!(!r.contains(": Player"));
}

#[test]
fn u42_local_type_annotation_no_value() {
    let src = r#"local zone: Part
zone = Instance.new("Part")"#;
    let r = transpile(src, "u42.wrm").unwrap();
    assert!(r.contains("local zone"));
    assert!(!r.contains(": Part"));
    assert!(r.contains("zone = Instance.new(\"Part\")"));
}

#[test]
fn u43_array_type_annotation() {
    let src = r#"local zones: Part[] = workspace:GetChildren()
for zone in zones {
    zone.Anchored = true
}"#;
    let r = transpile(src, "u43.wrm").unwrap();
    assert!(r.contains("local zones = workspace:GetChildren()"));
    assert!(!r.contains(": Part[]"));
    assert!(r.contains("for"));
}

#[test]
fn u44_function_param_type_annotation() {
    let src = r#"function greet(player: Player, message: string) {
    print(f"{player.Name}: {message}")
}"#;
    let r = transpile(src, "u44.wrm").unwrap();
    assert!(r.contains("function greet(player, message)"));
    assert!(!r.contains(": Player"));
    assert!(!r.contains(": string"));
}

#[test]
fn u45_function_return_type_annotation() {
    let src = r#"function getPlayer(name: string) {
    return game.Players:FindFirstChild(name)
}"#;
    let r = transpile(src, "u45.wrm").unwrap();
    assert!(r.contains("function getPlayer(name)"));
    assert!(!r.contains(": string"));
}

#[test]
fn u46_for_loop_type_annotation() {
    let src = r#"for zone: Part in workspace:GetChildren() {
    zone.Anchored = true
}"#;
    let r = transpile(src, "u46.wrm").unwrap();
    assert!(r.contains("for"));
    assert!(!r.contains(": Part"));
}

#[test]
fn u47_multiple_params_all_typed() {
    let src = r#"function createBeam(from: Vector3, to: Vector3, color: Color3) {
    print(from)
    print(to)
    print(color)
}"#;
    let r = transpile(src, "u47.wrm").unwrap();
    assert!(r.contains("function createBeam(from, to, color)"));
}

#[test]
fn u48_typed_local_and_typed_param_together() {
    let src = r#"local p: Player = Players.LocalPlayer
function heal(target: Player) {
    target.Health += 10
}
heal(p)"#;
    let r = transpile(src, "u48.wrm").unwrap();
    assert!(r.contains("local p = Players.LocalPlayer"));
    assert!(r.contains("function heal(target)"));
    assert!(!r.contains(": Player"));
}

// ═══════════════════════════════════════════════════════════════════════
// U-SECTION — For-Loop Fixes
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn u49_get_tagged_uses_ipairs() {
    let src = r#"local zones = CollectionService:GetTagged("zone")
for zone in zones {
    print(zone.Name)
}"#;
    let r = transpile(src, "u49.wrm").unwrap();
    assert!(r.contains("for _, zone in ipairs(zones) do"));
}

#[test]
fn u50_get_children_uses_ipairs() {
    let src = r#"local kids = workspace:GetChildren()
for kid in kids {
    print(kid.Name)
}"#;
    let r = transpile(src, "u50.wrm").unwrap();
    assert!(r.contains("for _, kid in ipairs(kids) do"));
}

#[test]
fn u51_get_descendants_uses_ipairs() {
    let src = r#"local all = workspace:GetDescendants()
for item in all {
    print(item.Name)
}"#;
    let r = transpile(src, "u51.wrm").unwrap();
    assert!(r.contains("for _, item in ipairs(all) do"));
}

#[test]
fn u52_generic_for_pairs_uses_key_position() {
    let src = r#"local data = {key = "value", answer = 42}
for k in data {
    print(k)
}"#;
    let r = transpile(src, "u52.wrm").unwrap();
    assert!(r.contains("for k, _ in pairs(data) do"));
}

#[test]
fn u53_mixed_for_array_and_table() {
    let src = r#"local arr = ["a", "b", "c"]
local tbl = {x = 1, y = 2}
for item in arr { print(item) }
for key in tbl { print(key) }"#;
    let r = transpile(src, "u53.wrm").unwrap();
    assert!(r.contains("for _, item in ipairs(arr) do"));
    assert!(r.contains("for key, _ in pairs(tbl) do"));
}

// ═══════════════════════════════════════════════════════════════════════
// U-SECTION — Safe-Chain Behavior (no wrapping)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn u54_deep_chain_no_safe_wrapping() {
    let src = r#"local x = obj.a.b.c.d
print(x)"#;
    let r = transpile(src, "u54.wrm").unwrap();
    assert!(r.contains("local x = obj.a.b.c.d"));
    assert!(!r.contains(" and "));
}

#[test]
fn u55_mid_chain_no_wrapping() {
    let src = r#"local ui = zone.Display.Canvas.InfoDisplay
ui.Size = UDim2.new(1,0,1,0)"#;
    let r = transpile(src, "u55.wrm").unwrap();
    assert!(r.contains("local ui = zone.Display.Canvas.InfoDisplay"));
    assert!(!r.contains(" and "));
}

#[test]
fn u56_self_chain_still_plain() {
    let src = r#"public class Box {
    pos = Vector3.new(0,0,0)
    public function move(x: number, y: number, z: number) {
        self.pos = self.pos + Vector3.new(x,y,z)
    }
}"#;
    let r = transpile(src, "u56.wrm").unwrap();
    assert!(r.contains("self.pos = self.pos + Vector3.new(x, y, z)"));
}

// ═══════════════════════════════════════════════════════════════════════
// U-SECTION — OverlapParams / RaycastParams (global recognition)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn u57_overlap_params_new_no_warning() {
    let src = r#"local op = OverlapParams.new()
op.FilterType = Enum.RaycastFilterType.Include
workspace:GetPartBoundsInRadius(Vector3.new(0,0,0), 50, op)"#;
    let r = transpile(src, "u57.wrm").unwrap();
    assert!(r.contains("local op = OverlapParams.new()"));
    assert!(r.contains("Enum.RaycastFilterType.Include"));
}

#[test]
fn u58_raycast_params_creation() {
    let src = r#"local rp = RaycastParams.new()
rp.FilterType = Enum.RaycastFilterType.Exclude
rp.IgnoreWater = true
workspace:Raycast(Vector3.new(0,0,0), Vector3.new(0,100,0), rp)"#;
    let r = transpile(src, "u58.wrm").unwrap();
    assert!(r.contains("local rp = RaycastParams.new()"));
    assert!(r.contains("rp.IgnoreWater = true"));
}

// ═══════════════════════════════════════════════════════════════════════
// U-SECTION — Private Class Edge Cases
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn u59_private_class_with_typed_constructor() {
    let src = r#"private class Counter {
    count = 0
    public function increment(amount: number) {
        self.count += amount
    }
}"#;
    let r = transpile(src, "u59.wrm").unwrap();
    assert!(r.contains("__private_Counter"));
    assert!(r.contains("function Counter:increment(amount)"));
}

#[test]
fn u60_private_method_chaining_self() {
    let src = r#"private class Pipeline {
    data = {}
    private function process() {
        self.clean()
        self.normalize()
    }
    private function clean() {
        self.data.cleaned = true
    }
    private function normalize() {
        self.data.normalized = true
    }
    public function run() {
        self.process()
    }
}"#;
    let r = transpile(src, "u60.wrm").unwrap();
    assert!(r.contains("__private_Pipeline"));
    assert!(r.contains("function Pipeline:run()"));
}

#[test]
fn u61_private_var_deep_chain() {
    let src = r#"private class Data {
    config = {value = 42}
    public function read() {
        return self.config.value
    }
}"#;
    let r = transpile(src, "u61.wrm").unwrap();
    assert!(r.contains("__private_Data"));
    assert!(r.contains("self.config.value"));
}

// ═══════════════════════════════════════════════════════════════════════
// U-SECTION — Comp / Misc Edge Cases
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn u62_list_comp_with_local_variable() {
    let src = r#"
local function find(vals, target) {
    local found = [val for val in vals if val == target]
    return found.length > 0
}"#;
    let r = transpile(src, "u62.wrm").unwrap();
    assert!(r.contains("local found = "));
}

#[test]
fn u63_nested_try_catch() {
    let src = r##"
try {
    try {
        risky()
    } catch inner {
        print(f"inner: {inner}")
    }
} catch outer {
    print(f"outer: {outer}")
}"##;
    let r = transpile(src, "u63.wrm").unwrap();
    assert!(r.contains("pcall"));
    assert!(r.contains("print"));
}

#[test]
fn u64_empty_loop_does_not_crash() {
    let src = r#"for zone in {} {
    true
}"#;
    let r = transpile(src, "u64.wrm").unwrap();
    assert!(r.contains("for"));
}

#[test]
fn u65_bool_literal_conversion() {
    let src = r#"print(1 == 1)
print(1 == true)
print(1 != false)
print(false)"#;
    let r = transpile(src, "u65.wrm").unwrap();
    assert!(r.contains("print(1 == 1)"));
}

#[test]
fn u66_public_function_with_doc_comment() {
    let src = r#"
public class Utility {
    public function add(a: number, b: number) {
        return a + b
    }
}
local u = Utility.new()
print(u.add(3, 4))"#;
    let r = transpile(src, "u66.wrm").unwrap();
    assert!(r.contains("Utility"));
    assert!(r.contains("return a + b"));
}

#[test]
fn u67_member_chain_with_method_call() {
    let src = r#"local parent = part.Parent:FindFirstChild("Handle")
if (parent) {
    parent.Transparency = 0.5
}"#;
    let r = transpile(src, "u67.wrm").unwrap();
    assert!(r.contains("local parent = part.Parent:FindFirstChild(\"Handle\")"));
}

#[test]
fn u68_tween_service_create_with_type() {
    let src = r#"local tween: Tween = TweenService:Create(obj, info, {Value = 1})
tween:Play()
tween:Cancel()"#;
    let r = transpile(src, "u68.wrm").unwrap();
    assert!(r.contains("local tween = TweenService:Create(obj, info, {"));
    assert!(r.contains("tween:Play()"));
    assert!(r.contains("tween:Cancel()"));
}

#[test]
fn u69_signal_connection_with_callback() {
    let src = r#"local player: Player = Players.LocalPlayer
player.CharacterAdded:Connect(function(character: Model) {
    print(character.Name)
})"#;
    let r = transpile(src, "u69.wrm").unwrap();
    assert!(r.contains("player.CharacterAdded:Connect(function(character)"));
    assert!(!r.contains(": Player"));
    assert!(!r.contains(": Model"));
}

#[test]
fn u70_enum_dot_chain_in_assignment() {
    let src = r#"local params = OverlapParams.new()
params.FilterType = Enum.RaycastFilterType.Include
params.CollisionGroup = "Players""#;
    let r = transpile(src, "u70.wrm").unwrap();
    assert!(r.contains("Enum.RaycastFilterType.Include"));
    assert!(r.contains("params.CollisionGroup"));
}

#[test]
fn u71_for_in_over_direct_array_literal() {
    let src = r#"for item in {1, 2, 3, 4} {
    print(item)
}"#;
    let r = transpile(src, "u71.wrm").unwrap();
    assert!(r.contains("for"));
    assert!(r.contains("print(item)"));
}

#[test]
fn u72_range_for_with_step() {
    let src = r#"for i in range(0, 10, 2) {
    print(i)
}"#;
    let r = transpile(src, "u72.wrm").unwrap();
    assert!(r.contains("for i = 0, 10 - 1, 2 do"));
}

#[test]
fn u73_range_for_two_args() {
    let src = r#"for i in range(5, 15) {
    print(i)
}"#;
    let r = transpile(src, "u73.wrm").unwrap();
    assert!(r.contains("for i = 5, 15 - 1 do"));
}

#[test]
fn u74_range_for_single_arg() {
    let src = r#"for i in range(10) {
    print(i)
}"#;
    let r = transpile(src, "u74.wrm").unwrap();
    assert!(r.contains("for i = 0, 10 - 1 do"));
}

#[test]
fn u75_table_literal_with_mixed_keys() {
    let src = r#"local config = {
    name = "test",
    {x = 1, y = 2},
    count = 3
}
print(config.name)"#;
    let r = transpile(src, "u75.wrm").unwrap();
    assert!(r.contains("config = {"));
    assert!(r.contains("name = \"test\""));
    assert!(r.contains("print(config.name)"));
}

#[test]
fn u76_nil_value_in_table() {
    let src = r#"local t = {a = 1, b = nil}
t.b = 2"#;
    let r = transpile(src, "u76.wrm").unwrap();
    assert!(r.contains("t.b = 2"));
}

#[test]
fn u77_logical_or_with_nil_guard() {
    let src = r#"local name = player.Name or "Unknown"
local age = config.age or 0"#;
    let r = transpile(src, "u77.wrm").unwrap();
    assert!(r.contains("player.Name or \"Unknown\""));
    assert!(r.contains("config.age or 0"));
}

#[test]
fn u78_nested_function_with_capture() {
    let src = r#"local function outer(x) {
    local function inner(y) {
        return x + y
    }
    return inner(10)
}"#;
    let r = transpile(src, "u78.wrm").unwrap();
    assert!(r.contains("function outer(x)"));
    assert!(r.contains("function inner(y)"));
}

#[test]
fn u79_string_concat_with_variable() {
    let src = r#"local prefix = "Hello"
local msg = prefix + " " + "World"
print(msg)"#;
    let r = transpile(src, "u79.wrm").unwrap();
    assert!(r.contains("prefix"));
    assert!(r.contains("\"World\""));
}

#[test]
fn u80_method_call_return_assignment() {
    let src = r#"local parts = CollectionService:GetTagged("zone")
local first = parts[1]
if (first) {
    first.BrickColor = BrickColor.new("Bright red")
}"#;
    let r = transpile(src, "u80.wrm").unwrap();
    assert!(r.contains("CollectionService:GetTagged("));
    assert!(r.contains("BrickColor.new("));
}
