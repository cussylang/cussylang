use crate::{
    ast::Span,
    diagnostic::{Diagnostic, Result},
};
#[cfg(unix)]
pub fn native1(library: &str, symbol: &str, arg: f64, s: &Span) -> Result<f64> {
    use std::ffi::{CString, c_char, c_int, c_void};
    #[cfg_attr(target_os = "linux", link(name = "dl"))]
    unsafe extern "C" {
        fn dlopen(path: *const c_char, flags: c_int) -> *mut c_void;
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
        fn dlclose(handle: *mut c_void) -> c_int;
        fn dlerror() -> *const c_char;
    }
    let lib = CString::new(library)
        .map_err(|_| Diagnostic::new("FFI", s, "library path contains NUL"))?;
    let sym = CString::new(symbol).map_err(|_| Diagnostic::new("FFI", s, "symbol contains NUL"))?;
    // Explicit --allow-ffi capability. The caller must supply a double(double) ABI symbol.
    unsafe {
        let handle = dlopen(
            if library.is_empty() {
                std::ptr::null()
            } else {
                lib.as_ptr()
            },
            2,
        );
        if handle.is_null() {
            let e = dlerror();
            let msg = if e.is_null() {
                "dlopen failed".into()
            } else {
                std::ffi::CStr::from_ptr(e).to_string_lossy().into_owned()
            };
            return Err(Diagnostic::new("FFI", s, msg));
        }
        let ptr = dlsym(handle, sym.as_ptr());
        if ptr.is_null() {
            dlclose(handle);
            return Err(Diagnostic::new(
                "FFI",
                s,
                format!("symbol `{symbol}` not found"),
            ));
        }
        let function: unsafe extern "C" fn(f64) -> f64 = std::mem::transmute(ptr);
        let result = function(arg);
        dlclose(handle);
        Ok(result)
    }
}
#[cfg(not(unix))]
pub fn native1(_library: &str, _symbol: &str, _arg: f64, s: &Span) -> Result<f64> {
    Err(Diagnostic::new(
        "FFI",
        s,
        "dynamic FFI currently supports Unix platforms only",
    ))
}
