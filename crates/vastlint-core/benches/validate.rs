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

/// Nine ads, every CTV Ad Portfolio failure mode, and the legacy extension
/// container throughout. This is the shape the element dispatch in
/// `rules/elements.rs` walks hardest: deep trees, standardised extension
/// containers it must descend into, and a high finding count.
const CTV_PORTFOLIO: &str = include_str!("../tests/fixtures/err_ctv_portfolio_legacy_all_modes.xml");

/// A conforming portfolio tag. The defective one above exits some rules early
/// on their first failure, so a clean document of the same shape is the more
/// honest measure of full-traversal cost.
const CTV_PORTFOLIO_CLEAN: &str = include_str!("../tests/fixtures/ok_ctv_portfolio_vast_2_0.xml");

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

    group.bench_function("ctv_portfolio_9_ads", |b| {
        b.iter(|| vastlint_core::validate(black_box(CTV_PORTFOLIO)))
    });
    group.bench_function("ctv_portfolio_clean", |b| {
        b.iter(|| vastlint_core::validate(black_box(CTV_PORTFOLIO_CLEAN)))
    });

    // Element-count stress. The dispatch pass visits every node in the document
    // and builds a path for it, so cost scales with node count rather than with
    // finding count. A 50-ad pod is the worst realistic case for that.
    let big_pod = {
        let inner = VALID_4_2
            .trim()
            .trim_start_matches(r#"<?xml version="1.0" encoding="UTF-8"?>"#)
            .trim();
        let open_end = inner.find('>').expect("fixture has a root element");
        let body_end = inner.rfind("</VAST>").expect("fixture has a closing tag");
        let body = &inner[open_end + 1..body_end];
        format!(
            r#"<VAST version="4.2">{}</VAST>"#,
            body.repeat(50)
                .replace(r#"<Ad id="#, r#"<Ad sequence="1" id="#)
        )
    };
    group.bench_function("pod_50_ads", |b| {
        b.iter(|| vastlint_core::validate(black_box(&big_pod)))
    });

    group.finish();
}

criterion_group!(benches, bench_validate);
criterion_main!(benches);
