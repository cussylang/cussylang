//! Checked scalar C emission. Generated programs contain no interpreter or AST.
use crate::{
    ast::*,
    checker::{Checked, builtin_types, resolve_type},
    diagnostic::{Diagnostic, Result},
};
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
struct Symbol {
    ty: Type,
    name: String,
    function: Option<String>,
    ready: Option<String>,
}
#[derive(Clone)]
struct Value {
    ty: Type,
    code: String,
    function: Option<String>,
}
impl Value {
    fn new(ty: Type, code: impl Into<String>) -> Self {
        Self {
            ty,
            code: code.into(),
            function: None,
        }
    }
}
struct Emitter<'a> {
    checked: &'a Checked,
    scopes: Vec<HashMap<String, Symbol>>,
    serial: usize,
    ret: Type,
    breaks: Vec<String>,
    continues: Vec<String>,
}

/// Emit standalone GNU C11 for the supported, statically checked scalar subset.
/// Unsupported language operations produce a source-located diagnostic rather
/// than embedding the interpreter or changing their behavior silently.
pub fn emit(program: &Program, checked: &Checked) -> Result<String> {
    Emitter::new(checked).program(program)
}

fn unsupported(span: &Span, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new("NATIVE_UNSUPPORTED", span, message)
        .help("use `cussy run` or `cussy build` for the full interpreter language")
}
fn c_text(text: &str) -> String {
    let mut out = String::from("\"");
    for byte in text.bytes() {
        match byte {
            b'"' => out.push_str("\\\""),
            b'\\' => out.push_str("\\\\"),
            b'?' => out.push_str("\\?"),
            32..=126 => out.push(char::from(byte)),
            _ => out.push_str(&format!("\\{byte:03o}")),
        }
    }
    out.push('"');
    out
}
fn location(span: &Span) -> String {
    format!(
        "(cx_loc){{{}, {}, {}}}",
        c_text(&span.file),
        span.line,
        span.col
    )
}
fn string_value(text: &str) -> Value {
    Value::new(
        Type::String,
        format!("(cx_str){{{}, {}}}", c_text(text), text.len()),
    )
}
fn constant_string(e: &Expr) -> Option<String> {
    match &e.kind {
        ExprKind::String(text) => Some(text.clone()),
        ExprKind::Binary(a, op, b) if op == "+" => {
            let mut a = constant_string(a)?;
            a.push_str(&constant_string(b)?);
            Some(a)
        }
        _ => None,
    }
}
fn c_type(ty: &Type, span: &Span) -> Result<&'static str> {
    Ok(match ty {
        Type::Int => "int64_t",
        Type::UInt => "uint64_t",
        Type::Float => "double",
        Type::Bool => "bool",
        Type::Char => "uint32_t",
        Type::String => "cx_str",
        Type::Void => "void",
        _ => {
            return Err(unsupported(
                span,
                format!("native compilation does not support type `{ty}`"),
            ));
        }
    })
}
fn truth(value: &Value, span: &Span) -> Result<String> {
    if value.ty.numeric() || value.ty == Type::Bool {
        Ok(format!("(({}) != 0)", value.code))
    } else {
        Err(unsupported(
            span,
            "native conditions require a numeric or boolean value",
        ))
    }
}
fn zero(ty: &Type, span: &Span) -> Result<String> {
    c_type(ty, span)?;
    Ok(if *ty == Type::String {
        "(cx_str){\"\", 0}"
    } else {
        "0"
    }
    .into())
}
fn block_expression(prefix: &str, value: Value) -> Value {
    Value {
        code: format!("({{ {prefix} {}; }})", value.code),
        ..value
    }
}

