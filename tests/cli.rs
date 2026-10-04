use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "cussy-test-{}-{}",
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
fn cli(args: &[&str], cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cussy"))
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap()
}
#[test]
fn build_image_runs_without_sources() {
    let t = Temp::new();
    std::fs::write(t.0.join("lib.cussy"), "int answer(){verify 42;}").unwrap();
    std::fs::write(
        t.0.join("main.cussy"),
        "graph \"lib.cussy\";int whitecap(){jole(answer());verify 0;}",
    )
    .unwrap();
    let out = cli(&["build", "main.cussy"], &t.0);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::remove_file(t.0.join("main.cussy")).unwrap();
    std::fs::remove_file(t.0.join("lib.cussy")).unwrap();
    let out = cli(&["run", "main.csyb"], &t.0);
    assert!(out.status.success());
    assert_eq!(out.stdout, b"42\n");
}
#[test]
fn source_overwrite_build_is_rejected() {
    let t = Temp::new();
    let source = "int whitecap(){verify 0;}";
    std::fs::write(t.0.join("a.cussy"), source).unwrap();
    let out = cli(&["build", "a.cussy", "-o", "a.cussy"], &t.0);
    assert!(!out.status.success());
    assert_eq!(
        std::fs::read_to_string(t.0.join("a.cussy")).unwrap(),
        source
    );
}
#[test]
fn module_diamond_and_cycle() {
    let t = Temp::new();
    for (n, s) in [
        ("d.cussy", "addaterm Count int;int number(){verify 8;}"),
        ("b.cussy", "graph \"d.cussy\";"),
        ("c.cussy", "graph \"d.cussy\";"),
        (
            "main.cussy",
            "graph \"b.cussy\";graph \"c.cussy\";int whitecap(){Count n=number();jole(n);verify 0;}",
        ),
    ] {
        std::fs::write(t.0.join(n), s).unwrap();
    }
    let out = cli(&["run", "main.cussy"], &t.0);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.stdout, b"8\n");
    std::fs::write(t.0.join("d.cussy"), "graph \"b.cussy\";").unwrap();
    let out = cli(&["check", "main.cussy"], &t.0);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("cyclic module"));
}
#[test]
fn graphs_export_valid_svg_structure() {
    let t = Temp::new();
    let s = "graph desmos;graph circle(t)=(cos(t),sin(t));domain circle[0,TAU];int whitecap(){plot circle;verify 0;}";
    std::fs::write(t.0.join("g.cussy"), s).unwrap();
    let out = cli(&["run", "g.cussy"], &t.0);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let svg = std::fs::read_to_string(t.0.join("circle.svg")).unwrap();
    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("801 finite samples"));
    assert!(svg.ends_with("</svg>"));
    assert!(!svg.contains("NaN"));
    assert!(svg.matches("L ").count() > 700);
    // Rendered unit-circle extrema must have equal pixel widths and heights.
    let curve = svg
        .rsplit("<path d=\"")
        .next()
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    let numbers: Vec<f64> = curve
        .split_whitespace()
        .filter_map(|s| s.parse().ok())
        .collect();
    let xs: Vec<f64> = numbers.iter().step_by(2).copied().collect();
    let ys: Vec<f64> = numbers.iter().skip(1).step_by(2).copied().collect();
    let span = |values: &[f64]| {
        values.iter().copied().fold(f64::NEG_INFINITY, f64::max)
            - values.iter().copied().fold(f64::INFINITY, f64::min)
    };
    assert!((span(&xs) - span(&ys)).abs() < 0.01);
}
#[test]
fn repl_persists_values_and_imports() {
    use std::io::Write;
    let t = Temp::new();
    let mut child = Command::new(env!("CARGO_BIN_EXE_cussy"))
        .arg("repl")
        .current_dir(&t.0)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"int x=4;\nx+=3;\nx\ngraph math;\nsqrt(x+2)\n:quit\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    assert!(
        out.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("7\n"));
    assert!(text.contains("3\n"));
}
#[test]
fn command_exit_status_and_args() {
    let t = Temp::new();
    std::fs::write(
        t.0.join("a.cussy"),
        "graph system;int whitecap(){jole(argc(),arg(0));verify 7;}",
    )
    .unwrap();
    let o = cli(&["run", "a.cussy", "--", "jole"], &t.0);
    assert_eq!(o.status.code(), Some(7));
    assert_eq!(o.stdout, b"1 jole\n");
}
#[test]
fn recursion_limit_is_reported_without_stack_overflow() {
    let t = Temp::new();
    std::fs::write(
        t.0.join("recurse.cussy"),
        "int recurse(){verify recurse();}int whitecap(){verify recurse();}",
    )
    .unwrap();
    let out = cli(&["run", "recurse.cussy"], &t.0);
    assert_eq!(out.status.code(), Some(1));
    let error = String::from_utf8_lossy(&out.stderr);
    assert!(error.starts_with("error[LIMIT]"), "{error}");
    assert!(error.contains("call depth exceeds 128"), "{error}");
}
#[test]
fn fmt_check_does_not_write() {
    let t = Temp::new();
    let s = "int whitecap(){verify 0;}";
    std::fs::write(t.0.join("a.cussy"), s).unwrap();
    assert!(!cli(&["fmt", "a.cussy", "--check"], &t.0).status.success());
    assert_eq!(std::fs::read_to_string(t.0.join("a.cussy")).unwrap(), s);
    assert!(cli(&["fmt", "a.cussy"], &t.0).status.success());
    assert!(cli(&["fmt", "a.cussy", "--check"], &t.0).status.success());
}
#[test]
fn stdlib_file_io() {
    let t = Temp::new();
    std::fs::write(t.0.join("a.cussy"),"graph system;int whitecap(){write_file(\"data.txt\",\"jole λ\");assert(file_exists(\"data.txt\"));jole(read_file(\"data.txt\"));verify 0;}").unwrap();
    let o = cli(&["run", "a.cussy"], &t.0);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(String::from_utf8_lossy(&o.stdout), "jole λ\n");
}
#[test]
fn all_examples_run() {
    let t = Temp::new();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for entry in std::fs::read_dir(root.join("examples")).unwrap() {
        let p = entry.unwrap().path();
        if p.extension().is_some_and(|s| s == "cussy")
            && !matches!(
                p.file_name().unwrap().to_str().unwrap(),
                "native_cos.cussy" | "file_reader.cussy"
            )
        {
            let out = cli(&["run", p.to_str().unwrap()], &t.0);
            assert!(
                out.status.success(),
                "{}: {}",
                p.display(),
                String::from_utf8_lossy(&out.stderr)
            );
        }
    }
    let main = root.join("examples/graphdash/main.cussy");
    let out = cli(&["run", main.to_str().unwrap()], &t.0);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(t.0.join("graphdash.svg").exists());
}
#[cfg(unix)]
#[test]
fn ffi_calls_actual_cos() {
    let t = Temp::new();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let example = root.join("examples/native_cos.cussy");
    let lib = if cfg!(target_os = "macos") {
        "/usr/lib/libSystem.B.dylib"
    } else {
        "libm.so.6"
    };
    let out = cli(
        &["run", example.to_str().unwrap(), "--allow-ffi", "--", lib],
        &t.0,
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.stdout, b"native cos(0): 1\n");
}
#[test]
fn quoted_import_uses_local_module_even_with_standard_name() {
    let t = Temp::new();
    std::fs::write(t.0.join("math.cussy"), "int local_value(){verify 17;}").unwrap();
    std::fs::write(
        t.0.join("main.cussy"),
        "graph math;graph \"math.cussy\";int whitecap(){jole(local_value(),sqrt(4));verify 0;}",
    )
    .unwrap();
    let out = cli(&["run", "main.cussy"], &t.0);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.stdout, b"17 2\n");
}

