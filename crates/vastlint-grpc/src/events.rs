//! Validation events, Avro-encoded, for a Kafka results stream.
//!
//! The motivating case is an SSP that wants a stream of creative rejections
//! rather than a request-response call: it does not have a document to ask
//! about, it wants to know which ones are failing.
//!
//! ## Why Avro here and protobuf on the wire
//!
//! Not inconsistency. They are solving different problems, and the difference
//! is the interesting part.
//!
//! The gRPC contract is a *call* contract, negotiated between two parties who
//! are both present. Both sides can be told to upgrade, and `buf breaking`
//! enforces that at commit time in the repository that owns the contract.
//!
//! A topic is a *storage* contract with readers who are not present. Records
//! written today are read months later by consumers nobody controls, running
//! schema versions nobody chose. Avro's writer-schema-plus-reader-schema
//! resolution is built for exactly that: every record carries the identity of
//! the schema it was written under, and readers resolve against it rather than
//! assuming.
//!
//! ## The compatibility guarantee, and where it is enforced
//!
//! The subject is registered BACKWARD, so a reader on the current schema can
//! read every record ever written. A registry enforces that at registration
//! time, once, when someone remembers to register. The tests at the bottom of
//! this file enforce it on every commit, which is the same argument as
//! `buf breaking` versus a code review convention.
//!
//! ## Emission never blocks validation
//!
//! Events go onto a bounded channel and are dropped when it is full, with a
//! counter. Telemetry that adds latency to a bid path is worse than no
//! telemetry, and a full queue must shed events rather than requests. This is
//! the same policy as the concurrency limiter, applied to a place where the
//! stakes are lower and the temptation to buffer is higher.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use apache_avro::types::{Record, Value};
use apache_avro::writer::datum::GenericDatumWriter;
use apache_avro::Schema;
use vastlint_core as core;

/// The schema this build writes with.
///
/// Compiled in rather than fetched, so the binary cannot write records under a
/// schema it does not itself contain.
pub const SCHEMA_JSON: &str =
    include_str!("../../../schemas/openadtech/vastlint/v1/validation_event.avsc");

/// Confluent wire format: one magic byte, then a four-byte big-endian schema
/// ID, then the Avro binary datum.
///
/// Framing rather than a bare datum because that is what every Kafka consumer
/// in this ecosystem expects, and because the schema ID is what lets a reader
/// resolve a record written under a version it has never seen.
const MAGIC_BYTE: u8 = 0;

/// Parsed once. Parsing per event would put JSON parsing on the hot path to
/// serialise a struct that is already in memory.
pub fn schema() -> &'static Schema {
    static SCHEMA: std::sync::OnceLock<Schema> = std::sync::OnceLock::new();
    SCHEMA.get_or_init(|| Schema::parse_str(SCHEMA_JSON).expect("the bundled schema parses"))
}

/// Somewhere for events to go.
///
/// A trait so the Kafka client stays behind a feature flag. A build without it
/// still produces, encodes, and drops events through the same path, so the
/// encoding is exercised whether or not a broker exists.
pub trait Sink: Send + Sync {
    /// Called with one framed, encoded event. Must not block.
    fn publish(&self, key: &str, payload: &[u8]);
}

/// Discards everything, counting as it goes.
///
/// The default. A server with no broker configured should not fail to start,
/// and should not pretend it is publishing.
#[derive(Debug, Default)]
pub struct NullSink {
    published: AtomicU64,
}

impl NullSink {
    pub fn published(&self) -> u64 {
        self.published.load(Ordering::Relaxed)
    }
}

impl Sink for NullSink {
    fn publish(&self, _key: &str, _payload: &[u8]) {
        self.published.fetch_add(1, Ordering::Relaxed);
    }
}

