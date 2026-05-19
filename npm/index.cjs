/**
 * vastlint — CommonJS shim
 *
 * Node.js consumers using `require('vastlint')` land here.
 * The actual implementation is the wasm-pack --target nodejs output.
 */

// wasm-pack --target nodejs produces CJS. assemble.js copies it to vastlint_wasm_cjs.js
// at the package root during the build.
const wasm = require('./vastlint_wasm_cjs.js');

module.exports = {
  validate: wasm.validate,
  validateWithOptions: wasm.validateWithOptions,
  rules: wasm.rules,
  fix: wasm.fix,
  fixWithOptions: wasm.fixWithOptions,
  inspectDocument: wasm.inspectDocument,
  validateFiltered(xml, minSeverity = 'error') {
    const order = { error: 2, warning: 1, info: 0 };
    const min = order[minSeverity] ?? 0;
    const result = wasm.validate(xml);
    return {
      ...result,
      issues: result.issues.filter(i => (order[i.severity] ?? 0) >= min),
    };
  },
};
