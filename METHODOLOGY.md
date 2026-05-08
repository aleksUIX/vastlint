# Rule Derivation Methodology

vastlint rules are derived from multiple authoritative sources. The mapping from source to severity is mechanical, not editorial.

---

## Sources

### 1. W3C XML 1.0 — well-formedness

All input is parsed against the W3C XML 1.0 Recommendation (*Extensible Markup Language 1.0*, W3C REC-xml). A document that is not well-formed is rejected before any spec rule runs. Well-formedness is a binary, formally defined property — no interpretation involved.

### 2. IAB VAST XSD formal schemas

IAB Tech Lab publishes W3C XML Schema Definition (XSD) files for VAST 2.0.1, 3.0, 4.0, 4.1, and 4.2 in the [InteractiveAdvertisingBureau/vast](https://github.com/InteractiveAdvertisingBureau/vast) repository.

XSD is a W3C Recommendation (formally: *XML Schema Part 1: Structures*, W3C REC-xmlschema-1). Constraints expressed in XSD have no ambiguity: `use="required"`, `minOccurs > 0`, and enumerated type restrictions are machine-verifiable by definition. No interpretation is involved.

VAST 4.3 has no published XSD. Rules for 4.3 are derived exclusively from source 3.

### 3. IAB spec prose — RFC 2119 normative key words

The IAB VAST specification is written in controlled normative language. The key words MUST, MUST NOT, REQUIRED, SHALL, SHALL NOT, SHOULD, SHOULD NOT, RECOMMENDED, MAY, and OPTIONAL are used in the RFC 2119 / BCP 14 sense ([RFC 2119](https://www.rfc-editor.org/rfc/rfc2119), updated by [RFC 8174](https://www.rfc-editor.org/rfc/rfc8174)).

RFC 2119 is a Controlled Natural Language (CNL): a deliberately restricted subset of English where specific words have formally defined, machine-interpretable meanings. Parsing spec prose for RFC 2119 key words is therefore not free-form interpretation — each sentence yields a binary predicate over the document model.

### 4. RFC 3986 — URI syntax

All URL fields in a VAST document are validated against the URI generic syntax defined in [RFC 3986](https://www.rfc-editor.org/rfc/rfc3986). Validation is delegated to the `url` crate, which implements the WHATWG URL Standard (a superset of RFC 3986). An unparseable URL is a structural error regardless of what the VAST spec says about the field.

### 5. ISO 4217 — currency codes

The `currency` attribute on `<Pricing>` elements is validated against the ISO 4217 three-letter alphabetic code list. ISO 4217 is the international standard for currency codes, maintained by the ISO 4217 Maintenance Agency.

### 6. IANA Media Types

MIME types on `<MediaFile>`, `<InteractiveCreativeFile>`, `<Mezzanine>`, and `<ClosedCaptionFile>` elements are validated against the IANA Media Types registry. The registry is the authoritative list of registered media type strings.

### 7. IAB Tech Lab SIMID spec

Rules for `<InteractiveCreativeFile apiFramework="SIMID">` are derived from the IAB Tech Lab SIMID specification (versions 1.0, 1.0.1, 1.1, 1.2), a separate normative document from the VAST spec. The same RFC 2119 extraction process applies.

### 8. Industry practice and addenda (advisory only)

The CTV Addendum (2024) and operational best practices (HTTPS enforcement, duplicate impression detection, wrapper depth limits) are not captured in XSD or normative prose. Rules derived from these sources are advisory and assigned `warning` or `info` severity. They are labelled `IndustryBestPractice` or `Inferred` in the rule catalog.

---

## Severity mapping

| Source | Criterion | Severity |
|---|---|---|
| W3C XML 1.0 | Not well-formed | `error` (parse stage) |
| XSD | `use="required"` or `minOccurs > 0` | `error` |
| XSD | Enumerated type restriction | `error` |
| Spec prose (VAST, SIMID) | MUST / MUST NOT / REQUIRED / SHALL | `error` |
| Spec prose (VAST, SIMID) | SHOULD / SHOULD NOT / RECOMMENDED | `warning` |
| Spec prose (VAST, SIMID) | MAY / OPTIONAL | not raised |
| RFC 3986 | URI parse failure | `warning` |
| ISO 4217 | Code not in registry | `warning` |
| IANA Media Types | Unrecognised type | `warning` |
| Industry practice / addenda | Security, interoperability, CTV | `warning` or `info` |

---

## Derivation process

Each rule follows the same three steps:

1. **Normative requirement extraction** — identify the XSD constraint or RFC 2119 key word sentence in the source document
2. **Formalization** — express the requirement as a predicate over the internal document model (element present, attribute in enumerated set, value matches pattern, etc.)
3. **Test case derivation** — produce one valid and one invalid fixture for each predicate

The spec quote or XSD line that justifies each rule and its severity is recorded in a source comment next to the rule implementation. No rule ships without a traceable citation.

This process corresponds to the conformance test suite derivation methodology described in W3C QA Framework: Specification Guidelines ([W3C QA Framework](https://www.w3.org/TR/qaframe-spec/)) and ISO/IEC 9646 (Conformance Testing Methodology and Framework).

---

## Challenging a rule

To dispute an `error`-severity rule, you need to dispute one of:

- the IAB VAST XSD schema, or
- the IAB VAST or SIMID specification prose, or
- the RFC 2119 definition of MUST or SHOULD

To dispute a `warning`-severity rule derived from RFC 3986, ISO 4217, or IANA Media Types, you need to dispute the relevant external standard.

The severity mapping is a direct consequence of those documents. It is not a product decision.

---

## References

- [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119) — Key words for use in RFCs to Indicate Requirement Levels (Bradner, 1997)
- [RFC 8174](https://www.rfc-editor.org/rfc/rfc8174) — Ambiguity of Uppercase vs Lowercase in RFC 2119 Key Words (Leiba, 2017)
- [RFC 3986](https://www.rfc-editor.org/rfc/rfc3986) — Uniform Resource Identifier (URI): Generic Syntax (Berners-Lee et al., 2005)
- [W3C XML 1.0](https://www.w3.org/TR/xml/) — Extensible Markup Language 1.0 (5th ed., 2008)
- [W3C XML Schema Part 1: Structures](https://www.w3.org/TR/xmlschema-1/) — W3C REC-xmlschema-1 (2004)
- [W3C QA Framework: Specification Guidelines](https://www.w3.org/TR/qaframe-spec/) — W3C Working Group Note (2005)
- ISO 4217 — Codes for the representation of currencies (ISO, current edition)
- [IANA Media Types registry](https://www.iana.org/assignments/media-types/) — maintained by IANA
- ISO/IEC 9646 — Information technology: OSI Conformance Testing Methodology and Framework (1994)
- [IAB Tech Lab VAST repository](https://github.com/InteractiveAdvertisingBureau/vast) — published XSD schemas
- [IAB Tech Lab SIMID](https://iabtechlab.com/simid/) — Secure Interactive Media Interface Definition
