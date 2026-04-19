//! Fuzz target: validate with arbitrary wrapper depth.
//!
//! Exercises the wrapper-chain depth-limit logic — the validator must handle
//! any combination of XML input and wrapper depth without panicking.
#![no_main]

use libfuzzer_sys::fuzz_target;
use vastlint_core::{validate_with_context, ValidationContext};

fuzz_target!(|data: &[u8]| {
    // Need at least 1 byte for the depth, rest is XML.
    if data.len() < 2 {
        return;
    }
    let wrapper_depth = data[0];
    if let Ok(xml) = std::str::from_utf8(&data[1..]) {
        let ctx = ValidationContext {
            wrapper_depth,
            ..Default::default()
        };
        // Must not panic for any depth (0-255) + any XML input.
        let _ = validate_with_context(xml, ctx);
    }
});
