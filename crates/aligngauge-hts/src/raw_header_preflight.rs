//! Resource-safe raw HTSlib header preflight for malformed-input cleanup.

use std::path::Path;
use std::ptr::NonNull;

use aligngauge_core::{AlignGaugeError, ErrorCategory};
use rust_htslib::htslib;
use rust_htslib::utils::path_to_cstring;

struct HtsFileGuard(Option<NonNull<htslib::htsFile>>);

impl HtsFileGuard {
    fn new(pointer: *mut htslib::htsFile) -> Option<Self> {
        NonNull::new(pointer).map(|pointer| Self(Some(pointer)))
    }

    fn pointer(&self) -> *mut htslib::htsFile {
        self.0.expect("HTS file guard must own a pointer").as_ptr()
    }

    fn close(mut self) -> i32 {
        let pointer = self.0.take().expect("HTS file guard must own a pointer");
        // SAFETY: `pointer` is owned by this guard, was returned by `hts_open`, and is
        // removed from the guard before `hts_close` so Drop cannot close it twice.
        unsafe { htslib::hts_close(pointer.as_ptr()) }
    }
}

impl Drop for HtsFileGuard {
    fn drop(&mut self) {
        if let Some(pointer) = self.0.take() {
            // SAFETY: a pointer held by the guard is an owned live `htsFile*` returned by
            // `hts_open`. Taking it prevents a second close.
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

/// Validate that HTSlib can open and parse an alignment header while retaining ownership of
/// every raw allocation on failure.
///
/// `rust-htslib` 1.0.1 returns from `bam::Reader::new` without closing the `htsFile*` when
/// `sam_hdr_read` returns null. This preflight keeps that malformed-header path out of the
/// high-level constructor while still using the high-level reader for all real traversal.
pub(crate) fn preflight_header_open(
    input: &Path,
    format_name: &'static str,
) -> Result<(), AlignGaugeError> {
    let path = path_to_cstring(&input).ok_or_else(|| {
        AlignGaugeError::new(
            ErrorCategory::InputCorrupt,
            format!("failed to represent {format_name} input path for HTSlib header preflight"),
        )
        .with_detail("input", input.to_string_lossy().into_owned())
    })?;

    // SAFETY: both C strings are NUL-terminated and live for the duration of the call.
    let raw_file = unsafe { htslib::hts_open(path.as_ptr(), c"r".as_ptr()) };
    let file = HtsFileGuard::new(raw_file).ok_or_else(|| {
        AlignGaugeError::new(
            ErrorCategory::InputCorrupt,
            format!("failed to open {format_name} '{}' during HTSlib header preflight", input.display()),
        )
        .with_detail("input", input.to_string_lossy().into_owned())
    })?;

    // SAFETY: `file.pointer()` is a live, owned `htsFile*` until the guard closes it.
    let raw_header = unsafe { htslib::sam_hdr_read(file.pointer()) };
    let header = SamHeaderGuard::new(raw_header).ok_or_else(|| {
        AlignGaugeError::new(
            ErrorCategory::InputCorrupt,
            format!("failed to read {format_name} header from '{}'", input.display()),
        )
        .with_detail("input", input.to_string_lossy().into_owned())
    })?;

    drop(header);
    let close_status = file.close();
    if close_status != 0 {
        return Err(AlignGaugeError::new(
            ErrorCategory::InputCorrupt,
            format!("HTSlib failed to close {format_name} header preflight for '{}'", input.display()),
        )
        .with_detail("input", input.to_string_lossy().into_owned())
        .with_detail("hts_close_status", i64::from(close_status)));
    }

    Ok(())
}
