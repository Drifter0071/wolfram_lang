use crate::ast::{Expr, Stmt, TableField};
use crate::constants::ROBLOX_GLOBALS;
use crate::roblox_config::{
    resolve_import, resolve_project_import, DeploymentEntry, RobloxProjectConfig,
};
use crate::rojo_config::RojoPathMapping;
use crate::types::InferredType;
use std::collections::HashSet;

// ==========================================
// 4. THE GENERATOR (JSON AST -> Luau)
// ==========================================

struct GenContext {
    class_name: Option<String>,
    private_vars: HashSet<String>,
    private_methods: HashSet<String>,
    scopes: Vec<std::collections::HashMap<String, InferredType>>,
    roblox_mode: bool,
    roblox_config: Option<RobloxProjectConfig>,
    rojo_mappings: Option<Vec<RojoPathMapping>>,
    deployments: Vec<DeploymentEntry>,
    out_dir: String,
    importing_file: Option<String>,
    services: Vec<String>,
    module_prefix: Option<String>,
    module_exports: HashSet<String>,
}

impl GenContext {
    fn push_scope(&mut self) {
        self.scopes.push(std::collections::HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare_var(&mut self, name: String, var_type: InferredType) {
        if let Some(top_scope) = self.scopes.last_mut() {
            top_scope.insert(name, var_type);
        }
    }

    fn lookup_var(&self, name: &str) -> InferredType {
        for scope in self.scopes.iter().rev() {
            if let Some(var_type) = scope.get(name) {
                return var_type.clone();
            }
        }
        InferredType::Unknown
    }

    fn is_roblox_global(&self, name: &str) -> bool {
        self.roblox_mode && ROBLOX_GLOBALS.contains(&name)
    }

    fn resolve_roblox_import(&mut self, import_path: &str, alias: &str) -> String {
        let importing = self.importing_file.as_deref().unwrap_or("");

        let mut require_line =
            |path: String, svc: Option<String>| -> String {
                if let Some(svc_name) = svc {
                    if svc_name != "script" && !self.services.contains(&svc_name) {
                        self.services.push(svc_name.clone());
                    }
                }
                format!("local {} = require({})\n", alias, path)
            };

        if !import_path.starts_with('.') {
            if let Some(config) = &self.roblox_config {
                if let Some((require_path, service)) =
                    resolve_project_import(import_path, config, &self.deployments)
                {
                    return require_line(require_path, service);
                }
            }
            if let Some(ref mappings) = self.rojo_mappings {
                let src_import = format!(
                    "src/{}",
                    import_path.trim_start_matches("./").trim_start_matches('/')
                );
                if let Some((require_path, service)) = RojoPathMapping::resolve_import_to_require(
                    mappings,
                    &src_import,
                    import_path,
                    &self.out_dir,
                ) {
                    if service != "script" && !self.services.contains(&service) {
                        self.services.push(service.clone());
                    }
                    return format!("local {} = require({})\n", alias, require_path);
                }
            }
        }

        if let Some(config) = &self.roblox_config {
            if let Some((require_path, service)) =
                resolve_import(importing, import_path, config, &self.deployments)
            {
                return require_line(require_path, service);
            }
        }

        if let Some(ref mappings) = self.rojo_mappings {
            if let Some((require_path, service)) = RojoPathMapping::resolve_import_to_require(
                mappings,
                importing,
                import_path,
                &self.out_dir,
            ) {
                if service != "script" && !self.services.contains(&service) {
                    self.services.push(service.clone());
                }
                return format!("local {} = require({})\n", alias, require_path);
            }
        }

        let clean = import_path
            .trim_start_matches("./")
            .trim_start_matches('/')
            .replace("..", "_up_")
            .replace('/', "_")
            .replace('\\', "_");
        format!("local {} = require(script.Parent.{})\n", alias, clean)
    }
}

fn infer_expr_type(expr: &Expr, ctx: &GenContext) -> InferredType {
    match expr {
        Expr::Array(_) => InferredType::Array(Box::new(InferredType::Unknown)),
        Expr::Table(_) => InferredType::Table,
        Expr::Number(_) => InferredType::Number,
        Expr::Str(_) => InferredType::String,
        Expr::FString(_) => InferredType::String,
        Expr::Bool(_) => InferredType::Bool,
        Expr::Nil => InferredType::Unknown,
        Expr::Ident(name) => ctx.lookup_var(name),
        Expr::Grouping(inner) => infer_expr_type(inner, ctx),
        Expr::UnaryMinus(inner) => infer_expr_type(inner, ctx),
        Expr::Binary { .. } => InferredType::Unknown,
        Expr::Call { .. } => InferredType::Unknown,
        Expr::MethodCall { .. } => InferredType::Unknown,
        Expr::Member { .. } => InferredType::Unknown,
        Expr::Index { .. } => InferredType::Unknown,
        Expr::SelfExpr => InferredType::Unknown,
        Expr::Function { .. } => InferredType::Unknown,
        Expr::AwaitExpr(_) => InferredType::Unknown,
        Expr::ListComp { .. } => InferredType::Array(Box::new(InferredType::Unknown)),
        Expr::Ternary {
            then_expr,
            else_expr,
            ..
        } => {
            let t_type = infer_expr_type(then_expr, ctx);
            let e_type = infer_expr_type(else_expr, ctx);
            if t_type == e_type {
                t_type
            } else {
                InferredType::Unknown
            }
        }
        Expr::Logical { .. } => InferredType::Bool,
        Expr::Not(_) => InferredType::Bool,
    }
}

fn module_ref(ctx: &GenContext, name: &str) -> Option<String> {
    if let Some(ref prefix) = ctx.module_prefix {
        Some(format!("{}.{}", prefix, name))
    } else {
        None
    }
}

fn generate_stmt(stmt: &Stmt, indent: usize, ctx: &mut GenContext) -> String {
    let ind = "    ".repeat(indent);
    match stmt {
        Stmt::Local { name, value, .. } => {
            if ctx.is_roblox_global(name) {
                let val_str = value
                    .as_ref()
                    .map(|v| format!(" = {}", generate_expr(v, ctx)))
                    .unwrap_or_default();
                return format!("{}{}{}\n", ind, name, val_str);
            }
            let inferred_type = if let Some(val) = value {
                infer_expr_type(val, ctx)
            } else {
                InferredType::Unknown
            };
            ctx.declare_var(name.clone(), inferred_type);
            let val_str = value
                .as_ref()
                .map(|v| format!(" = {}", generate_expr(v, ctx)))
                .unwrap_or_default();
            format!("{}local {}{}\n", ind, name, val_str)
        }
        Stmt::Assign {
            target, value, op, ..
        } => {
            let inferred_type = infer_expr_type(value, ctx);
            if let Expr::Ident(name) = target {
                let is_private_member = ctx.class_name.is_some()
                    && (ctx.private_vars.contains(name) || ctx.private_methods.contains(name));
                if !is_private_member {
                    ctx.declare_var(name.clone(), inferred_type);
                }
            }
            let assign_op = match op {
                Some(o) => format!(" {}= ", o),
                None => " = ".to_string(),
            };
            format!(
                "{}{}{}{}\n",
                ind,
                generate_expr_lvalue(target, ctx),
                assign_op,
                generate_expr(value, ctx)
            )
        }
        Stmt::Return { value, .. } => {
            let val_str = value
                .as_ref()
                .map(|v| format!(" {}", generate_expr(v, ctx)))
                .unwrap_or_default();
            format!("{}return{}\n", ind, val_str)
        }
        Stmt::If {
            cond,
            then_block,
            else_if_blocks,
            else_block,
            ..
        } => {
            ctx.push_scope();
            let mut s = format!("{}if {} then\n", ind, generate_expr(cond, ctx));
            for b in then_block {
                s.push_str(&generate_stmt(b, indent + 1, ctx));
            }
            ctx.pop_scope();
            for (ei_cond, ei_block) in else_if_blocks {
                ctx.push_scope();
                s.push_str(&format!(
                    "{}elseif {} then\n",
                    ind,
                    generate_expr(ei_cond, ctx)
                ));
                for b in ei_block {
                    s.push_str(&generate_stmt(b, indent + 1, ctx));
                }
                ctx.pop_scope();
            }
            if let Some(else_b) = else_block {
                ctx.push_scope();
                s.push_str(&format!("{}else\n", ind));
                for b in else_b {
                    s.push_str(&generate_stmt(b, indent + 1, ctx));
                }
                ctx.pop_scope();
            }
            s.push_str(&format!("{}end\n", ind));
            s
        }
        Stmt::While { cond, block, .. } => {
            ctx.push_scope();
            let mut s = format!("{}while {} do\n", ind, generate_expr(cond, ctx));
            for b in block {
                s.push_str(&generate_stmt(b, indent + 1, ctx));
            }
            ctx.pop_scope();
            s.push_str(&format!("{}end\n", ind));
            s
        }
        Stmt::For {
            var, iter, block, ..
        } => {
            ctx.push_scope();
            let mut s = String::new();
            let is_range = if let Expr::Call { func, .. } = iter {
                func == "range"
            } else {
                false
            };

            if is_range {
                if let Expr::Call { args, .. } = iter {
                    match args.len() {
                        1 => {
                            s.push_str(&format!(
                                "{}for {} = 0, {} - 1 do\n",
                                ind,
                                var,
                                generate_expr(&args[0], ctx)
                            ));
                        }
                        2 => {
                            s.push_str(&format!(
                                "{}for {} = {}, {} - 1 do\n",
                                ind,
                                var,
                                generate_expr(&args[0], ctx),
                                generate_expr(&args[1], ctx)
                            ));
                        }
                        3 => {
                            s.push_str(&format!(
                                "{}for {} = {}, {} - 1, {} do\n",
                                ind,
                                var,
                                generate_expr(&args[0], ctx),
                                generate_expr(&args[1], ctx),
                                generate_expr(&args[2], ctx)
                            ));
                        }
                        _ => s.push_str(&format!("{}-- Invalid range arguments\n", ind)),
                    }
                }
            } else {
                let iter_type = infer_expr_type(iter, ctx);
                match iter_type {
                    InferredType::Array(_) => {
                        s.push_str(&format!(
                            "{}for _, {} in ipairs({}) do\n",
                            ind,
                            var,
                            generate_expr(iter, ctx)
                        ));
                    }
                    InferredType::Table => {
                        s.push_str(&format!(
                            "{}for {}, _ in pairs({}) do\n",
                            ind,
                            var,
                            generate_expr(iter, ctx)
                        ));
                    }
                    _ => {
                        s.push_str(&format!(
                            "{}for {}, _ in pairs({}) do\n",
                            ind,
                            var,
                            generate_expr(iter, ctx)
                        ));
                    }
                }
            }
            for b in block {
                s.push_str(&generate_stmt(b, indent + 1, ctx));
            }
            ctx.pop_scope();
            s.push_str(&format!("{}end\n", ind));
            s
        }
        Stmt::FuncDef {
            name,
            params,
            param_defaults,
            block,
            ..
        } => {
            ctx.push_scope();
            let mut s = if let Some(ref_name) = module_ref(ctx, name) {
                format!("{}{} = function({})\n", ind, ref_name, params.join(", "))
            } else {
                format!("{}local function {}({})\n", ind, name, params.join(", "))
            };
            for (i, default) in param_defaults.iter().enumerate() {
                if let Some(default_expr) = default {
                    let pname = &params[i];
                    s.push_str(&format!(
                        "{}    if {} == nil then {} = {}\n",
                        ind,
                        pname,
                        pname,
                        generate_expr(default_expr, ctx)
                    ));
                }
            }
            for b in block {
                s.push_str(&generate_stmt(b, indent + 1, ctx));
            }
            ctx.pop_scope();
            s.push_str(&format!("{}end\n", ind));
            s
        }
        Stmt::ExprStmt { expr, .. } => format!("{}{}\n", ind, generate_expr(expr, ctx)),
        Stmt::Import { path, alias, .. } => {
            if ctx.roblox_mode && ctx.roblox_config.is_some() {
                ctx.resolve_roblox_import(path, alias)
            } else {
                format!("{}local {} = require(\"{}\")\n", ind, alias, path)
            }
        }
        Stmt::Break { .. } => format!("{}break\n", ind),
        Stmt::Continue { .. } => format!("{}continue\n", ind),
        Stmt::TryCatch {
            try_block,
            catch_clauses,
            finally_block,
            ..
        } => {
            let mut s = String::new();
            let fn_name = format!("__try_{}", ind.len());
            s.push_str(&format!("{}local function {}()\n", ind, fn_name));
            for b in try_block {
                s.push_str(&generate_stmt(b, indent + 1, ctx));
            }
            s.push_str(&format!("{}end\n", ind));
            s.push_str(&format!("{}local ok, err = pcall({})\n", ind, fn_name));
            for (i, (_type_name, var_name, block)) in catch_clauses.iter().enumerate() {
                let binding = var_name.as_deref().unwrap_or("err");
                if i == 0 {
                    s.push_str(&format!("{}if not ok then\n", ind));
                } else {
                    s.push_str(&format!("{}elseif not ok then\n", ind));
                }
                let bind_line = if var_name.is_some() {
                    format!("{}    local {} = err\n", ind, binding)
                } else {
                    String::new()
                };
                s.push_str(&bind_line);
                for b in block {
                    s.push_str(&generate_stmt(b, indent + 1, ctx));
                }
            }
            if !catch_clauses.is_empty() {
                s.push_str(&format!("{}end\n", ind));
            }
            if let Some(finally) = finally_block {
                s.push_str(&format!("{}do\n", ind));
                for b in finally {
                    s.push_str(&generate_stmt(b, indent + 1, ctx));
                }
                s.push_str(&format!("{}end\n", ind));
            }
            s
        }
        Stmt::DecoratedStmt { stmt, .. } => {
            // Generate the inner statement; decorators are metadata comments
            generate_stmt(stmt, indent, ctx)
        }
        Stmt::ClassDef { name, body, .. } => {
            let ref_name = module_ref(ctx, name);

            let mut private_vars = HashSet::new();
            let mut private_methods = HashSet::new();
            for b in body {
                match b {
                    Stmt::Local {
                        name: v_name,
                        access,
                        ..
                    } if access == "private" => {
                        private_vars.insert(v_name.clone());
                    }
                    Stmt::FuncDef {
                        name: m_name,
                        access,
                        ..
                    } if access == "private" => {
                        private_methods.insert(m_name.clone());
                    }
                    _ => {}
                }
            }

            let mut class_ctx = GenContext {
                class_name: Some(name.clone()),
                private_vars,
                private_methods,
                scopes: vec![std::collections::HashMap::new()],
                roblox_mode: false,
                roblox_config: None,
                rojo_mappings: None,
                deployments: Vec::new(),
                out_dir: String::new(),
                importing_file: None,
                services: Vec::new(),
                module_prefix: ctx.module_prefix.clone(),
                module_exports: ctx.module_exports.clone(),
            };

            let has_init = body
                .iter()
                .any(|b| matches!(b, Stmt::FuncDef { name: m_name, .. } if m_name == "init"));

            let name_use: String;
            let empty_decl: String;
            if let Some(ref r) = ref_name {
                name_use = r.clone();
                empty_decl = format!("{} = {{}}\n", r);
            } else {
                name_use = name.clone();
                empty_decl = format!("local {} = {{}}\n", name);
            }

            let mut s = String::new();
            s.push_str(&format!("-- Auto-generated Class: {}\n", name));
            s.push_str(&format!(
                "local __private_{} = setmetatable({{}}, {{__mode = \"k\"}})\n",
                name
            ));
            s.push_str(&empty_decl);
            s.push_str(&format!("{}.__index = {}\n\n", name_use, name_use));

            if has_init {
                s.push_str(&format!("function {}.new(...)\n", name_use));
            } else {
                s.push_str(&format!("function {}.new()\n", name_use));
            }
            s.push_str(&format!("    local self = setmetatable({{}}, {})\n", name_use));
            s.push_str(&format!("    __private_{}[self] = {{}}\n", name));

            for b in body {
                if let Stmt::Local {
                    name: v_name,
                    value,
                    access,
                    ..
                } = b
                {
                    let val_str = value
                        .as_ref()
                        .map(|v| generate_expr(v, &class_ctx))
                        .unwrap_or("nil".into());
                    if access == "private" {
                        s.push_str(&format!(
                            "    __private_{}[self].{} = {}\n",
                            name, v_name, val_str
                        ));
                    } else if value.is_some() {
                        s.push_str(&format!("    self.{} = {}\n", v_name, val_str));
                    }
                }
            }

            for b in body {
                if let Stmt::FuncDef {
                    name: m_name,
                    params,
                    block,
                    access,
                    ..
                } = b
                {
                    if access == "private" {
                        s.push_str(&format!(
                            "    __private_{}[self].{} = function({})\n",
                            name, m_name, params.join(", ")
                        ));
                        class_ctx.push_scope();
                        for mb in block {
                            s.push_str(&generate_stmt(mb, 2, &mut class_ctx));
                        }
                        class_ctx.pop_scope();
                        s.push_str("    end\n");
                    }
                }
            }

            if has_init {
                let init_is_private = body.iter().any(|b| matches!(b,
                    Stmt::FuncDef { name: n, access, .. } if n == "init" && access == "private"));
                if init_is_private {
                    s.push_str(&format!("    __private_{}[self].init(self, ...)\n", name));
                } else {
                    s.push_str("    self:init(...)\n");
                }
            }
            s.push_str("    return self\nend\n\n");

            for b in body {
                if let Stmt::FuncDef {
                    name: m_name,
                    params,
                    block,
                    access,
                    ..
                } = b
                {
                    if access == "public" {
                        s.push_str(&format!("function {}:{}({})\n", name_use, m_name, params.join(", ")));
                        class_ctx.push_scope();
                        for mb in block {
                            s.push_str(&generate_stmt(mb, 1, &mut class_ctx));
                        }
                        class_ctx.pop_scope();
                        s.push_str("end\n");
                    }
                }
            }

            // Generate public forwarding stubs for private methods so they
            // are callable externally via instance:method() — delegates to
            // the shadow table where the implementation lives.
            for b in body {
                if let Stmt::FuncDef {
                    name: m_name,
                    params,
                    access,
                    ..
                } = b
                {
                    if access == "private" {
                        s.push_str(&format!("function {}:{}({})\n", name_use, m_name, params.join(", ")));
                        let mut forward_args = format!("    return __private_{}[self].{}(self", name, m_name);
                        for p in params {
                            forward_args.push_str(&format!(", {}", p));
                        }
                        forward_args.push_str(")\n");
                        s.push_str(&forward_args);
                        s.push_str("end\n");
                    }
                }
            }
            s
        }
        Stmt::EnumDef { name, variants, .. } => {
            ctx.declare_var(name.clone(), InferredType::Table);
            let variant_strs: Vec<String> = variants
                .iter()
                .map(|v| format!("{} = \"{}\"", v, v))
                .collect();
            if let Some(ref_name) = module_ref(ctx, name) {
                format!(
                    "{}{} = table.freeze({{{}}})\n",
                    ind, ref_name, variant_strs.join(", ")
                )
            } else {
                format!(
                    "{}local {} = table.freeze({{{}}})\n",
                    ind, name, variant_strs.join(", ")
                )
            }
        }
        Stmt::StructDef { name, fields, .. } => {
            ctx.declare_var(name.clone(), InferredType::Table);
            let field_names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
            let params = field_names.join(", ");
            let field_assignments: Vec<String> = field_names
                .iter()
                .map(|f| format!("{} = {}", f, f))
                .collect();
            if let Some(ref_name) = module_ref(ctx, name) {
                let mut s = format!("{}{} = {{}}\n", ind, ref_name);
                s.push_str(&format!("{}function {}.new({})\n", ind, ref_name, params));
                s.push_str(&format!("{}    return {{{}}}\n", ind, field_assignments.join(", ")));
                s.push_str(&format!("{}end\n", ind));
                s
            } else {
                let mut s = format!("{}local {} = {{}}\n", ind, name);
                s.push_str(&format!("{}function {}.new({})\n", ind, name, params));
                s.push_str(&format!("{}    return {{{}}}\n", ind, field_assignments.join(", ")));
                s.push_str(&format!("{}end\n", ind));
                s
            }
        }
    }
}

fn is_simple_chain_root(expr: &Expr) -> bool {
    matches!(expr, Expr::Ident(_) | Expr::SelfExpr | Expr::Member { .. })
}

fn generate_safe_member_chain(expr: &Expr, ctx: &GenContext) -> String {
    fn collect_member_parts(expr: &Expr, ctx: &GenContext) -> (Vec<String>, String) {
        match expr {
            Expr::Member {
                obj,
                field,
                is_colon,
            } => {
                let (mut parts, root) = collect_member_parts(obj, ctx);
                if field == "length" && !is_colon {
                    let last_idx = parts.len() - 1;
                    parts[last_idx] = format!("#{}", parts[last_idx]);
                } else {
                    let sep = if *is_colon { ":" } else { "." };
                    let last = parts.last().unwrap().clone();
                    parts.push(format!("{}{}{}", last, sep, field));
                }
                (parts, root)
            }
            _ => {
                let root = generate_expr(expr, ctx);
                (vec![root], String::new())
            }
        }
    }

    let (parts, _) = collect_member_parts(expr, ctx);
    format!("({})", parts.join(" and "))
}

fn generate_expr(expr: &Expr, ctx: &GenContext) -> String {
    generate_expr_impl(expr, ctx, true)
}

fn process_fstring_interpolations(raw: &str, ctx: &GenContext) -> String {
    let mut result = String::with_capacity(raw.len());
    let chars: Vec<char> = raw.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '{' && i + 1 < chars.len() && chars[i + 1] == '{' {
            result.push('{');
            i += 2;
            continue;
        }
        if chars[i] == '}' && i + 1 < chars.len() && chars[i + 1] == '}' {
            result.push('}');
            i += 2;
            continue;
        }
        if chars[i] == '{' {
            let mut depth = 1;
            let mut expr_str = String::new();
            i += 1;
            while i < chars.len() && depth > 0 {
                if chars[i] == '{' {
                    depth += 1;
                } else if chars[i] == '}' {
                    depth -= 1;
                    if depth == 0 {
                        i += 1;
                        break;
                    }
                }
                expr_str.push(chars[i]);
                i += 1;
            }
            if depth == 0 {
                match crate::parser::parse_expr_str(&expr_str) {
                    Ok(parsed) => {
                        result.push('{');
                        result.push_str(&generate_expr_impl(&parsed, ctx, false));
                        result.push('}');
                    }
                    Err(_) => {
                        result.push_str(&format!("{{{expr_str}}}"));
                    }
                }
            } else {
                result.push('{');
                result.push_str(&expr_str);
            }
            continue;
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}

fn generate_expr_lvalue(expr: &Expr, ctx: &GenContext) -> String {
    generate_expr_impl(expr, ctx, false)
}

fn generate_expr_impl(expr: &Expr, ctx: &GenContext, safe_chain: bool) -> String {
    match expr {
        Expr::Number(n) => n.to_string(),
        Expr::Str(s) => s.clone(),
        Expr::FString(s) => {
            let inner = &s[2..s.len() - 1];
            let processed = process_fstring_interpolations(inner, ctx);
            format!("`{}`", processed)
        }
        Expr::Bool(b) => b.to_string(),
        Expr::Nil => "nil".into(),
        Expr::Ident(name) => {
            // 1. Check local scope first — lexical shadowing takes precedence
            if ctx.lookup_var(name) != InferredType::Unknown {
                return name.clone();
            }
            // 2. Resolve private class members to shadow-table access
            if let Some(class_name) = &ctx.class_name {
                if ctx.private_vars.contains(name) || ctx.private_methods.contains(name) {
                    return format!("__private_{}[self].{}", class_name, name);
                }
            }
            // 3. Module exports get prefixed
            if ctx.module_exports.contains(name) {
                if let Some(ref prefix) = ctx.module_prefix {
                    return format!("{}.{}", prefix, name);
                }
            }
            name.clone()
        }
        Expr::SelfExpr => "self".into(),
        Expr::UnaryMinus(e) => format!("-{}", generate_expr(e, ctx)),
        Expr::Grouping(e) => format!("({})", generate_expr(e, ctx)),
        Expr::Array(elements) => {
            let el_strs: Vec<String> = elements.iter().map(|e| generate_expr(e, ctx)).collect();
            format!("{{{}}}", el_strs.join(", "))
        }
        Expr::Table(fields) => {
            let field_strs: Vec<String> = fields
                .iter()
                .map(|f| match f {
                    TableField::Pair { key, value } => {
                        let val_str = generate_expr(value, ctx);
                        match key {
                            Expr::Ident(_) => format!("{} = {}", generate_expr(key, ctx), val_str),
                            _ => format!("[{}] = {}", generate_expr(key, ctx), val_str),
                        }
                    }
                    TableField::Value(v) => generate_expr(v, ctx),
                })
                .collect();
            format!("{{{}}}", field_strs.join(", "))
        }
        Expr::Index { obj, index } => {
            format!("{}[{}]", generate_expr(obj, ctx), generate_expr(index, ctx))
        }
        Expr::Call { func, args } => {
            if func == "len" && args.len() == 1 {
                return format!("#{}", generate_expr(&args[0], ctx));
            }
            let arg_strs: Vec<String> = args.iter().map(|a| generate_expr(a, ctx)).collect();
            format!("{}({})", func, arg_strs.join(", "))
        }
        Expr::MethodCall {
            obj,
            field,
            is_colon,
            args,
        } => {
            let arg_strs: Vec<String> = args.iter().map(|a| generate_expr(a, ctx)).collect();
            if let Some(class_name) = &ctx.class_name {
                if ctx.private_methods.contains(field) {
                    let obj_str = generate_expr(obj, ctx);
                    let args_str = if arg_strs.is_empty() {
                        obj_str.clone()
                    } else {
                        format!("{}, {}", obj_str, arg_strs.join(", "))
                    };
                    return format!(
                        "__private_{}[{}].{}({})",
                        class_name, obj_str, field, args_str
                    );
                }
            }
            let sep = if *is_colon { ":" } else { "." };
            format!(
                "{}{}{}({})",
                generate_expr(obj, ctx),
                sep,
                field,
                arg_strs.join(", ")
            )
        }
        Expr::Member {
            obj,
            field,
            is_colon,
        } => {
            if field == "length" && !is_colon {
                let inner = generate_expr_impl(obj, ctx, safe_chain);
                return format!("#{}", inner);
            }
            if let Some(class_name) = &ctx.class_name {
                if ctx.private_vars.contains(field) || ctx.private_methods.contains(field) {
                    return format!(
                        "__private_{}[{}].{}",
                        class_name,
                        generate_expr_impl(obj, ctx, safe_chain),
                        field
                    );
                }
            }
            if safe_chain && matches!(&**obj, Expr::Member { .. }) && is_simple_chain_root(obj) {
                return generate_safe_member_chain(expr, ctx);
            }
            let sep = if *is_colon { ":" } else { "." };
            format!(
                "{}{}{}",
                generate_expr_impl(obj, ctx, safe_chain),
                sep,
                field
            )
        }
        Expr::Binary { left, op, right } => {
            if op == "==" {
                if let Expr::Bool(true) = **right {
                    return generate_expr(left, ctx);
                } else if let Expr::Bool(true) = **left {
                    return generate_expr(right, ctx);
                }
            }
            format!(
                "{} {} {}",
                generate_expr(left, ctx),
                op,
                generate_expr(right, ctx)
            )
        }
        Expr::Ternary {
            cond,
            then_expr,
            else_expr,
        } => {
            format!(
                "(if {} then {} else {})",
                generate_expr(cond, ctx),
                generate_expr(then_expr, ctx),
                generate_expr(else_expr, ctx)
            )
        }
        Expr::Logical { left, op, right } => {
            format!(
                "{} {} {}",
                generate_expr(left, ctx),
                op,
                generate_expr(right, ctx)
            )
        }
        Expr::Not(e) => {
            format!("not {}", generate_expr(e, ctx))
        }
        Expr::Function { params, block } => {
            let mut fn_ctx = GenContext {
                class_name: ctx.class_name.clone(),
                private_vars: ctx.private_vars.clone(),
                private_methods: ctx.private_methods.clone(),
                scopes: ctx.scopes.clone(),
                roblox_mode: ctx.roblox_mode,
                roblox_config: ctx.roblox_config.clone(),
                rojo_mappings: ctx.rojo_mappings.clone(),
                deployments: ctx.deployments.clone(),
                out_dir: ctx.out_dir.clone(),
                importing_file: ctx.importing_file.clone(),
                services: Vec::new(),
                module_prefix: ctx.module_prefix.clone(),
                module_exports: ctx.module_exports.clone(),
            };
            fn_ctx.push_scope();
            let mut s = format!("function({})\n", params.join(", "));
            for b in block {
                s.push_str(&generate_stmt(b, 1, &mut fn_ctx));
            }
            fn_ctx.pop_scope();
            s.push_str("end");
            s
        }
        Expr::AwaitExpr(inner) => generate_expr(inner, ctx),
        Expr::ListComp { elt, generators } => {
            let mut s = String::from("(function()\n    local _result = {}\n");
            for gen in generators {
                let is_range = if let Expr::Call { func, .. } = &gen.iter {
                    func == "range"
                } else {
                    false
                };
                if is_range {
                    if let Expr::Call { args, .. } = &gen.iter {
                        match args.len() {
                            1 => s.push_str(&format!(
                                "    for {} = 0, {} - 1 do\n",
                                gen.var,
                                generate_expr(&args[0], ctx)
                            )),
                            2 => s.push_str(&format!(
                                "    for {} = {}, {} - 1 do\n",
                                gen.var,
                                generate_expr(&args[0], ctx),
                                generate_expr(&args[1], ctx)
                            )),
                            3 => s.push_str(&format!(
                                "    for {} = {}, {} - 1, {} do\n",
                                gen.var,
                                generate_expr(&args[0], ctx),
                                generate_expr(&args[1], ctx),
                                generate_expr(&args[2], ctx)
                            )),
                            _ => s.push_str("    -- Invalid range\n"),
                        }
                    }
                } else {
                    s.push_str(&format!(
                        "    for _, {} in ipairs({}) do\n",
                        gen.var,
                        generate_expr(&gen.iter, ctx)
                    ));
                }
                if let Some(ref cond) = gen.condition {
                    s.push_str(&format!("        if {} then\n", generate_expr(cond, ctx)));
                    s.push_str(&format!(
                        "            table.insert(_result, {})\n",
                        generate_expr(elt, ctx)
                    ));
                    s.push_str("        end\n");
                } else {
                    s.push_str(&format!(
                        "        table.insert(_result, {})\n",
                        generate_expr(elt, ctx)
                    ));
                }
                s.push_str("    end\n");
            }
            s.push_str("    return _result\nend)()");
            s
        }
    }
}

pub fn generate(
    ast: &[Stmt],
    roblox_mode: bool,
    config: Option<&RobloxProjectConfig>,
    importing_file: Option<&str>,
    rojo_mappings: Option<&[RojoPathMapping]>,
    deployments: &[DeploymentEntry],
    out_dir: &str,
) -> String {
    let mut module_exports: HashSet<String> = HashSet::new();
    for stmt in ast {
        match stmt {
            Stmt::ClassDef { name, access, .. } if access == "public" => { module_exports.insert(name.clone()); }
            Stmt::EnumDef { name, access, .. } if access == "public" => { module_exports.insert(name.clone()); }
            Stmt::StructDef { name, access, .. } if access == "public" => { module_exports.insert(name.clone()); }
            Stmt::FuncDef { name, access, .. } if access == "public" => { module_exports.insert(name.clone()); }
            Stmt::Local { name, access, .. } if access == "public" => { module_exports.insert(name.clone()); }
            _ => {}
        }
    }

    let module_prefix = if !module_exports.is_empty() {
        Some("module".to_string())
    } else {
        None
    };

    let mut output = String::new();

    // Module wrapper header
    if module_prefix.is_some() {
        output.push_str("local module = {}\n");
    }

    let mut global_ctx = GenContext {
        class_name: None,
        private_vars: HashSet::new(),
        private_methods: HashSet::new(),
        scopes: vec![std::collections::HashMap::new()],
        roblox_mode,
        roblox_config: config.cloned(),
        rojo_mappings: rojo_mappings.map(|m| m.to_vec()),
        deployments: deployments.to_vec(),
        out_dir: out_dir.to_string(),
        importing_file: importing_file.map(|s| s.to_string()),
        services: Vec::new(),
        module_prefix,
        module_exports,
    };

    // Generate imports first (to collect services)
    let has_imports = ast.iter().any(|s| matches!(s, Stmt::Import { .. }));
    let mut import_lines: Vec<String> = Vec::new();
    if has_imports {
        for stmt in ast {
            if let Stmt::Import { .. } = stmt {
                import_lines.push(generate_stmt(stmt, 0, &mut global_ctx));
            }
        }
    }

    // Prepend service declarations
    if !global_ctx.services.is_empty() {
        let mut seen = HashSet::new();
        for svc in &global_ctx.services {
            if seen.insert(svc.clone()) {
                output.push_str(&format!("local {} = game:GetService(\"{}\")\n", svc, svc));
            }
        }
        output.push('\n');
    }

    // Output imports
    for line in &import_lines {
        output.push_str(line);
    }
    if !import_lines.is_empty() {
        output.push('\n');
    }

    // Output body
    for stmt in ast {
        if !matches!(stmt, Stmt::Import { .. }) {
            output.push_str(&generate_stmt(stmt, 0, &mut global_ctx));
        }
    }

    if roblox_mode {
        if !global_ctx.module_exports.is_empty() {
            output.push_str("\nreturn module\n");
        }
        return output;
    }

    if !global_ctx.module_exports.is_empty() {
        output.push_str("\nreturn module\n");
    }

    output
}
