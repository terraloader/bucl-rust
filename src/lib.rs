/// WASM library entry point for BUCL.
///
/// Exposes three C-ABI functions that JavaScript can call directly after
/// instantiating the `.wasm` module:
///
/// | Function | Description |
/// |---|---|
/// | `bucl_alloc(size) -> *mut u8` | Allocate `size` bytes; JS writes source here |
/// | `bucl_free(ptr, size)` | Free a buffer previously returned by this module |
/// | `bucl_run(src_ptr, src_len) -> *mut u8` | Run BUCL; returns `[u32-le len][utf-8 bytes]` |
///
/// The standard library BUCL functions (`strpos`, `substr`, `reverse`,
/// `explode`, `implode`, `maxlength`, `slice`) are embedded at compile time
/// via `include_str!` so they are available without a filesystem.
///
/// On WASM the `random` function needs a `js_math_random` import from the host
/// (see `demo/index.html` for the JS glue).

mod ast;
mod error;
mod evaluator;
mod functions;
mod lexer;
mod parser;

use std::alloc::{alloc, dealloc, Layout};

use evaluator::Evaluator;

// ---------------------------------------------------------------------------
// Exported C-ABI surface
// ---------------------------------------------------------------------------

/// Allocate a byte buffer of `size` bytes and return its pointer.
/// The caller is responsible for freeing it with `bucl_free`.
#[no_mangle]
pub extern "C" fn bucl_alloc(size: usize) -> *mut u8 {
    let layout = Layout::from_size_align(size, 1).expect("invalid layout");
    unsafe { alloc(layout) }
}

/// Free a buffer previously returned by `bucl_alloc` or `bucl_run`.
#[no_mangle]
pub extern "C" fn bucl_free(ptr: *mut u8, size: usize) {
    if ptr.is_null() || size == 0 {
        return;
    }
    let layout = Layout::from_size_align(size, 1).expect("invalid layout");
    unsafe { dealloc(ptr, layout) };
}

/// Run a BUCL script.
///
/// * `src_ptr` — pointer to UTF-8 encoded source (allocated by `bucl_alloc`).
/// * `src_len` — byte length of the source.
///
/// Returns a pointer to a buffer with layout:
/// ```
/// [4 bytes little-endian u32 = output_len][output_len bytes of UTF-8]
/// ```
/// The caller must free the returned pointer with `bucl_free(ptr, 4 + output_len)`.
#[no_mangle]
pub extern "C" fn bucl_run(src_ptr: *const u8, src_len: usize) -> *mut u8 {
    let source = unsafe {
        let slice = std::slice::from_raw_parts(src_ptr, src_len);
        std::str::from_utf8(slice).unwrap_or("")
    };

    let output = run_internal(source);
    let out_bytes = output.as_bytes();
    let total = 4 + out_bytes.len();

    let layout = Layout::from_size_align(total, 1).expect("invalid layout");
    let ptr = unsafe { alloc(layout) };

    let len_bytes = (out_bytes.len() as u32).to_le_bytes();
    unsafe {
        std::ptr::copy_nonoverlapping(len_bytes.as_ptr(), ptr, 4);
        std::ptr::copy_nonoverlapping(out_bytes.as_ptr(), ptr.add(4), out_bytes.len());
    }

    ptr
}

// ---------------------------------------------------------------------------
// Internal engine
// ---------------------------------------------------------------------------

fn run_internal(source: &str) -> String {
    let mut eval = Evaluator::new();
    embed_stdlib(&mut eval);
    functions::register_all(&mut eval);

    match parser::parse(source) {
        Ok(stmts) => match eval.evaluate_statements(&stmts) {
            Ok(()) => eval.output_buffer.join("\n"),
            Err(e) => format!("[error] {}", e),
        },
        Err(e) => format!("[parse error] {}", e),
    }
}

/// Pre-load the standard BUCL library into the evaluator so they are
/// available without a filesystem (essential for WASM builds).
fn embed_stdlib(eval: &mut Evaluator) {
    let stdlib: &[(&str, &str)] = &[
        ("substr",    include_str!("../functions/substr.bucl")),
        ("strpos",    include_str!("../functions/strpos.bucl")),
        ("reverse",   include_str!("../functions/reverse.bucl")),
        ("explode",   include_str!("../functions/explode.bucl")),
        ("implode",   include_str!("../functions/implode.bucl")),
        ("maxlength", include_str!("../functions/maxlength.bucl")),
        ("slice",     include_str!("../functions/slice.bucl")),
    ];
    for (name, src) in stdlib {
        eval.embedded_functions.insert(name.to_string(), src.to_string());
    }
}
