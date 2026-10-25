# XML parsing rejects malformed documents and bounds resource use.

The parser reports an error for unclosed or multiple roots, duplicate or
malformed attributes, undefined entities, invalid XML 1.0 characters and names,
invalid XML declarations, and text or CDATA outside the document root. Attribute
references are decoded and XML attribute normalization is applied before rules
inspect their values.

Every core entry point uses the same limits: 4 MiB of input, 128 element levels,
and 100,000 elements. XML 1.1 declarations and DOCTYPE processing are unsupported.
Custom and external entities are never expanded or fetched.

These checks change behavior for every consumer of the core library, including
the CLI, gRPC service, WebAssembly and FFI. Previously accepted malformed input
now produces the existing parse-error finding. Well-formed documents beyond the
resource limits, or using unsupported XML features, also produce that finding.
Callers should not interpret every parse error as proof of malformed XML.

This change does not add monitoring, rule configuration, a transport protocol,
or new fields to the public validation result. The parser tests cover rejected
inputs, valid references and declarations, and resource boundaries.
