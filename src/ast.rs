use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Span {
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub len: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Type {
    Int,
    UInt,
    Float,
    Bool,
    Char,
    String,
    Void,
    List,
    Point,
    Vector,
    Named(String),
    Array(Box<Type>),
    Ptr(Box<Type>),
    Null,
    Callable(Vec<Type>, Box<Type>, bool),
    Any,
}
impl Type {
    pub fn numeric(&self) -> bool {
        matches!(self, Self::Int | Self::UInt | Self::Float)
    }
}
impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Int => write!(f, "int"),
            Self::UInt => write!(f, "unsigned long"),
            Self::Float => write!(f, "float"),
            Self::Bool => write!(f, "bool"),
            Self::Char => write!(f, "char"),
            Self::String => write!(f, "string"),
            Self::Void => write!(f, "void"),
            Self::List => write!(f, "list"),
            Self::Point => write!(f, "point"),
            Self::Vector => write!(f, "vector"),
            Self::Named(s) => write!(f, "{s}"),
            Self::Array(t) => write!(f, "{t}[]"),
            Self::Ptr(t) => write!(f, "{t}*"),
            Self::Null => write!(f, "cooked"),
            Self::Callable(_, r, _) => write!(f, "function -> {r}"),
            Self::Any => write!(f, "value"),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ExprKind {
    Int(i64),
    UInt(u64),
    Float(f64),
    Bool(bool),
    Char(char),
    String(String),
    Null,
    Var(String),
    Array(Vec<Expr>),
    Point(Box<Expr>, Box<Expr>),
    Record(String, Vec<(String, Expr)>),
    Unary(String, Box<Expr>),
    Binary(Box<Expr>, String, Box<Expr>),
    Assign(Box<Expr>, String, Box<Expr>),
    Update(Box<Expr>, i64, bool),
    Call(Box<Expr>, Vec<Expr>),
    Index(Box<Expr>, Box<Expr>),
    Field(Box<Expr>, String),
    Conditional(Box<Expr>, Box<Expr>, Box<Expr>),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum StmtKind {
    Block(Vec<Stmt>),
    Var {
        name: String,
        ty: Type,
        value: Option<Expr>,
        size: Option<Expr>,
        locked: bool,
    },
    Expr(Expr),
    If(Expr, Box<Stmt>, Option<Box<Stmt>>),
    While(Expr, Box<Stmt>),
    For(Option<Box<Stmt>>, Option<Expr>, Option<Expr>, Box<Stmt>),
    Return(Option<Expr>),
    Break,
    Continue,
    Stream(Box<Stmt>),
    Bounds(String, Expr, Expr, bool),
    Plot(String, Option<Expr>),
    Switch(Expr, Vec<(Option<Expr>, Vec<Stmt>)>),
    Empty,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub ret: Type,
    pub params: Vec<(String, Type)>,
    pub body: Stmt,
    pub span: Span,
    pub graph: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Item {
    Import(String, Span),
    Struct(String, Vec<(String, Type)>, Span),
    Alias(String, Type, Span),
    Function(Function),
    Global(Stmt),
    ValueAlias(String, Expr, Span),
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Program {
    pub items: Vec<Item>,
    pub sources: BTreeMap<String, String>,
}
