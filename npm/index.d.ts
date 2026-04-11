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
  /** Detected VAST version string (e.g. "4.2"), or null if unknown. */
  version: string | null;
  issues: Issue[];
  summary: Summary;
}

export interface RuleMeta {
  id: string;
  default_severity: 'error' | 'warning' | 'info';
  description: string;
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
