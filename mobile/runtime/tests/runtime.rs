use cussy::{
    ast::Span,
    checker::Checked,
    loader::Loader,
    runtime::{Options, Runtime},
    value::Value,
};
use cussy_mobile::{MAX_FUEL, MAX_SOURCE_BYTES, run_source};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "cussy-mobile-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn run(source: &str) -> cussy_mobile::RunResponse {
    run_source(source.as_bytes(), 0)
}
fn capability(source: &str) {
    let result = run(source);
    assert!(!result.ok, "unexpected success: {result:?}");
    assert_eq!(result.exit_code, None);
    assert!(
        result.diagnostic.contains("error[CAPABILITY]"),
        "{}",
        result.diagnostic
    );
}

#[test]
fn embedded_math_unicode_and_nonzero_exit_are_successful() {
    let result = run("graph math; int whitecap(){ jole(\"λ 🎲\", sqrt(9)); verify 7; }");
    assert!(result.ok, "{}", result.diagnostic);
    assert_eq!(result.exit_code, Some(7));
    assert_eq!(result.output, "λ 🎲 3\n");
    assert!(result.diagnostic.is_empty());
}

#[test]
fn arrays_objects_pointers_graph_values_and_gd_remain_available() {
    let source = r#"
        graph memory;
        graph gd;
        graph desmos;
        object Box { int value; };
        int whitecap() {
            int nums[] = [3, 1, 2];
            int* p = alloc(nums[0]);
            *p += 4;
            Box box = Box { value: *p };
            free(p);
            list values = [1, 2, 3];
            Regression model = regression(values, values * 2);
            Slider control = slider(1, 0, 10, 1);
            slide(&control, 2);
            Player player = player_new();
            start_attempt(&player);
            jole(box.value, nums[1], predict(model, 4), control.value, player.attempts);
            verify 0;
        }
    "#;
    let result = run(source);
    assert!(result.ok, "{}", result.diagnostic);
    assert_eq!(result.output, "7 1 8 3 1\n");
}

#[test]
fn errors_preserve_prior_output_and_fixed_source_location() {
    let result = run("int whitecap(){jole(\"before\"); verify 1 / 0;}");
    assert!(!result.ok);
    assert_eq!(result.output, "before\n");
    assert_eq!(result.exit_code, None);
    assert!(result.diagnostic.contains("AURA_OVERFLOW"));
    assert!(result.diagnostic.contains("program.cussy:1:"));
    let empty = run("");
    assert!(empty.diagnostic.contains("program.cussy:1:1"));
    assert!(!run("int whitecap( {").ok);
}

#[test]
fn input_and_execution_limits_produce_diagnostics() {
    assert!(run_source(&[0xff], 0).diagnostic.contains("UTF8"));
    assert!(
        run_source(&vec![b' '; MAX_SOURCE_BYTES + 1], 0)
            .diagnostic
            .contains("LIMIT")
    );
    let result = run_source(b"int whitecap(){ticker(verified){}verify 0;}", 100);
    assert!(!result.ok);
    assert!(result.diagnostic.contains("LIMIT"));
    assert!(run_source(b"int whitecap(){verify 0;}", u64::MAX).ok);
    assert_eq!(MAX_FUEL, 5_000_000);
}

#[test]
fn filesystem_imports_are_rejected_before_file_loading() {
    let dir = Temp::new();
    let file = dir.0.join("private.cussy");
    std::fs::write(&file, "locked int secret = 42;").unwrap();
    let quoted = serde_json::to_string(&file.to_string_lossy()).unwrap();
    capability(&format!("graph {quoted}; int whitecap(){{verify 0;}}"));
    capability("graph unknown_module; int whitecap(){verify 0;}");
    capability("graph \"math.cussy\"; int whitecap(){verify 0;}");
    let error = Loader::embedded_only().file(&file).unwrap_err();
    assert_eq!(error.code, "CAPABILITY");
}

#[test]
fn embedded_modules_ignore_same_named_local_files() {
    let dir = Temp::new();
    std::fs::write(dir.0.join("math.cussy"), "this is not valid Cussy").unwrap();
    let program = Loader::embedded_only()
        .source(
            "program.cussy",
            "graph math; int whitecap(){jole(sqrt(9));verify 0;}",
            &dir.0,
        )
        .unwrap();
    assert!(program.sources.contains_key("std:math"));
    assert!(
        !program
            .sources
            .keys()
            .any(|name| name.contains(&dir.0.to_string_lossy().to_string()))
    );
}

