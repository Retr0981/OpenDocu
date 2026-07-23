//! OpenDocu C ABI — the single native surface consumed by Node, Python, and C++.
//!
//! ## Design
//! All functions use a JSON-in / JSON-out protocol over an opaque handle to
//! keep the ABI tiny and stable:
//!
//! - [`opendocu_version`] → null-terminated version string.
//! - [`opendocu_reduce`] → take document bytes + options JSON, return a
//!   heap-allocated *handle* to a reduction result.
//! - [`opendocu_result_json`] → borrow a handle, return its JSON as a pointer
//!   + length (caller copies then calls [`opendocu_free_string`]).
//! - [`opendocu_free_string`] / [`opendocu_free_handle`] → release memory.
//!
//! Error handling: `opendocu_reduce` returns a null pointer on failure. Use
//! [`opendocu_last_error`] to retrieve the last error message for the calling
//! thread.
//!
//! ## Memory ownership
//! - Strings returned *from* OpenDocu (version, result JSON, last error) are
//!   owned by OpenDocu and must be freed with [`opendocu_free_string`].
//! - Handles are owned by the caller and must be freed with
//!   [`opendocu_free_handle`].
//! - Input byte buffers and option JSON are borrowed for the duration of the
//!   call only.

#![allow(clippy::missing_safety_doc)]

use std::ffi::CString;
use std::ptr;

use opendocu_core::{reduce, ProcessingOptions};
use opendocu_structures::ReducedDocument;
use opendocu_core::{extract, ExtractionOptions};

// ---------------------------------------------------------------------------
// Thread-local last-error
// ---------------------------------------------------------------------------

thread_local! {
    static LAST_ERROR: std::cell::RefCell<Option<CString>> = std::cell::RefCell::new(None);
}

fn set_last_error(msg: impl Into<String>) {
    let c = CString::new(msg.into()).unwrap_or_else(|_| CString::new("error").unwrap());
    LAST_ERROR.with(|cell| *cell.borrow_mut() = Some(c));
}

/// A boxed reduction result. Opaque to C; referenced by the returned handle.
struct BoxedResult {
    json: CString,
}

// ---------------------------------------------------------------------------
// Public C ABI
// ---------------------------------------------------------------------------

/// Returns the OpenDocu semver version. Caller must NOT free this string
/// (it points to static storage).
#[no_mangle]
pub extern "C" fn opendocu_version() -> *const libc::c_char {
    // SAFETY: concat! produces a string with no interior nuls.
    static VERSION: &[u8] = concat!(env!("CARGO_PKG_VERSION"), "\0").as_bytes();
    VERSION.as_ptr() as *const libc::c_char
}

/// Reduce a document. Returns an opaque non-null handle on success, or null on
/// failure (call [`opendocu_last_error`]).
///
/// - `input_ptr` / `input_len`: raw document bytes.
/// - `opts_ptr` / `opts_len`: optional JSON [`ProcessingOptions`] string. May
///   be a null pointer for defaults.
#[no_mangle]
pub unsafe extern "C" fn opendocu_reduce(
    input_ptr: *const u8,
    input_len: usize,
    opts_ptr: *const libc::c_char,
    opts_len: usize,
) -> *mut libc::c_void {
    if input_ptr.is_null() {
        set_last_error("input pointer is null");
        return ptr::null_mut();
    }

    let input = std::slice::from_raw_parts(input_ptr, input_len);

    let opts: ProcessingOptions = if opts_ptr.is_null() || opts_len == 0 {
        ProcessingOptions::default()
    } else {
        let opts_bytes = std::slice::from_raw_parts(opts_ptr as *const u8, opts_len);
        let opts_str = match std::str::from_utf8(opts_bytes) {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("options utf8: {e}"));
                return ptr::null_mut();
            }
        };
        match ProcessingOptions::from_json(opts_str) {
            Ok(o) => o,
            Err(e) => {
                set_last_error(format!("options json: {e}"));
                return ptr::null_mut();
            }
        }
    };

    match reduce(input, opts) {
        Ok(reduced) => {
            let boxed = BoxedResult::from_reduced(reduced);
            Box::into_raw(Box::new(boxed)) as *mut libc::c_void
        }
        Err(e) => {
            set_last_error(format!("{e}"));
            ptr::null_mut()
        }
    }
}

/// Extract structured data from a document. Returns an opaque handle on
/// success, or null on failure (call [`opendocu_last_error`]).
///
/// - `input_ptr` / `input_len`: raw document bytes.
/// - `opts_ptr` / `opts_len`: optional JSON [`ExtractionOptions`] string:
///   `{"extract_tables":true,"extract_key_values":true,"extract_entities":true}`.
///   May be a null pointer for defaults.
///
/// The returned handle is read with [`opendocu_result_json`] and freed with
/// [`opendocu_free_handle`] — same as a reduction handle.
#[no_mangle]
pub unsafe extern "C" fn opendocu_extract(
    input_ptr: *const u8,
    input_len: usize,
    opts_ptr: *const libc::c_char,
    opts_len: usize,
) -> *mut libc::c_void {
    if input_ptr.is_null() {
        set_last_error("input pointer is null");
        return ptr::null_mut();
    }

    let input = std::slice::from_raw_parts(input_ptr, input_len);

    let opts: ExtractionOptions = if opts_ptr.is_null() || opts_len == 0 {
        ExtractionOptions::default()
    } else {
        let opts_bytes = std::slice::from_raw_parts(opts_ptr as *const u8, opts_len);
        let opts_str = match std::str::from_utf8(opts_bytes) {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("options utf8: {e}"));
                return ptr::null_mut();
            }
        };
        match serde_json::from_str::<ExtractionOptions>(opts_str) {
            Ok(o) => o,
            Err(e) => {
                set_last_error(format!("options json: {e}"));
                return ptr::null_mut();
            }
        }
    };

    match extract(input, opts) {
        Ok(result) => {
            let json_str = result.to_json().unwrap_or_else(|_| "{}".into());
            let json = CString::new(json_str).unwrap_or_else(|_| CString::new("{}").unwrap());
            Box::into_raw(Box::new(BoxedResult { json })) as *mut libc::c_void
        }
        Err(e) => {
            set_last_error(format!("{e}"));
            ptr::null_mut()
        }
    }
}

