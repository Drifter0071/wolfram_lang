use crate::ast::{CompGenerator, Expr, Span, Stmt, StructField, TableField};
use crate::lexer::Token;

// ==========================================
// 3. THE PARSER (Recursive Descent with Precedence)
// ==========================================
pub struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    source: &'a str,
    spans: Vec<usize>,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: Vec<Token>, spans: Vec<usize>, source: &'a str) -> Self {
        Self {
            tokens,
            pos: 0,
            source,
            spans,
        }
    }

    fn current_offset(&self) -> usize {
        self.spans.get(self.pos).copied().unwrap_or(0)
    }

    fn pos_string(&self) -> String {
        let offset = self.current_offset();
        let prefix = &self.source[..offset.min(self.source.len())];
        let line = prefix.bytes().filter(|&b| b == b'\n').count() + 1;
        let col = offset.saturating_sub(prefix.rfind('\n').map(|i| i + 1).unwrap_or(0)) + 1;
        format!("line {line}, column {col}")
    }

    fn current_span(&self) -> Span {
        let start = self.spans.get(self.pos).copied().unwrap_or(0);
        let end = self
            .spans
            .get(self.pos.saturating_sub(1))
            .copied()
            .unwrap_or(start);
        Span::new(end, end)
    }

    fn err_expected(&self, expected: &str, found: &dyn std::fmt::Debug) -> String {
        format!(
            "{}: expected {}, found {:?}",
            self.pos_string(),
            expected,
            found
        )
    }

    fn err_msg(&self, msg: &str) -> String {
        format!("{}: {}", self.pos_string(), msg)
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<Token> {
        let tok = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        tok
    }

    fn expect(&mut self, expected: Token) -> Result<(), String> {
        if self.peek() == Some(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(self.err_expected(&format!("{:?}", expected), &self.peek()))
        }
    }

    fn parse_type_annotation(&mut self) -> Result<String, String> {
        let name = match self.advance() {
            Some(Token::Ident(n)) => n,
            _ => return Err(self.err_msg("Expected type name")),
        };
        if self.peek() == Some(&Token::LBracket) && self.peek_ahead(1) == Some(&Token::RBracket) {
            self.advance(); self.advance();
            return Ok(format!("{}[]", name));
        }
        Ok(name)
    }

    fn peek_ahead(&self, n: usize) -> Option<&Token> {
        self.tokens.get(self.pos + n)
    }

    fn is_stmt_start(tok: Option<&Token>) -> bool {
        matches!(
            tok,
            Some(Token::If)
                | Some(Token::While)
                | Some(Token::For)
                | Some(Token::Return)
                | Some(Token::Function)
                | Some(Token::Class)
                | Some(Token::EnumKw)
                | Some(Token::StructKw)
                | Some(Token::Import)
                | Some(Token::Local)
                | Some(Token::Public)
                | Some(Token::Elif)
                | Some(Token::Private)
                | Some(Token::Break)
                | Some(Token::Continue)
                | Some(Token::Ident(_))
                | Some(Token::True)
                | Some(Token::False)
                | Some(Token::Nil)
                | Some(Token::SelfKw)
                | Some(Token::Not)
                | Some(Token::Minus)
                | Some(Token::LParen)
                | Some(Token::LBracket)
                | Some(Token::LBrace)
                | Some(Token::Number(_))
                | Some(Token::StringLit(_))
                | Some(Token::FString(_))
                | Some(Token::Try)
                | Some(Token::Async)
        )
    }

    fn semicolon_or_end(&mut self) -> Result<(), String> {
        if self.peek() == Some(&Token::Semicolon) {
            self.advance();
            return Ok(());
        }
        if self.peek() == Some(&Token::RBrace)
            || self.peek().is_none()
            || Self::is_stmt_start(self.peek())
        {
            return Ok(());
        }
        Err(self.err_expected("';' or end of statement", &self.peek()))
    }

    pub fn parse_program(&mut self) -> Result<Vec<Stmt>, String> {
        let mut stmts = Vec::new();
        while self.peek().is_some() {
            stmts.push(self.parse_stmt()?);
        }
        Ok(stmts)
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        self.expect(Token::LBrace)?;
        let mut stmts = Vec::new();
        while self.peek() != Some(&Token::RBrace) {
            if self.peek().is_none() {
                return Err(self.err_msg("Unexpected end of block"));
            }
            stmts.push(self.parse_stmt()?);
        }
        self.expect(Token::RBrace)?;
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        match self.peek() {
            Some(Token::Local) => {
                if self.tokens.get(self.pos + 1) == Some(&Token::Function) {
                    self.parse_local_function()
                } else {
                    self.parse_local()
                }
            }
            Some(Token::If) => self.parse_if(),
            Some(Token::While) => self.parse_while(),
            Some(Token::For) => self.parse_for(),
            Some(Token::Return) => self.parse_return(),
            Some(Token::Function) => self.parse_function(),
            Some(Token::Async) => self.parse_async_function(),
            Some(Token::Class) => self.parse_class(),
            Some(Token::EnumKw) => self.parse_enum(),
            Some(Token::StructKw) => self.parse_struct(),
            Some(Token::Import) => self.parse_import(),
            Some(Token::Try) => self.parse_try_catch(),
            Some(Token::Break) => {
                self.advance();
                self.semicolon_or_end()?;
                Ok(Stmt::Break {
                    span: self.current_span(),
                })
            }
            Some(Token::Continue) => {
                self.advance();
                self.semicolon_or_end()?;
                Ok(Stmt::Continue {
                    span: self.current_span(),
                })
            }
            Some(Token::Public) | Some(Token::Private) => self.parse_modifier_stmt(),
            Some(Token::At) => self.parse_decorated_stmt(),
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_modifier_stmt(&mut self) -> Result<Stmt, String> {
        let access = match self.advance().unwrap() {
            Token::Public => "public".to_string(),
            Token::Private => "private".to_string(),
            _ => unreachable!(),
        };

        match self.peek() {
            Some(Token::Local) => {
                let mut stmt = self.parse_local()?;
                if let Stmt::Local { access: a, .. } = &mut stmt {
                    *a = access;
                }
                Ok(stmt)
            }
            Some(Token::Function) => {
                let mut stmt = self.parse_function()?;
                if let Stmt::FuncDef { access: a, .. } = &mut stmt {
                    *a = access;
                }
                Ok(stmt)
            }
            Some(Token::Class) => {
                let mut stmt = self.parse_class()?;
                if let Stmt::ClassDef { access: a, .. } = &mut stmt {
                    *a = access;
                }
                Ok(stmt)
            }
            Some(Token::EnumKw) => {
                let mut stmt = self.parse_enum()?;
                if let Stmt::EnumDef { access: a, .. } = &mut stmt {
                    *a = access;
                }
                Ok(stmt)
            }
            Some(Token::StructKw) => {
                let mut stmt = self.parse_struct()?;
                if let Stmt::StructDef { access: a, .. } = &mut stmt {
                    *a = access;
                }
                Ok(stmt)
            }
            _ => Err(self.err_msg(
                "Expected 'local', 'function', 'class', 'enum', or 'struct' after access modifier",
            )),
        }
    }

    fn parse_local(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Local)?;
        let name = match self.advance() {
            Some(Token::Ident(n)) => n,
            _ => return Err(self.err_msg("Expected variable name")),
        };
        let mut var_names = vec![name];
        // Multi-variable local: local a, b = expr()
        while self.peek() == Some(&Token::Comma) {
            self.advance(); // consume comma
            let v = match self.advance() {
                Some(Token::Ident(n)) => n,
                _ => return Err(self.err_msg("Expected variable name after comma")),
            };
            var_names.push(v);
        }
        let joined = var_names.join(", ");

        let mut type_annotation = None;
        if self.peek() == Some(&Token::Colon) {
            self.advance();
            type_annotation = self.parse_type_annotation().ok();
        }

        let mut value = None;
        if self.peek() == Some(&Token::Assign) {
            self.advance();
            value = Some(self.parse_expr()?);
        }
        self.semicolon_or_end()?;
        Ok(Stmt::Local {
            name: joined,
            value,
            access: "private".into(),
            type_annotation,
            span: self.current_span(),
        })
    }

    fn parse_local_function(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Local)?;
        self.expect(Token::Function)?;
        let name = match self.advance() {
            Some(Token::Ident(n)) => n,
            _ => return Err(self.err_msg("Expected function name")),
        };
        self.expect(Token::LParen)?;
        let (params, param_types, param_defaults) = self.parse_param_list()?;
        self.expect(Token::RParen)?;
        let block = self.parse_block()?;
        Ok(Stmt::FuncDef {
            name,
            params,
            param_types,
            param_defaults,
            block,
            access: "private".into(),
            is_async: false,
            return_type: None,
            span: self.current_span(),
        })
    }

    fn parse_if(&mut self) -> Result<Stmt, String> {
        self.expect(Token::If)?;
        self.expect(Token::LParen)?;
        let cond = self.parse_expr()?;
        self.expect(Token::RParen)?;
        let then_block = self.parse_block()?;

        let mut else_if_blocks = Vec::new();
        let mut else_block = None;

        // Handle `else if (...) { ... }` or `elif (...) { ... }`
        loop {
            if self.peek() == Some(&Token::Else) {
                self.advance();
                if self.peek() == Some(&Token::If) {
                    self.advance();
                    self.expect(Token::LParen)?;
                    let ei_cond = self.parse_expr()?;
                    self.expect(Token::RParen)?;
                    let ei_block = self.parse_block()?;
                    else_if_blocks.push((ei_cond, ei_block));
                } else {
                    else_block = Some(self.parse_block()?);
                    break;
                }
            } else if self.peek() == Some(&Token::Elif) {
                self.advance();
                self.expect(Token::LParen)?;
                let ei_cond = self.parse_expr()?;
                self.expect(Token::RParen)?;
                let ei_block = self.parse_block()?;
                else_if_blocks.push((ei_cond, ei_block));
            } else {
                break;
            }
        }

        Ok(Stmt::If {
            cond,
            then_block,
            else_if_blocks,
            else_block,
            span: self.current_span(),
        })
    }

    fn parse_while(&mut self) -> Result<Stmt, String> {
        self.expect(Token::While)?;
        self.expect(Token::LParen)?;
        let cond = self.parse_expr()?;
        self.expect(Token::RParen)?;
        let block = self.parse_block()?;
        Ok(Stmt::While {
            cond,
            block,
            span: self.current_span(),
        })
    }

    fn parse_for(&mut self) -> Result<Stmt, String> {
        self.expect(Token::For)?;
        let var = match self.advance() {
            Some(Token::Ident(n)) => n,
            _ => return Err(self.err_msg("Expected variable name in for loop")),
        };
        let mut type_annotation = None;
        // Support `for k, v in iter` multi-variable for-in
        if self.peek() == Some(&Token::Comma) {
            self.advance(); // consume comma
            let _second = match self.advance() {
                Some(Token::Ident(v)) => v,
                _ => return Err(self.err_msg("Expected second variable name after comma in for loop")),
            };
        }
        if self.peek() == Some(&Token::Colon) {
            self.advance();
            type_annotation = self.parse_type_annotation().ok();
        }
        self.expect(Token::In)?;
        let iter = self.parse_expr()?;
        let block = self.parse_block()?;
        Ok(Stmt::For {
            var,
            iter,
            block,
            type_annotation,
            span: self.current_span(),
        })
    }

    fn parse_return(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Return)?;
        let mut value = None;
        if self.peek() != Some(&Token::Semicolon) && self.peek() != Some(&Token::RBrace) {
            value = Some(self.parse_expr()?);
        }
        self.semicolon_or_end()?;
        Ok(Stmt::Return {
            value,
            span: self.current_span(),
        })
    }

    fn parse_function(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Function)?;
        self.parse_function_body(false)
    }

    fn parse_async_function(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Async)?;
        self.expect(Token::Function)?;
        self.parse_function_body(true)
    }

    fn parse_function_body(&mut self, is_async: bool) -> Result<Stmt, String> {
        let name = match self.advance() {
            Some(Token::Ident(n)) => n,
            _ => return Err(self.err_msg("Expected function name")),
        };
        self.expect(Token::LParen)?;
        let (params, param_types, param_defaults) = self.parse_param_list()?;
        self.expect(Token::RParen)?;
        let block = self.parse_block()?;
        Ok(Stmt::FuncDef {
            name,
            params,
            param_types,
            param_defaults,
            block,
            access: "private".into(),
            is_async,
            return_type: None,
            span: self.current_span(),
        })
    }

    fn parse_param_list(&mut self) -> Result<(Vec<String>, Vec<Option<String>>, Vec<Option<Expr>>), String> {
        let mut params = Vec::new();
        let mut param_types = Vec::new();
        let mut defaults = Vec::new();
        if self.peek() != Some(&Token::RParen) {
            let (name, param_type, default) = self.parse_param()?;
            params.push(name);
            param_types.push(param_type);
            defaults.push(default);
            while self.peek() == Some(&Token::Comma) {
                self.advance();
                let (name, param_type, default) = self.parse_param()?;
                params.push(name);
                param_types.push(param_type);
                defaults.push(default);
            }
        }
        Ok((params, param_types, defaults))
    }

    fn parse_param(&mut self) -> Result<(String, Option<String>, Option<Expr>), String> {
        let name = match self.advance() {
            Some(Token::Ident(n)) => n,
            _ => return Err(self.err_msg("Expected parameter name")),
        };
        let param_type = if self.peek() == Some(&Token::Colon) {
            self.advance();
            match self.advance() {
                Some(Token::Ident(t)) => Some(t),
                _ => return Err(self.err_msg("Expected type after colon in parameter")),
            }
        } else {
            None
        };
        let default = if self.peek() == Some(&Token::Assign) {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };
        Ok((name, param_type, default))
    }

    fn parse_class(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Class)?;
        let name = match self.advance() {
            Some(Token::Ident(n)) => n,
            _ => return Err(self.err_msg("Expected class name")),
        };
        let body = self.parse_block()?;
        Ok(Stmt::ClassDef {
            name,
            body,
            access: "private".into(),
            span: self.current_span(),
        })
    }

    fn parse_enum(&mut self) -> Result<Stmt, String> {
        self.expect(Token::EnumKw)?;
        let name = match self.advance() {
            Some(Token::Ident(n)) => n,
            _ => return Err(self.err_msg("Expected enum name")),
        };
        self.expect(Token::LBrace)?;
        let mut variants = Vec::new();
        if self.peek() != Some(&Token::RBrace) {
            if let Some(Token::Ident(v)) = self.advance() {
                variants.push(v);
            } else {
                return Err(self.err_msg("Expected enum variant identifier"));
            }
            while self.peek() == Some(&Token::Comma) {
                self.advance();
                if self.peek() == Some(&Token::RBrace) {
                    break;
                }
                if let Some(Token::Ident(v)) = self.advance() {
                    variants.push(v);
                } else {
                    return Err(self.err_msg("Expected enum variant identifier after comma"));
                }
            }
        }
        self.expect(Token::RBrace)?;
        Ok(Stmt::EnumDef {
            name,
            variants,
            access: "private".into(),
            span: self.current_span(),
        })
    }

    fn parse_struct(&mut self) -> Result<Stmt, String> {
        self.expect(Token::StructKw)?;
        let name = match self.advance() {
            Some(Token::Ident(n)) => n,
            _ => return Err(self.err_msg("Expected struct name")),
        };
        self.expect(Token::LBrace)?;
        let mut fields = Vec::new();
        if self.peek() != Some(&Token::RBrace) {
            if let Some(Token::Ident(f)) = self.advance() {
                let field = self.parse_struct_field(f)?;
                fields.push(field);
            } else {
                return Err(self.err_msg("Expected struct field identifier"));
            }
            while self.peek() == Some(&Token::Comma) {
                self.advance();
                if self.peek() == Some(&Token::RBrace) {
                    break;
                }
                if let Some(Token::Ident(f)) = self.advance() {
                    let field = self.parse_struct_field(f)?;
                    fields.push(field);
                } else {
                    return Err(self.err_msg("Expected struct field identifier after comma"));
                }
            }
        }
        self.expect(Token::RBrace)?;
        Ok(Stmt::StructDef {
            name,
            fields,
            access: "private".into(),
            span: self.current_span(),
        })
    }

    fn parse_struct_field(&mut self, name: String) -> Result<StructField, String> {
        if self.peek() == Some(&Token::Colon) {
            self.advance(); // consume colon
            let type_name = match self.advance() {
                Some(Token::Ident(t)) => t,
                _ => return Err(self.err_msg("Expected type after colon in struct field")),
            };
            Ok(StructField {
                name,
                field_type: Some(type_name),
            })
        } else {
            Ok(StructField {
                name,
                field_type: None,
            })
        }
    }

    fn parse_import(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Import)?;
        let path = match self.advance() {
            Some(Token::StringLit(s)) => {
                let inner = if s.starts_with('"') && s.ends_with('"') {
                    s[1..s.len() - 1].to_string()
                } else {
                    s.clone()
                };
                inner
            }
            _ => return Err(self.err_msg("Expected string literal for import path")),
        };
        self.expect(Token::As)?;
        let alias = match self.advance() {
            Some(Token::Ident(a)) => a,
            _ => return Err(self.err_msg("Expected identifier for import alias")),
        };
        self.semicolon_or_end()?;
        Ok(Stmt::Import {
            path,
            alias,
            span: self.current_span(),
        })
    }

    fn parse_try_catch(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Try)?;
        let try_block = self.parse_block()?;
        let mut catch_clauses: Vec<(Option<String>, Option<String>, Vec<Stmt>)> = Vec::new();
        while self.peek() == Some(&Token::Catch) {
            self.advance();
            if self.peek() == Some(&Token::LBrace) {
                catch_clauses.push((None, None, self.parse_block()?));
            } else {
                let first = match self.advance() {
                    Some(Token::Ident(n)) => n,
                    _ => return Err(self.err_msg("Expected identifier or '{' after catch")),
                };
                if self.peek() == Some(&Token::As) {
                    self.advance();
                    let var = match self.advance() {
                        Some(Token::Ident(v)) => v,
                        _ => return Err(self.err_msg("Expected variable name after 'as'")),
                    };
                    catch_clauses.push((Some(first), Some(var), self.parse_block()?));
                } else {
                    // simple catch varname { ... }
                    catch_clauses.push((None, Some(first), self.parse_block()?));
                }
            }
        }
        let finally_block = if self.peek() == Some(&Token::Finally) {
            self.advance();
            Some(self.parse_block()?)
        } else {
            None
        };
        Ok(Stmt::TryCatch {
            try_block,
            catch_clauses,
            finally_block,
            span: self.current_span(),
        })
    }

    fn parse_decorated_stmt(&mut self) -> Result<Stmt, String> {
        let mut decorators = Vec::new();
        while self.peek() == Some(&Token::At) {
            self.advance(); // consume @
            let name = match self.advance() {
                Some(Token::Ident(n)) => n,
                _ => return Err(self.err_msg("Expected decorator name after '@'")),
            };
            decorators.push(name);
        }
        let stmt = Box::new(self.parse_stmt()?);
        Ok(Stmt::DecoratedStmt {
            decorators,
            stmt,
            span: self.current_span(),
        })
    }

    fn parse_expr_stmt(&mut self) -> Result<Stmt, String> {
        let expr = self.parse_expr()?;
        if self.peek() == Some(&Token::Assign) {
            self.advance();
            let value = self.parse_expr()?;
            self.semicolon_or_end()?;
            Ok(Stmt::Assign {
                target: expr,
                value,
                op: None,
                span: self.current_span(),
            })
        } else if self.peek() == Some(&Token::PlusAssign)
            || self.peek() == Some(&Token::MinusAssign)
            || self.peek() == Some(&Token::StarAssign)
            || self.peek() == Some(&Token::SlashAssign)
            || self.peek() == Some(&Token::PercentAssign)
        {
            let op = match self.advance().unwrap() {
                Token::PlusAssign => "+",
                Token::MinusAssign => "-",
                Token::StarAssign => "*",
                Token::SlashAssign => "/",
                Token::PercentAssign => "%",
                _ => unreachable!(),
            };
            let value = self.parse_expr()?;
            self.semicolon_or_end()?;
            Ok(Stmt::Assign {
                target: expr,
                value,
                op: Some(op.to_string()),
                span: self.current_span(),
            })
        } else {
            self.semicolon_or_end()?;
            Ok(Stmt::ExprStmt {
                expr,
                span: self.current_span(),
            })
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_or()?;

        if self.peek() == Some(&Token::Question) {
            self.advance();
            let then_expr = self.parse_expr()?;
            self.expect(Token::Colon)?;
            let else_expr = self.parse_expr()?;
            expr = Expr::Ternary {
                cond: Box::new(expr),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
            };
        }

        Ok(expr)
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_and()?;

        loop {
            if self.peek() == Some(&Token::Or) {
                self.advance();
                let right = self.parse_and()?;
                expr = Expr::Logical {
                    left: Box::new(expr),
                    op: "or".into(),
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_comparison()?;

        loop {
            if self.peek() == Some(&Token::And) {
                self.advance();
                let right = self.parse_comparison()?;
                expr = Expr::Logical {
                    left: Box::new(expr),
                    op: "and".into(),
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_addition()?;

        loop {
            match self.peek() {
                Some(Token::EqEq) | Some(Token::NotEq) | Some(Token::Lt) | Some(Token::Gt)
                | Some(Token::LtEq) | Some(Token::GtEq) => {
                    let op = match self.advance().unwrap() {
                        Token::EqEq => "==",
                        Token::NotEq => "~=",
                        Token::Lt => "<",
                        Token::Gt => ">",
                        Token::LtEq => "<=",
                        Token::GtEq => ">=",
                        _ => unreachable!(),
                    }
                    .to_string();

                    let right = self.parse_addition()?;
                    expr = Expr::Binary {
                        left: Box::new(expr),
                        op,
                        right: Box::new(right),
                    };
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_addition(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_multiplication()?;

        loop {
            match self.peek() {
                Some(Token::Plus) | Some(Token::Minus) | Some(Token::DotDot) => {
                    let op = match self.advance().unwrap() {
                        Token::Plus => "+",
                        Token::Minus => "-",
                        Token::DotDot => "..",
                        _ => unreachable!(),
                    }
                    .to_string();

                    let right = self.parse_multiplication()?;
                    expr = Expr::Binary {
                        left: Box::new(expr),
                        op,
                        right: Box::new(right),
                    };
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_multiplication(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_unary()?;

        loop {
            match self.peek() {
                Some(Token::StarStar)
                | Some(Token::SlashSlash)
                | Some(Token::Caret)
                | Some(Token::Star)
                | Some(Token::Slash)
                | Some(Token::Percent) => {
                    let op = match self.advance().unwrap() {
                        Token::StarStar => "^",
                        Token::SlashSlash => "//",
                        Token::Caret => "^",
                        Token::Star => "*",
                        Token::Slash => "/",
                        Token::Percent => "%",
                        _ => unreachable!(),
                    }
                    .to_string();

                    let right = self.parse_unary()?;
                    expr = Expr::Binary {
                        left: Box::new(expr),
                        op,
                        right: Box::new(right),
                    };
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        if self.peek() == Some(&Token::Minus) {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::UnaryMinus(Box::new(expr)));
        }
        if self.peek() == Some(&Token::Not) {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::Not(Box::new(expr)));
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.peek() {
                Some(Token::LParen) => {
                    self.advance();
                    let mut args = Vec::new();
                    if self.peek() != Some(&Token::RParen) {
                        args.push(self.parse_expr()?);
                        while self.peek() == Some(&Token::Comma) {
                            self.advance();
                            args.push(self.parse_expr()?);
                        }
                    }
                    self.expect(Token::RParen)?;
                    if let Expr::Ident(name) = expr {
                        expr = Expr::Call { func: name, args };
                    } else if let Expr::Member {
                        obj,
                        field,
                        is_colon,
                    } = expr
                    {
                        expr = Expr::MethodCall {
                            obj,
                            field,
                            is_colon,
                            args,
                        };
                    } else {
                        return Err(self.err_msg("Complex call not supported yet"));
                    }
                }
                Some(Token::Dot) | Some(Token::Colon) => {
                    let is_colon = matches!(self.peek(), Some(Token::Colon));
                    if is_colon {
                        let is_method_call =
                            match (self.tokens.get(self.pos + 1), self.tokens.get(self.pos + 2)) {
                                (Some(Token::Ident(_)), Some(Token::LParen)) => true,
                                _ => false,
                            };
                        if !is_method_call {
                            break;
                        }
                    }
                    self.advance();
                    if let Some(Token::Ident(field)) = self.advance() {
                        expr = Expr::Member {
                            obj: Box::new(expr),
                            field,
                            is_colon,
                        };
                    } else {
                        return Err(self.err_msg("Expected field name"));
                    }
                }
                Some(Token::LBracket) => {
                    self.advance();
                    let index = self.parse_expr()?;
                    self.expect(Token::RBracket)?;
                    expr = Expr::Index {
                        obj: Box::new(expr),
                        index: Box::new(index),
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.advance() {
            Some(Token::Await) => {
                let expr = self.parse_expr()?;
                Ok(Expr::AwaitExpr(Box::new(expr)))
            }
            Some(Token::Number(n)) => Ok(Expr::Number(n)),
            Some(Token::StringLit(s)) => Ok(Expr::Str(s)),
            Some(Token::FString(s)) => Ok(Expr::FString(s)),
            Some(Token::True) => Ok(Expr::Bool(true)),
            Some(Token::False) => Ok(Expr::Bool(false)),
            Some(Token::Nil) => Ok(Expr::Nil),
            Some(Token::SelfKw) => Ok(Expr::SelfExpr),
            Some(Token::Ident(name)) => Ok(Expr::Ident(name)),

            Some(Token::LParen) => {
                // Try to detect arrow function: (params) -> expr
                let is_arrow = self
                    .peek()
                    .map_or(false, |t| matches!(t, Token::Ident(_) | Token::RParen));
                if is_arrow {
                    let save_pos = self.pos;
                    let (params, _, _) = self.parse_param_list()?;
                    if self.peek() == Some(&Token::RParen) {
                        self.advance(); // )
                        if self.peek() == Some(&Token::Arrow) {
                            self.advance(); // ->
                            let body = if self.peek() == Some(&Token::LBrace) {
                                self.parse_block()?
                            } else {
                                let expr = self.parse_expr()?;
                                vec![Stmt::Return {
                                    value: Some(expr),
                                    span: self.current_span(),
                                }]
                            };
                            return Ok(Expr::Function {
                                params,
                                block: body,
                            });
                        }
                    }
                    // Not an arrow function — restore and parse as grouping
                    self.pos = save_pos;
                }
                let expr = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(Expr::Grouping(Box::new(expr)))
            }

            Some(Token::LBracket) => {
                let mut elements = Vec::new();
                if self.peek() != Some(&Token::RBracket) {
                    elements.push(self.parse_expr()?);

                    // Check for list comprehension: [expr for var in iter]
                    if self.peek() == Some(&Token::For) {
                        self.advance();
                        let var = match self.advance() {
                            Some(Token::Ident(n)) => n,
                            _ => {
                                return Err(
                                    self.err_msg("Expected variable name in list comprehension")
                                )
                            }
                        };
                        self.expect(Token::In)?;
                        let iter = self.parse_expr()?;

                        let mut generators = Vec::new();
                        let condition = if self.peek() == Some(&Token::If) {
                            self.advance();
                            Some(self.parse_expr()?)
                        } else {
                            None
                        };
                        generators.push(CompGenerator {
                            var,
                            iter,
                            condition,
                        });

                        self.expect(Token::RBracket)?;
                        return Ok(Expr::ListComp {
                            elt: Box::new(elements.remove(0)),
                            generators,
                        });
                    }

                    while self.peek() == Some(&Token::Comma) {
                        self.advance();
                        if self.peek() == Some(&Token::RBracket) {
                            break;
                        }
                        elements.push(self.parse_expr()?);
                    }
                }
                self.expect(Token::RBracket)?;
                Ok(Expr::Array(elements))
            }

            Some(Token::LBrace) => {
                let mut fields = Vec::new();
                if self.peek() != Some(&Token::RBrace) {
                    fields.push(self.parse_table_field()?);
                    while self.peek() == Some(&Token::Comma) {
                        self.advance();
                        if self.peek() == Some(&Token::RBrace) {
                            break;
                        }
                        fields.push(self.parse_table_field()?);
                    }
                }
                self.expect(Token::RBrace)?;
                Ok(Expr::Table(fields))
            }

            Some(Token::Function) => {
                self.expect(Token::LParen)?;
                let (params, _, _) = self.parse_param_list()?;
                self.expect(Token::RParen)?;
                let block = self.parse_block()?;
                Ok(Expr::Function { params, block })
            }

            Some(tok) => Err(self.err_expected("expression", &tok)),
            None => Err(self.err_msg("Unexpected end of file")),
        }
    }

    fn parse_table_field(&mut self) -> Result<TableField, String> {
        let is_pair = match (self.tokens.get(self.pos), self.tokens.get(self.pos + 1)) {
            (Some(Token::Ident(_)), Some(Token::Assign)) => true,
            (Some(Token::Ident(_)), Some(Token::Colon)) => true,
            (Some(Token::StringLit(_)), Some(Token::Assign)) => true,
            (Some(Token::StringLit(_)), Some(Token::Colon)) => true,
            _ => false,
        };

        if is_pair {
            let key_expr = self.parse_primary()?;
            self.advance(); // consume '=' or ':'
            let value = self.parse_expr()?;
            Ok(TableField::Pair {
                key: key_expr,
                value,
            })
        } else {
            let value = self.parse_expr()?;
            Ok(TableField::Value(value))
        }
    }
}

pub fn parse_expr_str(source: &str) -> Result<Expr, String> {
    use logos::Logos;
    let tokens: Vec<crate::lexer::Token> = crate::lexer::Token::lexer(source)
        .filter_map(|r| r.ok())
        .collect();
    let spans: Vec<usize> = crate::lexer::Token::lexer(source)
        .spanned()
        .filter_map(|(r, span)| r.ok().map(|_| span.start))
        .collect();
    let mut parser = Parser::new(tokens, spans, source);
    let expr = parser.parse_expr()?;
    if parser.peek().is_some() {
        return Err("unexpected tokens after expression".into());
    }
    Ok(expr)
}
