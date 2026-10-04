use crate::{
    ast::*,
    diagnostic::{Diagnostic, Result},
    lexer::{Kind, Token, lex},
};
use std::collections::HashSet;
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    types: HashSet<String>,
    graph_expr: bool,
    depth: usize,
}
impl Parser {
    pub fn new(file: &str, source: &str, known: &HashSet<String>) -> Result<Self> {
        let tokens = lex(file, source, false)?;
        let mut types = known.clone();
        for t in [
            "int", "long", "unsigned", "float", "double", "bool", "char", "string", "void", "list",
            "point", "vector",
        ] {
            types.insert(t.into());
        }
        for pair in tokens.windows(2) {
            if matches!(&pair[0].kind,Kind::Ident(s) if s=="object"||s=="struct")
                && let Kind::Ident(s) = &pair[1].kind
            {
                types.insert(s.clone());
            }
        }
        Ok(Self {
            tokens,
            pos: 0,
            types,
            graph_expr: false,
            depth: 0,
        })
    }
    fn token(&self) -> &Token {
        &self.tokens[self.pos]
    }
    fn span(&self) -> Span {
        self.token().span.clone()
    }
    fn is(&self, s: &str) -> bool {
        matches!(&self.token().kind,Kind::Ident(n)|Kind::Sym(n) if n==s)
    }
    fn peek(&self, n: usize, s: &str) -> bool {
        self.tokens
            .get(self.pos + n)
            .is_some_and(|t| matches!(&t.kind,Kind::Ident(x)|Kind::Sym(x) if x==s))
    }
    fn eat(&mut self, s: &str) -> bool {
        if self.is(s) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
    fn need(&mut self, s: &str) -> Result<()> {
        if self.eat(s) {
            Ok(())
        } else {
            Err(Diagnostic::new(
                "JOLE42",
                &self.span(),
                format!("expected `{s}`, received `{}`", self.token().raw),
            )
            .help(format!("insert `{s}` here")))
        }
    }
    fn ident(&mut self) -> Result<String> {
        if let Kind::Ident(s) = self.token().kind.clone() {
            self.pos += 1;
            Ok(s)
        } else {
            Err(Diagnostic::new(
                "JOLE42",
                &self.span(),
                "expected identifier",
            ))
        }
    }
    fn is_type(&self) -> bool {
        matches!(&self.token().kind,Kind::Ident(n) if self.types.contains(n))
    }
    fn ty(&mut self) -> Result<Type> {
        let name = self.ident()?;
        let mut t = match name.as_str() {
            "int" | "long" => Type::Int,
            "unsigned" => {
                self.eat("long");
                self.eat("int");
                Type::UInt
            }
            "float" | "double" => Type::Float,
            "bool" => Type::Bool,
            "char" => Type::Char,
            "string" => Type::String,
            "void" => Type::Void,
            "list" => Type::List,
            "point" => Type::Point,
            "vector" => Type::Vector,
            _ => Type::Named(name),
        };
        loop {
            if self.eat("*") {
                t = Type::Ptr(Box::new(t));
            } else if self.is("[") && self.peek(1, "]") {
                self.pos += 2;
                t = Type::Array(Box::new(t));
            } else {
                break;
            }
        }
        Ok(t)
    }
    pub fn parse(mut self) -> Result<Vec<Item>> {
        let mut items = Vec::new();
        while self.token().kind != Kind::Eof {
            let sp = self.span();
            if self.eat("graph") {
                if let Kind::Str(path) = self.token().kind.clone() {
                    self.pos += 1;
                    self.need(";")?;
                    items.push(Item::Import(path, sp));
                    continue;
                }
                let name = self.ident()?;
                if self.eat(";") {
                    items.push(Item::Import(name, sp));
                    continue;
                }
                self.need("(")?;
                let param = self.ident()?;
                self.need(")")?;
                self.need("=")?;
                self.graph_expr = true;
                let expr = self.expr(0)?;
                self.graph_expr = false;
                self.need(";")?;
                let body = Stmt {
                    kind: StmtKind::Return(Some(expr)),
                    span: sp.clone(),
                };
                items.push(Item::Function(Function {
                    name,
                    ret: Type::Any,
                    params: vec![(param, Type::Float)],
                    body,
                    span: sp,
                    graph: true,
                }));
            } else if self.eat("object") || self.eat("struct") {
                let name = self.ident()?;
                self.types.insert(name.clone());
                self.need("{")?;
                let mut fields = Vec::new();
                while !self.eat("}") {
                    let t = self.ty()?;
                    let n = self.ident()?;
                    self.need(";")?;
                    fields.push((n, t));
                }
                self.need(";")?;
                items.push(Item::Struct(name, fields, sp));
            } else if self.eat("addaterm") {
                let name = self.ident()?;
                if self.is_type() && !self.peek(1, "(") {
                    let t = self.ty()?;
                    self.need(";")?;
                    self.types.insert(name.clone());
                    items.push(Item::Alias(name, t, sp));
                } else {
                    self.eat("=");
                    let e = self.expr(0)?;
                    self.need(";")?;
                    items.push(Item::ValueAlias(name, e, sp));
                }
            } else if self.eat("lore") {
                let name = self.ident()?;
                self.types.insert(name.clone());
                items.push(Item::Alias(name, Type::Int, sp.clone()));
                self.need("{")?;
                let mut next = Some(0i64);
                while !self.eat("}") {
                    let s = self.span();
                    let name = self.ident()?;
                    let n = if self.eat("=") {
                        if let Kind::Int(i) = self.token().kind {
                            self.pos += 1;
                            i
                        } else {
                            return Err(Diagnostic::new(
                                "JOLE42",
                                &self.span(),
                                "lore values must be integer literals",
                            ));
                        }
                    } else {
                        next.ok_or_else(|| Diagnostic::new("AURA_OVERFLOW", &s, "lore overflow"))?
                    };
                    items.push(Item::ValueAlias(
                        name,
                        Expr {
                            kind: ExprKind::Int(n),
                            span: s.clone(),
                        },
                        s,
                    ));
                    next = n.checked_add(1);
                    if !self.eat(",") {
                        self.need("}")?;
                        break;
                    }
                }
                self.need(";")?;
            } else if self.is("domain") || self.is("range") {
                items.push(Item::Global(self.stmt()?));
            } else {
                let locked = self.eat("locked") || self.eat("const");
                let ty = self.ty()?;
                let name = self.ident()?;
                if self.eat("(") {
                    if locked {
                        return Err(Diagnostic::new(
                            "JOLE42",
                            &sp,
                            "functions cannot use locked",
                        ));
                    }
                    let mut params = Vec::new();
                    if !self.eat(")") {
                        if self.is("void") && self.peek(1, ")") {
                            self.pos += 2;
                        } else {
                            loop {
                                let t = self.ty()?;
                                let n = self.ident()?;
                                params.push((n, t));
                                if self.eat(")") {
                                    break;
                                }
                                self.need(",")?;
                            }
                        }
                    }
                    let body = self.stmt()?;
                    if !matches!(body.kind, StmtKind::Block(_)) {
                        return Err(Diagnostic::new(
                            "JOLE42",
                            &sp,
                            "function body requires braces",
                        ));
                    }
                    items.push(Item::Function(Function {
                        name,
                        ret: ty,
                        params,
                        body,
                        span: sp,
                        graph: false,
                    }));
                } else {
                    items.push(Item::Global(self.decl_tail(name, ty, locked, sp)?));
                }
            }
        }
        Ok(items)
    }
    fn decl_tail(&mut self, name: String, mut ty: Type, locked: bool, span: Span) -> Result<Stmt> {
        let mut size = None;
        if self.eat("[") {
            ty = Type::Array(Box::new(ty));
            if !self.is("]") {
                size = Some(self.expr(0)?);
            }
            self.need("]")?;
        }
        let value = if self.eat("=") {
            Some(self.expr(0)?)
        } else {
            None
        };
        self.need(";")?;
        Ok(Stmt {
            kind: StmtKind::Var {
                name,
                ty,
                value,
                size,
                locked,
            },
            span,
        })
    }
    fn stmt(&mut self) -> Result<Stmt> {
        self.depth += 1;
        if self.depth > 256 {
            return Err(Diagnostic::new(
                "LIMIT",
                &self.span(),
                "syntax nesting exceeds 256",
            ));
        }
        let result = self.stmt_inner();
        self.depth -= 1;
        result
    }
    fn stmt_inner(&mut self) -> Result<Stmt> {
        let span = self.span();
        let kind = if self.eat("{") {
            let mut ss = Vec::new();
            while !self.eat("}") {
                if self.token().kind == Kind::Eof {
                    return Err(Diagnostic::new("JOLE42", &span, "unclosed block"));
                }
                ss.push(self.stmt()?);
            }
            StmtKind::Block(ss)
        } else if self.eat(";") {
            StmtKind::Empty
        } else if self.eat("check") || self.eat("if") {
            self.need("(")?;
            let e = self.expr(0)?;
            self.need(")")?;
            let t = Box::new(self.stmt()?);
            let f = if self.eat("otherwise") || self.eat("else") {
                Some(Box::new(self.stmt()?))
            } else {
                None
            };
            StmtKind::If(e, t, f)
        } else if self.eat("ticker") || self.eat("while") {
            self.need("(")?;
            let e = self.expr(0)?;
            self.need(")")?;
            StmtKind::While(e, Box::new(self.stmt()?))
        } else if self.eat("attempt") || self.eat("for") {
            self.need("(")?;
            let init = if self.eat(";") {
                None
            } else {
                Some(Box::new(self.stmt()?))
            };
            let cond = if self.eat(";") {
                None
            } else {
                let e = self.expr(0)?;
                self.need(";")?;
                Some(e)
            };
            let step = if self.eat(")") {
                None
            } else {
                let e = self.expr(0)?;
                self.need(")")?;
                Some(e)
            };
            StmtKind::For(init, cond, step, Box::new(self.stmt()?))
        } else if self.eat("verify") || self.eat("return") {
            StmtKind::Return(if self.eat(";") {
                None
            } else {
                let e = self.expr(0)?;
                self.need(";")?;
                Some(e)
            })
        } else if self.eat("crash") || self.eat("break") {
            self.need(";")?;
            StmtKind::Break
        } else if self.eat("noclip") || self.eat("continue") {
            self.need(";")?;
            StmtKind::Continue
        } else if self.is("stream") && self.peek(1, "{") {
            self.pos += 1;
            StmtKind::Stream(Box::new(self.stmt()?))
        } else if self.is("domain") || self.is("range") {
            let range = self.eat("range");
            if !range {
                self.need("domain")?;
            }
            let n = self.ident()?;
            self.need("[")?;
            let a = self.expr(0)?;
            self.need(",")?;
            let b = self.expr(0)?;
            self.need("]")?;
            self.need(";")?;
            StmtKind::Bounds(n, a, b, range)
        } else if self.eat("plot") {
            let name = self.ident()?;
            let path = if self.eat("to") {
                Some(self.expr(0)?)
            } else {
                None
            };
            self.need(";")?;
            StmtKind::Plot(name, path)
        } else if self.eat("trigger") || self.eat("switch") {
            self.need("(")?;
            let e = self.expr(0)?;
            self.need(")")?;
            self.need("{")?;
            let mut arms = Vec::new();
            while !self.eat("}") {
                let label = if self.eat("checkpoint") || self.eat("case") {
                    Some(self.expr(0)?)
                } else {
                    if !self.eat("practice") {
                        self.need("default")?;
                    }
                    None
                };
                self.need(":")?;
                let mut body = Vec::new();
                while !["checkpoint", "case", "practice", "default", "}"]
                    .iter()
                    .any(|s| self.is(s))
                {
                    body.push(self.stmt()?);
                }
                arms.push((label, body));
            }
            StmtKind::Switch(e, arms)
        } else if self.is("locked") || self.is("const") || (self.is_type() && !self.peek(1, "(")) {
            let locked = self.eat("locked") || self.eat("const");
            let t = self.ty()?;
            let name = self.ident()?;
            return self.decl_tail(name, t, locked, span);
        } else {
            let e = self.expr(0)?;
            self.need(";")?;
            StmtKind::Expr(e)
        };
        Ok(Stmt { kind, span })
    }
    fn expr(&mut self, min: u8) -> Result<Expr> {
        self.depth += 1;
        if self.depth > 256 {
            return Err(Diagnostic::new(
                "LIMIT",
                &self.span(),
                "expression nesting exceeds 256",
            ));
        }
        let result = self.expr_inner(min);
        self.depth -= 1;
        result
    }
    fn expr_inner(&mut self, min: u8) -> Result<Expr> {
        let span = self.span();
        let token = self.token().clone();
        self.pos += 1;
        let kind = match token.kind {
            Kind::Int(v) => ExprKind::Int(v),
            Kind::UInt(v) => ExprKind::UInt(v),
            Kind::Float(v) => ExprKind::Float(v),
            Kind::Str(s) => ExprKind::String(s),
            Kind::Char(c) => ExprKind::Char(c),
            Kind::Ident(s) if s == "verified" || s == "true" => ExprKind::Bool(true),
            Kind::Ident(s) if s == "unverified" || s == "false" => ExprKind::Bool(false),
            Kind::Ident(s) if s == "cooked" || s == "NULL" => ExprKind::Null,
            Kind::Ident(n) => {
                if self.types.contains(&n) && self.eat("{") {
                    let mut fields = Vec::new();
                    while !self.eat("}") {
                        let f = self.ident()?;
                        self.need(":")?;
                        fields.push((f, self.expr(0)?));
                        if !self.eat(",") {
                            self.need("}")?;
                            break;
                        }
                    }
                    ExprKind::Record(n, fields)
                } else {
                    ExprKind::Var(n)
                }
            }
            Kind::Sym(s) if s == "(" => {
                let e = self.expr(0)?;
                if self.eat(",") {
                    let y = self.expr(0)?;
                    self.need(")")?;
                    ExprKind::Point(Box::new(e), Box::new(y))
                } else {
                    self.need(")")?;
                    e.kind
                }
            }
            Kind::Sym(s) if s == "[" => {
                let mut es = Vec::new();
                while !self.eat("]") {
                    es.push(self.expr(0)?);
                    if !self.eat(",") {
                        self.need("]")?;
                        break;
                    }
                }
                ExprKind::Array(es)
            }
            Kind::Sym(s) if ["-", "+", "!", "&", "*", "++", "--"].contains(&s.as_str()) => {
                let e = Box::new(self.expr(if s == "-" || s == "+" { 9 } else { 10 })?);
                if s == "++" || s == "--" {
                    ExprKind::Update(e, if s == "++" { 1 } else { -1 }, true)
                } else {
                    ExprKind::Unary(s, e)
                }
            }
            _ => {
                return Err(Diagnostic::new(
                    "JOLE42",
                    &span,
                    format!("expected expression, received `{}`", token.raw),
                ));
            }
        };
        let mut lhs = Expr { kind, span };
        loop {
            let sp = lhs.span.clone();
            if min <= 10 {
                if self.eat("(") {
                    let mut args = Vec::new();
                    while !self.eat(")") {
                        args.push(self.expr(0)?);
                        if !self.eat(",") {
                            self.need(")")?;
                            break;
                        }
                    }
                    lhs = Expr {
                        kind: ExprKind::Call(Box::new(lhs), args),
                        span: sp,
                    };
                    continue;
                }
                if self.eat("[") {
                    let i = self.expr(0)?;
                    self.need("]")?;
                    lhs = Expr {
                        kind: ExprKind::Index(Box::new(lhs), Box::new(i)),
                        span: sp,
                    };
                    continue;
                }
                if self.is(".") || self.is("->") {
                    let ptr = self.eat("->");
                    if !ptr {
                        self.need(".")?;
                    }
                    if ptr {
                        lhs = Expr {
                            kind: ExprKind::Unary("*".into(), Box::new(lhs)),
                            span: sp.clone(),
                        };
                    }
                    let f = self.ident()?;
                    lhs = Expr {
                        kind: ExprKind::Field(Box::new(lhs), f),
                        span: sp,
                    };
                    continue;
                }
                if self.is("++") || self.is("--") {
                    let delta = if self.eat("++") {
                        1
                    } else {
                        self.pos += 1;
                        -1
                    };
                    lhs = Expr {
                        kind: ExprKind::Update(Box::new(lhs), delta, false),
                        span: sp,
                    };
                    continue;
                }
            }
            if min <= 2 && self.eat("?") {
                let t = self.expr(0)?;
                self.need(":")?;
                let f = self.expr(2)?;
                lhs = Expr {
                    kind: ExprKind::Conditional(Box::new(lhs), Box::new(t), Box::new(f)),
                    span: sp,
                };
                continue;
            }
            let (op, prec, right) = match &self.token().kind {
                Kind::Sym(s) => match s.as_str() {
                    "=" | "+=" | "-=" | "*=" | "/=" | "%=" => (s.clone(), 1, true),
                    "||" => (s.clone(), 3, false),
                    "&&" => (s.clone(), 4, false),
                    "==" | "!=" => (s.clone(), 5, false),
                    "<" | ">" | "<=" | ">=" => (s.clone(), 6, false),
                    "+" | "-" => (s.clone(), 7, false),
                    "*" | "/" | "%" => (s.clone(), 8, false),
                    "^" => (s.clone(), 9, true),
                    _ => break,
                },
                Kind::Ident(_)
                    if self.graph_expr
                        && matches!(
                            lhs.kind,
                            ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::UInt(_)
                        ) =>
                {
                    ("implicit*".into(), 8, false)
                }
                _ => break,
            };
            if prec < min {
                break;
            }
            if op != "implicit*" {
                self.pos += 1;
            }
            let rhs = self.expr(if right { prec } else { prec + 1 })?;
            let kind = if prec == 1 {
                ExprKind::Assign(Box::new(lhs), op, Box::new(rhs))
            } else {
                ExprKind::Binary(
                    Box::new(lhs),
                    if op == "implicit*" { "*".into() } else { op },
                    Box::new(rhs),
                )
            };
            lhs = Expr { kind, span: sp };
        }
        Ok(lhs)
    }
}
