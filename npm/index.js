/**
 * vastlint — VAST XML validator
 *
 * ESM entry point. Loads the WASM module lazily on first call.
 *
 * @example
 * import { validate } from 'vastlint';
 *
 * const result = validate(xmlString);
 * if (!result.summary.valid) {
 *   result.issues
 *     .filter(i => i.severity === 'error')
 *     .forEach(i => console.error(`[${i.id}] ${i.message} at ${i.path}`));
 * }
 */

// wasm-pack generates vastlint_wasm.js (ESM glue) and vastlint_wasm_bg.wasm.
// assemble.js copies both to the package root during the build.
import * as _wasm from './vastlint_wasm.js';
export const { validate, validateWithOptions, rules } = _wasm;

/**
 * Validate a VAST XML string and return only issues at or above a minimum severity.
 *
 * @param {string} xml
 * @param {"error"|"warning"|"info"} [minSeverity="error"]
 * @returns {import('./index.d.ts').ValidationResult}
 */
export function validateFiltered(xml, minSeverity = 'error') {
  const order = { error: 2, warning: 1, info: 0 };
  const min = order[minSeverity] ?? 0;
  const result = _wasm.validate(xml);
  return {
    ...result,
    issues: result.issues.filter(i => (order[i.severity] ?? 0) >= min),
  };
}
