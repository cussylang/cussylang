//! In-process, capability-restricted Cussy runtime for iOS and Android.
//!
//! The C entry point is synchronous: app callers must invoke it on a background
//! task. Each invocation creates its own interpreter on a 16 MiB worker stack.
use cussy::{
    ast::Span,
    checker::Checker,
    diagnostic::Diagnostic,
    lexer::{Kind, lex},
    loader::Loader,
    runtime::{Options, Runtime},
};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    ffi::{CString, c_char},
    panic::{AssertUnwindSafe, catch_unwind},
    path::Path,
    ptr, thread,
};

pub const MAX_SOURCE_BYTES: usize = 1024 * 1024;
pub const DEFAULT_FUEL: u64 = 5_000_000;
pub const MAX_FUEL: u64 = DEFAULT_FUEL;
pub const MAX_TOKENS: usize = 8192;
pub const MAX_SEGMENT_TOKENS: usize = 1024;
const WORKER_STACK_BYTES: usize = 16 * 1024 * 1024;
const SOURCE_NAME: &str = "program.cussy";

/// Stable response schema shared by the C ABI and Rust callers.
#[derive(Debug, Serialize)]
pub struct RunResponse {
    pub ok: bool,
    pub exit_code: Option<i64>,
    pub output: String,
    pub diagnostic: String,
}
impl RunResponse {
    fn error(code: &'static str, message: &str) -> Self {
        let span = Span {
            file: SOURCE_NAME.into(),
            line: 1,
            col: 1,
            len: 1,
        };
        Self::diagnostic(
            Diagnostic::new(code, &span, message),
            &BTreeMap::new(),
            String::new(),
        )
    }
    fn diagnostic(
        mut error: Diagnostic,
        sources: &BTreeMap<String, String>,
        output: String,
    ) -> Self {
        if error.span.file.is_empty() {
            error.span.file = SOURCE_NAME.into();
            error.span.line = error.span.line.max(1);
            error.span.col = error.span.col.max(1);
            error.span.len = error.span.len.max(1);
        }
        Self {
            ok: false,
            exit_code: None,
            output,
            diagnostic: error.render(sources, "normal"),
        }
    }
}

/// Execute source without filesystem imports or host I/O, on an isolated worker
/// stack. Fuel is bounded, but this is not an OS process or memory sandbox.
pub fn run_source(source: &[u8], fuel: u64) -> RunResponse {
    if source.len() > MAX_SOURCE_BYTES {
        return RunResponse::error("LIMIT", "mobile source exceeds 1 MiB");
    }
    let source = match std::str::from_utf8(source) {
        Ok(source) => source.to_owned(),
        Err(_) => return RunResponse::error("UTF8", "source must be valid UTF-8"),
    };
    let fuel = if fuel == 0 {
        DEFAULT_FUEL
    } else {
        fuel.min(MAX_FUEL)
    };
    let worker = thread::Builder::new()
        .name("cussy-mobile".into())
        .stack_size(WORKER_STACK_BYTES)
        .spawn(move || {
            catch_unwind(AssertUnwindSafe(|| execute(&source, fuel))).unwrap_or_else(|_| {
                RunResponse::error("INTERNAL", "interpreter panicked; run was stopped")
            })
        });
    match worker {
        Ok(worker) => worker.join().unwrap_or_else(|_| {
            RunResponse::error("INTERNAL", "interpreter worker failed; run was stopped")
        }),
        Err(_) => RunResponse::error("INTERNAL", "cannot start the interpreter worker"),
    }
}

