// vastlint — TypeScript definitions
// These types describe the plain objects returned from the WASM module.

export interface Issue {
  /** Stable rule identifier, e.g. "VAST-2.0-root-version". */
  id: string;
  severity: 'error' | 'warning' | 'info';
  message: string;
  /** XPath-like location, e.g. "/VAST/Ad[0]/InLine/AdSystem". Null for document-level issues. */
  path: string | null;
  /** Short spec reference, e.g. "IAB VAST 4.1 §3.4.1". */
  spec_ref: string;
  /** 1-based line number of the opening tag that triggered this issue. Null for document-level issues. */
  line: number | null;
  /** 1-based column number (byte offset within the line) of the opening tag. Null for document-level issues. */
  col: number | null;
}

export interface Summary {
  errors: number;
  warnings: number;
  infos: number;
  /** True when errors === 0. */
  valid: boolean;
}

export interface ValidationResult {
  /** Detected document type: plain VAST, a VMAP playlist, or a DAAST audio ad. */
  document_type: 'VAST' | 'VMAP' | 'DAAST';
  /** Detected VAST version string (e.g. "4.2"), or null if unknown. */
  version: string | null;
  issues: Issue[];
  summary: Summary;
}

export interface RuleMeta {
  id: string;
  default_severity: 'error' | 'warning' | 'info';
  description: string;
  /** The external standard this rule is derived from. */
  source: 'VAST spec' | 'VAST XSD' | 'W3C XML 1.0' | 'RFC 3986' | 'IANA Media Types' | 'ISO 4217' | 'Ad-ID' | 'inferred';
}

export interface ValidateOptions {
  /** Current wrapper chain depth. Default: 0. */
  wrapper_depth?: number;
  /** Maximum allowed wrapper chain depth. Default: 5. */
  max_wrapper_depth?: number;
  /**
   * Per-rule severity overrides. Keys are rule IDs (see `rules()`).
   * Use "off" to silence a rule entirely.
   */
  rule_overrides?: Record<string, 'error' | 'warning' | 'info' | 'off'>;
}

export type InspectAdType = 'Wrapper' | 'InLine' | 'Unknown';

export interface InspectMediaFile {
  url: string;
  mimeType: string;
  delivery: string;
  width: string;
  height: string;
  bitrate: string;
}

export interface InspectDocumentResult {
  adType: InspectAdType;
  adSystem: string;
  adTitle: string;
  duration: string;
  impressionCount: number;
  trackingEventCount: number;
  mediaFiles: InspectMediaFile[];
  companionCount: number;
  wrapperUri: string | null;
}

/**
 * Validate a VAST XML string using default settings.
 *
 * @example
 * import { validate } from 'vastlint';
 * const result = validate(xml);
 * if (!result.summary.valid) { ... }
 */
export declare function validate(xml: string): ValidationResult;

/**
 * Validate a VAST XML string with caller-supplied options.
 *
 * @example
 * import { validateWithOptions } from 'vastlint';
 * const result = validateWithOptions(xml, {
 *   wrapper_depth: 2,
 *   rule_overrides: { 'VAST-2.0-mediafile-https': 'off' },
 * });
 */
export declare function validateWithOptions(
  xml: string,
  options: ValidateOptions
): ValidationResult;

/**
 * Extract creative and wrapper metadata from a single VAST XML document.
 */
export declare function inspectDocument(xml: string): InspectDocumentResult;

/**
 * Like `validate`, but filters the returned issues to those at or above
 * `minSeverity`. Useful when you only care about hard errors.
 *
 * @param minSeverity Defaults to "error".
 */
export declare function validateFiltered(
  xml: string,
  minSeverity?: 'error' | 'warning' | 'info'
): ValidationResult;

/**
 * Returns the full catalog of all known validation rules.
 */
export declare function rules(): RuleMeta[];

/** A single fix that was automatically applied to the document. */
export interface AppliedFix {
  /** The rule ID this fix addresses, e.g. `"VAST-2.0-mediafile-https"`. */
  rule_id: string;
  /** Human-readable description of what was changed. */
  description: string;
  /** XPath-like path to the element that was modified. */
  path: string;
}

export interface FixResult {
  /** The repaired VAST XML string. */
  xml: string;
  /** All fixes that were successfully applied, in document order. */
  applied: AppliedFix[];
  /** Issues that remain after all fixes were applied (require manual intervention). */
  remaining: Issue[];
}

/**
 * Fix a VAST XML string using default settings.
 * Upgrades http:// URLs to https://, repairs safe SIMID attributes, and removes deprecated attributes.
 *
 * @example
 * import { fix } from 'vastlint';
 * const result = fix(xml);
 * console.log(result.xml);           // repaired XML
 * console.log(result.applied);       // what was changed
 * console.log(result.remaining);     // issues that need manual attention
 */
export declare function fix(xml: string): FixResult;

/**
 * Fix a VAST XML string with caller-supplied options.
 * Accepts the same options object as `validateWithOptions`.
 */
export declare function fixWithOptions(xml: string, options: ValidateOptions): FixResult;
