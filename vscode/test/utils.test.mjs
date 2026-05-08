/**
 * Unit tests for the pure utility functions in vscode/out/utils.js.
 *
 * Run with: node --test test/utils.test.mjs
 * (Node 20+ built-in test runner, no extra dependencies required.)
 *
 * The test file imports the *compiled* JavaScript output (out/utils.js),
 * so run `tsc` or `npm run compile-emit` before executing these tests.
 */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const __filename = fileURLToPath(import.meta.url);
const __dirname  = dirname(__filename);
const require    = createRequire(import.meta.url);

const { applyTemplateIgnore, extractVastBlocks, mapBlockIssuePosition } =
  require(join(__dirname, '..', 'out', 'utils.js'));

// ── applyTemplateIgnore ───────────────────────────────────────────────────────

test('applyTemplateIgnore: empty regexStr returns text unchanged', () => {
  const text = '<VAST version="{{VERSION}}">...</VAST>';
  assert.equal(applyTemplateIgnore(text, ''), text);
});

test('applyTemplateIgnore: Mustache expression replaced with zeros of same length', () => {
  const text = 'prefix {{FOO}} suffix';
  const result = applyTemplateIgnore(text, '\\{\\{[^}]*\\}\\}');
  // {{FOO}} is 7 chars → should become 0000000
  assert.equal(result, 'prefix 0000000 suffix');
  assert.equal(result.length, text.length, 'length must be preserved');
});

test('applyTemplateIgnore: ERB expression <%= val %> replaced same length', () => {
  const text = 'before <%= myVar %> after';
  const result = applyTemplateIgnore(text, '<%=?.*?%>');
  assert.equal(result.length, text.length, 'length must be preserved');
  // The expression must be replaced with all zeros
  const replaced = result.slice('before '.length, result.length - ' after'.length);
  assert.match(replaced, /^0+$/, 'replaced section must be all zeros');
});

test('applyTemplateIgnore: multiple matches all replaced', () => {
  const text = '{{A}} text {{BB}} more {{CCC}}';
  const result = applyTemplateIgnore(text, '\\{\\{[^}]*\\}\\}');
  assert.equal(result.length, text.length);
  // No {{ or }} should remain
  assert.ok(!result.includes('{{'), 'no {{ should remain');
  assert.ok(!result.includes('}}'), 'no }} should remain');
});

test('applyTemplateIgnore: invalid regex returns text unchanged', () => {
  const text = '<VAST>[invalid</VAST>';
  const result = applyTemplateIgnore(text, '[invalid');
  assert.equal(result, text, 'invalid regex must return original text');
});

test('applyTemplateIgnore: multiline match with gs flags works', () => {
  const text = 'line1\n${VAR}\nline3';
  const result = applyTemplateIgnore(text, '\\$\\{[^}]+\\}');
  assert.equal(result.length, text.length);
  assert.equal(result, 'line1\n000000\nline3');
});

// ── extractVastBlocks ─────────────────────────────────────────────────────────

const SIMPLE_VAST = `<VAST version="2.0"><Ad id="1"><InLine/></Ad></VAST>`;

test('extractVastBlocks: no VAST in text returns empty array', () => {
  const blocks = extractVastBlocks('<html><body>no vast here</body></html>');
  assert.deepEqual(blocks, []);
});

test('extractVastBlocks: single block at start of file — startLine=0, startCol=0', () => {
  const blocks = extractVastBlocks(SIMPLE_VAST);
  assert.equal(blocks.length, 1);
  assert.equal(blocks[0].startLine, 0);
  assert.equal(blocks[0].startCol, 0);
  assert.equal(blocks[0].xml, SIMPLE_VAST);
});

test('extractVastBlocks: block on second line — startLine=1', () => {
  const text = 'preamble\n' + SIMPLE_VAST;
  const blocks = extractVastBlocks(text);
  assert.equal(blocks.length, 1);
  assert.equal(blocks[0].startLine, 1, 'block should be on line index 1 (second line)');
  assert.equal(blocks[0].startCol, 0);
});

test('extractVastBlocks: block with column offset — startCol correct', () => {
  // Block doesn't start at column 0 (e.g. embedded in template)
  const prefix = 'var xml = ';
  const text = prefix + SIMPLE_VAST;
  const blocks = extractVastBlocks(text);
  assert.equal(blocks.length, 1);
  assert.equal(blocks[0].startLine, 0);
  assert.equal(blocks[0].startCol, prefix.length);
});

test('extractVastBlocks: block on third line with column offset', () => {
  const text = 'line0\nline1\n  some prefix ' + SIMPLE_VAST;
  const blocks = extractVastBlocks(text);
  assert.equal(blocks.length, 1);
  assert.equal(blocks[0].startLine, 2);
  assert.equal(blocks[0].startCol, '  some prefix '.length);
});