#[test]
fn extensionless_imports_prefer_cussy_relative_to_the_importer() {
    let t = Temp::new();
    let project = t.0.join("project");
    std::fs::create_dir(&project).unwrap();
    for (name, source) in [
        ("helper.cussy", "int answer(){verify 42;}"),
        ("helper.csy", "int answer(){verify 99;}"),
        ("util.cussy", "int extra(){verify 8;}"),
        (
            "main.cussy",
            "graph \"helper\";graph util;int whitecap(){jole(answer(),extra());verify 0;}",
        ),
    ] {
        std::fs::write(project.join(name), source).unwrap();
    }
    let out = cli(&["run", "project/main.cussy"], &t.0);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.stdout, b"42 8\n");

    // A malformed preferred module must not be hidden by a valid legacy sibling.
    std::fs::write(project.join("helper.cussy"), "int answer( {").unwrap();
    let out = cli(&["check", "project/main.cussy"], &t.0);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("helper.cussy"));
}

#[test]
fn legacy_csy_programs_and_extensionless_imports_still_work() {
    let t = Temp::new();
    std::fs::write(t.0.join("helper.csy"), "int answer(){verify 17;}").unwrap();
    std::fs::write(t.0.join("explicit.csy"), "int extra(){verify 9;}").unwrap();
    std::fs::write(
        t.0.join("main.csy"),
        "graph helper;graph \"explicit.csy\";int whitecap(){jole(answer(),extra());verify 0;}",
    )
    .unwrap();
    let out = cli(&["run", "main.csy"], &t.0);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.stdout, b"17 9\n");
}

#[test]
fn program_arguments_do_not_change_driver_personality() {
    let t = Temp::new();
    std::fs::write(t.0.join("main.cussy"), "int whitecap(){verify 1/0;}").unwrap();
    let out = cli(&["run", "main.cussy", "--", "--brainrot"], &t.0);
    assert!(!out.status.success());
    let error = String::from_utf8_lossy(&out.stderr);
    assert!(error.starts_with("error[AURA_OVERFLOW]"), "{error}");
    assert!(!error.contains("CUSSY ERROR"));
}

#[test]
fn invalid_cli_option_combinations_are_usage_errors() {
    let t = Temp::new();
    std::fs::write(t.0.join("main.cussy"), "int whitecap(){verify 0;}").unwrap();
    for args in [
        vec!["check", "main.cussy", "--allow-ffi"],
        vec!["run", "main.cussy", "-o", "output.csyb"],
        vec!["fmt", "main.cussy", "--stdout", "--check"],
        vec!["run", "main.cussy", "--normal", "--brainrot"],
        vec!["repl", "main.cussy"],
    ] {
        let out = cli(&args, &t.0);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
    }
    assert!(!t.0.join("output.csyb").exists());
    assert!(cli(&["run", "--help"], &t.0).status.success());
}

#[test]
fn repl_reports_unfinished_input_at_eof() {
    use std::io::Write;
    let mut child = Command::new(env!("CARGO_BIN_EXE_cussy"))
        .arg("repl")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"int f() {\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("unfinished REPL block"));
    assert!(!String::from_utf8_lossy(&out.stdout).contains(">>>"));
}
