use cussy::{
    checker::Checker,
    loader::Loader,
    runtime::{Options, Runtime},
};
use std::path::Path;

fn runtime(source: &str) -> Runtime {
    let program = Loader::default()
        .source("frames.cussy", source, Path::new("."))
        .unwrap();
    let checked = Checker::check(&program, true).unwrap();
    let mut runtime = Runtime::new(checked, Options::default());
    runtime.initialize(&program).unwrap();
    runtime
}

fn call(runtime: &mut Runtime, function: &str) -> cussy::diagnostic::Result<cussy::value::Value> {
    let span = runtime.checked.functions[function].span.clone();
    runtime.call(function, vec![], &span)
}

#[test]
fn nested_calls_share_globals_but_hide_caller_locals() {
    let mut runtime = runtime(
        "int count=0; int x=7;
         int inner(){count++;verify x;}
         int recurse(int n){int x=n;check(n==0){verify inner();}
             verify recurse(n-1)+x;}
         int whitecap(){int x=100;jole(recurse(4),x,count);verify 0;}",
    );
    call(&mut runtime, "whitecap").unwrap();
    assert_eq!(runtime.output, "17 100 1\n");
    assert_eq!(runtime.scopes.len(), 1);
}

#[test]
fn failed_nested_call_restores_frame_and_invalidates_escaped_local() {
    let mut runtime = runtime(
        "int* escaped=cooked; int x=9;
         int explode(){int x=123;escaped=&x;verify 1/0;}
         int outer(){int x=456;verify explode();}
         int read_global(){verify x;}
         int read_expired(){verify *escaped;}
         int whitecap(){verify 0;}",
    );
    assert_eq!(
        call(&mut runtime, "outer").unwrap_err().code,
        "AURA_OVERFLOW"
    );
    assert_eq!(runtime.scopes.len(), 1);
    assert!(matches!(
        call(&mut runtime, "read_global").unwrap(),
        cussy::value::Value::Int(9)
    ));
    assert_eq!(
        call(&mut runtime, "read_expired").unwrap_err().code,
        "COOKED"
    );
    assert_eq!(runtime.scopes.len(), 1);
}

#[test]
fn recursive_depth_error_restores_frame_stack() {
    // Unoptimized interpreter frames need more stack than the test harness's
    // small worker stack when deliberately exercising the full language limit.
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(|| {
            let mut runtime = runtime("int forever(){verify forever();}int whitecap(){verify 0;}");
            assert_eq!(call(&mut runtime, "forever").unwrap_err().code, "LIMIT");
            assert_eq!(runtime.scopes.len(), 1);
            call(&mut runtime, "whitecap").unwrap();
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn caller_pointer_remains_live_during_nested_call() {
    let mut runtime = runtime(
        "void inner(int* value){*value+=5;}
         void outer(int* value){int ignored=99;inner(value);}
         int whitecap(){int value=3;outer(&value);jole(value);verify 0;}",
    );
    call(&mut runtime, "whitecap").unwrap();
    assert_eq!(runtime.output, "8\n");
}
