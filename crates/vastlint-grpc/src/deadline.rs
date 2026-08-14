//! Client deadline propagation.
//!
//! gRPC clients express a deadline as a `grpc-timeout` header holding a
//! duration relative to when the request was sent. tonic does not act on it for
//! you, so a server that ignores it will happily spend a second producing a
//! result for a caller that gave up after ten milliseconds. In a bid path that
//! is not merely wasteful: the work competes for the same capacity as requests
//! that can still be answered in time.
//!
//! Wire format is a decimal value followed by a one-character unit, per the
//! gRPC over HTTP/2 specification:
//!
//! ```text
//! H  hours    M  minutes    S  seconds
//! m  millis   u  micros     n  nanos
//! ```

use std::time::Duration;

use tonic::metadata::MetadataMap;

/// The header gRPC clients use to express a deadline.
const TIMEOUT_HEADER: &str = "grpc-timeout";

/// Reads the caller's remaining time budget, if it declared one.
///
/// Returns `None` when the header is absent, which means the caller accepted
/// whatever the server takes. A malformed header is also `None` rather than an
/// error: refusing a request over an unparseable timeout would be a worse
/// failure than serving it, and the specification's own guidance is to treat
/// the value as advisory.
pub fn remaining(metadata: &MetadataMap) -> Option<Duration> {
    let raw = metadata.get(TIMEOUT_HEADER)?.to_str().ok()?;
    parse(raw)
}

/// Parses a `grpc-timeout` value into a duration.
fn parse(raw: &str) -> Option<Duration> {
    let (value, unit) = raw.split_at(raw.len().checked_sub(1)?);
    let value: u64 = value.parse().ok()?;

    // Saturating rather than wrapping: the spec caps the value at 8 digits, but
    // a non-conforming client sending a large hour count should get "a very
    // long time" rather than a wrapped-around short one.
    match unit {
        "H" => Some(Duration::from_secs(value.saturating_mul(3600))),
        "M" => Some(Duration::from_secs(value.saturating_mul(60))),
        "S" => Some(Duration::from_secs(value)),
        "m" => Some(Duration::from_millis(value)),
        "u" => Some(Duration::from_micros(value)),
        "n" => Some(Duration::from_nanos(value)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_unit() {
        assert_eq!(parse("3H"), Some(Duration::from_secs(10_800)));
        assert_eq!(parse("2M"), Some(Duration::from_secs(120)));
        assert_eq!(parse("30S"), Some(Duration::from_secs(30)));
        assert_eq!(parse("250m"), Some(Duration::from_millis(250)));
        assert_eq!(parse("500u"), Some(Duration::from_micros(500)));
        assert_eq!(parse("100n"), Some(Duration::from_nanos(100)));
    }

    /// Case matters: `M` is minutes and `m` is milliseconds. Getting this
    /// backwards turns a 100ms budget into a 100 minute one.
    #[test]
    fn units_are_case_sensitive() {
        assert_eq!(parse("100M"), Some(Duration::from_secs(6_000)));
        assert_eq!(parse("100m"), Some(Duration::from_millis(100)));
    }

    #[test]
    fn rejects_malformed_values() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("m"), None);
        assert_eq!(parse("100"), None);
        assert_eq!(parse("100x"), None);
        assert_eq!(parse("-5S"), None);
        assert_eq!(parse("1.5S"), None);
        assert_eq!(parse("abcS"), None);
    }

    #[test]
    fn zero_is_a_valid_deadline_meaning_no_time_left() {
        assert_eq!(parse("0m"), Some(Duration::ZERO));
    }

    #[test]
    fn absent_header_means_no_deadline() {
        assert_eq!(remaining(&MetadataMap::new()), None);
    }

    #[test]
    fn reads_the_header_when_present() {
        let mut metadata = MetadataMap::new();
        metadata.insert(TIMEOUT_HEADER, "40m".parse().unwrap());
        assert_eq!(remaining(&metadata), Some(Duration::from_millis(40)));
    }

    #[test]
    fn malformed_header_is_treated_as_no_deadline() {
        let mut metadata = MetadataMap::new();
        metadata.insert(TIMEOUT_HEADER, "not-a-timeout".parse().unwrap());
        assert_eq!(remaining(&metadata), None);
    }

    #[test]
    fn large_hour_counts_saturate_rather_than_wrap() {
        assert_eq!(
            parse(&format!("{}H", u64::MAX)),
            Some(Duration::from_secs(u64::MAX))
        );
    }
}
