//! Validation throughput benchmark.
//!
//! ARCHITECTURE.md targets sub-1ms validation for typical VAST documents.
//! Run with `cargo bench -p vastlint-core`; criterion prints per-iteration
//! time, so the target is met when every `validate/*` benchmark reports a
//! mean under 1 ms.

use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

const VALID_2_0: &str = include_str!("../tests/fixtures/valid_2.0.xml");
const VALID_4_2: &str = include_str!("../tests/fixtures/valid_4.2.xml");
const VALID_4_3_VERIFICATION: &str =
    include_str!("../tests/fixtures/valid_4.3_with_verification.xml");

fn bench_validate(c: &mut Criterion) {
    let mut group = c.benchmark_group("validate");

    group.bench_function("valid_2.0", |b| {
        b.iter(|| vastlint_core::validate(black_box(VALID_2_0)))
    });
    group.bench_function("valid_4.2", |b| {
        b.iter(|| vastlint_core::validate(black_box(VALID_4_2)))
    });
    group.bench_function("valid_4.3_with_verification", |b| {
        b.iter(|| vastlint_core::validate(black_box(VALID_4_3_VERIFICATION)))
    });

    // A 10-ad pod stresses per-ad rule loops harder than any single fixture.
    let pod = {
        let inner = VALID_4_2
            .trim()
            .trim_start_matches(r#"<?xml version="1.0" encoding="UTF-8"?>"#)
            .trim();
        let open_end = inner.find('>').expect("fixture has a root element");
        let body_end = inner.rfind("</VAST>").expect("fixture has a closing tag");
        let body = &inner[open_end + 1..body_end];
        format!(
            r#"<VAST version="4.2">{}</VAST>"#,
            body.repeat(10)
                .replace(r#"<Ad id="#, r#"<Ad sequence="1" id="#)
        )
    };
    group.bench_function("pod_10_ads", |b| {
        b.iter(|| vastlint_core::validate(black_box(&pod)))
    });

    group.finish();
}

criterion_group!(benches, bench_validate);
criterion_main!(benches);