test('extractVastBlocks: two VAST blocks in one document — both found', () => {
  const text = SIMPLE_VAST + '\n' + SIMPLE_VAST;
  const blocks = extractVastBlocks(text);
  assert.equal(blocks.length, 2, 'should find both VAST blocks');
  assert.equal(blocks[0].startLine, 0);
  assert.equal(blocks[0].startCol, 0);
  assert.equal(blocks[1].startLine, 1);
  assert.equal(blocks[1].startCol, 0);
});

test('extractVastBlocks: two blocks separated by non-VAST content', () => {
  const text = 'context1\n' + SIMPLE_VAST + '\nsome text\n' + SIMPLE_VAST + '\nend';
  const blocks = extractVastBlocks(text);
  assert.equal(blocks.length, 2);
  assert.equal(blocks[0].startLine, 1);
  assert.equal(blocks[1].startLine, 3);
});

test('extractVastBlocks: xml content preserved exactly', () => {
  const blocks = extractVastBlocks(SIMPLE_VAST);
  assert.equal(blocks[0].xml, SIMPLE_VAST);
});

test('extractVastBlocks: unclosed VAST tag returns empty', () => {
  // Missing </VAST> — should not return a partial block
  const text = '<VAST version="2.0"><Ad id="1"><InLine/></Ad>';
  const blocks = extractVastBlocks(text);
  assert.equal(blocks.length, 0);
});

// ── mapBlockIssuePosition ─────────────────────────────────────────────────────

test('mapBlockIssuePosition: block at (0,0), issue line=1 col=1 → docLine=0 docCol=0', () => {
  const block = { startLine: 0, startCol: 0 };
  const { docLine, docCol } = mapBlockIssuePosition(1, 1, block);
  assert.equal(docLine, 0);
  assert.equal(docCol, 0);
});

test('mapBlockIssuePosition: first line of block applies startCol offset', () => {
  // Block starts at line 5, col 10.  Issue is on first block line (blockLine=1).
  // Expected: docLine = 5+0=5, docCol = 10+(3-1)=12
  const block = { startLine: 5, startCol: 10 };
  const { docLine, docCol } = mapBlockIssuePosition(1, 3, block);
  assert.equal(docLine, 5);
  assert.equal(docCol, 12);
});

test('mapBlockIssuePosition: subsequent lines do NOT apply startCol offset', () => {
  // Block starts at line 5, col 10.  Issue is on second block line (blockLine=2).
  // Expected: docLine = 5+1=6, docCol = 3-1=2 (no startCol applied)
  const block = { startLine: 5, startCol: 10 };
  const { docLine, docCol } = mapBlockIssuePosition(2, 3, block);
  assert.equal(docLine, 6);
  assert.equal(docCol, 2);
});

test('mapBlockIssuePosition: block at middle of document, issue on block line 3', () => {
  const block = { startLine: 10, startCol: 4 };
  // blockLineIdx = 3-1 = 2 (not zero) → no col offset
  const { docLine, docCol } = mapBlockIssuePosition(3, 7, block);
  assert.equal(docLine, 12); // 10+2
  assert.equal(docCol, 6);   // 7-1 (no offset since line > 0)
});

test('mapBlockIssuePosition: col=1 on first line → docCol equals startCol', () => {
  const block = { startLine: 0, startCol: 8 };
  const { docLine, docCol } = mapBlockIssuePosition(1, 1, block);
  assert.equal(docLine, 0);
  assert.equal(docCol, 8); // 8 + (1-1) = 8
});

// ── Integration: extractVastBlocks + mapBlockIssuePosition ───────────────────

test('position mapping: issue in second block is correctly offset', () => {
  // Place two blocks — second block starts on line 3, col 2
  const block1Xml = SIMPLE_VAST;
  const block2Xml = SIMPLE_VAST;
  const text = block1Xml + '\nline2\n  ' + block2Xml;
  const blocks = extractVastBlocks(text);
  assert.equal(blocks.length, 2);

  const b2 = blocks[1];
  assert.equal(b2.startLine, 2);
  assert.equal(b2.startCol, 2);

  // Suppose validator reports issue at block-relative line=1, col=1 (the <VAST tag)
  const { docLine, docCol } = mapBlockIssuePosition(1, 1, b2);
  assert.equal(docLine, 2); // block starts on doc line 2
  assert.equal(docCol, 2);  // startCol applied on first line
});

test('position mapping: issue on line 2 of second block has no col offset', () => {
  const multiLineVast = `<VAST version="2.0">
  <Ad id="1"><InLine/></Ad>
</VAST>`;
  const text = 'line0\n' + multiLineVast;
  const blocks = extractVastBlocks(text);
  assert.equal(blocks.length, 1);
  assert.equal(blocks[0].startLine, 1);
  assert.equal(blocks[0].startCol, 0);

  // Issue at block-relative line=2 col=3 (inside the block)
  const { docLine, docCol } = mapBlockIssuePosition(2, 3, blocks[0]);
  assert.equal(docLine, 2); // line 1 + (2-1) = 2
  assert.equal(docCol, 2);  // col 3-1=2, no startCol (blockLineIdx=1)
});
