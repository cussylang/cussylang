use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::{
        OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
};

static NEXT: AtomicUsize = AtomicUsize::new(0);
static COMPILER: OnceLock<Option<OsString>> = OnceLock::new();

struct Temp(PathBuf);

impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "cussy-native-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn source(&self, source: &str) -> PathBuf {
        let path = self.0.join("main.cussy");
        std::fs::write(&path, source).unwrap();
        path
    }

    fn executable(&self) -> PathBuf {
        self.0.join(if cfg!(windows) {
            "program.exe"
        } else {
            "program"
        })
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn native_available() -> Option<&'static OsString> {
    COMPILER
        .get_or_init(|| {
            let compiler = std::env::var_os("CUSSY_CC").unwrap_or_else(|| "clang".into());
            if Command::new(&compiler)
                .arg("--version")
                .output()
                .is_ok_and(|output| output.status.success())
            {
                Some(compiler)
            } else {
                assert_ne!(
                    std::env::var("CUSSY_REQUIRE_NATIVE").as_deref(),
                    Ok("1"),
                    "CUSSY_REQUIRE_NATIVE=1 requires a working clang or CUSSY_CC compiler"
                );
                eprintln!("skipping native execution checks: clang/CUSSY_CC is unavailable");
                None
            }
        })
        .as_ref()
}

fn driver(arguments: &[&str], cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cussy"))
        .args(arguments)
        .current_dir(cwd)
        .output()
        .unwrap()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "status={:?}\nstdout={}\nstderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn compile(temp: &Temp, source: &Path, compiler: &OsString, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cussy"))
        .arg("compile")
        .arg(source)
        .arg("-o")
        .arg(temp.executable())
        .arg("--cc")
        .arg(compiler)
        .args(extra)
        .current_dir(&temp.0)
        .output()
        .unwrap()
}

fn compare_execution(temp: &Temp, source: &str, compiler: &OsString, extra: &[&str]) {
    let path = temp.source(source);
    let interpreted = driver(
        &["run", path.to_str().unwrap(), "--fuel", "50000000"],
        &temp.0,
    );
    assert_success(&compile(temp, &path, compiler, extra));
    let native = Command::new(temp.executable())
        .current_dir(&temp.0)
        .output()
        .unwrap();
    assert_eq!(native.status.code(), interpreted.status.code());
    assert_eq!(
        native.stdout,
        interpreted.stdout,
        "native stderr: {}",
        String::from_utf8_lossy(&native.stderr)
    );
    assert!(interpreted.stderr.is_empty(), "{interpreted:?}");
    assert!(native.stderr.is_empty(), "{native:?}");
}