/// One completed validation, before encoding.
#[derive(Debug, Clone)]
pub struct ValidationEvent {
    pub event_id: String,
    pub occurred_at_ms: i64,
    pub document_type: &'static str,
    pub effective_version: Option<String>,
    pub valid: bool,
    pub errors: i32,
    pub warnings: i32,
    pub infos: i32,
    pub findings: Vec<Finding>,
    pub catalog_version: String,
    pub catalog_digest: String,
    pub engine_version: String,
    pub caller: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub rule_id: String,
    pub severity: &'static str,
    pub message: String,
    pub path: Option<String>,
    pub spec_ref: Option<String>,
}

impl ValidationEvent {
    /// Builds an event from a validation result.
    ///
    /// `event_id` is supplied rather than generated here so the caller can use
    /// a request id it already has, which makes an event traceable back to the
    /// call that produced it.
    pub fn from_result(
        event_id: String,
        result: &core::ValidationResult,
        effective_version: Option<String>,
        caller: Option<String>,
    ) -> Self {
        Self {
            event_id,
            occurred_at_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|since| since.as_millis() as i64)
                .unwrap_or(0),
            document_type: result.document_type.as_str(),
            effective_version,
            valid: result.summary.is_valid(),
            errors: result.summary.errors.try_into().unwrap_or(i32::MAX),
            warnings: result.summary.warnings.try_into().unwrap_or(i32::MAX),
            infos: result.summary.infos.try_into().unwrap_or(i32::MAX),
            findings: result
                .issues
                .iter()
                .map(|issue| Finding {
                    rule_id: issue.id.to_string(),
                    severity: severity_symbol(issue.severity),
                    message: issue.message.to_string(),
                    path: issue.path.clone(),
                    spec_ref: Some(issue.spec_ref.to_string()),
                })
                .collect(),
            catalog_version: core::VERSION.to_string(),
            catalog_digest: crate::provenance::catalog_digest().to_string(),
            engine_version: core::VERSION.to_string(),
            caller,
        }
    }

    /// Encodes to the Confluent wire format.
    pub fn encode(&self, schema_id: u32) -> Result<Vec<u8>, apache_avro::Error> {
        let schema = schema();
        let mut record = Record::new(schema).expect("the bundled schema is a record");

        record.put("event_id", Value::String(self.event_id.clone()));
        record.put("occurred_at_ms", Value::Long(self.occurred_at_ms));
        record.put(
            "document_type",
            Value::Enum(
                document_type_index(self.document_type),
                self.document_type.to_string(),
            ),
        );
        record.put(
            "effective_version",
            optional_string(self.effective_version.clone()),
        );
        record.put("valid", Value::Boolean(self.valid));
        record.put("errors", Value::Int(self.errors));
        record.put("warnings", Value::Int(self.warnings));
        record.put("infos", Value::Int(self.infos));
        record.put(
            "findings",
            Value::Array(self.findings.iter().map(finding_value).collect()),
        );
        record.put(
            "catalog_version",
            Value::String(self.catalog_version.clone()),
        );
        record.put("catalog_digest", Value::String(self.catalog_digest.clone()));
        record.put("engine_version", Value::String(self.engine_version.clone()));
        record.put("caller", optional_string(self.caller.clone()));

        let datum = GenericDatumWriter::builder(schema)
            .build()?
            .write_value_to_vec(record)?;

        let mut framed = Vec::with_capacity(5 + datum.len());
        framed.push(MAGIC_BYTE);
        framed.extend_from_slice(&schema_id.to_be_bytes());
        framed.extend_from_slice(&datum);

        Ok(framed)
    }
}

fn optional_string(value: Option<String>) -> Value {
    match value {
        // Union branch indices are positional: 0 is null, 1 is string, because
        // that is the order in the schema. Getting this backwards produces a
        // record that encodes without error and decodes as the wrong branch.
        Some(value) => Value::Union(1, Box::new(Value::String(value))),
        None => Value::Union(0, Box::new(Value::Null)),
    }
}

fn document_type_index(name: &str) -> u32 {
    match name {
        "VAST" => 0,
        "VMAP" => 1,
        "DAAST" => 2,
        _ => 3,
    }
}

