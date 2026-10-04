use cussy::{
    checker::Checker,
    diagnostic::Diagnostic,
    loader::Loader,
    runtime::{Options, Runtime},
};
use std::path::Path;
fn compile(src: &str) -> Result<(cussy::ast::Program, cussy::checker::Checked), Diagnostic> {
    let p = Loader::default().source("test.cussy", src, Path::new("."))?;
    let c = Checker::check(&p, true)?;
    Ok((p, c))
}
fn run(src: &str) -> Result<(i64, String), Diagnostic> {
    let (p, c) = compile(src)?;
    let mut r = Runtime::new(c, Options::default());
    let code = r.run(&p)?;
    Ok((code, r.output))
}
fn output(src: &str) -> String {
    run(src).unwrap().1
}
fn fails(src: &str, code: &str) {
    let err = run(src).unwrap_err();
    assert_eq!(err.code, code, "{err}");
}
#[test]
fn recursion_and_forward_calls() {
    assert_eq!(
        output(
            "int whitecap(){jole(f(10));verify 0;} int f(int n){check(n<2){verify n;}verify f(n-1)+f(n-2);}"
        ),
        "55\n"
    );
}
#[test]
fn loops_continue_and_break() {
    assert_eq!(
        output(
            "int whitecap(){int s=0;attempt(int i=0;i<10;i++){check(i%2==0){noclip;}check(i==7){crash;}s+=i;}jole(s);verify 0;}"
        ),
        "9\n"
    );
}
#[test]
fn loop_iteration_scopes() {
    assert_eq!(
        output("int whitecap(){int i=0;ticker(i++<3) int x=2;int x=7;jole(x);verify 0;}"),
        "7\n"
    );
}
#[test]
fn short_circuit_and_lazy_piecewise() {
    assert_eq!(
        output(
            "int whitecap(){int n=0;bool a=unverified && ++n>0;bool b=verified || ++n>0;jole(n,verified ? 4 : 1/0);verify 0;}"
        ),
        "0 4\n"
    );
}
#[test]
fn lexical_not_dynamic_scope() {
    assert_eq!(
        output("int x=1;int f(){verify x;}int whitecap(){int x=2;jole(f());verify 0;}"),
        "1\n"
    );
}
#[test]
fn arithmetic_precedence() {
    assert_eq!(
        output("int whitecap(){jole(2+3*4,2^3^2,-2^2);int a=3;int* p=&a;jole(*p * 4);verify 0;}"),
        "14 512 -4\n12\n"
    );
}
#[test]
fn exact_integer_arithmetic() {
    assert_eq!(
        output("int whitecap(){int a=9007199254740993;jole(a+2,a>9007199254740992,7/2);verify 0;}"),
        "9007199254740995 verified 3\n"
    );
}
#[test]
fn unsigned_full_width() {
    assert_eq!(
        output(
            "int whitecap(){unsigned long a=18446744073709551614u;a++;yap(\"%u\\n\",a);verify 0;}"
        ),
        "18446744073709551615\n"
    );
}
#[test]
fn overflow_diagnostics() {
    fails(
        "int whitecap(){jole(9223372036854775807+1);verify 0;}",
        "AURA_OVERFLOW",
    );
    fails("int whitecap(){jole(1/0);verify 0;}", "AURA_OVERFLOW");
}
#[test]
fn unsigned_negative_rejected() {
    fails(
        "int whitecap(){unsigned long n=-1;verify 0;}",
        "AURA_OVERFLOW",
    );
}
#[test]
fn lexer_unicode_and_escapes() {
    assert_eq!(
        output(
            "/* nested /* yes */ okay */ int whitecap(){string s=\"jole 🎲\\n\";char c='λ';jole(len(s),c,s[5]);verify 0;}"
        ),
        "7 λ 🎲\n"
    );
}
#[test]
fn unterminated_literal_and_comment() {
    fails("int whitecap(){jole(\"oops);}", "JOLE42");
    fails("/* oops", "JOLE42");
}
#[test]
fn locked_field_is_locked() {
    fails(
        "object C{int a;};int whitecap(){locked C c=C{a:1};c.a=2;verify 0;}",
        "LOCKED",
    );
}
#[test]
fn cannot_take_mutable_address_of_locked() {
    fails(
        "int whitecap(){locked int a=1;int* p=&a;verify 0;}",
        "LOCKED",
    );
}
#[test]
fn missing_return_and_bad_return() {
    fails("int whitecap(){check(verified){verify 0;}}", "WASHED");
    fails("int whitecap(){verify \"no\";}", "AURA-12");
}
#[test]
fn undefined_and_wrong_arity() {
    fails("int whitecap(){jole(ghost);verify 0;}", "D404");
    fails(
        "int f(int x){verify x;}int whitecap(){verify f();}",
        "AURA-12",
    );
}
#[test]
fn check_dead_branch_types() {
    fails(
        "int whitecap(){check(unverified){int x=\"no\";}verify 0;}",
        "AURA-12",
    );
}
#[test]
fn illegal_loop_control() {
    fails("int whitecap(){noclip;verify 0;}", "JOLE42");
    fails("int whitecap(){crash;verify 0;}", "JOLE42");
}
#[test]
fn duplicate_declarations() {
    fails("int whitecap(){int x=1;int x=2;verify 0;}", "JOLE42");
    fails("object C{int x;int x;};int whitecap(){verify 0;}", "JOLE42");
}
#[test]
fn pointers_and_aliasing() {
    assert_eq!(
        output(
            "void bump(int* p){*p+=2;}int whitecap(){int x=4;int* p=&x;int* q=p;bump(q);jole(x,*p);verify 0;}"
        ),
        "6 6\n"
    );
}
#[test]
fn dangling_stack_pointer() {
    fails(
        "int* f(){int x=2;verify &x;}int whitecap(){int* p=f();jole(*p);verify 0;}",
        "COOKED",
    );
}
#[test]
fn escaped_block_pointer() {
    fails(
        "int whitecap(){int* p=cooked;{int x=2;p=&x;}jole(*p);verify 0;}",
        "COOKED",
    );
}
#[test]
fn null_and_double_free() {
    fails("int whitecap(){int* p=cooked;jole(*p);verify 0;}", "COOKED");
    fails(
        "int whitecap(){int* p=alloc(1);free(p);free(p);verify 0;}",
        "COOKED",
    );
}
#[test]
fn stack_free_is_rejected() {
    fails("int whitecap(){int x=2;free(&x);verify 0;}", "COOKED");
}
#[test]
fn use_after_free_through_alias() {
    fails(
        "int whitecap(){int* p=alloc(1);int* q=p;free(p);jole(*q);verify 0;}",
        "COOKED",
    );
}
#[test]
fn copied_arrays_do_not_alias() {
    assert_eq!(
        output("int whitecap(){int a[]=[1,2];int b[]=a;b[0]=9;jole(a,b);verify 0;}"),
        "[1, 2] [9, 2]\n"
    );
}
#[test]
fn record_copy_and_field_address() {
    assert_eq!(
        output(
            "object C{int a;};int whitecap(){C c=C{a:1};int* p=&c.a;c=C{a:7};C d=c;d.a=9;jole(*p,c.a,d.a);verify 0;}"
        ),
        "7 7 9\n"
    );
}
#[test]
fn recursive_object_uses_pointer() {
    assert!(compile("object Node{int value;Node* next;};int whitecap(){Node n=Node{value:1,next:cooked};verify 0;}").is_ok());
    fails(
        "object Node{Node next;};int whitecap(){verify 0;}",
        "AURA-12",
    );
}
#[test]
fn array_bounds_and_sizes() {
    fails("int whitecap(){int a[2]=[1];verify 0;}", "DOMAIN");
    fails("int whitecap(){int a[2];jole(a[2]);verify 0;}", "DOMAIN");
    fails(
        "int whitecap(){int a[]=[1];jole(a[-1]);verify 0;}",
        "DOMAIN",
    );
}
#[test]
fn array_pointer_mutates_caller() {
    assert_eq!(
        output("void f(int[]* a){(*a)[0]=8;}int whitecap(){int a[]=[1];f(&a);jole(a);verify 0;}"),
        "[8]\n"
    );
}
#[test]
fn list_broadcast_and_list_zip() {
    assert_eq!(
        output("int whitecap(){list a=[1,2,3];list b=2*a+a;jole(b);verify 0;}"),
        "[3, 6, 9]\n"
    );
    fails(
        "int whitecap(){list a=[1];list b=[1,2];jole(a+b);verify 0;}",
        "DOMAIN",
    );
}
#[test]
fn point_vector_arithmetic() {
    assert_eq!(
        output("graph graph;int whitecap(){vector a=(3,4);jole(magnitude(a),a*2);verify 0;}"),
        "5 (6, 8)\n"
    );
}
#[test]
fn aliases_types_constants_functions() {
    assert_eq!(
        output(
            "addaterm count unsigned long;addaterm jole string;addaterm say = jole;addaterm PAPA \"whitecaplol\";int whitecap(){count n=2;jole s=PAPA;say(s,n);verify 0;}"
        ),
        "whitecaplol 2\n"
    );
}
#[test]
fn trigger_has_no_fallthrough() {
    assert_eq!(
        output(
            "int whitecap(){trigger(2){checkpoint 1:jole(1);checkpoint 2:jole(2);practice:jole(3);}verify 0;}"
        ),
        "2\n"
    );
}
#[test]
fn duplicate_switch_labels() {
    fails(
        "int whitecap(){trigger(1){checkpoint 1:crash;checkpoint 1:crash;}verify 0;}",
        "JOLE42",
    );
}
#[test]
fn stream_scope_restores_on_return() {
    assert_eq!(
        output(
            "graph stream;void f(){stream{assert(streamstatus());verify;}}int whitecap(){f();jole(streamstatus());verify 0;}"
        ),
        "unverified\n"
    );
}
#[test]
fn optional_brainrot() {
    assert!(compile("int whitecap(){verify 0;}").is_ok());
    fails("int whitecap(){papa();verify 0;}", "D404");
    assert_eq!(
        output(
            "graph brainrot;int whitecap(){aura_loss(125);jole(washed(),aura_check());verify 0;}"
        ),
        "verified -25\n"
    );
}
#[test]
fn standard_library_numeric_and_string() {
    assert_eq!(
        output(
            "graph string;graph math;int whitecap(){jole(slice(\"aλc\",1,1),uppercase(\"jole\"),sqrt(9));verify 0;}"
        ),
        "λ JOLE 3\n"
    );
}
#[test]
fn regression_has_known_solution() {
    assert_eq!(
        output(
            "graph desmos;int whitecap(){Regression r=regression([1,2,3],[3,5,7]);jole(r.slope,r.intercept,r.r2);verify 0;}"
        ),
        "2 1 1\n"
    );
    fails(
        "graph desmos;int whitecap(){Regression r=regression([1,1],[2,3]);verify 0;}",
        "DOMAIN",
    );
}
#[test]
fn gd_collision_practice_and_noclip() {
    assert_eq!(
        output(
            "graph gd;int whitecap(){Player p=player_new();Hitbox a=Hitbox{x:0,y:0,width:2,height:2};Hitbox b=Hitbox{x:1,y:1,width:1,height:1};assert(hits(p,a,b));set_noclip(&p,verified);assert(!hits(p,a,b));set_practice(&p,verified);advance(&p,42);set_checkpoint(&p);start_attempt(&p);jole(p.percent,p.attempts);verify 0;}"
        ),
        "42 1\n"
    );
}
#[test]
fn fuel_stops_infinite_loop() {
    let (p, c) = compile("int whitecap(){ticker(verified){}verify 0;}").unwrap();
    let mut r = Runtime::new(
        c,
        Options {
            fuel: 100,
            ..Options::default()
        },
    );
    assert_eq!(r.run(&p).unwrap_err().code, "LIMIT");
}
#[test]
fn ffi_requires_explicit_capability() {
    fails(
        "graph system;int whitecap(){jole(native1(\"\",\"cos\",0));verify 0;}",
        "FFI",
    );
}
#[test]
fn formatter_preserves_behavior_and_comments() {
    let source =
        "// keep me\nint whitecap(){string s=\"{ ; }\";/* hello */jole(s,2+3*4);verify 0;}";
    let formatted = cussy::formatter::format("fmt", source).unwrap();
    assert_eq!(output(source), output(&formatted));
    assert!(formatted.contains("// keep me"));
    assert_eq!(
        formatted,
        cussy::formatter::format("fmt", &formatted).unwrap()
    );
}
#[test]
fn diagnostic_has_file_location_explanation() {
    let e = compile("int whitecap(){verify \"no\";}").err().unwrap();
    let rendered = e.render(
        &std::collections::BTreeMap::from([(
            "test.cussy".into(),
            "int whitecap(){verify \"no\";}".into(),
        )]),
        "brainrot",
    );
    assert!(rendered.contains("catastrophic aura loss"));
    assert!(rendered.contains("technical explanation"));
    assert!(rendered.contains("test.cussy:1:"));
    assert!(rendered.contains('^'));
}
#[test]
fn mixed_unsigned_operations_are_checked_and_exact() {
    assert_eq!(
        output(
            "int whitecap(){unsigned long n=9007199254740993u;n+=2;jole(n>1000,n==9007199254740995,n==9007199254740994);verify 0;}"
        ),
        "verified verified unverified\n"
    );
    fails(
        "int whitecap(){unsigned long n=1;jole(n + -1);verify 0;}",
        "AURA_OVERFLOW",
    );
}
#[test]
fn boolean_return_and_equality_conversion() {
    assert_eq!(
        run("int whitecap(){assert(1==verified);verify verified;}")
            .unwrap()
            .0,
        1
    );
}