#[test]
fn scalar_benchmarks_match_interpreted_execution() {
    let Some(compiler) = native_available() else {
        return;
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for case in ["fibonacci", "integer_recurrence", "primes"] {
        let source = std::fs::read_to_string(
            root.join("benchmarks/languages")
                .join(format!("{case}.cussy")),
        )
        .unwrap();
        compare_execution(&Temp::new(), &source, compiler, &[]);
    }
}

#[test]
fn side_effects_keep_left_to_right_order_and_short_circuit() {
    let Some(compiler) = native_available() else {
        return;
    };
    let source = r#"
        int next=0;
        int take(){next++;verify next;}
        int pair(int a,int b){verify a*10+b;}
        int whitecap(){
            jole(pair(take(),take()),next);
            int x=4;
            jole(x++,++x,x);
            x=1;
            x+=(x=5);
            jole(x);
            jole(take()+take(),next);
            bool ignored=unverified && take()>0;
            bool accepted=verified || take()>0;
            jole(ignored,accepted,verified ? take() : 1/0,next);
            verify 7;
        }
    "#;
    // Optimizations must preserve the language's evaluation order too.
    for optimization in ["-O0", "-O3"] {
        compare_execution(&Temp::new(), source, compiler, &[optimization]);
    }
}

#[test]
fn globals_aliases_shadowing_and_math_match_interpreter() {
    let Some(compiler) = native_available() else {
        return;
    };
    compare_execution(
        &Temp::new(),
        r#"
            graph math;
            addaterm Count int;
            addaterm say = jole;
            Count count=4;
            Count twice(Count x){verify x*2;}
            addaterm double_it = twice;
            Count initialized=double_it(count);
            int read_global(){verify count;}
            int whitecap(){
                int count=99;
                {int count=2;say(count);}
                say(count,read_global(),initialized,sqrt(9),pow(2,3));
                verify 0;
            }
        "#,
        compiler,
        &[],
    );
}

#[test]
fn void_aliases_and_function_values_preserve_types_and_side_effects() {
    let Some(compiler) = native_available() else {
        return;
    };
    compare_execution(
        &Temp::new(),
        r#"
            void f(){jole("called");verify;}
            void g(){verify;}
            addaterm alias=f;
            addaterm nothing=g();
            int whitecap(){
                jole(f,alias,f==alias,f==g);
                yap("%v\n",f);
                jole(hitbox(nothing),f()==g());
                verify 0;
            }
        "#,
        compiler,
        &[],
    );
}

#[test]
fn signed_zero_math_and_decimal_conversion_match_interpreter() {
    let Some(compiler) = native_available() else {
        return;
    };
    let source = r#"
        graph math;
        int whitecap(){
            jole(min(-0.0,0.0),min(0.0,-0.0),max(-0.0,0.0),max(0.0,-0.0));
            jole(to_float("+001.e+2"),to_float("-0"),to_float(".1"));
            jole(to_float("1.e2"),to_float("1e-9999"));
            jole(to_int("-9223372036854775808"),to_int("+9223372036854775807"));
            verify 0;
        }
    "#;
    for optimization in ["-O0", "-O3"] {
        compare_execution(&Temp::new(), source, compiler, &[optimization]);
    }
}

#[test]
fn loops_switch_break_and_continue_match_interpreter() {
    let Some(compiler) = native_available() else {
        return;
    };
    compare_execution(
        &Temp::new(),
        r#"
            int whitecap(){
                int sum=0;
                attempt(int i=0;i<8;i++){
                    trigger(i){
                        checkpoint 2:noclip;
                        checkpoint 5:sum+=50;crash;
                        practice:sum+=i;
                    }
                    check(i==6){crash;}
                }
                int n=0;
                ticker(n<4){n++;check(n==2){noclip;}sum+=n;}
                jole(sum,n);
                verify 0;
            }
        "#,
        compiler,
        &[],
    );
}

#[test]
fn numeric_unicode_and_formatted_output_match_interpreter() {
    let Some(compiler) = native_available() else {
        return;
    };
    compare_execution(
        &Temp::new(),
        r#"
            int whitecap(){
                int exact=9007199254740993;
                unsigned long full=18446744073709551615u;
                string text="jole λ";
                char ch='λ';
                yap("%d %u %.2f %s %c %b %%\n",exact,full,2.5,text,ch,verified);
                jole(exact+2,exact>9007199254740992,7/2,text,ch);
                verify 0;
            }
        "#,
        compiler,
        &[],
    );
}

#[test]
fn floating_point_display_matches_interpreter_at_rounding_boundaries() {
    let Some(compiler) = native_available() else {
        return;
    };
    compare_execution(
        &Temp::new(),
        r#"
            int whitecap(){
                jole(0.0000000298023223876953125,-0.0000000298023223876953125);
                jole(1e20,1e-20,-0.0);
                jole(1.7976931348623157e308,5e-324);
                verify 0;
            }
        "#,
        compiler,
        &[],
    );
}

#[test]
fn string_nuls_unicode_indices_and_mixed_integer_comparisons_match_interpreter() {
    let Some(compiler) = native_available() else {
        return;
    };
    compare_execution(
        &Temp::new(),
        r#"
            int whitecap(){
                string embedded="a\0λ";
                jole(len(embedded),embedded[1],embedded[2],embedded=="a\0λ");
                yap("%s",embedded);
                unsigned long exact=9007199254740993u;
                jole(exact==9007199254740993,exact>9007199254740992);
                jole(exact<18446744073709551615u,to_int(exact),to_int(verified));
                verify 0;
            }
        "#,
        compiler,
        &[],
    );
}

#[test]
fn initialization_and_recursion_errors_match_interpreter() {
    let Some(compiler) = native_available() else {
        return;
    };
    for (source, code) in [
        (
            "int first=read_later();int later=2;int read_later(){verify later;}int whitecap(){verify 0;}",
            "D404",
        ),
        (
            "int recurse(){verify recurse();}int whitecap(){verify recurse();}",
            "LIMIT",
        ),
        (
            "int whitecap(){jole(to_int(18446744073709551615u));verify 0;}",
            "DOMAIN",
        ),
    ] {
        let temp = Temp::new();
        let path = temp.source(source);
        let interpreted = driver(&["run", path.to_str().unwrap()], &temp.0);
        assert_success(&compile(&temp, &path, compiler, &[]));
        let native = Command::new(temp.executable()).output().unwrap();
        assert_eq!(
            interpreted.status.code(),
            Some(1),
            "{source}: {interpreted:?}"
        );
        assert_eq!(native.status.code(), Some(1), "{source}: {native:?}");
        assert_eq!(native.stdout, interpreted.stdout);
        assert!(String::from_utf8_lossy(&interpreted.stderr).contains(code));
        assert!(
            String::from_utf8_lossy(&native.stderr).contains(code),
            "{native:?}"
        );
    }
}

#[test]
fn arithmetic_errors_exit_with_diagnostics_instead_of_undefined_behavior() {
    let Some(compiler) = native_available() else {
        return;
    };
    for expression in [
        "9223372036854775807+1",
        "1/0",
        "(-9223372036854775807-1)/-1",
        "18446744073709551615u+1u",
    ] {
        let temp = Temp::new();
        let path = temp.source(&format!("int whitecap(){{jole({expression});verify 0;}}"));
        let interpreted = driver(&["run", path.to_str().unwrap()], &temp.0);
        assert_success(&compile(&temp, &path, compiler, &[]));
        let native = Command::new(temp.executable()).output().unwrap();
        assert_eq!(interpreted.status.code(), Some(1));
        assert_eq!(native.status.code(), Some(1), "{expression}: {native:?}");
        assert_eq!(native.stdout, interpreted.stdout);
        assert!(
            String::from_utf8_lossy(&native.stderr).contains("AURA_OVERFLOW"),
            "{expression}: {native:?}"
        );
    }
}

#[test]
fn compiled_executable_runs_after_all_source_files_are_removed() {
    let Some(compiler) = native_available() else {
        return;
    };
    let temp = Temp::new();
    let dependency = temp.0.join("helper.cussy");
    std::fs::write(&dependency, "int answer(){verify 42;}").unwrap();
    let path = temp.source("graph \"helper.cussy\";int whitecap(){jole(answer());verify 9;}");
    assert_success(&compile(&temp, &path, compiler, &[]));
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file(dependency).unwrap();
    let native = Command::new(temp.executable())
        .current_dir(&temp.0)
        .env("PATH", "")
        .output()
        .unwrap();
    assert_eq!(native.status.code(), Some(9));
    assert_eq!(native.stdout, b"42\n");
    assert!(native.stderr.is_empty());
}

#[test]
fn native_arguments_match_interpreter_for_unicode_empty_and_quoted_values() {
    let Some(compiler) = native_available() else {
        return;
    };
    let temp = Temp::new();
    let path = temp.source(
        r#"
            graph system;
            int whitecap(){
                jole(argc());
                attempt(int i=0;i<argc();i++){
                    string value=arg(i);
                    yap("[%d:%s]\n",len(value),value);
                }
                verify 0;
            }
        "#,
    );
    let arguments = [
        "λ 🎲",
        "",
        "two words",
        "a\"quote\"b",
        "ends-with-backslash\\",
        r"two trailing \\",
        "slash-before-quote\\\"",
        "--brainrot",
    ];
    let interpreted = Command::new(env!("CARGO_BIN_EXE_cussy"))
        .arg("run")
        .arg(&path)
        .arg("--")
        .args(arguments)
        .current_dir(&temp.0)
        .output()
        .unwrap();
    assert_success(&interpreted);
    assert_success(&compile(&temp, &path, compiler, &[]));
    let native = Command::new(temp.executable())
        .args(arguments)
        .current_dir(&temp.0)
        .output()
        .unwrap();
    assert_success(&native);
    assert_eq!(native.stdout, interpreted.stdout);
    assert!(interpreted.stderr.is_empty(), "{interpreted:?}");
    assert!(native.stderr.is_empty(), "{native:?}");
}

#[test]
fn assembly_and_object_emission_produce_nonempty_artifacts() {
    let Some(compiler) = native_available() else {
        return;
    };
    let temp = Temp::new();
    let path = temp.source("int whitecap(){jole(42);verify 0;}");
    for (kind, filename) in [("asm", "program.s"), ("obj", "program.o")] {
        let output = temp.0.join(filename);
        let result = Command::new(env!("CARGO_BIN_EXE_cussy"))
            .arg("compile")
            .arg(&path)
            .arg("--emit")
            .arg(kind)
            .arg("-o")
            .arg(&output)
            .arg("--cc")
            .arg(compiler)
            .current_dir(&temp.0)
            .output()
            .unwrap();
        assert_success(&result);
        assert!(std::fs::metadata(output).unwrap().len() > 0);
    }
}

#[test]
fn c_emission_does_not_require_an_installed_compiler() {
    let temp = Temp::new();
    temp.source("int whitecap(){jole(42);verify 0;}");
    let result = driver(
        &[
            "compile",
            "main.cussy",
            "--emit",
            "c",
            "-o",
            "program.c",
            "--cc",
            "definitely-missing-cussy-compiler",
        ],
        &temp.0,
    );
    assert_success(&result);
    assert!(
        !std::fs::read_to_string(temp.0.join("program.c"))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn unsupported_programs_fail_explicitly_without_interpreter_fallback() {
    let temp = Temp::new();
    temp.source("int whitecap(){list values=[1,2];jole(values);verify 0;}");
    let result = driver(
        &["compile", "main.cussy", "--emit", "c", "-o", "program.c"],
        &temp.0,
    );
    assert_eq!(result.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&result.stderr).contains("NATIVE_UNSUPPORTED"));
    assert!(!temp.0.join("program.c").exists());
}

#[test]
fn invalid_native_flags_are_usage_errors() {
    let temp = Temp::new();
    temp.source("int whitecap(){verify 0;}");
    for options in [
        vec!["--emit", "not-a-format"],
        vec!["-O9"],
        vec!["--cpu", "not-a-cpu-mode"],
        vec!["--cc"],
        vec!["--fuel", "100"],
    ] {
        let mut arguments = vec!["compile", "main.cussy"];
        arguments.extend(options);
        let result = driver(&arguments, &temp.0);
        assert_eq!(result.status.code(), Some(2), "{arguments:?}: {result:?}");
    }
}

#[test]
fn failed_compiler_preserves_existing_output_and_source() {
    let temp = Temp::new();
    let source = "int whitecap(){verify 0;}";
    let path = temp.source(source);
    std::fs::write(temp.executable(), b"existing executable bytes").unwrap();
    // Cussy itself is an existing executable that rejects C compiler arguments.
    let compiler = OsString::from(env!("CARGO_BIN_EXE_cussy"));
    let result = compile(&temp, &path, &compiler, &[]);
    assert_eq!(result.status.code(), Some(1));
    assert_eq!(std::fs::read_to_string(path).unwrap(), source);
    assert_eq!(
        std::fs::read(temp.executable()).unwrap(),
        b"existing executable bytes"
    );
}

#[test]
fn native_output_cannot_overwrite_main_or_imported_source() {
    let temp = Temp::new();
    let source = "graph \"helper.cussy\";int whitecap(){verify answer();}";
    let helper = "int answer(){verify 0;}";
    temp.source(source);
    std::fs::write(temp.0.join("helper.cussy"), helper).unwrap();
    for output in ["main.cussy", "helper.cussy"] {
        let result = driver(
            &["compile", "main.cussy", "--emit", "c", "-o", output],
            &temp.0,
        );
        assert_eq!(result.status.code(), Some(1), "{output}: {result:?}");
        assert_eq!(
            std::fs::read_to_string(temp.0.join("main.cussy")).unwrap(),
            source
        );
        assert_eq!(
            std::fs::read_to_string(temp.0.join("helper.cussy")).unwrap(),
            helper
        );
    }
}
