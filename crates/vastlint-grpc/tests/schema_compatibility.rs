//! BACKWARD compatibility for the validation event schema, proven rather than
//! configured.
//!
//! A schema registry set to BACKWARD checks compatibility once, at registration
//! time, when somebody remembers to register. That is a real check and it is
//! not enough: it lives in a running service, it is easy to bypass with a
//! force-register, and it says nothing until the schema is already being
//! deployed.
//!
//! These tests do the same check on every commit, in the repository that owns
//! the schema, in exactly the way `buf breaking` does for the protobuf
//! contract. Two mechanisms reached from different directions: the registry
//! enforces at registration, the test enforces at commit, and the one that
//! catches a mistake before it is deployed is worth more.
//!
//! ## What BACKWARD actually means
//!
//! A reader using the NEW schema can read data written under an OLD one. It is
//! the guarantee that matters when consumers upgrade before producers, and when
//! a topic holds months of records written under schemas nobody is running any
//! more.
//!
//! Concretely, within one subject:
//!
//!   - Adding a field with a default is allowed. Old data has no value for it;
//!     the reader fills in the default.
//!   - Adding a field without a default is not. The reader has nothing to put
//!     there, and every old record becomes undecodable.
//!   - Removing a field is allowed for readers, since the reader simply ignores
//!     what it does not know about.
//!   - Renaming is removal plus addition, so it fails on the addition half.
//!
//! Each of those is a test below, including the ones that must fail.

use apache_avro::reader::datum::GenericDatumReader;
use apache_avro::types::{Record, Value};
use apache_avro::writer::datum::GenericDatumWriter;
use apache_avro::Schema;

/// The schema this build ships. The starting point for every evolution below.
const CURRENT: &str = include_str!("../../../schemas/openadtech/vastlint/v1/validation_event.avsc");

/// Writes a minimal record under `writer`, then reads it back under `reader`.
///
/// This is the whole compatibility question in one function: can a consumer on
/// the reader schema decode bytes produced by a producer on the writer schema.
fn read_old_data_with_new_schema(writer_json: &str, reader_json: &str) -> Result<Value, String> {
    let writer = Schema::parse_str(writer_json).map_err(|error| error.to_string())?;
    let reader = Schema::parse_str(reader_json).map_err(|error| error.to_string())?;

    let mut record = Record::new(&writer).ok_or("writer schema is not a record")?;
    record.put("event_id", Value::String("evt-1".to_string()));
    record.put("occurred_at_ms", Value::Long(1));
    record.put("document_type", Value::Enum(0, "VAST".to_string()));
    record.put("effective_version", Value::Union(0, Box::new(Value::Null)));
    record.put("valid", Value::Boolean(true));
    record.put("errors", Value::Int(0));
    record.put("warnings", Value::Int(0));
    record.put("infos", Value::Int(0));
    record.put("findings", Value::Array(vec![]));
    record.put("catalog_version", Value::String("0.11.7".to_string()));
    record.put("catalog_digest", Value::String("sha256:abc".to_string()));
    record.put("engine_version", Value::String("0.11.7".to_string()));
    record.put("caller", Value::Union(0, Box::new(Value::Null)));

    let datum = GenericDatumWriter::builder(&writer)
        .build()
        .and_then(|encoder| encoder.write_value_to_vec(record))
        .map_err(|error| error.to_string())?;

    let mut bytes = datum.as_slice();
    GenericDatumReader::builder(&writer)
        .maybe_reader_schema(Some(&reader))
        .build()
        .and_then(|decoder| decoder.read_value(&mut bytes))
        .map_err(|error| error.to_string())
}

fn field(record: &Value, name: &str) -> Option<Value> {
    let Value::Record(fields) = record else {
        return None;
    };
    fields
        .iter()
        .find(|(field, _)| field == name)
        .map(|(_, value)| value.clone())
}

/// Inserts a field into the schema JSON just before the closing bracket of the
/// top-level field list, which is where a new field would realistically go.
fn with_extra_field(schema_json: &str, field_json: &str) -> String {
    let mut parsed: serde_json::Value = serde_json::from_str(schema_json).expect("schema is JSON");
    let fields = parsed["fields"].as_array_mut().expect("fields is an array");
    fields.push(serde_json::from_str(field_json).expect("field is JSON"));
    parsed.to_string()
}