fn execute(source: &str, fuel: u64) -> RunResponse {
    if let Err(error) = check_source_complexity(source) {
        let sources = BTreeMap::from([(SOURCE_NAME.into(), source.into())]);
        return RunResponse::diagnostic(error, &sources, String::new());
    }
    let mut loader = Loader::embedded_only();
    let program = match loader.source(SOURCE_NAME, source, Path::new(".")) {
        Ok(program) => program,
        Err(error) => return RunResponse::diagnostic(error, &loader.sources, String::new()),
    };
    let checked = match Checker::check(&program, true) {
        Ok(checked) => checked,
        Err(error) => return RunResponse::diagnostic(error, &program.sources, String::new()),
    };
    let mut runtime = Runtime::new(
        checked,
        Options {
            fuel,
            echo: false,
            allow_ffi: false,
            allow_host_io: false,
            args: Vec::new(),
        },
    );
    match runtime.run(&program) {
        Ok(exit_code) => RunResponse {
            ok: true,
            exit_code: Some(exit_code),
            output: runtime.output,
            diagnostic: String::new(),
        },
        Err(error) => RunResponse::diagnostic(error, &program.sources, runtime.output),
    }
}

fn check_source_complexity(source: &str) -> cussy::diagnostic::Result<()> {
    let tokens = lex(SOURCE_NAME, source, false)?;
    let mut total = 0;
    let mut segment = 0;
    for token in tokens {
        if token.kind == Kind::Eof {
            continue;
        }
        total += 1;
        if total > MAX_TOKENS {
            return Err(Diagnostic::new(
                "LIMIT",
                &token.span,
                "mobile source exceeds 8192 tokens",
            ));
        }
        if matches!(&token.kind, Kind::Sym(symbol) if matches!(symbol.as_str(), ";" | "{" | "}")) {
            segment = 0;
        } else {
            segment += 1;
            if segment > MAX_SEGMENT_TOKENS {
                return Err(Diagnostic::new(
                    "LIMIT",
                    &token.span,
                    "mobile expression/declaration exceeds 1024 tokens between statement or block boundaries",
                ));
            }
        }
    }
    Ok(())
}

fn json_pointer(response: RunResponse) -> *mut c_char {
    let json = serde_json::to_string(&response).unwrap_or_else(|_| {
        r#"{"ok":false,"exit_code":null,"output":"","diagnostic":"cannot encode mobile response"}"#.into()
    });
    match CString::new(json) {
        Ok(text) => text.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

/// ABI contract version, independent of the Cussy language version.
#[unsafe(no_mangle)]
pub extern "C" fn cussy_mobile_api_version() -> u32 {
    1
}

/// Run UTF-8 source and return an owned JSON string; free it with
/// [`cussy_mobile_free`]. No recoverable Rust panic crosses this C ABI boundary.
///
/// # Safety
/// For nonzero `source_len`, `source` must point to that many readable bytes and
/// remain valid for this call. A null pointer is accepted only for zero length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cussy_mobile_run(
    source: *const u8,
    source_len: usize,
    fuel: u64,
) -> *mut c_char {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let response = if source_len > MAX_SOURCE_BYTES {
            RunResponse::error("LIMIT", "mobile source exceeds 1 MiB")
        } else if source.is_null() && source_len != 0 {
            RunResponse::error("INPUT", "source pointer is null with nonzero length")
        } else {
            let bytes = if source_len == 0 {
                &[]
            } else {
                // SAFETY: the caller promises readable storage, checked for null
                // and bounded in length above; it is copied before worker use.
                unsafe { std::slice::from_raw_parts(source, source_len) }
            };
            run_source(bytes, fuel)
        };
        json_pointer(response)
    }));
    result.unwrap_or_else(|_| {
        catch_unwind(|| json_pointer(RunResponse::error("INTERNAL", "mobile runtime panicked")))
            .unwrap_or(ptr::null_mut())
    })
}

/// Release a result allocated by [`cussy_mobile_run`]. Null is a no-op.
///
/// # Safety
/// A non-null `result` must be a live pointer returned by this library's run
/// function. It must not have been freed before and must be freed exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cussy_mobile_free(result: *mut c_char) {
    if !result.is_null() {
        let _ = catch_unwind(AssertUnwindSafe(|| {
            // SAFETY: ownership of a live CString allocation is transferred back
            // to its original allocator under the caller's documented contract.
            drop(unsafe { CString::from_raw(result) });
        }));
    }
}
