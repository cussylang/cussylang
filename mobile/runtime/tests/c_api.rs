use cussy_mobile::{
    MAX_SOURCE_BYTES, cussy_mobile_api_version, cussy_mobile_free, cussy_mobile_run,
};
use serde_json::Value;
use std::{ffi::CStr, ptr};

unsafe fn response(pointer: *const u8, length: usize, fuel: u64) -> Value {
    let result = unsafe { cussy_mobile_run(pointer, length, fuel) };
    assert!(!result.is_null());
    let json = unsafe { CStr::from_ptr(result) }
        .to_str()
        .unwrap()
        .to_owned();
    unsafe { cussy_mobile_free(result) };
    serde_json::from_str(&json).unwrap()
}

#[test]
fn c_abi_runs_utf8_and_returns_complete_owned_json() {
    let source = "graph math; int whitecap(){jole(\"λ 🎲\",sqrt(9));verify 0;}";
    assert_eq!(cussy_mobile_api_version(), 1);
    let json = unsafe { response(source.as_ptr(), source.len(), 0) };
    assert_eq!(json["ok"], true);
    assert_eq!(json["exit_code"], 0);
    assert_eq!(json["output"], "λ 🎲 3\n");
    assert_eq!(json["diagnostic"], "");
}

#[test]
fn c_abi_rejects_invalid_inputs_without_reading_them() {
    let null = unsafe { response(ptr::null(), 1, 0) };
    assert_eq!(null["ok"], false);
    assert!(null["diagnostic"].as_str().unwrap().contains("INPUT"));
    let oversized = unsafe { response(ptr::null(), MAX_SOURCE_BYTES + 1, 0) };
    assert!(oversized["diagnostic"].as_str().unwrap().contains("LIMIT"));
    let empty = unsafe { response(ptr::null(), 0, 0) };
    assert_eq!(empty["ok"], false);
    assert_eq!(empty["exit_code"], Value::Null);
    assert!(
        empty["diagnostic"]
            .as_str()
            .unwrap()
            .contains("program.cussy")
    );
    let invalid = unsafe { response([0xff].as_ptr(), 1, 0) };
    assert!(invalid["diagnostic"].as_str().unwrap().contains("UTF8"));
    unsafe { cussy_mobile_free(ptr::null_mut()) };
}

#[test]
fn c_abi_json_preserves_embedded_nul_and_runtime_output_before_error() {
    let source = b"int whitecap(){jole(\"a\\0b\");verify 1/0;}";
    let json = unsafe { response(source.as_ptr(), source.len(), 0) };
    assert_eq!(json["ok"], false);
    assert_eq!(json["exit_code"], Value::Null);
    assert_eq!(json["output"], "a\0b\n");
    assert!(
        json["diagnostic"]
            .as_str()
            .unwrap()
            .contains("AURA_OVERFLOW")
    );
}

#[test]
fn c_abi_repeated_allocations_and_frees_remain_independent() {
    let source = b"int whitecap(){jole(\"jole\");verify 9;}";
    for _ in 0..16 {
        let json = unsafe { response(source.as_ptr(), source.len(), 0) };
        assert_eq!(json["ok"], true);
        assert_eq!(json["exit_code"], 9);
        assert_eq!(json["output"], "jole\n");
    }
}