#[test]
fn the_shipped_schema_reads_its_own_data() {
    let value = read_old_data_with_new_schema(CURRENT, CURRENT).expect("self-compatible");
    assert_eq!(
        field(&value, "event_id"),
        Some(Value::String("evt-1".to_string()))
    );
}

/// The allowed evolution, and the one that will actually happen: somebody wants
/// a new attribute on the event.
#[test]
fn adding_a_field_with_a_default_is_backward_compatible() {
    let next = with_extra_field(
        CURRENT,
        r#"{"name":"wrapper_depth","type":"int","default":0}"#,
    );

    let value = read_old_data_with_new_schema(CURRENT, &next)
        .expect("a reader on the new schema must read old data");

    assert_eq!(
        field(&value, "wrapper_depth"),
        Some(Value::Int(0)),
        "the reader fills in the default for data written before the field existed"
    );
}

#[test]
fn adding_an_optional_field_is_backward_compatible() {
    let next = with_extra_field(
        CURRENT,
        r#"{"name":"origin_url","type":["null","string"],"default":null}"#,
    );

    let value = read_old_data_with_new_schema(CURRENT, &next).expect("optional additions are safe");

    assert_eq!(
        field(&value, "origin_url"),
        Some(Value::Union(0, Box::new(Value::Null)))
    );
}

/// The failure this whole file exists to catch. Without a default the reader
/// has nothing to put in the field, so every record ever written becomes
/// undecodable, and the topic's history is lost to the new consumer.
#[test]
fn adding_a_field_without_a_default_breaks_backward_compatibility() {
    let next = with_extra_field(CURRENT, r#"{"name":"tenant_id","type":"string"}"#);

    let result = read_old_data_with_new_schema(CURRENT, &next);

    assert!(
        result.is_err(),
        "a required field with no default must not be readable against older data, \
         otherwise this test is not checking anything"
    );
}

/// A rename is a removal plus an addition, and the addition half is what fails.
/// Worth its own test because "rename" sounds harmless in a pull request title.
#[test]
fn renaming_a_field_breaks_backward_compatibility() {
    let mut parsed: serde_json::Value = serde_json::from_str(CURRENT).expect("schema is JSON");
    let fields = parsed["fields"].as_array_mut().expect("fields");
    for entry in fields.iter_mut() {
        if entry["name"] == "catalog_digest" {
            entry["name"] = serde_json::Value::String("ruleset_digest".to_string());
        }
    }

    let result = read_old_data_with_new_schema(CURRENT, &parsed.to_string());

    assert!(
        result.is_err(),
        "renaming a required field must fail: old records carry the old name and the \
         reader requires the new one"
    );
}

/// Dropping a field is safe in this direction, which is easy to get backwards.
/// The reader ignores what it does not know about, so old data still decodes.
/// It is FORWARD compatibility that a removal breaks, and this subject is not
/// registered FORWARD.
#[test]
fn removing_an_optional_field_is_backward_compatible() {
    let mut parsed: serde_json::Value = serde_json::from_str(CURRENT).expect("schema is JSON");
    let fields = parsed["fields"].as_array_mut().expect("fields");
    fields.retain(|entry| entry["name"] != "caller");

    let value = read_old_data_with_new_schema(CURRENT, &parsed.to_string())
        .expect("a reader may ignore a field it no longer knows about");

    assert!(
        field(&value, "caller").is_none(),
        "the removed field is absent from the decoded record"
    );
}

/// Enum symbols are the other place a schema quietly breaks. The default on the
/// enum is what lets an old reader survive meeting a symbol added later.
#[test]
fn the_document_type_enum_has_a_default_for_unknown_symbols() {
    let parsed: serde_json::Value = serde_json::from_str(CURRENT).expect("schema is JSON");
    let fields = parsed["fields"].as_array().expect("fields");

    let document_type = fields
        .iter()
        .find(|entry| entry["name"] == "document_type")
        .expect("document_type present");

    assert_eq!(
        document_type["type"]["default"], "UNKNOWN",
        "without an enum default, a reader meeting a document type added later fails to \
         decode the whole record rather than the one field"
    );

    let severity_default = fields
        .iter()
        .find(|entry| entry["name"] == "findings")
        .and_then(|entry| entry["type"]["items"]["fields"].as_array())
        .and_then(|finding_fields| {
            finding_fields
                .iter()
                .find(|entry| entry["name"] == "severity")
        })
        .map(|entry| entry["type"]["default"].clone())
        .expect("severity present");

    assert_eq!(severity_default, "UNKNOWN");
}