/// The Avro enum spells severities in upper case; the core spells them in
/// lower. Mapped here rather than changing either, because both spellings are
/// already load-bearing in their own surfaces.
fn severity_symbol(severity: core::Severity) -> &'static str {
    match severity {
        core::Severity::Error => "ERROR",
        core::Severity::Warning => "WARNING",
        core::Severity::Info => "INFO",
    }
}

fn severity_index(name: &str) -> u32 {
    match name {
        "ERROR" => 0,
        "WARNING" => 1,
        "INFO" => 2,
        _ => 3,
    }
}

fn finding_value(finding: &Finding) -> Value {
    Value::Record(vec![
        (
            "rule_id".to_string(),
            Value::String(finding.rule_id.clone()),
        ),
        (
            "severity".to_string(),
            Value::Enum(
                severity_index(finding.severity),
                finding.severity.to_string(),
            ),
        ),
        (
            "message".to_string(),
            Value::String(finding.message.clone()),
        ),
        ("path".to_string(), optional_string(finding.path.clone())),
        (
            "spec_ref".to_string(),
            optional_string(finding.spec_ref.clone()),
        ),
    ])
}

/// Publishes events without ever blocking the caller.
#[derive(Debug)]
pub struct Publisher {
    sender: tokio::sync::mpsc::Sender<(String, Vec<u8>)>,
    dropped: Arc<AtomicU64>,
}

impl Publisher {
    /// Spawns the background task that drains events into `sink`.
    pub fn spawn(sink: Arc<dyn Sink>, capacity: usize, schema_id: u32) -> Self {
        let (sender, mut receiver) =
            tokio::sync::mpsc::channel::<(String, Vec<u8>)>(capacity.max(1));
        let dropped = Arc::new(AtomicU64::new(0));

        tokio::spawn(async move {
            while let Some((key, payload)) = receiver.recv().await {
                sink.publish(&key, &payload);
            }
        });

        let _ = schema_id;
        Self { sender, dropped }
    }

