//! Narrow audited ownership shim around the `HTSlib` header-open boundary.
//!
//! The rest of `AlignGauge` forbids unsafe Rust. This private crate exists solely to own raw
//! `htsFile*` and `sam_hdr_t*` values on the malformed-header path that rust-htslib 1.0.1 does not
//! close before returning `BamOpen`.

use std::error::Error;
use std::fmt;
use std::path::Path;
use std::ptr::NonNull;

use rust_htslib::htslib;
use rust_htslib::utils::path_to_cstring;

/// Failures produced while proving `HTSlib` can open, parse, and close one alignment header.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum HeaderPreflightError {
    /// The path could not be represented as a C string.
    PathEncoding,
    /// `hts_open` returned null.
    Open,
    /// `sam_hdr_read` returned null.
    HeaderRead,
    /// `hts_close` returned a nonzero status.
    Close(i32),
}

impl fmt::Display for HeaderPreflightError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PathEncoding => formatter.write_str("alignment path is not representable for HTSlib"),
            Self::Open => formatter.write_str("HTSlib failed to open the alignment"),
            Self::HeaderRead => formatter.write_str("HTSlib failed to read the alignment header"),
            Self::Close(status) => write!(formatter, "HTSlib header preflight close failed with status {status}"),
        }
    }
}

impl Error for HeaderPreflightError {}

struct HtsFileGuard(Option<NonNull<htslib::htsFile>>);

impl HtsFileGuard {
    fn new(pointer: *mut htslib::htsFile) -> Option<Self> {
        NonNull::new(pointer).map(|pointer| Self(Some(pointer)))
    }

    fn pointer(&self) -> *mut htslib::htsFile {
        self.0.expect("HTS file guard owns a pointer until close").as_ptr()
    }

    fn close(mut self) -> i32 {
        let pointer = self.0.take().expect("HTS file guard owns a pointer until close");
        // SAFETY: `pointer` was returned by `hts_open`, is uniquely owned by this guard, and is
        // removed from the guard before this call so Drop cannot close it twice.
        unsafe { htslib::hts_close(pointer.as_ptr()) }
    }
}

impl Drop for HtsFileGuard {
    fn drop(&mut self) {
        if let Some(pointer) = self.0.take() {
            // SAFETY: a pointer retained by the guard is a unique live `htsFile*` returned by
            // `hts_open`. Taking it before closing prevents double-close.
            unsafe {
                htslib::hts_close(pointer.as_ptr());
            }
        }
    }
}

struct SamHeaderGuard(NonNull<htslib::sam_hdr_t>);

impl SamHeaderGuard {
    fn new(pointer: *mut htslib::sam_hdr_t) -> Option<Self> {
        NonNull::new(pointer).map(Self)
    }
}

impl Drop for SamHeaderGuard {
    fn drop(&mut self) {
        // SAFETY: this guard uniquely owns the header returned by `sam_hdr_read`.
        unsafe {
            htslib::sam_hdr_destroy(self.0.as_ptr());
        }
    }
}

/// Open, parse, destroy, and close an alignment header with owned raw-resource cleanup.
///
/// # Errors
/// Returns a typed error if path conversion, open, header parsing, or close fails. All raw
/// resources already acquired are released before the error is returned.
pub fn preflight_header(input: &Path) -> Result<(), HeaderPreflightError> {
    let path = path_to_cstring(&input).ok_or(HeaderPreflightError::PathEncoding)?;

    // SAFETY: both C strings are NUL-terminated and remain live for the duration of the call.
    let raw_file = unsafe { htslib::hts_open(path.as_ptr(), c"r".as_ptr()) };
    let file = HtsFileGuard::new(raw_file).ok_or(HeaderPreflightError::Open)?;

    // SAFETY: `file.pointer()` is a live, uniquely owned `htsFile*` until the guard closes it.
    let raw_header = unsafe { htslib::sam_hdr_read(file.pointer()) };
    let header = SamHeaderGuard::new(raw_header).ok_or(HeaderPreflightError::HeaderRead)?;

    drop(header);
    let status = file.close();
    if status != 0 {
        return Err(HeaderPreflightError::Close(status));
    }
    Ok(())
}
