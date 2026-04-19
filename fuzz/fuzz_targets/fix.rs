//! Fuzz target: fix arbitrary byte sequences.
//!
//! The auto-fix pass must never panic — it should always produce a result
//! (possibly unchanged) and a list of applied fixes.
#![no_main]

use libfuzzer_sys::fuzz_target;
use vastlint_core::fix;

fuzz_target!(|data: &[u8]| {
    if let Ok(xml) = std::str::from_utf8(data) {
        // Must not panic — the fix pass must handle any XML-like input.
        let _ = fix(xml);
    }
});
