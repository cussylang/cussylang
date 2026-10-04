use crate::{
    diagnostic::Result,
    lexer::{Kind, lex},
};

/// A conservative token formatter: it never rewrites literal contents or invents syntax.
pub fn format(file: &str, source: &str) -> Result<String> {
    let tokens = lex(file, source, true)?;
    let mut out = String::new();
    let mut line = String::new();
    let mut indent = 0usize;
    let mut parens = 0usize;
    let mut previous = "";
    fn flush(out: &mut String, line: &mut String, indent: usize) {
        if !line.trim().is_empty() {
            out.push_str(&"    ".repeat(indent));
            out.push_str(line.trim());
            out.push('\n');
        }
        line.clear();
    }
    fn space(line: &mut String) {
        if !line.is_empty() && !line.ends_with(' ') {
            line.push(' ');
        }
    }
    for (i, t) in tokens.iter().enumerate() {
        if t.kind == Kind::Eof {
            break;
        }
        let next = tokens.get(i + 1).map_or("", |t| t.raw.as_str());
        if t.kind == Kind::Comment {
            flush(&mut out, &mut line, indent);
            for text in t.raw.lines() {
                out.push_str(&"    ".repeat(indent));
                out.push_str(text.trim());
                out.push('\n');
            }
            // A comment is whitespace, including between two operator tokens.
            previous = "";
            continue;
        }
        match t.raw.as_str() {
            "(" => {
                parens += 1;
                if [
                    "check", "if", "ticker", "while", "attempt", "for", "trigger", "switch",
                ]
                .contains(&previous)
                {
                    space(&mut line);
                }
                line.push('(');
            }
            ")" => {
                parens = parens.saturating_sub(1);
                line = line.trim_end().into();
                line.push(')');
            }
            "[" => line.push('['),
            "]" => {
                line = line.trim_end().into();
                line.push(']');
            }
            "." | "->" => {
                line = line.trim_end().into();
                line.push_str(&t.raw);
            }
            "++" | "--" => {
                // Keep adjacent operators separate: `a + ++b` must not become `a+++b`.
                if ["+", "-", "++", "--"].contains(&previous) {
                    space(&mut line);
                }
                line.push_str(&t.raw);
            }
            "{" => {
                space(&mut line);
                line.push('{');
                flush(&mut out, &mut line, indent);
                indent += 1;
            }
            "}" => {
                flush(&mut out, &mut line, indent);
                indent = indent.saturating_sub(1);
                line.push('}');
                if ![";", ",", ")", "]", "otherwise", "else"].contains(&next) {
                    flush(&mut out, &mut line, indent);
                }
            }
            ";" => {
                line = line.trim_end().into();
                line.push(';');
                if parens == 0 {
                    flush(&mut out, &mut line, indent);
                } else {
                    line.push(' ');
                }
            }
            "," => {
                line = line.trim_end().into();
                line.push_str(", ");
            }
            _ => {
                if !["(", "[", ".", "->", "++", "--"].contains(&previous) {
                    space(&mut line);
                }
                line.push_str(&t.raw);
            }
        }
        previous = &t.raw;
    }
    flush(&mut out, &mut line, indent);
    Ok(out)
}
