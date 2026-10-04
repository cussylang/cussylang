use crate::{
    ast::*,
    diagnostic::{Diagnostic, Result},
    parser::Parser,
};
use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
};
pub fn standard(name: &str) -> Option<&'static str> {
    Some(
        match name.trim_end_matches(".cussy").trim_end_matches(".csy") {
            "stdio" => include_str!("../stdlib/stdio.cussy"),
            "math" => include_str!("../stdlib/math.cussy"),
            "desmos" => include_str!("../stdlib/desmos.cussy"),
            "gd" => include_str!("../stdlib/gd.cussy"),
            "graph" => include_str!("../stdlib/graph.cussy"),
            "memory" => include_str!("../stdlib/memory.cussy"),
            "string" => include_str!("../stdlib/string.cussy"),
            "time" => include_str!("../stdlib/time.cussy"),
            "random" => include_str!("../stdlib/random.cussy"),
            "system" => include_str!("../stdlib/system.cussy"),
            "brainrot" => include_str!("../stdlib/brainrot.cussy"),
            "jole" => include_str!("../stdlib/jole.cussy"),
            "stream" => include_str!("../stdlib/stream.cussy"),
            "papa" => include_str!("../stdlib/papa.cussy"),
            _ => return None,
        },
    )
}
#[derive(Default)]
pub struct Loader {
    pub sources: BTreeMap<String, String>,
    items: Vec<Item>,
    done: HashSet<String>,
    active: HashSet<String>,
    types: HashSet<String>,
}
impl Loader {
    pub fn file(&mut self, path: &Path) -> Result<Program> {
        self.load_file(
            path,
            &Span {
                file: path.display().to_string(),
                line: 1,
                col: 1,
                len: 1,
            },
        )?;
        Ok(Program {
            items: self.items.clone(),
            sources: self.sources.clone(),
        })
    }
    pub fn source(&mut self, name: &str, source: &str, base: &Path) -> Result<Program> {
        self.load(name, source, base)?;
        Ok(Program {
            items: self.items.clone(),
            sources: self.sources.clone(),
        })
    }
    fn load_file(&mut self, path: &Path, span: &Span) -> Result<()> {
        let path = path.canonicalize().map_err(|e| {
            Diagnostic::new(
                "D404",
                span,
                format!("cannot load module {}: {e}", path.display()),
            )
        })?;
        let text = std::fs::read_to_string(&path)
            .map_err(|e| Diagnostic::new("D404", span, e.to_string()))?;
        self.load(
            &path.display().to_string(),
            &text,
            path.parent().unwrap_or(Path::new(".")),
        )
    }
    fn load(&mut self, name: &str, source: &str, base: &Path) -> Result<()> {
        if self.done.contains(name) {
            return Ok(());
        }
        if self.active.len() > 64 {
            return Err(Diagnostic::new(
                "LIMIT",
                &Span::default(),
                "module nesting exceeds 64",
            ));
        }
        if !self.active.insert(name.into()) {
            return Err(Diagnostic::new(
                "D404",
                &Span {
                    file: name.into(),
                    line: 1,
                    col: 1,
                    len: 1,
                },
                "cyclic module import",
            ));
        }
        self.sources.insert(name.into(), source.into());
        // Load imports before parsing declarations so imported type aliases are recognized.
        let ts = crate::lexer::lex(name, source, false)?;
        let mut depth = 0;
        for (i, t) in ts.iter().enumerate() {
            if t.raw == "{" {
                depth += 1;
            }
            if t.raw == "}" {
                depth -= 1;
            }
            if depth == 0 && t.raw == "graph" && ts.get(i + 2).is_some_and(|t| t.raw == ";") {
                let module = match &ts[i + 1].kind {
                    crate::lexer::Kind::Ident(s) | crate::lexer::Kind::Str(s) => s,
                    _ => continue,
                };
                let embedded = if matches!(&ts[i + 1].kind, crate::lexer::Kind::Ident(_)) {
                    standard(module)
                } else {
                    None
                };
                if let Some(text) = embedded {
                    self.load(&format!("std:{module}"), text, base)?;
                } else {
                    let mut p = PathBuf::from(module);
                    if p.extension().is_none() {
                        p.set_extension("cussy");
                        // Prefer the primary extension without breaking older modules.
                        // Existing but invalid/unreadable .cussy files must report errors.
                        if !base.join(&p).exists() {
                            let legacy = p.with_extension("csy");
                            if base.join(&legacy).is_file() {
                                p = legacy;
                            }
                        }
                    }
                    self.load_file(&base.join(p), &t.span)?;
                }
            }
        }
        let parsed = Parser::new(name, source, &self.types)?.parse()?;
        for item in parsed {
            match &item {
                Item::Import(_, _) => continue,
                Item::Alias(n, _, _) | Item::Struct(n, _, _) => {
                    self.types.insert(n.clone());
                }
                _ => {}
            }
            self.items.push(item);
        }
        self.active.remove(name);
        self.done.insert(name.into());
        Ok(())
    }
}
