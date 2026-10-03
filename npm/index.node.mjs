/**
 * vastlint: Node.js ESM entry point
 *
 * `import { validate } from 'vastlint'` in plain Node lands here. index.js
 * imports the .wasm file as an ES module, which only bundlers understand, so
 * Node gets the CommonJS build re-exported as named ESM exports.
 */

import cjs from './index.cjs';

export const {
  validate,
  validateWithOptions,
  rules,
  fix,
  fixWithOptions,
  inspectDocument,
  validateFiltered,
} = cjs;
