use crate::{
    ast::Span,
    ast::Type,
    diagnostic::{Diagnostic, Result},
};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
pub type Cell = Rc<RefCell<Slot>>;
#[derive(Clone, Debug)]
pub struct Slot {
    pub value: Value,
    pub ty: Type,
    pub alive: bool,
    pub mutable: bool,
    pub heap: bool,
}
#[derive(Clone, Debug)]
pub enum Value {
    Int(i64),
    UInt(u64),
    Float(f64),
    Bool(bool),
    Char(char),
    String(String),
    Void,
    Array(Vec<Cell>, Type),
    List(Vec<Cell>),
    Point(Cell, Cell, bool),
    Record(String, BTreeMap<String, Cell>),
    Pointer(Option<Cell>, Type),
    Function(String),
}
pub fn cell(v: Value, ty: Type, mutable: bool) -> Cell {
    Rc::new(RefCell::new(Slot {
        value: v,
        ty,
        alive: true,
        mutable,
        heap: false,
    }))
}
impl Value {
    pub fn ty(&self) -> Type {
        match self {
            Self::Int(_) => Type::Int,
            Self::UInt(_) => Type::UInt,
            Self::Float(_) => Type::Float,
            Self::Bool(_) => Type::Bool,
            Self::Char(_) => Type::Char,
            Self::String(_) => Type::String,
            Self::Void => Type::Void,
            Self::Array(_, t) => Type::Array(Box::new(t.clone())),
            Self::List(_) => Type::List,
            Self::Point(_, _, v) => {
                if *v {
                    Type::Vector
                } else {
                    Type::Point
                }
            }
            Self::Record(n, _) => Type::Named(n.clone()),
            Self::Pointer(_, t) => Type::Ptr(Box::new(t.clone())),
            Self::Function(_) => Type::Any,
        }
    }
    pub fn number(&self, s: &Span) -> Result<f64> {
        match self {
            Self::Int(n) => Ok(*n as f64),
            Self::UInt(n) => Ok(*n as f64),
            Self::Float(n) => Ok(*n),
            _ => Err(Diagnostic::new("AURA-12", s, "expected a number")),
        }
    }
    pub fn integer(&self, s: &Span) -> Result<i64> {
        if let Self::Int(n) = self {
            Ok(*n)
        } else {
            Err(Diagnostic::new("AURA-12", s, "expected int"))
        }
    }
    pub fn string(&self, s: &Span) -> Result<String> {
        if let Self::String(n) = self {
            Ok(n.clone())
        } else {
            Err(Diagnostic::new("AURA-12", s, "expected string"))
        }
    }
    pub fn truth(&self) -> bool {
        match self {
            Self::Bool(b) => *b,
            Self::Int(n) => *n != 0,
            Self::UInt(n) => *n != 0,
            Self::Float(n) => *n != 0.0,
            _ => false,
        }
    }
    pub fn copy(&self) -> Self {
        match self {
            Self::Array(a, t) => Self::Array(a.iter().map(copy_cell).collect(), t.clone()),
            Self::List(a) => Self::List(a.iter().map(copy_cell).collect()),
            Self::Point(x, y, v) => Self::Point(copy_cell(x), copy_cell(y), *v),
            Self::Record(n, m) => Self::Record(
                n.clone(),
                m.iter().map(|(k, v)| (k.clone(), copy_cell(v))).collect(),
            ),
            _ => self.clone(),
        }
    }
    pub fn elements(&self, s: &Span) -> Result<Vec<Value>> {
        match self {
            Self::List(a) | Self::Array(a, _) => {
                Ok(a.iter().map(|c| c.borrow().value.clone()).collect())
            }
            _ => Err(Diagnostic::new("AURA-12", s, "expected list or array")),
        }
    }
    pub fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::UInt(a), Self::UInt(b)) => a == b,
            (Self::Int(a), Self::UInt(b)) | (Self::UInt(b), Self::Int(a)) => {
                u64::try_from(*a).ok() == Some(*b)
            }
            (Self::Int(a), Self::Bool(b)) | (Self::Bool(b), Self::Int(a)) => *a == i64::from(*b),
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Char(a), Self::Char(b)) => a == b,
            (Self::String(a), Self::String(b)) => a == b,
            (Self::Void, Self::Void) => true,
            (Self::Pointer(a, _), Self::Pointer(b, _)) => match (a, b) {
                (None, None) => true,
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                _ => false,
            },
            (Self::Array(a, _), Self::Array(b, _)) | (Self::List(a), Self::List(b)) => {
                a.len() == b.len()
                    && a.iter()
                        .zip(b)
                        .all(|(a, b)| a.borrow().value.same(&b.borrow().value))
            }
            (Self::Point(a, b, _), Self::Point(c, d, _)) => {
                a.borrow().value.same(&c.borrow().value) && b.borrow().value.same(&d.borrow().value)
            }
            (Self::Record(a, m), Self::Record(b, n)) => {
                a == b
                    && m.iter().all(|(k, v)| {
                        n.get(k)
                            .is_some_and(|w| v.borrow().value.same(&w.borrow().value))
                    })
            }
            (Self::Function(a), Self::Function(b)) => a == b,
            _ if self.ty().numeric() && other.ty().numeric() => {
                self.number(&Span::default()).ok() == other.number(&Span::default()).ok()
            }
            _ => false,
        }
    }
}
fn copy_cell(c: &Cell) -> Cell {
    let c = c.borrow();
    cell(c.value.copy(), c.ty.clone(), c.mutable)
}
pub fn invalidate(c: &Cell) {
    let mut slot = c.borrow_mut();
    slot.alive = false;
    invalidate_value(&slot.value);
}
fn invalidate_value(v: &Value) {
    match v {
        Value::Array(a, _) | Value::List(a) => a.iter().for_each(invalidate),
        Value::Point(x, y, _) => {
            invalidate(x);
            invalidate(y);
        }
        Value::Record(_, m) => m.values().for_each(invalidate),
        _ => {}
    }
}
pub fn write_cell(c: &Cell, v: Value, s: &Span) -> Result<()> {
    let mut slot = c.borrow_mut();
    if !slot.alive {
        return Err(Diagnostic::new(
            "COOKED",
            s,
            "access to freed storage or a variable whose scope has ended",
        ));
    }
    if !slot.mutable {
        return Err(Diagnostic::new(
            "LOCKED",
            s,
            "cannot write to locked storage",
        ));
    }
    match (&mut slot.value, &v) {
        (Value::Array(a, _), Value::Array(b, _)) | (Value::List(a), Value::List(b))
            if a.len() == b.len() =>
        {
            for (x, y) in a.iter().zip(b) {
                write_cell(x, y.borrow().value.copy(), s)?;
            }
            return Ok(());
        }
        (Value::Record(n, a), Value::Record(m, b)) if n == m => {
            for (k, x) in a {
                write_cell(x, b[k].borrow().value.copy(), s)?;
            }
            return Ok(());
        }
        (Value::Point(x, y, _), Value::Point(a, b, _)) => {
            write_cell(x, a.borrow().value.copy(), s)?;
            write_cell(y, b.borrow().value.copy(), s)?;
            return Ok(());
        }
        _ => {}
    }
    invalidate_value(&slot.value);
    slot.value = v;
    Ok(())
}
impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Int(n) => write!(f, "{n}"),
            Self::UInt(n) => write!(f, "{n}"),
            Self::Float(n) => write!(f, "{n}"),
            Self::Bool(b) => write!(f, "{}", if *b { "verified" } else { "unverified" }),
            Self::Char(c) => write!(f, "{c}"),
            Self::String(s) => write!(f, "{s}"),
            Self::Void => write!(f, "void"),
            Self::Array(a, _) | Self::List(a) => {
                write!(f, "[")?;
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v.borrow().value)?;
                }
                write!(f, "]")
            }
            Self::Point(x, y, _) => write!(f, "({}, {})", x.borrow().value, y.borrow().value),
            Self::Record(n, m) => {
                write!(f, "{n} {{ ")?;
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}: {}", v.borrow().value)?;
                }
                write!(f, " }}")
            }
            Self::Pointer(None, _) => write!(f, "cooked"),
            Self::Pointer(Some(c), _) => write!(
                f,
                "<{} pointer>",
                if c.borrow().alive {
                    "checked"
                } else {
                    "cooked"
                }
            ),
            Self::Function(n) => write!(f, "<function {n}>"),
        }
    }
}
