use crate::ast::Span;
use std::collections::BTreeMap;
#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub code: &'static str,
    pub message: String,
    pub span: Span,
    pub help: Option<String>,
}
pub type Result<T> = std::result::Result<T, Diagnostic>;
impl Diagnostic {
    pub fn new(code: &'static str, span: &Span, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            span: span.clone(),
            help: None,
        }
    }
    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
    pub fn render(&self, sources: &BTreeMap<String, String>, personality: &str) -> String {
        let meme = match self.code {
            "D404" => "desmos expression not found",
            "AURA-12" => "bro suffered catastrophic aura loss",
            "COOKED" => "this pointer is cooked",
            "WASHED" => "function reached the exit without verification",
            "JOLE42" => "expected jole, received unjole",
            "LOCKED" => "bro tried to unglaze a locked value",
            "DOMAIN" => "your graph left the chat",
            "AURA_OVERFLOW" => "bro gained more aura than the universe can store",
            "STREAM69" => "whitecaplol did not hop on stream",
            "PAPA01" => "whitecaplol has not been declared as papa in this scope",
            "LIMIT" => "ticker entered infinite jolemaxxing",
            _ => "the yapcompiler is cooked",
        };
        let mut out = if personality == "normal" {
            format!("error[{}]: {}\n", self.code, self.message)
        } else {
            format!(
                "CUSSY ERROR {}: {}\n{}\ntechnical explanation: {}\n",
                self.code,
                meme,
                if personality == "jole" {
                    "jole failure"
                } else {
                    ""
                },
                self.message
            )
        };
        out.push_str(&format!(
            " --> {}:{}:{}\n",
            self.span.file, self.span.line, self.span.col
        ));
        if let Some(line) = sources
            .get(&self.span.file)
            .and_then(|s| s.lines().nth(self.span.line.saturating_sub(1)))
        {
            out.push_str(&format!(
                "{:>4} | {}\n     | {}{}\n",
                self.span.line,
                line,
                " ".repeat(self.span.col.saturating_sub(1)),
                "^".repeat(self.span.len.clamp(1, 60))
            ));
        }
        if let Some(h) = &self.help {
            out.push_str(&format!("suggestion: {h}\n"));
        }
        out
    }
}
impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}: {}",
            self.span.file, self.span.line, self.span.col, self.message
        )
    }
}
impl std::error::Error for Diagnostic {}
