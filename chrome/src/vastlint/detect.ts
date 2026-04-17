/**
 * VAST XML detection helpers.
 *
 * A string is a VAST candidate if it contains the opening tag of a VAST root
 * element (with or without a namespace / version attribute).
 * We intentionally keep this fast and loose — false positives are filtered out
 * by the WASM parser which will simply return a validation error.
 */

// Matches the opening of a <VAST …> tag (case-insensitive, with optional BOM/whitespace)
export const VAST_SIGNATURE_RE = /(?:<\?xml[^>]*>[\s\S]{0,200})?<VAST[\s>\/]/i;
