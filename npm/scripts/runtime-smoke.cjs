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

test('plain Node ESM import resolves the node entry by package name', () => {
  const os = require('node:os');
  const { execFileSync } = require('node:child_process');
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'vastlint-esm-'));
  const pkgRoot = path.resolve(__dirname, '..');
  fs.mkdirSync(path.join(tmp, 'node_modules'));
  fs.symlinkSync(pkgRoot, path.join(tmp, 'node_modules', 'vastlint'));
  const xmlPath = path.join(tmp, 'tag.xml');
  fs.writeFileSync(xmlPath, readFixture(fixtureNames()[0]));
  fs.writeFileSync(
    path.join(tmp, 'probe.mjs'),
    [
      "import { createRequire } from 'node:module';",
      "import { readFileSync } from 'node:fs';",
      "import * as esm from 'vastlint';",
      "const cjs = createRequire(import.meta.url)('vastlint');",
      "const xml = readFileSync(process.argv[1], 'utf8');",
      "const a = esm.validate(xml).issues.map((i) => i.id);",
      "const b = cjs.validate(xml).issues.map((i) => i.id);",
      "if (JSON.stringify(Object.keys(esm).sort()) !== JSON.stringify(Object.keys(cjs).sort())) process.exit(2);",
      "if (JSON.stringify(a) !== JSON.stringify(b)) process.exit(3);",
    ].join('\n')
  );
  execFileSync(process.execPath, [path.join(tmp, 'probe.mjs'), xmlPath], {
    cwd: tmp,
    stdio: 'inherit',
  });
});

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