impl<'a> Emitter<'a> {
    fn new(checked: &'a Checked) -> Self {
        let mut scope = HashMap::new();
        for (name, ty) in builtin_types() {
            scope.insert(
                name.clone(),
                Symbol {
                    ty,
                    name: "0".into(),
                    function: Some(name),
                    ready: None,
                },
            );
        }
        Self {
            checked,
            scopes: vec![scope],
            serial: 0,
            ret: Type::Void,
            breaks: vec![],
            continues: vec![],
        }
    }
    fn fresh(&mut self, kind: &str) -> String {
        let id = self.serial;
        self.serial += 1;
        format!("cx_{kind}_{id}")
    }
    fn resolve(&self, ty: &Type, span: &Span) -> Result<Type> {
        resolve_type(ty, self.checked, span, &mut HashSet::new())
    }
    fn symbol(&self, name: &str, span: &Span) -> Result<Symbol> {
        self.scopes
            .iter()
            .rev()
            .find_map(|s| s.get(name).cloned())
            .ok_or_else(|| unsupported(span, format!("native identifier `{name}` is unavailable")))
    }
    fn snapshot(&mut self, value: Value, prefix: &mut String, span: &Span) -> Result<Value> {
        if value.ty == Type::Void || value.function.is_some() {
            prefix.push_str(&format!("(void)({}); ", value.code));
            return Ok(Value {
                code: "0".into(),
                ..value
            });
        }
        let name = self.fresh("t");
        prefix.push_str(&format!(
            "{} {name} = {}; ",
            c_type(&value.ty, span)?,
            value.code
        ));
        Ok(Value {
            code: name,
            ..value
        })
    }
    fn coerce(&self, value: &Value, want: &Type, span: &Span) -> Result<String> {
        let want = self.resolve(want, span)?;
        if want == value.ty {
            Ok(value.code.clone())
        } else {
            match (&want, &value.ty) {
                (Type::Float, ty) if ty.numeric() => Ok(format!("((double)({}))", value.code)),
                (Type::Int, Type::Bool) => Ok(format!("((int64_t)({}))", value.code)),
                (Type::UInt, Type::Int) => {
                    Ok(format!("cx_unsigned({}, {})", value.code, location(span)))
                }
                _ => Err(unsupported(
                    span,
                    format!(
                        "native conversion from {} to {want} is not supported",
                        value.ty
                    ),
                )),
            }
        }
    }
    fn program(mut self, program: &Program) -> Result<String> {
        let mut declarations = String::new();
        for item in &program.items {
            match item {
                Item::Function(original) => {
                    let f = &self.checked.functions[&original.name];
                    let ret = self.resolve(&f.ret, &f.span)?;
                    c_type(&ret, &f.span)?;
                    let mut params = vec![];
                    for (_, ty) in &f.params {
                        let ty = self.resolve(ty, &f.span)?;
                        c_type(&ty, &f.span)?;
                        params.push(ty);
                    }
                    let name = self.fresh("fn");
                    self.scopes[0].insert(
                        f.name.clone(),
                        Symbol {
                            ty: Type::Callable(params.clone(), Box::new(ret.clone()), false),
                            name: name.clone(),
                            function: Some(f.name.clone()),
                            ready: None,
                        },
                    );
                    let mut signature = params
                        .iter()
                        .map(|t| c_type(t, &f.span).map(str::to_owned))
                        .collect::<Result<Vec<_>>>()?;
                    signature.push("cx_loc".into());
                    declarations.push_str(&format!(
                        "static {} {name}({});\n",
                        c_type(&ret, &f.span)?,
                        signature.join(", ")
                    ));
                }
                Item::Struct(_, _, span) => {
                    return Err(unsupported(
                        span,
                        "native compilation does not yet support object declarations",
                    ));
                }
                _ => {}
            }
        }
        let mut initialize = String::new();
        for item in &program.items {
            match item {
                Item::Global(statement) => {
                    if let StmtKind::Var {
                        name,
                        ty,
                        value,
                        size,
                        ..
                    } = &statement.kind
                    {
                        if size.is_some() {
                            return Err(unsupported(
                                &statement.span,
                                "native arrays are not supported",
                            ));
                        }
                        let ty = self.resolve(ty, &statement.span)?;
                        let cty = c_type(&ty, &statement.span)?;
                        let value = if let Some(value) = value {
                            let value = self.expr(value)?;
                            self.coerce(&value, &ty, &statement.span)?
                        } else {
                            zero(&ty, &statement.span)?
                        };
                        let storage = self.fresh("global");
                        let ready = self.fresh("ready");
                        declarations
                            .push_str(&format!("static {cty} {storage};\nstatic bool {ready};\n"));
                        initialize.push_str(&format!("{storage} = {value}; {ready} = true;\n"));
                        self.scopes[0].insert(
                            name.clone(),
                            Symbol {
                                ty,
                                name: storage,
                                function: None,
                                ready: Some(ready),
                            },
                        );
                    } else {
                        return Err(unsupported(
                            &statement.span,
                            "native compilation supports only scalar global declarations",
                        ));
                    }
                }
                Item::ValueAlias(name, expr, span) => {
                    let value = self.expr(expr)?;
                    let ready = self.fresh("ready");
                    declarations.push_str(&format!("static bool {ready};\n"));
                    let storage = if value.function.is_some() || value.ty == Type::Void {
                        initialize.push_str(&format!("(void)({}); {ready} = true;\n", value.code));
                        "0".into()
                    } else {
                        let storage = self.fresh("alias");
                        declarations
                            .push_str(&format!("static {} {storage};\n", c_type(&value.ty, span)?));
                        initialize
                            .push_str(&format!("{storage} = {}; {ready} = true;\n", value.code));
                        storage
                    };
                    self.scopes[0].insert(
                        name.clone(),
                        Symbol {
                            ty: value.ty,
                            name: storage,
                            function: value.function,
                            ready: Some(ready),
                        },
                    );
                }
                _ => {}
            }
        }
        let mut functions = String::new();
        for item in &program.items {
            if let Item::Function(original) = item {
                let f = self.checked.functions[&original.name].clone();
                self.ret = self.resolve(&f.ret, &f.span)?;
                let symbol = self.symbol(&f.name, &f.span)?;
                self.scopes.push(HashMap::new());
                let mut params = vec![];
                for (name, ty) in &f.params {
                    let ty = self.resolve(ty, &f.span)?;
                    let storage = self.fresh("param");
                    params.push(format!("{} {storage}", c_type(&ty, &f.span)?));
                    self.scopes.last_mut().unwrap().insert(
                        name.clone(),
                        Symbol {
                            ty,
                            name: storage,
                            function: None,
                            ready: None,
                        },
                    );
                }
                params.push("cx_loc cx_call".into());
                let body = self.stmt(&f.body)?;
                self.scopes.pop();
                let finish = if self.ret == Type::Void {
                    "--cx_depth; return;".to_owned()
                } else {
                    "cx_fail(\"WASHED\", \"function exited without verify\", cx_call);".into()
                };
                functions.push_str(&format!("static {} {}({}) {{\ncx_check_depth(cx_call); ++cx_depth;\n{body}\n{finish}\n}}\n", c_type(&self.ret, &f.span)?, symbol.name, params.join(", ")));
            }
        }
        let entry = self
            .checked
            .functions
            .get("whitecap")
            .or_else(|| self.checked.functions.get("main"))
            .ok_or_else(|| {
                unsupported(
                    &Span::default(),
                    "native executable requires whitecap() or main()",
                )
            })?;
        let symbol = self.symbol(&entry.name, &entry.span)?;
        let entry_ret = self.resolve(&entry.ret, &entry.span)?;
        if !entry.params.is_empty() || !matches!(entry_ret, Type::Void | Type::Int) {
            return Err(unsupported(
                &entry.span,
                "native entry must take no arguments and return int or void",
            ));
        }
        let call = format!("{}({})", symbol.name, location(&entry.span));
        let finish = if entry_ret == Type::Void {
            format!("{call}; return 0;")
        } else {
            format!("return (int)((uint64_t){call} & UINT64_C(255));")
        };
        Ok(format!(
            "/* Generated by Cussy {}. Compile as GNU C11; do not enable fast-math.\n{}\n*/\n{}\n{}\n{declarations}\n{functions}\nint main(int argc, char **argv) {{\ncx_setup_output();\ncx_setup_args(argc, argv, {});\n{initialize}\n{finish}\n}}\n",
            crate::VERSION,
            include_str!("../LICENSE"),
            include_str!("codegen_float.h"),
            include_str!("codegen_runtime.h"),
            location(&entry.span),
        ))
    }
    fn scoped(&mut self, stmt: &Stmt) -> Result<String> {
        self.scopes.push(HashMap::new());
        let result = self.stmt(stmt);
        self.scopes.pop();
        Ok(format!("{{\n{}\n}}", result?))
    }
    fn stmt(&mut self, stmt: &Stmt) -> Result<String> {
        let span = &stmt.span;
        Ok(match &stmt.kind {
            StmtKind::Block(statements) => {
                self.scopes.push(HashMap::new());
                let mut out = String::from("{\n");
                for statement in statements {
                    out.push_str(&self.stmt(statement)?);
                    out.push('\n');
                }
                self.scopes.pop();
                out.push_str("}\n");
                out
            }
            StmtKind::Var {
                name,
                ty,
                value,
                size,
                ..
            } => {
                if size.is_some() {
                    return Err(unsupported(span, "native arrays are not supported"));
                }
                let ty = self.resolve(ty, span)?;
                let value = if let Some(value) = value {
                    let value = self.expr(value)?;
                    self.coerce(&value, &ty, span)?
                } else {
                    zero(&ty, span)?
                };
                let storage = self.fresh("local");
                let out = format!("{} {storage} = {value};", c_type(&ty, span)?);
                self.scopes.last_mut().unwrap().insert(
                    name.clone(),
                    Symbol {
                        ty,
                        name: storage,
                        function: None,
                        ready: None,
                    },
                );
                out
            }
            StmtKind::Expr(expr) => format!("(void)({});", self.expr(expr)?.code),
            StmtKind::If(condition, yes, no) => {
                let condition = truth(&self.expr(condition)?, &condition.span)?;
                let yes = self.scoped(yes)?;
                let no = if let Some(no) = no {
                    format!(" else {}", self.scoped(no)?)
                } else {
                    String::new()
                };
                format!("if ({condition}) {yes}{no}")
            }
            StmtKind::While(condition, body) => {
                let start = self.fresh("loop");
                let end = self.fresh("end");
                let condition = truth(&self.expr(condition)?, &condition.span)?;
                self.breaks.push(end.clone());
                self.continues.push(start.clone());
                let body = self.scoped(body)?;
                self.breaks.pop();
                self.continues.pop();
                format!(
                    "{{ {start}:; if (!({condition})) goto {end}; {body}\ngoto {start}; {end}:; }}"
                )
            }
            StmtKind::For(init, condition, step, body) => {
                self.scopes.push(HashMap::new());
                let init = if let Some(init) = init {
                    self.stmt(init)?
                } else {
                    String::new()
                };
                let condition = if let Some(condition) = condition {
                    truth(&self.expr(condition)?, &condition.span)?
                } else {
                    "true".into()
                };
                let start = self.fresh("loop");
                let next = self.fresh("next");
                let end = self.fresh("end");
                self.breaks.push(end.clone());
                self.continues.push(next.clone());
                let body = self.scoped(body)?;
                self.breaks.pop();
                self.continues.pop();
                let step = if let Some(step) = step {
                    format!("(void)({});", self.expr(step)?.code)
                } else {
                    String::new()
                };
                self.scopes.pop();
                format!(
                    "{{ {init}\n{start}:; if (!({condition})) goto {end}; {body}\n{next}:; {step} goto {start}; {end}:; }}"
                )
            }
            StmtKind::Return(value) => {
                if let Some(value) = value {
                    let value = self.expr(value)?;
                    let code = self.coerce(&value, &self.ret, span)?;
                    if self.ret == Type::Void {
                        format!("(void)({code}); --cx_depth; return;")
                    } else {
                        let name = self.fresh("return");
                        format!(
                            "{{ {} {name} = {code}; --cx_depth; return {name}; }}",
                            c_type(&self.ret, span)?
                        )
                    }
                } else {
                    "--cx_depth; return;".into()
                }
            }
            StmtKind::Break => format!(
                "goto {};",
                self.breaks
                    .last()
                    .ok_or_else(|| unsupported(span, "break outside native loop/switch"))?
            ),
            StmtKind::Continue => format!(
                "goto {};",
                self.continues
                    .last()
                    .ok_or_else(|| unsupported(span, "continue outside native loop"))?
            ),
            StmtKind::Switch(expr, arms) => {
                let mut prefix = String::new();
                let value = self.expr(expr)?;
                let value = self.snapshot(value, &mut prefix, span)?;
                let end = self.fresh("switch_end");
                self.breaks.push(end.clone());
                let mut branches = String::new();
                let mut fallback = String::new();
                for (label, body) in arms {
                    self.scopes.push(HashMap::new());
                    let mut body_code = String::new();
                    for statement in body {
                        body_code.push_str(&self.stmt(statement)?);
                        body_code.push('\n');
                    }
                    self.scopes.pop();
                    if let Some(label) = label {
                        let label = self.expr(label)?;
                        let test = self.operation(&value, "==", &label, span)?;
                        branches.push_str(&format!(
                            "if ({}) {{ {body_code} goto {end}; }}\n",
                            test.code
                        ));
                    } else {
                        fallback = format!("{{ {body_code} }}");
                    }
                }
                self.breaks.pop();
                format!("{{ {prefix}\n{branches}\n{fallback}\n{end}:; }}")
            }
            StmtKind::Empty => ";".into(),
            StmtKind::Stream(_) => {
                return Err(unsupported(
                    span,
                    "stream blocks are not yet supported by native compilation",
                ));
            }
            StmtKind::Bounds(..) | StmtKind::Plot(..) => {
                return Err(unsupported(
                    span,
                    "native graph bounds and SVG plotting are not supported",
                ));
            }
        })
    }
    fn expr(&mut self, expr: &Expr) -> Result<Value> {
        let span = &expr.span;
        let loc = location(span);
        Ok(match &expr.kind {
            ExprKind::Int(value) => Value::new(
                Type::Int,
                if *value == i64::MIN {
                    "INT64_MIN".into()
                } else if *value < 0 {
                    format!("(-INT64_C({}))", -value)
                } else {
                    format!("INT64_C({value})")
                },
            ),
            ExprKind::UInt(value) => Value::new(Type::UInt, format!("UINT64_C({value})")),
            ExprKind::Float(value) => {
                let bits = value.to_bits();
                let sign = if bits >> 63 == 1 { "-" } else { "" };
                let exponent = (bits >> 52) & 0x7ff;
                let fraction = bits & ((1u64 << 52) - 1);
                let code = if value.is_nan() {
                    "NAN".into()
                } else if value.is_infinite() {
                    format!("{sign}INFINITY")
                } else if exponent == 0 {
                    format!("{sign}0x0.{fraction:013x}p-1022")
                } else {
                    format!("{sign}0x1.{fraction:013x}p{}", exponent as i32 - 1023)
                };
                Value::new(Type::Float, code)
            }
            ExprKind::Bool(value) => Value::new(Type::Bool, if *value { "true" } else { "false" }),
            ExprKind::Char(value) => Value::new(Type::Char, format!("UINT32_C({})", *value as u32)),
            ExprKind::String(value) => string_value(value),
            ExprKind::Var(name) => {
                let symbol = self.symbol(name, span)?;
                let code = if let Some(ready) = &symbol.ready {
                    format!("({{ cx_ready({ready}, {loc}); {}; }})", symbol.name)
                } else {
                    symbol.name
                };
                Value {
                    ty: symbol.ty,
                    code,
                    function: symbol.function,
                }
            }
            ExprKind::Unary(op, value) => {
                if op == "&" || op == "*" {
                    return Err(unsupported(
                        span,
                        "native pointers and address/dereference operations are not supported",
                    ));
                }
                let value = self.expr(value)?;
                match op.as_str() {
                    "+" => value,
                    "!" => Value::new(Type::Bool, format!("(!{})", truth(&value, span)?)),
                    "-" => match value.ty {
                        Type::Int => {
                            Value::new(Type::Int, format!("cx_ineg({}, {loc})", value.code))
                        }
                        Type::UInt => {
                            Value::new(Type::UInt, format!("cx_uneg({}, {loc})", value.code))
                        }
                        Type::Float => Value::new(Type::Float, format!("(-({}))", value.code)),
                        _ => {
                            return Err(unsupported(
                                span,
                                "native negation requires a scalar number",
                            ));
                        }
                    },
                    _ => {
                        return Err(unsupported(
                            span,
                            format!("unsupported native unary operator `{op}`"),
                        ));
                    }
                }
            }
            ExprKind::Binary(a, op, b) => {
                if let Some(text) = constant_string(expr) {
                    if text.len() > 8 * 1024 * 1024 {
                        return Err(unsupported(
                            span,
                            "constant string concatenation exceeds 8 MiB",
                        ));
                    }
                    return Ok(string_value(&text));
                }
                let a = self.expr(a)?;
                let b = self.expr(b)?;
                if op == "&&" || op == "||" {
                    Value::new(
                        Type::Bool,
                        format!("({} {op} {})", truth(&a, span)?, truth(&b, span)?),
                    )
                } else {
                    let mut prefix = String::new();
                    let a = self.snapshot(a, &mut prefix, span)?;
                    let b = self.snapshot(b, &mut prefix, span)?;
                    block_expression(&prefix, self.operation(&a, op, &b, span)?)
                }
            }
            ExprKind::Assign(lhs, op, rhs) => {
                let ExprKind::Var(name) = &lhs.kind else {
                    return Err(unsupported(
                        span,
                        "native assignment supports scalar variables only",
                    ));
                };
                let symbol = self.symbol(name, span)?;
                let mut prefix = if let Some(ready) = &symbol.ready {
                    format!("cx_ready({ready}, {loc}); ")
                } else {
                    String::new()
                };
                let rhs = self.expr(rhs)?;
                let rhs = self.snapshot(rhs, &mut prefix, span)?;
                let value = if op == "=" {
                    rhs
                } else {
                    self.operation(
                        &Value::new(symbol.ty.clone(), symbol.name.clone()),
                        &op[..1],
                        &rhs,
                        span,
                    )?
                };
                let code = self.coerce(&value, &symbol.ty, span)?;
                prefix.push_str(&format!("{} = {code};", symbol.name));
                block_expression(&prefix, Value::new(symbol.ty, symbol.name))
            }
            ExprKind::Update(expr, delta, before) => {
                let ExprKind::Var(name) = &expr.kind else {
                    return Err(unsupported(
                        span,
                        "native updates support scalar variables only",
                    ));
                };
                let symbol = self.symbol(name, span)?;
                let mut prefix = if let Some(ready) = &symbol.ready {
                    format!("cx_ready({ready}, {loc}); ")
                } else {
                    String::new()
                };
                let old = self.snapshot(
                    Value::new(symbol.ty.clone(), symbol.name.clone()),
                    &mut prefix,
                    span,
                )?;
                let (op, one) = if symbol.ty == Type::UInt {
                    (
                        if *delta > 0 { "+" } else { "-" },
                        Value::new(Type::UInt, "UINT64_C(1)"),
                    )
                } else {
                    ("+", Value::new(Type::Int, format!("INT64_C({delta})")))
                };
                let next = self.operation(&old, op, &one, span)?;
                prefix.push_str(&format!("{} = {}; ", symbol.name, next.code));
                block_expression(
                    &prefix,
                    if *before {
                        Value::new(symbol.ty, symbol.name)
                    } else {
                        old
                    },
                )
            }
            ExprKind::Call(function, args) => self.call(function, args, span)?,
            ExprKind::Index(value, index) => {
                let value = self.expr(value)?;
                if value.ty != Type::String {
                    return Err(unsupported(span, "native indexing supports strings only"));
                }
                let index = self.expr(index)?;
                if index.ty != Type::Int {
                    return Err(unsupported(span, "native string indices require int"));
                }
                let mut prefix = String::new();
                let value = self.snapshot(value, &mut prefix, span)?;
                let index = self.snapshot(index, &mut prefix, span)?;
                block_expression(
                    &prefix,
                    Value::new(
                        Type::Char,
                        format!("cx_string_index({}, {}, {loc})", value.code, index.code),
                    ),
                )
            }
            ExprKind::Conditional(condition, yes, no) => {
                let condition = self.expr(condition)?;
                let yes = self.expr(yes)?;
                let no = self.expr(no)?;
                if yes.ty != no.ty || yes.function.is_some() || no.function.is_some() {
                    return Err(unsupported(
                        span,
                        "native conditional branches must have the same scalar type; mixed branch types retain dynamic types in the interpreter",
                    ));
                }
                c_type(&yes.ty, span)?;
                Value::new(
                    yes.ty,
                    format!(
                        "({} ? ({}) : ({}))",
                        truth(&condition, span)?,
                        yes.code,
                        no.code
                    ),
                )
            }
            ExprKind::Null
            | ExprKind::Array(_)
            | ExprKind::Point(..)
            | ExprKind::Record(..)
            | ExprKind::Field(..) => {
                return Err(unsupported(
                    span,
                    "native compilation does not yet support pointers, arrays, lists, points, or objects",
                ));
            }
        })
    }
    fn operation(&self, a: &Value, op: &str, b: &Value, span: &Span) -> Result<Value> {
        let loc = location(span);
        let equality = if op == "==" || op == "!=" {
            let code = match (&a.ty, &b.ty) {
                (Type::String, Type::String) => format!("cx_string_equal({}, {})", a.code, b.code),
                (Type::Int, Type::UInt) => format!(
                    "(({}) >= 0 && (uint64_t)({}) == ({}))",
                    a.code, a.code, b.code
                ),
                (Type::UInt, Type::Int) => format!(
                    "(({}) >= 0 && ({}) == (uint64_t)({}))",
                    b.code, a.code, b.code
                ),
                (Type::Int, Type::Bool)
                | (Type::Bool, Type::Int)
                | (Type::Bool, Type::Bool)
                | (Type::Char, Type::Char) => format!("(({}) == ({}))", a.code, b.code),
                (Type::Void, Type::Void) => "true".into(),
                (Type::Callable(..), Type::Callable(..)) => {
                    if a.function == b.function {
                        "true".into()
                    } else {
                        "false".into()
                    }
                }
                (left, right) if left.numeric() && right.numeric() => {
                    if *left == Type::Float || *right == Type::Float {
                        format!("((double)({}) == (double)({}))", a.code, b.code)
                    } else {
                        format!("(({}) == ({}))", a.code, b.code)
                    }
                }
                _ => return Err(unsupported(span, "unsupported native equality operands")),
            };
            Some(if op == "!=" {
                format!("!({code})")
            } else {
                code
            })
        } else {
            None
        };
        if let Some(code) = equality {
            return Ok(Value::new(Type::Bool, code));
        }
        if !a.ty.numeric() || !b.ty.numeric() {
            return Err(unsupported(
                span,
                "native arithmetic requires scalar numbers; dynamic string concatenation is not supported",
            ));
        }
        let mixed_unsigned = matches!(
            (&a.ty, &b.ty),
            (Type::Int, Type::UInt) | (Type::UInt, Type::Int)
        );
        let acode = if mixed_unsigned && a.ty == Type::Int {
            format!("cx_unsigned({}, {loc})", a.code)
        } else {
            a.code.clone()
        };
        let bcode = if mixed_unsigned && b.ty == Type::Int {
            format!("cx_unsigned({}, {loc})", b.code)
        } else {
            b.code.clone()
        };
        let ty = if a.ty == Type::Float || b.ty == Type::Float || op == "^" {
            Type::Float
        } else if a.ty == Type::UInt || b.ty == Type::UInt {
            Type::UInt
        } else {
            Type::Int
        };
        if ["<", ">", "<=", ">="].contains(&op) {
            let (acode, bcode) = if ty == Type::Float {
                (format!("(double)({acode})"), format!("(double)({bcode})"))
            } else {
                (acode, bcode)
            };
            return Ok(Value::new(
                Type::Bool,
                format!("(({acode}) {op} ({bcode}))"),
            ));
        }
        let code = if ty == Type::Float {
            let (acode, bcode) = (format!("(double)({acode})"), format!("(double)({bcode})"));
            match op {
                "+" | "-" | "*" => format!("cx_finite(({acode}) {op} ({bcode}), {loc})"),
                "/" => format!("cx_fdiv({acode}, {bcode}, {loc})"),
                "%" => format!("cx_frem({acode}, {bcode}, {loc})"),
                "^" => format!("cx_finite(pow({acode}, {bcode}), {loc})"),
                _ => {
                    return Err(unsupported(
                        span,
                        format!("unsupported native numeric operator `{op}`"),
                    ));
                }
            }
        } else {
            let kind = if ty == Type::Int { "i" } else { "u" };
            let name = match op {
                "+" => "add",
                "-" => "sub",
                "*" => "mul",
                "/" => "div",
                "%" => "rem",
                _ => {
                    return Err(unsupported(
                        span,
                        format!("unsupported native numeric operator `{op}`"),
                    ));
                }
            };
            format!("cx_{kind}{name}({acode}, {bcode}, {loc})")
        };
        Ok(Value::new(ty, code))
    }
    fn boxed(&self, value: &Value, span: &Span) -> Result<String> {
        let (kind, member, code) = match &value.ty {
            Type::Int => ("INT", "i", value.code.clone()),
            Type::UInt => ("UINT", "u", value.code.clone()),
            Type::Float => ("FLOAT", "f", value.code.clone()),
            Type::Bool => ("BOOL", "b", value.code.clone()),
            Type::Char => ("CHAR", "c", value.code.clone()),
            Type::String => ("STRING", "s", value.code.clone()),
            Type::Void => return Ok("(cx_value){.kind = CX_VOID}".into()),
            Type::Callable(..) => {
                let name = value.function.as_ref().ok_or_else(|| {
                    unsupported(span, "native dynamic function values are not supported")
                })?;
                (
                    "FUNCTION",
                    "s",
                    string_value(&format!("<function {name}>")).code,
                )
            }
            _ => {
                return Err(unsupported(
                    span,
                    format!("cannot format native type {}", value.ty),
                ));
            }
        };
        Ok(format!(
            "(cx_value){{.kind = CX_{kind}, .as.{member} = {code}}}"
        ))
    }
    fn call(&mut self, function: &Expr, args: &[Expr], span: &Span) -> Result<Value> {
        let function = self.expr(function)?;
        let target = function
            .function
            .clone()
            .ok_or_else(|| unsupported(span, "native calls require a statically known function"))?;
        let mut prefix = format!("(void)({}); ", function.code);
        let mut values = Vec::with_capacity(args.len());
        for arg in args {
            let value = self.expr(arg)?;
            values.push(self.snapshot(value, &mut prefix, &arg.span)?);
        }
        let value = if let Some(definition) = self.checked.functions.get(&target) {
            let definition = definition.clone();
            prefix.push_str(&format!("cx_check_depth({}); ", location(span)));
            let mut params = vec![];
            for (value, (_, ty)) in values.iter().zip(&definition.params) {
                let ty = self.resolve(ty, span)?;
                let code = self.coerce(value, &ty, span)?;
                let value = self.snapshot(Value::new(ty, code), &mut prefix, span)?;
                params.push(value.code);
            }
            params.push(location(span));
            let name = &self.scopes[0][&target].name;
            Value::new(
                self.resolve(&definition.ret, span)?,
                format!("{name}({})", params.join(", ")),
            )
        } else {
            self.builtin(&target, &values, span)?
        };
        Ok(block_expression(&prefix, value))
    }
    fn builtin(&self, name: &str, values: &[Value], span: &Span) -> Result<Value> {
        let loc = location(span);
        let number = |value: &Value| -> Result<String> {
            if value.ty.numeric() {
                Ok(format!("((double)({}))", value.code))
            } else {
                Err(unsupported(
                    span,
                    "native numeric builtin requires numeric arguments",
                ))
            }
        };
        let array = |values: &[Value]| -> Result<String> {
            if values.is_empty() {
                Ok("NULL".into())
            } else {
                Ok(format!(
                    "(const cx_value[]){{{}}}",
                    values
                        .iter()
                        .map(|v| self.boxed(v, span))
                        .collect::<Result<Vec<_>>>()?
                        .join(", ")
                ))
            }
        };
        Ok(match name {
            "jole" => Value::new(
                Type::Void,
                format!("cx_jole({}, {}, {loc})", values.len(), array(values)?),
            ),
            "yap" => Value::new(
                Type::Void,
                format!(
                    "cx_yap({}, {}, {}, {loc})",
                    values[0].code,
                    values.len() - 1,
                    array(&values[1..])?
                ),
            ),
            "assert" => Value::new(
                Type::Void,
                format!(
                    "cx_assert({}, {}, {}, {loc})",
                    truth(&values[0], span)?,
                    values.len() > 1,
                    if values.len() > 1 {
                        self.boxed(&values[1], span)?
                    } else {
                        "(cx_value){.kind = CX_VOID}".into()
                    }
                ),
            ),
            "len" if values[0].ty == Type::String => Value::new(
                Type::Int,
                format!("((int64_t)cx_chars({}))", values[0].code),
            ),
            "hitbox" => Value::new(
                Type::Int,
                match values[0].ty {
                    Type::String => format!("((int64_t)({}).length)", values[0].code),
                    Type::Bool => "INT64_C(1)".into(),
                    Type::Char => "INT64_C(4)".into(),
                    Type::Void => "INT64_C(0)".into(),
                    _ => "INT64_C(8)".into(),
                },
            ),
            "to_int" => Value::new(
                Type::Int,
                match values[0].ty {
                    Type::Int => values[0].code.clone(),
                    Type::Bool | Type::Char => format!("((int64_t)({}))", values[0].code),
                    Type::UInt => format!("cx_uint_to_int({}, {loc})", values[0].code),
                    Type::Float => format!("cx_float_to_int({}, {loc})", values[0].code),
                    Type::String => format!("cx_parse_int({}, {loc})", values[0].code),
                    _ => {
                        return Err(unsupported(
                            span,
                            "native to_int does not support this type",
                        ));
                    }
                },
            ),
            "to_float" => Value::new(
                Type::Float,
                if values[0].ty == Type::String {
                    format!("cx_parse_float({}, {loc})", values[0].code)
                } else {
                    format!("cx_finite({}, {loc})", number(&values[0])?)
                },
            ),
            "to_string" if values[0].ty == Type::String => values[0].clone(),
            "__sin" | "__cos" | "__tan" | "__sqrt" | "__abs" | "__floor" | "__ceil" | "__round"
            | "__exp" | "__log" => {
                let function = if name == "__abs" { "fabs" } else { &name[2..] };
                Value::new(
                    Type::Float,
                    format!("cx_finite({function}({}), {loc})", number(&values[0])?),
                )
            }
            "__pow" | "__min" | "__max" | "__atan2" => {
                let function = match name {
                    "__min" => "cx_min",
                    "__max" => "cx_max",
                    _ => &name[2..],
                };
                Value::new(
                    Type::Float,
                    format!(
                        "cx_finite({function}({}, {}), {loc})",
                        number(&values[0])?,
                        number(&values[1])?
                    ),
                )
            }
            "__argc" => Value::new(Type::Int, "((int64_t)cx_arg_count)"),
            "__arg" => Value::new(
                Type::String,
                format!("cx_argument({}, {loc})", values[0].code),
            ),
            _ => {
                return Err(unsupported(
                    span,
                    format!("builtin `{name}` is not supported by native compilation"),
                ));
            }
        })
    }
}
