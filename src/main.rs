use cussy::{
    VERSION,
    ast::*,
    checker::Checker,
    diagnostic::Diagnostic,
    loader::Loader,
    runtime::{Options, Runtime},
    value::{Value, cell},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    io::{self, IsTerminal, Write},
    path::Path,
};
#[derive(Serialize, Deserialize)]
struct Artifact {
    format: String,
    version: String,
    program: Program,
}
fn help() {
    println!(
        "Cussy {VERSION} — C, but somebody opened Desmos.\n\nUsage:\n  cussy run <file.cussy|file.csyb> [--fuel N] [--allow-ffi] [-- args...]\n  cussy build <file.cussy> [-o file.csyb]\n  cussy check <file.cussy|file.csyb>\n  cussy fmt <file.cussy> [--check|--stdout]\n  cussy repl\n  cussy jole|stream|papa|aura|cooked|lore\n\nDiagnostics: --normal (default), --brainrot, --jole\nBuild emits a checked, portable AST image; run it with this Cussy version.\njole means jole. Not joke."
    );
}
fn main() {
    // Keep the interpreter's documented nesting limits independent of the
    // platform's default main-thread stack (only 1 MiB on Windows).
    let worker = std::thread::Builder::new()
        .name("cussy-cli".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(cli);
    let status = match worker {
        Ok(worker) => match worker.join() {
            Ok(status) => status,
            Err(panic) => std::panic::resume_unwind(panic),
        },
        Err(error) => {
            eprintln!("error: could not start Cussy: {error}");
            1
        }
    };
    std::process::exit(status);
}
fn cli() -> i32 {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        help();
        return 0;
    }
    if args[0] == "--version" || args[0] == "-V" {
        println!("cussy {VERSION}");
        return 0;
    }
    let mut mode = "normal";
    let mut personality_selected = false;
    let cmd = args[0].as_str();
    if args.get(1).is_some_and(|a| a == "--help" || a == "-h") {
        help();
        return 0;
    }
    match cmd {
        "jole" => {
            println!("jole");
            return 0;
        }
        "stream" => {
            println!("whitecaplol hop on stream");
            return 0;
        }
        "papa" => {
            println!("whitecaplol is our papa");
            return 0;
        }
        "aura" => {
            println!("aura: 100 | LOCKED IN | jolemaxxing enabled");
            return 0;
        }
        "cooked" => {
            println!("COOKED: this pointer has left the domain. Use alloc, then free once.");
            return 0;
        }
        "lore" => {
            println!("{}", include_str!("../docs/lore.txt"));
            return 0;
        }
        _ => {}
    }
    let mut options = Options {
        echo: true,
        ..Options::default()
    };
    let mut path = None;
    let mut output = None;
    let mut fmt_check = false;
    let mut stdout = false;
    let mut i = 1;
    while i < args.len() {
        let valid_command = match args[i].as_str() {
            "--allow-ffi" | "--fuel" => matches!(cmd, "run" | "repl"),
            "--check" | "--stdout" => cmd == "fmt",
            "-o" | "--output" => cmd == "build",
            "--" => matches!(cmd, "run" | "repl"),
            _ => true,
        };
        if !valid_command {
            eprintln!("error: {} is not supported by {cmd}", args[i]);
            return 2;
        }
        match args[i].as_str() {
            "--" => {
                options.args = args[i + 1..].to_vec();
                break;
            }
            "--normal" | "--brainrot" | "--jole" => {
                if personality_selected {
                    eprintln!("error: choose one diagnostic personality");
                    return 2;
                }
                mode = args[i].trim_start_matches("--");
                personality_selected = true;
            }
            "--allow-ffi" => options.allow_ffi = true,
            "--check" => fmt_check = true,
            "--stdout" => stdout = true,
            "--fuel" => {
                i += 1;
                match args.get(i).and_then(|s| s.parse::<u64>().ok()) {
                    Some(n) if n > 0 => options.fuel = n,
                    _ => {
                        eprintln!("error: --fuel requires a positive integer");
                        return 2;
                    }
                }
            }
            "-o" | "--output" => {
                i += 1;
                if let Some(p) = args.get(i) {
                    output = Some(p.clone());
                } else {
                    eprintln!("error: -o requires a path");
                    return 2;
                }
            }
            a if a.starts_with('-') => {
                eprintln!("error: unknown option {a}");
                return 2;
            }
            a => {
                if path.replace(a.to_string()).is_some() {
                    eprintln!("error: only one input path is allowed; program arguments follow --");
                    return 2;
                }
            }
        }
        i += 1;
    }
    if fmt_check && stdout {
        eprintln!("error: --check and --stdout are mutually exclusive");
        return 2;
    }
    if cmd == "repl" {
        if path.is_some() {
            eprintln!("error: repl does not accept a source path; use run to execute a file");
            return 2;
        }
        return repl(mode, options);
    }
    if !["run", "check", "build", "fmt"].contains(&cmd) {
        eprintln!("error: unknown command `{cmd}`; try cussy --help");
        return 2;
    }
    let Some(path) = path else {
        eprintln!("error: {cmd} requires an input file");
        return 2;
    };
    if cmd == "fmt" {
        let source = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{path}: {e}");
                return 1;
            }
        };
        match cussy::formatter::format(&path, &source) {
            Ok(formatted) => {
                if fmt_check {
                    if formatted != source {
                        eprintln!("{path}: formatting differs");
                        return 1;
                    }
                } else if stdout {
                    print!("{formatted}");
                } else if let Err(e) = std::fs::write(&path, formatted) {
                    eprintln!("{path}: {e}");
                    return 1;
                }
                return 0;
            }
            Err(e) => {
                eprint!("{}", e.render(&BTreeMap::from([(path, source)]), mode));
                return 1;
            }
        }
    }
    let p = match load(&path) {
        Ok(p) => p,
        Err(error) => {
            let (e, sources) = *error;
            eprint!("{}", e.render(&sources, mode));
            return 1;
        }
    };
    let checked = match Checker::check(&p, true) {
        Ok(c) => c,
        Err(e) => {
            eprint!("{}", e.render(&p.sources, mode));
            return 1;
        }
    };
    if cmd == "check" {
        println!("VERIFIED: {path}");
        return 0;
    }
    if cmd == "build" {
        let target = output.unwrap_or_else(|| {
            Path::new(&path)
                .with_extension("csyb")
                .display()
                .to_string()
        });
        let target_abs = absolute(&target);
        if p.sources
            .keys()
            .any(|p| !p.starts_with("std:") && absolute(p) == target_abs)
        {
            eprintln!("error: build output would overwrite a source file");
            return 1;
        }
        let artifact = Artifact {
            format: "cussy-ast-v1".into(),
            version: VERSION.into(),
            program: p,
        };
        let bytes = match serde_json::to_vec(&artifact) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("build serialization failed: {e}");
                return 1;
            }
        };
        if let Err(e) = std::fs::write(&target, bytes) {
            eprintln!("build failed: {e}");
            return 1;
        }
        println!("BUILD VERIFIED: {target}");
        if mode != "normal" {
            println!("whitecaplol is our papa");
        }
        return 0;
    }
    let mut rt = Runtime::new(checked, options);
    match rt.run(&p) {
        Ok(code) => (code & 255) as i32,
        Err(e) => {
            eprint!("{}", e.render(&p.sources, mode));
            1
        }
    }
}
fn absolute(path: &str) -> std::path::PathBuf {
    let p = Path::new(path);
    p.canonicalize().unwrap_or_else(|_| {
        if p.is_absolute() {
            p.to_owned()
        } else {
            std::env::current_dir().unwrap_or_default().join(p)
        }
    })
}
type LoadError = Box<(Diagnostic, BTreeMap<String, String>)>;
fn load(path: &str) -> std::result::Result<Program, LoadError> {
    if Path::new(path).extension().is_some_and(|e| e == "csyb") {
        let span = Span {
            file: path.into(),
            line: 1,
            col: 1,
            len: 1,
        };
        let bytes = std::fs::read(path).map_err(|e| {
            Box::new((Diagnostic::new("IO", &span, e.to_string()), BTreeMap::new()))
        })?;
        let a: Artifact = serde_json::from_slice(&bytes).map_err(|e| {
            Box::new((
                Diagnostic::new("BUILD", &span, format!("invalid AST image: {e}")),
                BTreeMap::new(),
            ))
        })?;
        if a.format != "cussy-ast-v1" || a.version != VERSION {
            return Err(Box::new((
                Diagnostic::new(
                    "BUILD",
                    &span,
                    "incompatible artifact format or Cussy version; rebuild from source",
                ),
                BTreeMap::new(),
            )));
        }
        Ok(a.program)
    } else {
        let mut l = Loader::default();
        l.file(Path::new(path))
            .map_err(|e| Box::new((e, l.sources)))
    }
}
fn repl(mode: &str, options: Options) -> i32 {
    let interactive = io::stdin().is_terminal();
    println!(
        "CUSSY REPL v{VERSION}\nC + Desmos + Geometry Dash + irreversible jole exposure\nwhitecaplol is our papa\nwhitecaplol hop on stream\n:help for help, :quit to escape"
    );
    let mut declarations = String::new();
    let mut runtime = None::<Runtime>;
    let mut initialized = HashSet::<String>::new();
    let mut buffer = String::new();
    loop {
        if interactive {
            print!("{}", if buffer.is_empty() { ">>> " } else { "... " });
            let _ = io::stdout().flush();
        }
        let mut line = String::new();
        match io::stdin().read_line(&mut line) {
            Ok(0) if !buffer.is_empty() => {
                eprintln!("error: unexpected end of input in unfinished REPL block");
                return 1;
            }
            Ok(0) => break,
            Ok(_) => {}
            Err(error) => {
                eprintln!("error: could not read REPL input: {error}");
                return 1;
            }
        }
        if buffer.is_empty() {
            match line.trim() {
                ":quit" | ":q" => break,
                ":help" => {
                    println!(
                        "Declarations persist. Expressions without ; print their value. Blocks span lines.\n:reset clears state; :quit exits. Import libraries with graph math;"
                    );
                    continue;
                }
                ":reset" => {
                    declarations.clear();
                    runtime = None;
                    initialized.clear();
                    continue;
                }
                "" => continue,
                _ => {}
            }
        }
        buffer.push_str(&line);
        if let Ok(ts) = cussy::lexer::lex("<repl>", &buffer, false) {
            let balance = ts.iter().fold(0i64, |a, t| {
                a + if t.raw == "{" {
                    1
                } else if t.raw == "}" {
                    -1
                } else {
                    0
                }
            });
            if balance > 0 {
                continue;
            }
        }
        let input = std::mem::take(&mut buffer);
        let candidate = format!("{declarations}\n{input}");
        let mut loader = Loader::default();
        let declaration = loader
            .source("<repl>", &candidate, Path::new("."))
            .and_then(|p| Checker::check(&p, false).map(|c| (p, c)));
        let (p, checked, is_decl) = match declaration {
            Ok((p, c)) => (p, c, true),
            Err(decl_error) => {
                let expr = input.trim();
                let statement = if expr.ends_with(';') || expr.ends_with('}') {
                    expr.to_string()
                } else {
                    format!("jole({expr});")
                };
                let source = format!("{declarations}\nvoid __repl_eval() {{\n{statement}\n}}");
                let mut l = Loader::default();
                match l
                    .source("<repl>", &source, Path::new("."))
                    .and_then(|p| Checker::check(&p, false).map(|c| (p, c)))
                {
                    Ok((p, c)) => (p, c, false),
                    Err(e) => {
                        let looks_decl = expr.starts_with("graph ")
                            || expr.starts_with("addaterm ")
                            || expr.starts_with("object ")
                            || expr.starts_with("locked ");
                        if looks_decl {
                            eprint!("{}", decl_error.render(&loader.sources, mode));
                        } else {
                            eprint!("{}", e.render(&l.sources, mode));
                        }
                        continue;
                    }
                }
            }
        };
        let rt = runtime.get_or_insert_with(|| Runtime::new(checked.clone(), options.clone()));
        rt.checked = checked;
        rt.options.fuel = options.fuel;
        for (n, f) in &rt.checked.functions {
            rt.scopes[0].insert(
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
        let mut new_items = Vec::new();
        let mut keys = Vec::new();
        for item in &p.items {
            let key = match item {
                Item::Global(s) => match &s.kind {
                    StmtKind::Var { name, .. } => Some(format!("var:{name}")),
                    _ => Some(format!("global:{}:{}", s.span.file, s.span.line)),
                },
                Item::ValueAlias(n, _, _) => Some(format!("alias:{n}")),
                _ => None,
            };
            if let Some(key) = key
                && !initialized.contains(&key)
            {
                new_items.push(item.clone());
                keys.push(key);
            }
        }
        match rt.initialize(&Program {
            items: new_items,
            sources: p.sources.clone(),
        }) {
            Ok(()) => {
                initialized.extend(keys);
            }
            Err(e) => {
                eprint!("{}", e.render(&p.sources, mode));
                continue;
            }
        }
        if is_decl {
            declarations = candidate;
        } else if let Err(e) = rt.call(
            "__repl_eval",
            vec![],
            &Span {
                file: "<repl>".into(),
                line: 1,
                col: 1,
                len: 1,
            },
        ) {
            eprint!("{}", e.render(&p.sources, mode));
        }
        rt.output.clear();
    }
    println!("escaped successfully");
    0
}
