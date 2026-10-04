use crate::{
    ast::Span,
    diagnostic::{Diagnostic, Result},
};
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Ident(String),
    Int(i64),
    UInt(u64),
    Float(f64),
    Str(String),
    Char(char),
    Sym(String),
    Comment,
    Eof,
}
#[derive(Clone, Debug)]
pub struct Token {
    pub kind: Kind,
    pub span: Span,
    pub raw: String,
}
pub fn lex(file: &str, text: &str, comments: bool) -> Result<Vec<Token>> {
    let c: Vec<char> = text.chars().collect();
    let (mut i, mut line, mut col) = (0, 1, 1);
    let mut out = Vec::new();
    while i < c.len() {
        let (start, sl, sc) = (i, line, col);
        let span = Span {
            file: file.into(),
            line: sl,
            col: sc,
            len: 1,
        };
        if c[i].is_whitespace() {
            if c[i] == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
            i += 1;
            continue;
        }
        let kind;
        if c[i] == '/' && c.get(i + 1) == Some(&'/') {
            i += 2;
            while i < c.len() && c[i] != '\n' {
                i += 1;
            }
            kind = Kind::Comment;
        } else if c[i] == '/' && c.get(i + 1) == Some(&'*') {
            i += 2;
            let mut depth = 1;
            while i < c.len() && depth > 0 {
                if c[i] == '/' && c.get(i + 1) == Some(&'*') {
                    depth += 1;
                    i += 2;
                } else if c[i] == '*' && c.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            if depth > 0 {
                return Err(Diagnostic::new(
                    "JOLE42",
                    &span,
                    "unterminated block comment",
                ));
            }
            kind = Kind::Comment;
        } else if c[i].is_ascii_alphabetic() || c[i] == '_' {
            i += 1;
            while i < c.len() && (c[i].is_ascii_alphanumeric() || c[i] == '_') {
                i += 1;
            }
            kind = Kind::Ident(c[start..i].iter().collect());
        } else if c[i].is_ascii_digit()
            || (c[i] == '.' && c.get(i + 1).is_some_and(char::is_ascii_digit))
        {
            i += 1;
            while i < c.len() && (c[i].is_ascii_digit() || c[i] == '_') {
                i += 1;
            }
            let mut float = c[start] == '.';
            if c.get(i) == Some(&'.') {
                float = true;
                i += 1;
                while i < c.len() && (c[i].is_ascii_digit() || c[i] == '_') {
                    i += 1;
                }
            }
            if c.get(i).is_some_and(|c| *c == 'e' || *c == 'E') {
                float = true;
                i += 1;
                if c.get(i).is_some_and(|c| *c == '+' || *c == '-') {
                    i += 1;
                }
                while i < c.len() && c[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let s: String = c[start..i].iter().filter(|c| **c != '_').collect();
            kind = if float {
                let n = s
                    .parse::<f64>()
                    .map_err(|_| Diagnostic::new("JOLE42", &span, "invalid float"))?;
                if !n.is_finite() {
                    return Err(Diagnostic::new(
                        "AURA_OVERFLOW",
                        &span,
                        "float literal is not finite",
                    ));
                }
                Kind::Float(n)
            } else if c.get(i) == Some(&'u') {
                i += 1;
                Kind::UInt(s.parse().map_err(|_| {
                    Diagnostic::new("AURA_OVERFLOW", &span, "unsigned literal exceeds 64 bits")
                })?)
            } else {
                Kind::Int(s.parse().map_err(|_| {
                    Diagnostic::new(
                        "AURA_OVERFLOW",
                        &span,
                        "integer literal exceeds signed 64 bits; use a u suffix for unsigned",
                    )
                })?)
            };
        } else if c[i] == '"' || c[i] == '\'' {
            let quote = c[i];
            i += 1;
            let mut s = String::new();
            while i < c.len() && c[i] != quote {
                if c[i] == '\\' {
                    i += 1;
                    let ch = match c.get(i) {
                        Some('n') => '\n',
                        Some('t') => '\t',
                        Some('r') => '\r',
                        Some('0') => '\0',
                        Some('\\') => '\\',
                        Some('"') => '"',
                        Some('\'') => '\'',
                        _ => {
                            return Err(Diagnostic::new(
                                "JOLE42",
                                &span,
                                "unknown or unfinished escape sequence",
                            ));
                        }
                    };
                    s.push(ch);
                    i += 1;
                } else {
                    if c[i] == '\n' {
                        return Err(Diagnostic::new(
                            "JOLE42",
                            &span,
                            "newline inside quoted literal; use \\n",
                        ));
                    }
                    s.push(c[i]);
                    i += 1;
                }
            }
            if i == c.len() {
                return Err(Diagnostic::new("JOLE42", &span, "unterminated literal"));
            }
            i += 1;
            kind = if quote == '"' {
                Kind::Str(s)
            } else {
                let mut chars = s.chars();
                let ch = chars
                    .next()
                    .ok_or_else(|| Diagnostic::new("JOLE42", &span, "empty char"))?;
                if chars.next().is_some() {
                    return Err(Diagnostic::new(
                        "JOLE42",
                        &span,
                        "char requires one Unicode scalar",
                    ));
                }
                Kind::Char(ch)
            };
        } else {
            let two: String = c[i..(i + 2).min(c.len())].iter().collect();
            if [
                "==", "!=", "<=", ">=", "&&", "||", "+=", "-=", "*=", "/=", "%=", "++", "--", "->",
            ]
            .contains(&two.as_str())
            {
                kind = Kind::Sym(two);
                i += 2;
            } else if "{}()[];,:.?+-*/%^!<>=&|".contains(c[i]) {
                kind = Kind::Sym(c[i].to_string());
                i += 1;
            } else {
                return Err(Diagnostic::new(
                    "JOLE42",
                    &span,
                    format!("unexpected character `{}`", c[i]),
                ));
            }
        }
        let raw: String = c[start..i].iter().collect();
        for ch in c[start..i].iter() {
            if *ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        if comments || kind != Kind::Comment {
            out.push(Token {
                kind,
                span: Span {
                    len: i - start,
                    ..span
                },
                raw,
            });
        }
    }
    out.push(Token {
        kind: Kind::Eof,
        span: Span {
            file: file.into(),
            line,
            col,
            len: 1,
        },
        raw: String::new(),
    });
    Ok(out)
}
