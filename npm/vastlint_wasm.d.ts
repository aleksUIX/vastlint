/* tslint:disable */
/* eslint-disable */

/**
 * Returns the full rule catalog as a JS array.
 *
 * Each element: `{ id: string, default_severity: string, description: string }`.
 */
export function rules(): any;

/**
 * Validate a VAST XML string.
 *
 * Returns a plain JavaScript object shaped like:
 * ```ts
 * {
 *   version: string | null,
 *   issues: Array<{
 *     id: string,
 *     severity: "error" | "warning" | "info",
 *     message: string,
 *     path: string | null,
 *     spec_ref: string,
 *   }>,
 *   summary: { errors: number, warnings: number, infos: number, valid: boolean },
 * }
 * ```
 *
 * Throws a JS `Error` if the argument is not a string.
 */
export function validate(xml: string): any;

/**
 * Validate a VAST XML string with caller-supplied options.
 *
 * `options` is a plain JS object with optional fields:
 * ```ts
 * {
 *   wrapper_depth?: number,
 *   max_wrapper_depth?: number,
 *   rule_overrides?: Record<string, "error" | "warning" | "info" | "off">,
 * }
 * ```
 */
export function validateWithOptions(xml: string, options: any): any;