#[test]
fn all_host_sinks_are_blocked_in_app_mode() {
    for source in [
        "graph system; int whitecap(){jole(read_file(\"private.txt\"));verify 0;}",
        "graph system; int whitecap(){jole(file_exists(\"private.txt\"));verify 0;}",
        "graph system; int whitecap(){jole(env(\"PATH\"));verify 0;}",
        "graph system; int whitecap(){jole(native1(\"\",\"cos\",0));verify 0;}",
        "graph time; int whitecap(){sleep(60);verify 0;}",
        "int whitecap(){jole(listen());verify 0;}",
    ] {
        capability(source);
    }
    let dir = Temp::new();
    let file = dir.0.join("unexpected.txt");
    let path = serde_json::to_string(&file.to_string_lossy()).unwrap();
    capability(&format!(
        "addaterm disguised __write_file; int whitecap(){{disguised({path},\"bad\");verify 0;}}"
    ));
    capability(&format!(
        "graph f(x) = x; int whitecap(){{plot f to {path};verify 0;}}"
    ));
    capability(&format!(
        "graph graph; int whitecap(){{point p[]=[(0,0),(1,1)];plot_points(p,{path});verify 0;}}"
    ));
    assert!(!file.exists());
}

#[test]
fn capability_checks_remain_at_direct_runtime_sinks() {
    let span = Span {
        file: "program.cussy".into(),
        line: 1,
        col: 1,
        len: 1,
    };
    let mut runtime = Runtime::new(
        Checked::default(),
        Options {
            allow_host_io: false,
            allow_ffi: true,
            echo: true,
            ..Options::default()
        },
    );
    assert_eq!(
        runtime
            .native(
                "__native1",
                vec![
                    Value::String(String::new()),
                    Value::String("cos".into()),
                    Value::Float(0.0)
                ],
                &span
            )
            .unwrap_err()
            .code,
        "CAPABILITY"
    );
    assert_eq!(
        runtime.native("listen", vec![], &span).unwrap_err().code,
        "CAPABILITY"
    );
    assert_eq!(
        runtime
            .plot("missing", "unused.svg", &span)
            .unwrap_err()
            .code,
        "CAPABILITY"
    );
    assert_eq!(
        runtime
            .emit("captured, not printed", &span)
            .unwrap_err()
            .code,
        "CAPABILITY"
    );
}

#[test]
fn simultaneous_runs_do_not_share_globals_or_configuration() {
    let source = "int count=0; int whitecap(){count++;jole(count);verify 0;}";
    let directory = std::env::current_dir().unwrap();
    let workers: Vec<_> = (0..4)
        .map(|_| std::thread::spawn(move || run(source)))
        .collect();
    for worker in workers {
        let response = worker.join().unwrap();
        assert!(response.ok, "{}", response.diagnostic);
        assert_eq!(response.output, "1\n");
    }
    assert_eq!(std::env::current_dir().unwrap(), directory);
    assert!(Options::default().allow_host_io);
}

#[test]
fn flat_expressions_and_large_programs_hit_mobile_complexity_limits() {
    let flat = format!("int whitecap(){{verify {};}}", vec!["1"; 600].join("+"));
    let result = run(&flat);
    assert!(!result.ok);
    assert!(result.diagnostic.contains("LIMIT"));
    assert!(result.diagnostic.contains("1024 tokens"));
    let large = format!("int whitecap(){{{}verify 0;}}", "jole();".repeat(2200));
    let result = run(&large);
    assert!(!result.ok);
    assert!(result.diagnostic.contains("8192 tokens"));
}

#[test]
fn large_unicode_strings_and_comments_do_not_consume_token_budget() {
    let text = "λ".repeat(70_000);
    let comment = "statement; + { } ".repeat(10_000);
    let source = format!("/* {comment} */ int whitecap(){{jole(len(\"{text}\"));verify 0;}}");
    assert!(source.len() < MAX_SOURCE_BYTES);
    let result = run(&source);
    assert!(result.ok, "{}", result.diagnostic);
    assert_eq!(result.output, "70000\n");
    let malformed = run("int whitecap(){jole(\"unterminated);}");
    assert!(malformed.diagnostic.contains("JOLE42"));
    assert!(!malformed.diagnostic.contains("tokens"));
}