#[test]
fn shadowed_alloc_is_not_callable() {
    let err = compile("int whitecap(){int alloc=1;int* p=alloc(2);verify 0;}")
        .err()
        .expect("a variable must not inherit a shadowed builtin's call behavior");
    assert_eq!(err.code, "AURA-12");
}

#[test]
fn builtin_record_result_requires_its_type_definition() {
    let err = compile("int whitecap(){jole(__regress([1,2],[3,4]).slope);verify 0;}")
        .err()
        .expect("Regression must be imported before it can be used");
    assert_eq!(err.code, "D404");
}

#[test]
fn duplicate_switch_labels_compare_values() {
    fails(
        "int whitecap(){trigger(1){checkpoint 1:crash;checkpoint verified:crash;}verify 0;}",
        "JOLE42",
    );
    fails(
        "int whitecap(){trigger(1u){checkpoint 1:crash;checkpoint 1u:crash;}verify 0;}",
        "JOLE42",
    );
}

#[test]
fn enum_maximum_needs_no_following_value() {
    assert_eq!(
        output(
            "lore Limits{MAX=9223372036854775807,RESET=0,NEXT,};int whitecap(){jole(MAX,NEXT);verify 0;}"
        ),
        "9223372036854775807 1\n"
    );
    assert!(compile("lore Limits{MAX=9223372036854775807,};int whitecap(){verify 0;}").is_ok());
    fails(
        "lore Limits{MAX=9223372036854775807,NEXT};int whitecap(){verify 0;}",
        "AURA_OVERFLOW",
    );
}
