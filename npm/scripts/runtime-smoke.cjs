const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const test = require('node:test');

const vastlint = require(path.resolve(__dirname, '..', 'index.cjs'));
const fixturesDir = path.resolve(
  __dirname,
  '..',
  '..',
  'crates',
  'vastlint-core',
  'tests',
  'fixtures'
);

function fixtureNames() {
  return fs
    .readdirSync(fixturesDir)
    .filter((name) => name.endsWith('.xml'))
    .sort();
}

function readFixture(name) {
  return fs.readFileSync(path.join(fixturesDir, name), 'utf8');
}

function issueIds(result) {
  return result.issues.map((issue) => issue.id);
}

test('packaged runtime validates the core fixture corpus without throwing', () => {
  for (const name of fixtureNames()) {
    const result = vastlint.validate(readFixture(name));
    assert.equal(Array.isArray(result.issues), true, `${name} should return an issues array`);
    assert.equal(typeof result.summary.errors, 'number', `${name} should return summary.errors`);
    assert.equal(typeof result.summary.warnings, 'number', `${name} should return summary.warnings`);
    assert.equal(typeof result.summary.infos, 'number', `${name} should return summary.infos`);
    assert.equal(typeof result.summary.valid, 'boolean', `${name} should return summary.valid`);
  }
});

test('packaged runtime avoids false version mismatches on valid fixtures', () => {
  for (const name of fixtureNames().filter((fixture) => fixture.startsWith('valid_'))) {
    const ids = issueIds(vastlint.validate(readFixture(name)));
    assert.equal(
      ids.includes('VAST-2.0-version-mismatch'),
      false,
      `${name} unexpectedly emitted VAST-2.0-version-mismatch: ${ids.join(', ')}`
    );
  }
});

test('packaged runtime preserves the explicit version mismatch fixture', () => {
  const ids = issueIds(vastlint.validate(readFixture('warn_version_mismatch.xml')));
  assert.equal(ids.includes('VAST-2.0-version-mismatch'), true, 'warn_version_mismatch.xml should still emit VAST-2.0-version-mismatch');
});

test('packaged runtime short-circuits malformed fixtures to parse error only', () => {
  for (const name of [
    'err_malformed_mismatched_close.xml',
    'err_malformed_broken_attr_quote.xml',
    'err_malformed_unclosed_cdata.xml',
  ]) {
    const ids = issueIds(vastlint.validate(readFixture(name)));
    assert.deepEqual(ids, ['VAST-2.0-parse-error'], `${name} should only emit VAST-2.0-parse-error, got ${ids.join(', ')}`);
  }
});