use crate::{
    ast::*,
    diagnostic::{Diagnostic, Result},
};
use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};
#[derive(Clone)]
pub struct Symbol {
    pub ty: Type,
    pub mutable: bool,
}
#[derive(Clone, Default)]
pub struct Checked {
    pub aliases: HashMap<String, Type>,
    pub structs: HashMap<String, Vec<(String, Type)>>,
    pub functions: HashMap<String, Rc<Function>>,
}
pub fn builtin_types() -> HashMap<String, Type> {
    use Type::*;
    let mut m = HashMap::new();
    let mut add = |name: &str, args: Vec<Type>, ret: Type, variadic: bool| {
        m.insert(name.into(), Callable(args, Box::new(ret), variadic));
    };
    add("jole", vec![], Void, true);
    add("yap", vec![String], Void, true);
    add("listen", vec![], String, false);
    add("assert", vec![Bool], Void, true);
    add("len", vec![Any], Int, false);
    add("hitbox", vec![Any], Int, false);
    add("to_string", vec![Any], String, false);
    add("to_int", vec![Any], Int, false);
    add("to_float", vec![Any], Float, false);
    add("alloc", vec![Any], Ptr(Box::new(Any)), false);
    add("free", vec![Any], Void, false);
    for n in [
        "sin", "cos", "tan", "sqrt", "abs", "floor", "ceil", "round", "exp", "log",
    ] {
        add(&format!("__{n}"), vec![Float], Float, false);
    }
    for n in ["pow", "min", "max", "atan2"] {
        add(&format!("__{n}"), vec![Float, Float], Float, false);
    }
    add("__clock", vec![], Float, false);
    add("__sleep", vec![Float], Void, false);
    add("__random", vec![], Float, false);
    add("__seed", vec![Int], Void, false);
    add("__read_file", vec![String], String, false);
    add("__write_file", vec![String, String], Void, false);
    add("__file_exists", vec![String], Bool, false);
    add("__env", vec![String], String, false);
    add("__arg", vec![Int], String, false);
    add("__argc", vec![], Int, false);
    add("__native1", vec![String, String, Float], Float, false);
    add("__slice", vec![String, Int, Int], String, false);
    add("__contains", vec![String, String], Bool, false);
    add("__replace", vec![String, String, String], String, false);
    add("__upper", vec![String], String, false);
    add("__lower", vec![String], String, false);
    add("__sum", vec![List], Float, false);
    add("__mean", vec![List], Float, false);
    add("__linspace", vec![Float, Float, Int], List, false);
    add(
        "__regress",
        vec![List, List],
        Named("Regression".into()),
        false,
    );
    add(
        "__plot_points",
        vec![Array(Box::new(Point)), String],
        Void,
        false,
    );
    add("__streamstatus", vec![], Bool, false);
    add("__hoponstream", vec![], Void, false);
    m
}
pub struct Checker {
    pub checked: Checked,
    scopes: Vec<HashMap<String, Symbol>>,
    ret: Type,
    loops: usize,
    switches: usize,
}
impl Checker {
    pub fn check(program: &Program, entry: bool) -> Result<Checked> {
        let globals = builtin_types()
            .into_iter()
            .map(|(n, ty)| (n, Symbol { ty, mutable: false }))
            .collect();
        let mut c = Self {
            checked: Checked::default(),
            scopes: vec![globals],
            ret: Type::Void,
            loops: 0,
            switches: 0,
        };
        let mut names = HashSet::new();
        for item in &program.items {
            match item {
                Item::Alias(n, t, s) => {
                    if !names.insert(n.clone()) {
                        return Err(Diagnostic::new(
                            "JOLE42",
                            s,
                            format!("duplicate type `{n}`"),
                        ));
                    }
                    c.checked.aliases.insert(n.clone(), t.clone());
                }
                Item::Struct(n, fields, s) => {
                    if !names.insert(n.clone()) {
                        return Err(Diagnostic::new(
                            "JOLE42",
                            s,
                            format!("duplicate type `{n}`"),
                        ));
                    }
                    let mut seen = HashSet::new();
                    for (f, _) in fields {
                        if !seen.insert(f) {
                            return Err(Diagnostic::new(
                                "JOLE42",
                                s,
                                format!("duplicate field `{f}`"),
                            ));
                        }
                    }
                    c.checked.structs.insert(n.clone(), fields.clone());
                }
                _ => {}
            }
        }
        for item in &program.items {
            match item {
                Item::Alias(_, t, s) => {
                    c.resolve(t, s)?;
                }
                Item::Struct(n, fields, s) => {
                    for (_, t) in fields {
                        c.valid_value_type(t, s)?;
                        c.finite_record(t, &mut vec![n.clone()], s)?;
                    }
                }
                _ => {}
            }
        }
        for item in &program.items {
            if let Item::Function(f) = item {
                let mut f = f.clone();
                // Graph expressions have an inferred scalar or point result; ordinary functions are explicit.
                if f.graph {
                    f.ret = if matches!(
                        &f.body.kind,
                        StmtKind::Return(Some(Expr {
                            kind: ExprKind::Point(..),
                            ..
                        }))
                    ) {
                        Type::Point
                    } else {
                        Type::Float
                    };
                }
                let ret = c.resolve(&f.ret, &f.span)?;
                let mut args = Vec::new();
                for (_, t) in &f.params {
                    c.valid_value_type(t, &f.span)?;
                    args.push(c.resolve(t, &f.span)?);
                }
                c.define(
                    &f.name,
                    Type::Callable(args, Box::new(ret), false),
                    false,
                    &f.span,
                )?;
                c.checked.functions.insert(f.name.clone(), Rc::new(f));
            }
        }
        for item in &program.items {
            match item {
                Item::Global(s) => {
                    c.stmt(s)?;
                }
                Item::ValueAlias(n, e, s) => {
                    let ty = c.expr(e)?;
                    c.define(n, ty, false, s)?;
                }
                _ => {}
            }
        }
        // Infer graph return types in declaration order, after constants are available.
        for item in &program.items {
            if let Item::Function(f) = item
                && f.graph
            {
                c.scopes.push(HashMap::new());
                for (n, t) in &f.params {
                    c.define(n, t.clone(), true, &f.span)?;
                }
                let ty = if let StmtKind::Return(Some(e)) = &f.body.kind {
                    c.expr(e)?
                } else {
                    Type::Float
                };
                c.scopes.pop();
                if !ty.numeric() && ty != Type::Point && ty != Type::Vector {
                    return Err(Diagnostic::new(
                        "AURA-12",
                        &f.span,
                        "graph must produce a number or point",
                    ));
                }
                let ty = if ty.numeric() { Type::Float } else { ty };
                Rc::make_mut(c.checked.functions.get_mut(&f.name).unwrap()).ret = ty.clone();
                c.scopes[0].get_mut(&f.name).unwrap().ty =
                    Type::Callable(vec![Type::Float], Box::new(ty), false);
            }
        }
        for item in &program.items {
            if let Item::Function(original) = item {
                let f = c.checked.functions[&original.name].clone();
                c.ret = c.resolve(&f.ret, &f.span)?;
                c.scopes.push(HashMap::new());
                for (n, t) in &f.params {
                    c.define(n, c.resolve(t, &f.span)?, true, &f.span)?;
                }
                let returns = c.stmt(&f.body)?;
                c.scopes.pop();
                if c.ret != Type::Void && !returns {
                    return Err(Diagnostic::new(
                        "WASHED",
                        &f.span,
                        format!("not every path in `{}` returns {}", f.name, c.ret),
                    )
                    .help("add verify on every branch, or use a void return type"));
                }
            }
        }
        if entry {
            let f = c
                .checked
                .functions
                .get("whitecap")
                .or_else(|| c.checked.functions.get("main"))
                .ok_or_else(|| {
                    Diagnostic::new(
                        "PAPA01",
                        &program.items.first().map(item_span).unwrap_or_default(),
                        "program requires int whitecap() or int main()",
                    )
                })?;
            let ret = c.resolve(&f.ret, &f.span)?;
            if !f.params.is_empty() || !matches!(ret, Type::Int | Type::Void) {
                return Err(Diagnostic::new(
                    "AURA-12",
                    &f.span,
                    "entry point must take no parameters and return int or void",
                ));
            }
        }
        Ok(c.checked)
    }
    pub fn resolve(&self, t: &Type, s: &Span) -> Result<Type> {
        resolve_type(t, &self.checked, s, &mut HashSet::new())
    }
    fn valid_value_type(&self, t: &Type, s: &Span) -> Result<()> {
        let r = self.resolve(t, s)?;
        if matches!(r, Type::Void | Type::Any | Type::Null) {
            return Err(Diagnostic::new(
                "AURA-12",
                s,
                format!("{r} is not a variable or field type"),
            ));
        }
        if let Type::Array(t) = r {
            self.valid_value_type(&t, s)?;
        }
        Ok(())
    }
    fn finite_record(&self, t: &Type, path: &mut Vec<String>, s: &Span) -> Result<()> {
        match self.resolve(t, s)? {
            Type::Named(n) => {
                if path.contains(&n) {
                    return Err(Diagnostic::new(
                        "AURA-12",
                        s,
                        "recursive object must use a pointer field",
                    ));
                }
                path.push(n.clone());
                for (_, t) in &self.checked.structs[&n] {
                    self.finite_record(t, path, s)?;
                }
                path.pop();
            }
            Type::Array(t) => self.finite_record(&t, path, s)?,
            _ => {}
        }
        Ok(())
    }
    fn define(&mut self, n: &str, t: Type, mutable: bool, s: &Span) -> Result<()> {
        let scope = self.scopes.last_mut().unwrap();
        if scope.contains_key(n) {
            return Err(Diagnostic::new(
                "JOLE42",
                s,
                format!("duplicate declaration `{n}`"),
            ));
        }
        scope.insert(n.into(), Symbol { ty: t, mutable });
        Ok(())
    }
    fn lookup(&self, n: &str, s: &Span) -> Result<Symbol> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(n))
            .cloned()
            .ok_or_else(|| {
                Diagnostic::new("D404", s, format!("undefined identifier `{n}`"))
                    .help("declare it or import its module with graph")
            })
    }
    fn expect(&self, want: &Type, got: &Type, s: &Span) -> Result<()> {
        if assignable(want, got) {
            Ok(())
        } else {
            Err(
                Diagnostic::new("AURA-12", s, format!("expected {want}, received {got}"))
                    .help("change the declared type or convert the value explicitly"),
            )
        }
    }
    fn condition(&mut self, e: &Expr) -> Result<()> {
        let t = self.expr(e)?;
        if t == Type::Bool || t.numeric() {
            Ok(())
        } else {
            Err(Diagnostic::new(
                "AURA-12",
                &e.span,
                "condition requires bool or number",
            ))
        }
    }
    fn scoped_stmt(&mut self, s: &Stmt) -> Result<bool> {
        self.scopes.push(HashMap::new());
        let r = self.stmt(s);
        self.scopes.pop();
        r
    }
    fn stmt(&mut self, s: &Stmt) -> Result<bool> {
        match &s.kind {
            StmtKind::Block(ss) => {
                self.scopes.push(HashMap::new());
                let mut returns = false;
                for s in ss {
                    returns |= self.stmt(s)?;
                }
                self.scopes.pop();
                Ok(returns)
            }
            StmtKind::Var {
                name,
                ty,
                value,
                size,
                locked,
            } => {
                self.valid_value_type(ty, &s.span)?;
                let ty = self.resolve(ty, &s.span)?;
                if let Some(size) = size {
                    let t = self.expr(size)?;
                    self.expect(&Type::Int, &t, &size.span)?;
                }
                if let Some(v) = value {
                    let t = self.expr(v)?;
                    self.expect(&ty, &t, &v.span)?;
                } else if *locked {
                    return Err(Diagnostic::new(
                        "LOCKED",
                        &s.span,
                        "locked variable requires an initializer",
                    ));
                }
                self.define(name, ty, !locked, &s.span)?;
                Ok(false)
            }
            StmtKind::Expr(e) => {
                self.expr(e)?;
                Ok(false)
            }
            StmtKind::If(e, t, f) => {
                self.condition(e)?;
                let a = self.scoped_stmt(t)?;
                let b = if let Some(f) = f {
                    self.scoped_stmt(f)?
                } else {
                    false
                };
                Ok(a && b)
            }
            StmtKind::While(e, b) => {
                self.condition(e)?;
                self.loops += 1;
                self.scoped_stmt(b)?;
                self.loops -= 1;
                Ok(false)
            }
            StmtKind::For(i, c, step, b) => {
                self.scopes.push(HashMap::new());
                if let Some(i) = i {
                    if !matches!(i.kind, StmtKind::Var { .. } | StmtKind::Expr(_)) {
                        return Err(Diagnostic::new(
                            "JOLE42",
                            &i.span,
                            "attempt initializer must be a variable or expression",
                        ));
                    }
                    self.stmt(i)?;
                }
                if let Some(c) = c {
                    self.condition(c)?;
                }
                if let Some(step) = step {
                    self.expr(step)?;
                }
                self.loops += 1;
                self.scoped_stmt(b)?;
                self.loops -= 1;
                self.scopes.pop();
                Ok(false)
            }
            StmtKind::Return(e) => {
                let got = if let Some(e) = e {
                    self.expr(e)?
                } else {
                    Type::Void
                };
                self.expect(&self.ret, &got, &s.span)?;
                Ok(true)
            }
            StmtKind::Break => {
                if self.loops + self.switches == 0 {
                    return Err(Diagnostic::new(
                        "JOLE42",
                        &s.span,
                        "crash requires a loop or trigger",
                    ));
                }
                Ok(false)
            }
            StmtKind::Continue => {
                if self.loops == 0 {
                    return Err(Diagnostic::new("JOLE42", &s.span, "noclip requires a loop"));
                }
                Ok(false)
            }
            StmtKind::Stream(b) => self.scoped_stmt(b),
            StmtKind::Bounds(n, a, b, _) => {
                self.graph_name(n, &s.span)?;
                for e in [a, b] {
                    let t = self.expr(e)?;
                    self.expect(&Type::Float, &t, &e.span)?;
                }
                Ok(false)
            }
            StmtKind::Plot(n, p) => {
                self.graph_name(n, &s.span)?;
                if let Some(p) = p {
                    let t = self.expr(p)?;
                    self.expect(&Type::String, &t, &p.span)?;
                }
                Ok(false)
            }
            StmtKind::Switch(e, arms) => {
                let ty = self.expr(e)?;
                if !matches!(
                    ty,
                    Type::Int | Type::UInt | Type::Char | Type::String | Type::Bool
                ) {
                    return Err(Diagnostic::new(
                        "AURA-12",
                        &e.span,
                        "trigger requires int, unsigned, char, string or bool",
                    ));
                }
                self.switches += 1;
                let mut defaults = 0;
                let mut seen = HashSet::new();
                for (label, body) in arms {
                    if let Some(label) = label {
                        if !matches!(
                            label.kind,
                            ExprKind::Int(_)
                                | ExprKind::UInt(_)
                                | ExprKind::Char(_)
                                | ExprKind::String(_)
                                | ExprKind::Bool(_)
                        ) {
                            return Err(Diagnostic::new(
                                "JOLE42",
                                &label.span,
                                "checkpoint labels must be literals",
                            ));
                        }
                        let t = self.expr(label)?;
                        self.expect(&ty, &t, &label.span)?;
                        // Numeric labels can have different syntax but match the same value.
                        let key = match &label.kind {
                            ExprKind::Int(n) => format!("number:{n}"),
                            ExprKind::UInt(n) => format!("number:{n}"),
                            ExprKind::Bool(b) => format!("number:{}", u8::from(*b)),
                            other => format!("{other:?}"),
                        };
                        if !seen.insert(key) {
                            return Err(Diagnostic::new(
                                "JOLE42",
                                &label.span,
                                "duplicate checkpoint",
                            ));
                        }
                    } else {
                        defaults += 1;
                    }
                    self.scopes.push(HashMap::new());
                    for s in body {
                        self.stmt(s)?;
                    }
                    self.scopes.pop();
                }
                if defaults > 1 {
                    return Err(Diagnostic::new(
                        "JOLE42",
                        &s.span,
                        "duplicate practice branch",
                    ));
                }
                self.switches -= 1;
                Ok(false)
            }
            StmtKind::Empty => Ok(false),
        }
    }
    fn graph_name(&self, n: &str, s: &Span) -> Result<()> {
        if self.checked.functions.get(n).is_some_and(|f| f.graph) {
            Ok(())
        } else {
            Err(Diagnostic::new(
                "D404",
                s,
                format!("`{n}` is not a graph expression"),
            ))
        }
    }
    fn lvalue(&mut self, e: &Expr) -> Result<Type> {
        match &e.kind {
            ExprKind::Var(n) => {
                let sym = self.lookup(n, &e.span)?;
                if !sym.mutable {
                    return Err(Diagnostic::new(
                        "LOCKED",
                        &e.span,
                        format!("`{n}` is locked"),
                    ));
                }
                Ok(sym.ty)
            }
            ExprKind::Index(base, _) | ExprKind::Field(base, _) => {
                self.lvalue(base)?;
                let b = self.expr(base)?;
                if b == Type::String {
                    return Err(Diagnostic::new(
                        "LOCKED",
                        &e.span,
                        "strings are immutable; assign a new string",
                    ));
                }
                self.expr(e)
            }
            ExprKind::Unary(op, _) if op == "*" => self.expr(e),
            _ => Err(Diagnostic::new(
                "AURA-12",
                &e.span,
                "assignment/address requires a mutable variable, field, index, or pointer dereference",
            )),
        }
    }
    fn binary(&self, a: &Type, op: &str, b: &Type, s: &Span) -> Result<Type> {
        if ["==", "!="].contains(&op) && (assignable(a, b) || assignable(b, a)) {
            return Ok(Type::Bool);
        }
        if ["&&", "||"].contains(&op)
            && (a.numeric() || *a == Type::Bool)
            && (b.numeric() || *b == Type::Bool)
        {
            return Ok(Type::Bool);
        }
        if a.numeric() && b.numeric() {
            return Ok(if ["<", ">", "<=", ">="].contains(&op) {
                Type::Bool
            } else if *a == Type::Float || *b == Type::Float || op == "^" {
                Type::Float
            } else if *a == Type::UInt || *b == Type::UInt {
                Type::UInt
            } else {
                a.clone()
            });
        }
        if *a == Type::String && *b == Type::String && op == "+" {
            return Ok(Type::String);
        }
        if ["+", "-", "*", "/", "%", "^"].contains(&op) {
            if (*a == Type::List && (b.numeric() || *b == Type::List))
                || (*b == Type::List && a.numeric())
            {
                return Ok(Type::List);
            }
            if matches!(a, Type::Point | Type::Vector) && a == b && ["+", "-"].contains(&op) {
                return Ok(a.clone());
            }
            if matches!(a, Type::Point | Type::Vector) && b.numeric() && ["*", "/"].contains(&op) {
                return Ok(a.clone());
            }
        }
        Err(Diagnostic::new(
            "AURA-12",
            s,
            format!("operator `{op}` does not accept {a} and {b}"),
        ))
    }
    fn expr(&mut self, e: &Expr) -> Result<Type> {
        use ExprKind::*;
        Ok(match &e.kind {
            Int(_) => Type::Int,
            UInt(_) => Type::UInt,
            Float(_) => Type::Float,
            Bool(_) => Type::Bool,
            Char(_) => Type::Char,
            String(_) => Type::String,
            Null => Type::Null,
            Var(n) => self.lookup(n, &e.span)?.ty,
            Array(es) => {
                let mut ty = Type::Any;
                for e in es {
                    let t = self.expr(e)?;
                    if ty == Type::Any {
                        ty = t;
                    } else if ty.numeric() && t.numeric() && ty != t {
                        if ty == Type::Float || t == Type::Float {
                            ty = Type::Float;
                        } else {
                            self.expect(&ty, &t, &e.span)?;
                        }
                    } else {
                        self.expect(&ty, &t, &e.span)?;
                    }
                }
                Type::Array(Box::new(ty))
            }
            Point(a, b) => {
                for e in [a, b] {
                    let t = self.expr(e)?;
                    self.expect(&Type::Float, &t, &e.span)?;
                }
                Type::Point
            }
            Record(n, fields) => {
                let ty = self.resolve(&Type::Named(n.clone()), &e.span)?;
                let name = if let Type::Named(n) = &ty {
                    n
                } else {
                    return Err(Diagnostic::new(
                        "AURA-12",
                        &e.span,
                        "record literal requires an object type",
                    ));
                };
                let defs = self.checked.structs[name].clone();
                let mut seen = HashSet::new();
                for (n, v) in fields {
                    if !seen.insert(n) {
                        return Err(Diagnostic::new(
                            "JOLE42",
                            &v.span,
                            format!("duplicate field `{n}`"),
                        ));
                    }
                    let t = defs
                        .iter()
                        .find(|(f, _)| f == n)
                        .ok_or_else(|| {
                            Diagnostic::new("D404", &v.span, format!("unknown field `{n}`"))
                        })?
                        .1
                        .clone();
                    let want = self.resolve(&t, &v.span)?;
                    let got = self.expr(v)?;
                    self.expect(&want, &got, &v.span)?;
                }
                ty
            }
            Unary(op, v) => {
                if op == "&" {
                    Type::Ptr(Box::new(self.lvalue(v)?))
                } else {
                    let t = self.expr(v)?;
                    match op.as_str() {
                        "*" => {
                            if let Type::Ptr(t) = t {
                                *t
                            } else {
                                return Err(Diagnostic::new(
                                    "COOKED",
                                    &e.span,
                                    "dereference requires a pointer",
                                ));
                            }
                        }
                        "!" => {
                            self.condition(v)?;
                            Type::Bool
                        }
                        "+" | "-"
                            if t.numeric()
                                || t == Type::List
                                || t == Type::Point
                                || t == Type::Vector =>
                        {
                            t
                        }
                        _ => {
                            return Err(Diagnostic::new(
                                "AURA-12",
                                &e.span,
                                format!("invalid unary `{op}` on {t}"),
                            ));
                        }
                    }
                }
            }
            Binary(a, op, b) => {
                let a = self.expr(a)?;
                let b = self.expr(b)?;
                self.binary(&a, op, &b, &e.span)?
            }
            Assign(a, op, b) => {
                let want = self.lvalue(a)?;
                let got = self.expr(b)?;
                if op == "=" {
                    self.expect(&want, &got, &e.span)?;
                } else {
                    let out = self.binary(&want, &op[..1], &got, &e.span)?;
                    self.expect(&want, &out, &e.span)?;
                }
                want
            }
            Update(a, _, _) => {
                let t = self.lvalue(a)?;
                if !t.numeric() {
                    return Err(Diagnostic::new(
                        "AURA-12",
                        &e.span,
                        "increment/decrement requires a numeric lvalue",
                    ));
                }
                t
            }
            Call(fun, args) => {
                let ft = self.expr(fun)?;
                if let Var(n) = &fun.kind
                    && matches!(ft, Type::Callable(..))
                {
                    if ["len", "hitbox"].contains(&n.as_str()) && args.len() == 1 {
                        let t = self.expr(&args[0])?;
                        if n == "len" && !matches!(t, Type::List | Type::Array(_) | Type::String) {
                            return Err(Diagnostic::new(
                                "AURA-12",
                                &e.span,
                                "len requires string, array, or list",
                            ));
                        }
                    }
                    if n == "alloc" && args.len() == 1 {
                        let t = self.expr(&args[0])?;
                        if matches!(t, Type::Void | Type::Null | Type::Callable(..)) {
                            return Err(Diagnostic::new(
                                "AURA-12",
                                &e.span,
                                "cannot allocate this type",
                            ));
                        }
                        return Ok(Type::Ptr(Box::new(t)));
                    }
                    if n == "free"
                        && args.len() == 1
                        && !matches!(self.expr(&args[0])?, Type::Ptr(_) | Type::Null)
                    {
                        return Err(Diagnostic::new(
                            "COOKED",
                            &e.span,
                            "free requires a pointer",
                        ));
                    }
                }
                if let Type::Callable(params, ret, varargs) = ft {
                    if (varargs && args.len() < params.len())
                        || (!varargs && args.len() != params.len())
                    {
                        return Err(Diagnostic::new(
                            "AURA-12",
                            &e.span,
                            format!(
                                "expected {}{} arguments, received {}",
                                if varargs { "at least " } else { "" },
                                params.len(),
                                args.len()
                            ),
                        ));
                    }
                    for (i, arg) in args.iter().enumerate() {
                        let t = self.expr(arg)?;
                        if let Some(want) = params.get(i) {
                            self.expect(want, &t, &arg.span)?;
                        } else if t == Type::Void {
                            return Err(Diagnostic::new(
                                "AURA-12",
                                &arg.span,
                                "void cannot be passed as a value",
                            ));
                        }
                    }
                    self.resolve(&ret, &e.span)?
                } else {
                    return Err(Diagnostic::new(
                        "AURA-12",
                        &e.span,
                        format!("{ft} is not callable"),
                    ));
                }
            }
            Index(a, i) => {
                let t = self.expr(i)?;
                self.expect(&Type::Int, &t, &i.span)?;
                match self.expr(a)? {
                    Type::Array(t) => *t,
                    Type::List => Type::Float,
                    Type::String => Type::Char,
                    t => {
                        return Err(Diagnostic::new(
                            "AURA-12",
                            &a.span,
                            format!("cannot index {t}"),
                        ));
                    }
                }
            }
            Field(a, n) => match self.expr(a)? {
                Type::Named(name) => {
                    let ty = self.checked.structs[&name]
                        .iter()
                        .find(|(f, _)| f == n)
                        .ok_or_else(|| {
                            Diagnostic::new("D404", &e.span, format!("{name} has no field `{n}`"))
                        })?
                        .1
                        .clone();
                    self.resolve(&ty, &e.span)?
                }
                Type::Point | Type::Vector if n == "x" || n == "y" => Type::Float,
                t => {
                    return Err(Diagnostic::new(
                        "D404",
                        &e.span,
                        format!("{t} has no field `{n}`"),
                    ));
                }
            },
            Conditional(c, t, f) => {
                self.condition(c)?;
                let a = self.expr(t)?;
                let b = self.expr(f)?;
                if assignable(&a, &b) {
                    a
                } else if assignable(&b, &a) {
                    b
                } else {
                    return Err(Diagnostic::new(
                        "AURA-12",
                        &e.span,
                        "piecewise branches have incompatible types",
                    ));
                }
            }
        })
    }
}
pub fn assignable(w: &Type, g: &Type) -> bool {
    w == g
        || *w == Type::Any
        || (*w == Type::Float && g.numeric())
        || (*w == Type::Int && *g == Type::Bool)
        || (*w == Type::UInt && *g == Type::Int)
        || (*w == Type::Vector && *g == Type::Point)
        || matches!((w, g), (Type::Ptr(_), Type::Null))
        || match (w, g) {
            (Type::Array(a), Type::Array(b)) => **b == Type::Any || assignable(a, b),
            (Type::List, Type::Array(t)) => t.numeric() || **t == Type::Any,
            _ => false,
        }
}
pub fn resolve_type(t: &Type, c: &Checked, s: &Span, seen: &mut HashSet<String>) -> Result<Type> {
    Ok(match t {
        Type::Named(n) => {
            if let Some(t) = c.aliases.get(n) {
                if !seen.insert(n.clone()) {
                    return Err(Diagnostic::new("AURA-12", s, "cyclic addaterm type alias"));
                }
                let r = resolve_type(t, c, s, seen)?;
                seen.remove(n);
                r
            } else if c.structs.contains_key(n) {
                t.clone()
            } else {
                return Err(Diagnostic::new("D404", s, format!("unknown type `{n}`")));
            }
        }
        Type::Ptr(t) => Type::Ptr(Box::new(resolve_type(t, c, s, seen)?)),
        Type::Array(t) => Type::Array(Box::new(resolve_type(t, c, s, seen)?)),
        _ => t.clone(),
    })
}
fn item_span(i: &Item) -> Span {
    match i {
        Item::Import(_, s)
        | Item::Struct(_, _, s)
        | Item::Alias(_, _, s)
        | Item::ValueAlias(_, _, s) => s.clone(),
        Item::Function(f) => f.span.clone(),
        Item::Global(s) => s.span.clone(),
    }
}