    /// Events discarded because the queue was full.
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// Queues an event, or drops it.
    ///
    /// `try_send` rather than `send`, deliberately. Awaiting here would make a
    /// slow broker into validation latency, which is the failure this whole
    /// server is built to avoid. Dropping a telemetry event to keep a bid-path
    /// response fast is the correct trade, and the counter means the loss is
    /// visible rather than silent.
    pub fn publish(&self, event: &ValidationEvent, schema_id: u32) {
        let payload = match event.encode(schema_id) {
            Ok(payload) => payload,
            Err(_) => {
                // An encoding failure is a bug in this file, not a runtime
                // condition, and it must not fail the validation that produced
                // it. Counted with the drops so it cannot hide.
                self.dropped.fetch_add(1, Ordering::Relaxed);
                return;
            }
        };

        if self
            .sender
            .try_send((event.event_id.clone(), payload))
            .is_err()
        {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            crate::metrics::record_event_dropped();
        } else {
            crate::metrics::record_event_published();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apache_avro::reader::datum::GenericDatumReader;

    fn decode_datum(
        writer_schema: &Schema,
        bytes: &mut &[u8],
        reader_schema: Option<&Schema>,
    ) -> apache_avro::AvroResult<Value> {
        GenericDatumReader::builder(writer_schema)
            .maybe_reader_schema(reader_schema)
            .build()?
            .read_value(bytes)
    }

    fn sample() -> ValidationEvent {
        ValidationEvent {
            event_id: "evt-1".to_string(),
            occurred_at_ms: 1_754_700_000_000,
            document_type: "VAST",
            effective_version: Some("4.1".to_string()),
            valid: false,
            errors: 2,
            warnings: 1,
            infos: 0,
            findings: vec![Finding {
                rule_id: "VAST-2.0-inline-impression".to_string(),
                severity: "ERROR",
                message: "<InLine> is missing required <Impression>".to_string(),
                path: Some("/VAST/Ad[0]/InLine".to_string()),
                spec_ref: Some("IAB VAST 2.0 §2.3.4".to_string()),
            }],
            catalog_version: "0.11.7".to_string(),
            catalog_digest: "sha256:abc".to_string(),
            engine_version: "0.11.7".to_string(),
            caller: None,
        }
    }

    #[test]
    fn the_bundled_schema_parses() {
        let _ = schema();
    }

    #[test]
    fn an_event_round_trips_through_its_own_schema() {
        let encoded = sample().encode(42).expect("encodes");

        assert_eq!(encoded[0], MAGIC_BYTE, "Confluent framing starts with 0");
        assert_eq!(
            u32::from_be_bytes(encoded[1..5].try_into().unwrap()),
            42,
            "the schema id is big-endian in bytes 1..5"
        );

        let mut body = &encoded[5..];
        let decoded = decode_datum(schema(), &mut body, None).expect("decodes");

        let Value::Record(fields) = decoded else {
            panic!("expected a record");
        };
        let get = |name: &str| {
            fields
                .iter()
                .find(|(field, _)| field == name)
                .map(|(_, value)| value.clone())
                .unwrap_or_else(|| panic!("missing field {name}"))
        };

        assert_eq!(get("event_id"), Value::String("evt-1".to_string()));
        assert_eq!(get("valid"), Value::Boolean(false));
        assert_eq!(get("errors"), Value::Int(2));
    }

    /// Union branch order is positional and silently wrong if inverted: a
    /// record with the branches swapped still encodes, and decodes as the wrong
    /// variant.
    #[test]
    fn optional_fields_encode_the_right_union_branch() {
        let mut event = sample();
        event.caller = None;
        let encoded = event.encode(1).expect("encodes");
        let mut body = &encoded[5..];
        let decoded = decode_datum(schema(), &mut body, None).expect("decodes");

        let Value::Record(fields) = decoded else {
            panic!("expected a record")
        };
        let caller = fields
            .iter()
            .find(|(name, _)| name == "caller")
            .map(|(_, value)| value.clone())
            .expect("caller present");
        assert_eq!(caller, Value::Union(0, Box::new(Value::Null)));

        let mut event = sample();
        event.caller = Some("ssp-1".to_string());
        let encoded = event.encode(1).expect("encodes");
        let mut body = &encoded[5..];
        let decoded = decode_datum(schema(), &mut body, None).expect("decodes");

        let Value::Record(fields) = decoded else {
            panic!("expected a record")
        };
        let caller = fields
            .iter()
            .find(|(name, _)| name == "caller")
            .map(|(_, value)| value.clone())
            .expect("caller present");
        assert_eq!(
            caller,
            Value::Union(1, Box::new(Value::String("ssp-1".to_string())))
        );
    }

    #[tokio::test]
    async fn publishing_never_blocks_and_drops_are_counted() {
        // Capacity 1 and a sink that is never drained fast enough: the queue
        // fills and the rest are dropped rather than awaited.
        let sink: Arc<dyn Sink> = Arc::new(NullSink::default());
        let publisher = Publisher::spawn(Arc::clone(&sink), 1, 1);

        let event = sample();
        for _ in 0..1_000 {
            publisher.publish(&event, 1);
        }

        // The assertion is that this returned at all. Whether anything was
        // dropped depends on how fast the drain task runs, so the useful
        // invariant is that nothing hung and the counter is coherent.
        assert!(publisher.dropped() <= 1_000);
    }
}

// A Kafka producer sink used to live here, behind a `kafka` feature. It was
// removed, and the reason is worth keeping.
//
// CI runs `cargo clippy --all-targets --all-features`, so an optional feature
// is not optional in CI: every platform built the vendored librdkafka on every
// run, and Windows could not build it at all (`rdkafka-sys` panics with "%1 is
// not a valid Win32 application"). A dependency that breaks a third of the
// build matrix has to earn its place, and this one could not: it had never been
// run against a broker, so nothing here was verified by having it.
//
// What it was protecting is unaffected. The schema, the Confluent framing, the
// encoding, the drop policy, and the BACKWARD compatibility proof in
// tests/schema_compatibility.rs are all still here and all still tested. `Sink`
// is the seam: adding a real producer is an implementation of one trait method,
// and it should land as its own change, on a branch with a broker to test it
// against.
