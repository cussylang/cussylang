use crate::{
    ast::*,
    checker::{Checked, builtin_types, resolve_type},
    diagnostic::{Diagnostic, Result},
    value::*,
};
use std::{
    collections::{HashMap, HashSet},
    io::Write,
    time::Instant,
};
#[derive(Clone, Debug)]
pub struct Options {
    pub fuel: u64,
    pub echo: bool,
    pub allow_ffi: bool,
    pub args: Vec<String>,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            fuel: 5_000_000,
            echo: false,
            allow_ffi: false,
            args: Vec::new(),
        }
    }
}
#[derive(Clone)]
enum Flow {
    Next,
    Return(Value),
    Break,
    Continue,
}
pub struct Runtime {
    pub checked: Checked,
    pub scopes: Vec<HashMap<String, Cell>>,
    pub output: String,
    pub options: Options,
    pub started: Instant,
    pub rng: u64,
    pub stream_depth: usize,
    pub hopped: bool,
    pub bounds: HashMap<String, (f64, f64)>,
    pub ranges: HashMap<String, (f64, f64)>,
    depth: usize,
    // Keep caller frames allocated during nested calls, but exclude their locals
    // from name lookup so functions retain lexical (rather than dynamic) scope.
    frame_start: usize,
}
impl Runtime {
    pub fn new(checked: Checked, options: Options) -> Self {
        let mut globals = HashMap::new();
        for (n, t) in builtin_types() {
            globals.insert(n.clone(), cell(Value::Function(n), t, false));
        }
        for (n, f) in &checked.functions {
            globals.insert(
                n.clone(),
                cell(
                    Value::Function(n.clone()),
                    Type::Callable(
                        f.params.iter().map(|(_, t)| t.clone()).collect(),
                        Box::new(f.ret.clone()),
                        false,
                    ),
                    false,
                ),
            );
        }
        Self {
            checked,
            scopes: vec![globals],
            output: String::new(),
            options,
            started: Instant::now(),
            rng: 0xC055_1E42,
            stream_depth: 0,
            hopped: false,
            bounds: HashMap::new(),
            ranges: HashMap::new(),
            depth: 0,
            frame_start: 1,
        }
    }
    pub fn initialize(&mut self, p: &Program) -> Result<()> {
        for item in &p.items {
            match item {
                Item::Global(s) => {
                    self.exec(s)?;
                }
                Item::ValueAlias(n, e, _) => {
                    let v = self.eval(e)?;
                    let t = v.ty();
                    self.scopes[0].insert(n.clone(), cell(v, t, false));
                }
                _ => {}
            }
        }
        Ok(())
    }
    pub fn run(&mut self, p: &Program) -> Result<i64> {
        self.initialize(p)?;
        let n = if self.checked.functions.contains_key("whitecap") {
            "whitecap"
        } else {
            "main"
        };
        let s = self.checked.functions[n].span.clone();
        match self.call(n, vec![], &s)? {
            Value::Int(n) => Ok(n),
            Value::Void => Ok(0),
            _ => Err(Diagnostic::new(
                "AURA-12",
                &s,
                "entry point did not return int or void",
            )),
        }
    }
    pub fn tick(&mut self, s: &Span) -> Result<()> {
        if self.options.fuel == 0 {
            return Err(Diagnostic::new("LIMIT", s, "execution budget exhausted")
                .help("fix the loop or increase --fuel N"));
        }
        self.options.fuel -= 1;
        Ok(())
    }
    pub fn emit(&mut self, text: &str, s: &Span) -> Result<()> {
        if self.output.len() + text.len() > 8 * 1024 * 1024 {
            return Err(Diagnostic::new("LIMIT", s, "captured output exceeds 8 MiB"));
        }
        self.output.push_str(text);
        if self.options.echo {
            let mut out = std::io::stdout().lock();
            out.write_all(text.as_bytes())
                .and_then(|_| out.flush())
                .map_err(|e| Diagnostic::new("IO", s, e.to_string()))?;
        }
        Ok(())
    }
    fn resolve(&self, t: &Type, s: &Span) -> Result<Type> {
        resolve_type(t, &self.checked, s, &mut HashSet::new())
    }
    pub fn coerce(&self, v: Value, t: &Type, s: &Span) -> Result<Value> {
        let t = self.resolve(t, s)?;
        Ok(match (&t, v) {
            (Type::Float, v) if v.ty().numeric() => Value::Float(v.number(s)?),
            (Type::Int, Value::Bool(b)) => Value::Int(i64::from(b)),
            (Type::UInt, Value::Int(n)) => Value::UInt(u64::try_from(n).map_err(|_| {
                Diagnostic::new(
                    "AURA_OVERFLOW",
                    s,
                    "negative value cannot initialize unsigned",
                )
            })?),
            (Type::Ptr(t), Value::Pointer(None, _)) => Value::Pointer(None, *t.clone()),
            (Type::Vector, Value::Point(x, y, _)) => Value::Point(x, y, true),
            (Type::Array(t), Value::Array(a, _)) => {
                let mut cells = Vec::new();
                for c in a {
                    let v = self.coerce(c.borrow().value.copy(), t, s)?;
                    cells.push(cell(v, *t.clone(), true));
                }
                Value::Array(cells, *t.clone())
            }
            (Type::List, Value::Array(a, _)) | (Type::List, Value::List(a)) => {
                let mut cells = Vec::new();
                for c in a {
                    cells.push(cell(
                        Value::Float(c.borrow().value.number(s)?),
                        Type::Float,
                        true,
                    ));
                }
                Value::List(cells)
            }
            (_, v)
                if t == v.ty()
                    || t == Type::Any
                    || matches!((&t, &v), (Type::Callable(..), Value::Function(_))) =>
            {
                v
            }
            (_, v) => {
                return Err(Diagnostic::new(
                    "AURA-12",
                    s,
                    format!("cannot store {} in {t}", v.ty()),
                ));
            }
        })
    }
    fn default(&self, t: &Type, s: &Span) -> Result<Value> {
        Ok(match self.resolve(t, s)? {
            Type::Int => Value::Int(0),
            Type::UInt => Value::UInt(0),
            Type::Float => Value::Float(0.),
            Type::Bool => Value::Bool(false),
            Type::Char => Value::Char('\0'),
            Type::String => Value::String(String::new()),
            Type::Void => Value::Void,
            Type::Ptr(t) => Value::Pointer(None, *t),
            Type::Array(t) => Value::Array(vec![], *t),
            Type::List => Value::List(vec![]),
            Type::Point | Type::Vector => Value::Point(
                cell(Value::Float(0.), Type::Float, true),
                cell(Value::Float(0.), Type::Float, true),
                *t == Type::Vector,
            ),
            Type::Named(n) => {
                let mut m = std::collections::BTreeMap::new();
                for (f, t) in &self.checked.structs[&n] {
                    let ty = self.resolve(t, s)?;
                    m.insert(f.clone(), cell(self.default(&ty, s)?, ty, true));
                }
                Value::Record(n, m)
            }
            _ => return Err(Diagnostic::new("AURA-12", s, "type has no default value")),
        })
    }
    fn lookup(&self, n: &str, s: &Span) -> Result<Cell> {
        self.scopes[self.frame_start..]
            .iter()
            .rev()
            .find_map(|scope| scope.get(n).cloned())
            .or_else(|| self.scopes[0].get(n).cloned())
            .ok_or_else(|| Diagnostic::new("D404", s, format!("undefined identifier `{n}`")))
    }
    fn read(&self, c: &Cell, s: &Span) -> Result<Value> {
        let c = c.borrow();
        if !c.alive {
            return Err(Diagnostic::new(
                "COOKED",
                s,
                "read from freed storage or an expired scope",
            ));
        }
        Ok(c.value.clone())
    }
    fn leave(&mut self) {
        if let Some(scope) = self.scopes.pop() {
            for c in scope.values() {
                invalidate(c);
            }
        }
    }
    fn scoped(&mut self, s: &Stmt) -> Result<Flow> {
        self.scopes.push(HashMap::new());
        let result = self.exec(s);
        self.leave();
        result
    }
    fn exec(&mut self, s: &Stmt) -> Result<Flow> {
        self.tick(&s.span)?;
        match &s.kind {
            StmtKind::Block(ss) => {
                self.scopes.push(HashMap::new());
                let r = (|| {
                    for s in ss {
                        let f = self.exec(s)?;
                        if !matches!(f, Flow::Next) {
                            return Ok(f);
                        }
                    }
                    Ok(Flow::Next)
                })();
                self.leave();
                r
            }
            StmtKind::Var {
                name,
                ty,
                value,
                size,
                locked,
            } => {
                let ty = self.resolve(ty, &s.span)?;
                let size = if let Some(size) = size {
                    let n = self.eval(size)?.integer(&size.span)?;
                    if !(0..=1_000_000).contains(&n) {
                        return Err(Diagnostic::new(
                            "DOMAIN",
                            &size.span,
                            "array size must be 0..1000000",
                        ));
                    }
                    Some(n as usize)
                } else {
                    None
                };
                let v = if let Some(e) = value {
                    self.eval(e)?.copy()
                } else if let (Type::Array(inner), Some(size)) = (&ty, size) {
                    let mut a = Vec::new();
                    for _ in 0..size {
                        a.push(cell(self.default(inner, &s.span)?, *inner.clone(), true));
                    }
                    Value::Array(a, *inner.clone())
                } else {
                    self.default(&ty, &s.span)?
                };
                let v = self.coerce(v, &ty, &s.span)?;
                if let (Some(n), Value::Array(a, _)) = (size, &v)
                    && n != a.len()
                {
                    return Err(Diagnostic::new(
                        "DOMAIN",
                        &s.span,
                        format!(
                            "array declares {n} elements but initializer has {}",
                            a.len()
                        ),
                    ));
                }
                self.scopes
                    .last_mut()
                    .unwrap()
                    .insert(name.clone(), cell(v, ty, !locked));
                Ok(Flow::Next)
            }
            StmtKind::Expr(e) => {
                self.eval(e)?;
                Ok(Flow::Next)
            }
            StmtKind::If(e, t, f) => {
                if self.eval(e)?.truth() {
                    self.scoped(t)
                } else if let Some(f) = f {
                    self.scoped(f)
                } else {
                    Ok(Flow::Next)
                }
            }
            StmtKind::While(e, b) => {
                while self.eval(e)?.truth() {
                    match self.scoped(b)? {
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        _ => {}
                    }
                }
                Ok(Flow::Next)
            }
            StmtKind::For(i, c, step, b) => {
                self.scopes.push(HashMap::new());
                let result = (|| {
                    if let Some(i) = i {
                        self.exec(i)?;
                    }
                    loop {
                        self.tick(&s.span)?;
                        if let Some(c) = c
                            && !self.eval(c)?.truth()
                        {
                            break;
                        }
                        match self.scoped(b)? {
                            Flow::Break => break,
                            Flow::Return(v) => return Ok(Flow::Return(v)),
                            _ => {}
                        }
                        if let Some(step) = step {
                            self.eval(step)?;
                        }
                    }
                    Ok(Flow::Next)
                })();
                self.leave();
                result
            }
            StmtKind::Return(e) => Ok(Flow::Return(if let Some(e) = e {
                self.eval(e)?.copy()
            } else {
                Value::Void
            })),
            StmtKind::Break => Ok(Flow::Break),
            StmtKind::Continue => Ok(Flow::Continue),
            StmtKind::Stream(b) => {
                self.stream_depth += 1;
                let r = self.scoped(b);
                self.stream_depth -= 1;
                r
            }
            StmtKind::Bounds(n, a, b, range) => {
                let a = self.eval(a)?.number(&a.span)?;
                let b = self.eval(b)?.number(&b.span)?;
                if !a.is_finite() || !b.is_finite() || a >= b {
                    return Err(Diagnostic::new(
                        "DOMAIN",
                        &s.span,
                        "bounds require finite min < max",
                    ));
                }
                if *range {
                    self.ranges.insert(n.clone(), (a, b));
                } else {
                    self.bounds.insert(n.clone(), (a, b));
                }
                Ok(Flow::Next)
            }
            StmtKind::Plot(n, path) => {
                let path = if let Some(p) = path {
                    self.eval(p)?.string(&p.span)?
                } else {
                    format!("{n}.svg")
                };
                self.plot(n, &path, &s.span)?;
                Ok(Flow::Next)
            }
            StmtKind::Switch(e, arms) => {
                let v = self.eval(e)?;
                let mut selected = None;
                let mut default = None;
                for (i, (label, _)) in arms.iter().enumerate() {
                    if let Some(label) = label {
                        if v.same(&self.eval(label)?) {
                            selected = Some(i);
                            break;
                        }
                    } else {
                        default = Some(i);
                    }
                }
                if let Some(i) = selected.or(default) {
                    self.scopes.push(HashMap::new());
                    let r = (|| {
                        for s in &arms[i].1 {
                            match self.exec(s)? {
                                Flow::Break => return Ok(Flow::Next),
                                Flow::Next => {}
                                f => return Ok(f),
                            }
                        }
                        Ok(Flow::Next)
                    })();
                    self.leave();
                    r
                } else {
                    Ok(Flow::Next)
                }
            }
            StmtKind::Empty => Ok(Flow::Next),
        }
    }
    fn lvalue(&mut self, e: &Expr) -> Result<Cell> {
        let c = match &e.kind {
            ExprKind::Var(n) => self.lookup(n, &e.span)?,
            ExprKind::Unary(op, p) if op == "*" => match self.eval(p)? {
                Value::Pointer(Some(c), _) => c,
                Value::Pointer(None, _) => {
                    return Err(Diagnostic::new(
                        "COOKED",
                        &e.span,
                        "cannot dereference cooked (null)",
                    ));
                }
                _ => return Err(Diagnostic::new("COOKED", &e.span, "not a pointer")),
            },
            ExprKind::Index(base, i) => {
                let base = self.eval(base)?;
                let i = self.eval(i)?.integer(&i.span)?;
                match base {
                    Value::Array(a, _) | Value::List(a) => {
                        if i < 0 {
                            return Err(Diagnostic::new("DOMAIN", &e.span, "negative index"));
                        }
                        a.get(i as usize).cloned().ok_or_else(|| {
                            Diagnostic::new(
                                "DOMAIN",
                                &e.span,
                                format!("index {i} outside array length {}", a.len()),
                            )
                        })?
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            "AURA-12",
                            &e.span,
                            "index is not assignable",
                        ));
                    }
                }
            }
            ExprKind::Field(base, f) => match self.eval(base)? {
                Value::Record(_, fields) => fields
                    .get(f)
                    .cloned()
                    .ok_or_else(|| Diagnostic::new("D404", &e.span, "missing field"))?,
                Value::Point(x, y, _) => {
                    if f == "x" {
                        x
                    } else {
                        y
                    }
                }
                _ => return Err(Diagnostic::new("D404", &e.span, "value has no fields")),
            },
            _ => {
                return Err(Diagnostic::new(
                    "AURA-12",
                    &e.span,
                    "expression is not assignable",
                ));
            }
        };
        // Validation must not clone the entire value, especially for aggregates.
        if !c.borrow().alive {
            return Err(Diagnostic::new(
                "COOKED",
                &e.span,
                "read from freed storage or an expired scope",
            ));
        }
        Ok(c)
    }
    pub fn eval(&mut self, e: &Expr) -> Result<Value> {
        self.tick(&e.span)?;
        use ExprKind::*;
        Ok(match &e.kind {
            Int(v) => Value::Int(*v),
            UInt(v) => Value::UInt(*v),
            Float(v) => Value::Float(*v),
            Bool(v) => Value::Bool(*v),
            Char(v) => Value::Char(*v),
            String(v) => Value::String(v.clone()),
            Null => Value::Pointer(None, Type::Void),
            Var(n) => {
                let c = self.lookup(n, &e.span)?;
                self.read(&c, &e.span)?
            }
            Array(es) => {
                let mut a = Vec::new();
                let mut ty = Type::Any;
                for e in es {
                    let v = self.eval(e)?.copy();
                    let t = v.ty();
                    if ty == Type::Any || t == Type::Float {
                        ty = t.clone();
                    }
                    a.push(cell(v, t, true));
                }
                let v = Value::Array(a, ty.clone());
                self.coerce(v, &Type::Array(Box::new(ty)), &e.span)?
            }
            Point(a, b) => {
                let a = self.eval(a)?.number(&a.span)?;
                let b = self.eval(b)?.number(&b.span)?;
                Value::Point(
                    cell(Value::Float(a), Type::Float, true),
                    cell(Value::Float(b), Type::Float, true),
                    false,
                )
            }
            Record(n, fields) => {
                let ty = self.resolve(&Type::Named(n.clone()), &e.span)?;
                let v = self.default(&ty, &e.span)?;
                if let Value::Record(_, ref map) = v {
                    for (f, e) in fields {
                        let c = &map[f];
                        let value = self.eval(e)?.copy();
                        let value = self.coerce(value, &c.borrow().ty, &e.span)?;
                        write_cell(c, value, &e.span)?;
                    }
                }
                v
            }
            Unary(op, v) => match op.as_str() {
                "&" => {
                    let c = self.lvalue(v)?;
                    let ty = c.borrow().ty.clone();
                    Value::Pointer(Some(c), ty)
                }
                "*" => {
                    let c = self.lvalue(e)?;
                    self.read(&c, &e.span)?
                }
                "!" => Value::Bool(!self.eval(v)?.truth()),
                "+" => self.eval(v)?,
                "-" => {
                    let v = self.eval(v)?;
                    match v {
                        Value::Int(n) => Value::Int(n.checked_neg().ok_or_else(|| {
                            Diagnostic::new("AURA_OVERFLOW", &e.span, "integer negation overflow")
                        })?),
                        Value::UInt(0) => Value::UInt(0),
                        Value::UInt(_) => {
                            return Err(Diagnostic::new(
                                "AURA_OVERFLOW",
                                &e.span,
                                "cannot negate unsigned value",
                            ));
                        }
                        Value::Float(n) => Value::Float(-n),
                        Value::List(_) | Value::Point(..) => {
                            self.binary(v, "*", Value::Float(-1.), &e.span)?
                        }
                        _ => return Err(Diagnostic::new("AURA-12", &e.span, "invalid negation")),
                    }
                }
                _ => unreachable!(),
            },
            Binary(a, op, b) => {
                let a = self.eval(a)?;
                if op == "&&" && !a.truth() {
                    Value::Bool(false)
                } else if op == "||" && a.truth() {
                    Value::Bool(true)
                } else {
                    let b = self.eval(b)?;
                    self.binary(a, op, b, &e.span)?
                }
            }
            Assign(a, op, b) => {
                let c = self.lvalue(a)?;
                let rhs = self.eval(b)?.copy();
                let value = if op == "=" {
                    rhs
                } else {
                    self.binary(self.read(&c, &e.span)?, &op[..1], rhs, &e.span)?
                };
                let value = self.coerce(value, &c.borrow().ty, &e.span)?;
                write_cell(&c, value.copy(), &e.span)?;
                value
            }
            Update(a, delta, prefix) => {
                let c = self.lvalue(a)?;
                let old = self.read(&c, &e.span)?;
                let (op, one) = match old {
                    Value::UInt(_) => (if *delta > 0 { "+" } else { "-" }, Value::UInt(1)),
                    _ => ("+", Value::Int(*delta)),
                };
                let next = self.binary(old.clone(), op, one, &e.span)?;
                write_cell(&c, next.clone(), &e.span)?;
                if *prefix { next } else { old }
            }
            Call(f, args) => {
                let f = self.eval(f)?;
                let mut values = Vec::with_capacity(args.len());
                for a in args {
                    values.push(self.eval(a)?.copy());
                }
                if let Value::Function(n) = f {
                    self.call(&n, values, &e.span)?
                } else {
                    return Err(Diagnostic::new("AURA-12", &e.span, "value is not callable"));
                }
            }
            Index(a, i) => {
                let a = self.eval(a)?;
                let i = self.eval(i)?.integer(&i.span)?;
                if i < 0 {
                    return Err(Diagnostic::new("DOMAIN", &e.span, "negative index"));
                }
                match a {
                    Value::Array(a, _) | Value::List(a) => {
                        let c = a.get(i as usize).ok_or_else(|| {
                            Diagnostic::new(
                                "DOMAIN",
                                &e.span,
                                format!("index {i} outside length {}", a.len()),
                            )
                        })?;
                        self.read(c, &e.span)?
                    }
                    Value::String(s) => {
                        Value::Char(s.chars().nth(i as usize).ok_or_else(|| {
                            Diagnostic::new("DOMAIN", &e.span, "string index out of bounds")
                        })?)
                    }
                    _ => return Err(Diagnostic::new("AURA-12", &e.span, "not indexable")),
                }
            }
            Field(a, f) => match self.eval(a)? {
                Value::Record(_, m) => self.read(
                    m.get(f)
                        .ok_or_else(|| Diagnostic::new("D404", &e.span, "missing field"))?,
                    &e.span,
                )?,
                Value::Point(x, y, _) => self.read(if f == "x" { &x } else { &y }, &e.span)?,
                _ => return Err(Diagnostic::new("D404", &e.span, "value has no fields")),
            },
            Conditional(c, t, f) => {
                if self.eval(c)?.truth() {
                    self.eval(t)?
                } else {
                    self.eval(f)?
                }
            }
        })
    }
    pub fn call(&mut self, n: &str, args: Vec<Value>, span: &Span) -> Result<Value> {
        self.tick(span)?;
        if let Some(f) = self.checked.functions.get(n).cloned() {
            if self.depth >= 128 {
                return Err(Diagnostic::new("LIMIT", span, "call depth exceeds 128"));
            }
            if args.len() != f.params.len() {
                return Err(Diagnostic::new("AURA-12", span, "argument count mismatch"));
            }
            let mut params = HashMap::with_capacity(f.params.len());
            for ((n, t), v) in f.params.iter().zip(args) {
                let ty = self.resolve(t, span)?;
                let v = self.coerce(v, &ty, span)?;
                params.insert(n.clone(), cell(v, ty, true));
            }
            let saved_frame = std::mem::replace(&mut self.frame_start, self.scopes.len());
            self.scopes.push(params);
            self.depth += 1;
            let result = self.exec(&f.body);
            self.leave();
            self.frame_start = saved_frame;
            self.depth -= 1;
            match result? {
                Flow::Return(v) => self.coerce(v, &f.ret, span),
                Flow::Next if f.ret == Type::Void => Ok(Value::Void),
                _ => Err(Diagnostic::new(
                    "WASHED",
                    span,
                    "function exited without verify",
                )),
            }
        } else {
            self.native(n, args, span)
        }
    }
    pub fn binary(&mut self, a: Value, op: &str, b: Value, s: &Span) -> Result<Value> {
        if op == "==" || op == "!=" {
            return Ok(Value::Bool(a.same(&b) == (op == "==")));
        }
        if op == "&&" || op == "||" {
            return Ok(Value::Bool(if op == "&&" {
                a.truth() && b.truth()
            } else {
                a.truth() || b.truth()
            }));
        }
        // Mixed signed/unsigned operations are checked, never C-style wrapping.
        let (a, b) = match (a, b) {
            (Value::Int(a), Value::UInt(b)) => (
                Value::UInt(u64::try_from(a).map_err(|_| {
                    Diagnostic::new(
                        "AURA_OVERFLOW",
                        s,
                        "negative operand in unsigned arithmetic",
                    )
                })?),
                Value::UInt(b),
            ),
            (Value::UInt(a), Value::Int(b)) => (
                Value::UInt(a),
                Value::UInt(u64::try_from(b).map_err(|_| {
                    Diagnostic::new(
                        "AURA_OVERFLOW",
                        s,
                        "negative operand in unsigned arithmetic",
                    )
                })?),
            ),
            pair => pair,
        };
        if let (Value::String(a), Value::String(b)) = (&a, &b)
            && op == "+"
        {
            if a.len() + b.len() > 8 * 1024 * 1024 {
                return Err(Diagnostic::new("LIMIT", s, "string exceeds 8 MiB"));
            }
            return Ok(Value::String(format!("{a}{b}")));
        }
        if matches!(a, Value::List(_)) || matches!(b, Value::List(_)) {
            let aa = if let Value::List(v) = &a {
                Some(v)
            } else {
                None
            };
            let bb = if let Value::List(v) = &b {
                Some(v)
            } else {
                None
            };
            if let (Some(a), Some(b)) = (aa, bb)
                && a.len() != b.len()
            {
                return Err(Diagnostic::new(
                    "DOMAIN",
                    s,
                    "list arithmetic requires equal lengths",
                ));
            }
            let len = aa.or(bb).unwrap().len();
            let mut out = Vec::new();
            for i in 0..len {
                self.tick(s)?;
                let av = aa.map_or_else(|| a.clone(), |v| v[i].borrow().value.clone());
                let bv = bb.map_or_else(|| b.clone(), |v| v[i].borrow().value.clone());
                let v = self.binary(av, op, bv, s)?;
                out.push(cell(v, Type::Float, true));
            }
            return Ok(Value::List(out));
        }
        if let Value::Point(x, y, vector) = &a {
            let (bx, by) = if let Value::Point(x, y, _) = &b {
                (x.borrow().value.clone(), y.borrow().value.clone())
            } else {
                (b.clone(), b.clone())
            };
            let x = self.binary(x.borrow().value.clone(), op, bx, s)?;
            let y = self.binary(y.borrow().value.clone(), op, by, s)?;
            return Ok(Value::Point(
                cell(x, Type::Float, true),
                cell(y, Type::Float, true),
                *vector,
            ));
        }
        let overflow =
            || Diagnostic::new("AURA_OVERFLOW", s, "integer overflow or division by zero");
        if let (Value::Int(a), Value::Int(b)) = (&a, &b) {
            let n = match op {
                "+" => a.checked_add(*b),
                "-" => a.checked_sub(*b),
                "*" => a.checked_mul(*b),
                "/" => a.checked_div(*b),
                "%" => a.checked_rem(*b),
                "<" => return Ok(Value::Bool(a < b)),
                ">" => return Ok(Value::Bool(a > b)),
                "<=" => return Ok(Value::Bool(a <= b)),
                ">=" => return Ok(Value::Bool(a >= b)),
                _ => None,
            };
            if op != "^" {
                return n.map(Value::Int).ok_or_else(overflow);
            }
        }
        if let (Value::UInt(a), Value::UInt(b)) = (&a, &b) {
            let n = match op {
                "+" => a.checked_add(*b),
                "-" => a.checked_sub(*b),
                "*" => a.checked_mul(*b),
                "/" => a.checked_div(*b),
                "%" => a.checked_rem(*b),
                "<" => return Ok(Value::Bool(a < b)),
                ">" => return Ok(Value::Bool(a > b)),
                "<=" => return Ok(Value::Bool(a <= b)),
                ">=" => return Ok(Value::Bool(a >= b)),
                _ => None,
            };
            if op != "^" {
                return n.map(Value::UInt).ok_or_else(overflow);
            }
        }
        let a = a.number(s)?;
        let b = b.number(s)?;
        let v = match op {
            "+" => a + b,
            "-" => a - b,
            "*" => a * b,
            "/" if b != 0. => a / b,
            "%" if b != 0. => a % b,
            "^" => a.powf(b),
            "<" => return Ok(Value::Bool(a < b)),
            ">" => return Ok(Value::Bool(a > b)),
            "<=" => return Ok(Value::Bool(a <= b)),
            ">=" => return Ok(Value::Bool(a >= b)),
            _ => {
                return Err(Diagnostic::new(
                    "DOMAIN",
                    s,
                    "invalid operation or division by zero",
                ));
            }
        };
        if !v.is_finite() {
            return Err(Diagnostic::new(
                "DOMAIN",
                s,
                "arithmetic produced a non-finite result",
            ));
        }
        Ok(Value::Float(v))
    }
}