/// Borrow a handle and get the result as a JSON string (pointer + length).
/// The returned pointer is owned by the handle; do not free it with
/// [`opendocu_free_string`]. It is valid until [`opendocu_free_handle`] is
/// called on the handle.
#[no_mangle]
pub unsafe extern "C" fn opendocu_result_json(
    handle: *mut libc::c_void,
    out_len: *mut usize,
) -> *const libc::c_char {
    if handle.is_null() {
        set_last_error("handle is null");
        if !out_len.is_null() {
            *out_len = 0;
        }
        return ptr::null();
    }
    let boxed = &*(handle as *const BoxedResult);
    let bytes = boxed.json.as_bytes();
    if !out_len.is_null() {
        *out_len = bytes.len();
    }
    boxed.json.as_ptr()
}

/// Returns the last error message for this thread, or null if there is none.
/// The caller must free the returned string with [`opendocu_free_string`].
#[no_mangle]
pub extern "C" fn opendocu_last_error() -> *mut libc::c_char {
    LAST_ERROR.with(|cell| {
        cell.borrow()
            .as_ref()
            .map(|c| {
                // Hand ownership to the caller: duplicate the string.
                unsafe { libc::strdup(c.as_ptr()) as *mut libc::c_char }
            })
            .unwrap_or(ptr::null_mut())
    })
}

/// Free a string previously returned *from* [`opendocu_last_error`].
/// Do NOT use this on the result of [`opendocu_result_json`] (that memory is
/// owned by the handle) or [`opendocu_version`] (static storage).
#[no_mangle]
pub unsafe extern "C" fn opendocu_free_string(ptr: *mut libc::c_char) {
    if !ptr.is_null() {
        libc::free(ptr as *mut libc::c_void);
    }
}

/// Free a reduction handle returned by [`opendocu_reduce`].
#[no_mangle]
pub unsafe extern "C" fn opendocu_free_handle(handle: *mut libc::c_void) {
    if handle.is_null() {
        return;
    }
    drop(Box::from_raw(handle as *mut BoxedResult));
}

impl BoxedResult {
    fn from_reduced(reduced: ReducedDocument) -> Self {
        let json_str = reduced.to_json().unwrap_or_else(|_| "{}".into());
        let json = CString::new(json_str).unwrap_or_else(|_| CString::new("{}").unwrap());
        BoxedResult { json }
    }
}

// ---------------------------------------------------------------------------
// Smoke tests that exercise the C ABI from Rust.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn version_is_accessible() {
        let v = opendocu_version();
        let s = unsafe { CStr::from_ptr(v) }.to_str().unwrap();
        assert!(!s.is_empty());
    }

    #[test]
    fn reduce_returns_valid_json() {
        let input = b"# Title\n\nSome body text here that is long enough to summarize properly.";
        let opts = b"";
        unsafe {
            let handle = opendocu_reduce(input.as_ptr(), input.len(), opts.as_ptr() as *const i8, opts.len());
            assert!(!handle.is_null(), "reduce returned null handle");

            let mut len = 0usize;
            let json_ptr = opendocu_result_json(handle, &mut len as *mut usize);
            assert!(!json_ptr.is_null());
            assert!(len > 0);

            let slice = std::slice::from_raw_parts(json_ptr as *const u8, len);
            let parsed: serde_json::Value = serde_json::from_slice(slice).unwrap();
            assert!(parsed["summary"].is_string());
            assert!(parsed["metrics"].is_object());

            opendocu_free_handle(handle);
        }
    }

    #[test]
    fn null_input_sets_error() {
        unsafe {
            let h = opendocu_reduce(ptr::null(), 0, ptr::null(), 0);
            assert!(h.is_null());
            let err = opendocu_last_error();
            assert!(!err.is_null());
            let msg = CStr::from_ptr(err).to_str().unwrap();
            assert!(!msg.is_empty());
            opendocu_free_string(err);
        }
    }

    #[test]
    fn extract_returns_structured_json() {
        let input = b"Invoice #INV-2024-001\nDate: 2024-03-15\nTotal: $1,250.00\nEmail: billing@example.com";
        let opts = b"";
        unsafe {
            let handle = opendocu_extract(input.as_ptr(), input.len(), opts.as_ptr() as *const i8, opts.len());
            assert!(!handle.is_null(), "extract returned null handle");

            let mut len = 0usize;
            let json_ptr = opendocu_result_json(handle, &mut len as *mut usize);
            assert!(!json_ptr.is_null());
            assert!(len > 0);

            let slice = std::slice::from_raw_parts(json_ptr as *const u8, len);
            let parsed: serde_json::Value = serde_json::from_slice(slice).unwrap();
            assert!(parsed["entities"].is_array());
            assert!(parsed["entities"].as_array().unwrap().len() >= 3); // date, money, email
            assert!(parsed["key_values"]["fields"].is_array());

            opendocu_free_handle(handle);
        }
    }
}
