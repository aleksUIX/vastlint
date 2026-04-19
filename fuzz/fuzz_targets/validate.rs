//! Fuzz target: validate arbitrary byte sequences.
//!
//! The validator must never panic regardless of input — it should always
//! return a structured result or propagate parse errors gracefully.
#![no_main]

use libfuzzer_sys::fuzz_target;
use vastlint_core::validate;

fuzz_target!(|data: &[u8]| {
    if let Ok(xml) = std::str::from_utf8(data) {
        // Must not panic — any input is valid to hand to the validator.
        let _ = validate(xml);
    }
});
