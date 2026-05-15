/**
 * Full-page VAST analysis tab.
 * Receives XML via chrome.storage.session key 'paste_xml' (set by popup)
 * or lets the user paste directly into the editor.
 */

import { validateVast } from '../vastlint/validator';
import type { ValidationResult, Issue } from '../types/vastlint';

function createNode<K extends keyof HTMLElementTagNameMap>(tag: K, className?: string): HTMLElementTagNameMap[K] {
  const el = document.createElement(tag);
  if (className) el.className = className;
  return el;
}

type DomContainer = DocumentFragment | HTMLElement;

function createStyledSpan(className: string, text: string): HTMLSpanElement {
  const span = createNode('span', className);
  span.textContent = text;
  return span;
}

function appendPlainText(parent: DomContainer, text: string) {
  if (!text) return;
  parent.appendChild(document.createTextNode(text));
}

function appendStyledText(parent: DomContainer, className: string, text: string) {
  if (!text) return;
  parent.appendChild(createStyledSpan(className, text));
}

function createTooltipRow(iss: Issue): HTMLDivElement {
  const row = createNode('div', 'tip-row');

  const meta = createNode('div', 'tip-meta');
  const idLink = createNode('a', 'tip-id');
  idLink.href = `https://vastlint.org/docs/rules/${encodeURIComponent(iss.id)}`;
  idLink.target = '_blank';
  idLink.rel = 'noopener';
  idLink.textContent = iss.id;
  meta.appendChild(idLink);

  if (iss.line) {
    const line = createNode('span', 'tip-line');
    line.textContent = `line ${iss.line}`;
    meta.appendChild(line);
  }

  const msg = createNode('div', 'tip-msg');
  msg.textContent = iss.message;

  row.appendChild(meta);
  row.appendChild(msg);
  return row;
}

function createTopbarPills(result: ValidationResult): DocumentFragment {
  const { errors, warnings, infos } = result.summary;
  const frag = document.createDocumentFragment();

  if (errors > 0) frag.appendChild(createNode('span', 'tpill tpill-err')).textContent = `${errors}E`;
  if (warnings > 0) frag.appendChild(createNode('span', 'tpill tpill-warn')).textContent = `${warnings}W`;
  if (infos > 0) frag.appendChild(createNode('span', 'tpill tpill-info')).textContent = `${infos}I`;
  if (errors + warnings + infos === 0) frag.appendChild(createNode('span', 'tpill tpill-ok')).textContent = '✓ Clean';
  if (result.version) frag.appendChild(createNode('span', 'ver-pill')).textContent = `VAST ${result.version}`;

  return frag;
}

function createIssueCard(iss: Issue): HTMLDivElement {
  const card = createNode('div', `issue-card sev-${iss.severity}`);
  card.dataset.sev = iss.severity;

  const meta = createNode('div', 'iss-meta');
  const idLink = createNode('a', 'iss-id');
  idLink.href = `https://vastlint.org/docs/rules/${encodeURIComponent(iss.id)}`;
  idLink.target = '_blank';
  idLink.rel = 'noopener';
  idLink.textContent = iss.id;
  meta.appendChild(idLink);

  const pathShort = iss.path ? iss.path.split('/').slice(-2).join('/') : null;
  if (pathShort) {
    const pathEl = createNode('span', 'iss-path');
    pathEl.title = iss.path ?? '';
    pathEl.textContent = pathShort;
    meta.appendChild(pathEl);
  }

  if (iss.line) {
    const line = createNode('span', 'iss-line');
    line.textContent = `line ${iss.line}`;
    meta.appendChild(line);
  }

  const msg = createNode('div', 'iss-msg');
  msg.textContent = iss.message;

  card.appendChild(meta);
  card.appendChild(msg);

  if (iss.spec_ref) {
    const spec = createNode('div', 'iss-spec');
    spec.textContent = iss.spec_ref;
    card.appendChild(spec);
  }

  return card;
}

function createIssueGroup(sev: 'error' | 'warning' | 'info', issues: Issue[]): HTMLDivElement {
  const group = createNode('div', 'issue-group');

  const hdr = createNode('div', 'issue-group-hdr');
  const badge = createNode('span', `sev-badge sev-badge-${sev}`);
  badge.textContent = sev.toUpperCase();
  const count = createNode('span', 'group-count');
  const label = sev === 'error' ? 'error' : sev === 'warning' ? 'warning' : 'advisory';
  count.textContent = `${issues.length} ${label}${issues.length !== 1 ? 's' : ''}`;
  hdr.appendChild(badge);
  hdr.appendChild(count);

  const items = createNode('div', 'issue-group-items');
  items.replaceChildren(...issues.map(createIssueCard));

  group.appendChild(hdr);
  group.appendChild(items);
  return group;
}

function showErrorMessage(container: HTMLElement, msg: string) {
  const box = createNode('div');
  box.style.padding = '20px';
  box.style.color = '#ef5350';
  box.style.fontSize = '13px';

  const strong = createNode('strong');
  strong.textContent = '⚠ Validation error';
  box.appendChild(strong);
  box.appendChild(document.createElement('br'));
  box.appendChild(document.createElement('br'));
  box.appendChild(document.createTextNode(msg));

  container.replaceChildren(box);
}

// ── XML Syntax Highlighter ───────────────────────────────────────────────────
function countNL(s: string): number {
  let n = 0; for (let i = 0; i < s.length; i++) if (s[i] === '\n') n++; return n;
}

function highlightXml(raw: string, lineIssues?: Map<number, Issue[]>): DocumentFragment {
  const out = document.createDocumentFragment();
  let i = 0;
  let curLine = 1;
  const squiggled = new Set<number>();

  function sq(content: Node, startLine: number, chunk: string): Node {
    if (!lineIssues || squiggled.has(startLine) || chunk.includes('\n')) return content;
    const issues = lineIssues.get(startLine);
    if (!issues) return content;
    squiggled.add(startLine);
    const sev = issues[0].severity;
    const wrapper = createNode('span', `sq sq-${sev}`);
    wrapper.appendChild(content);
    return wrapper;
  }

  while (i < raw.length) {
    if (raw.startsWith('<!--', i)) {
      const end = raw.indexOf('-->', i + 4);
      const chunk = end === -1 ? raw.slice(i) : raw.slice(i, end + 3);
      const sl = curLine; curLine += countNL(chunk);
      out.appendChild(sq(createStyledSpan('xt-comment', chunk), sl, chunk));
      i += chunk.length; continue;
    }
    if (raw.startsWith('<![CDATA[', i)) {
      const end = raw.indexOf(']]>', i + 9);
      const chunk = end === -1 ? raw.slice(i) : raw.slice(i, end + 3);
      const sl = curLine; curLine += countNL(chunk);
      out.appendChild(sq(createStyledSpan('xt-cdata', chunk), sl, chunk));
      i += chunk.length; continue;
    }
    if (raw.startsWith('<?', i)) {
      const end = raw.indexOf('?>', i + 2);
      const chunk = end === -1 ? raw.slice(i) : raw.slice(i, end + 2);
      const sl = curLine; curLine += countNL(chunk);
      out.appendChild(sq(createStyledSpan('xt-pi', chunk), sl, chunk));
      i += chunk.length; continue;
    }
    if (raw[i] === '<') {
      let j = i + 1; let inQuote = '';
      while (j < raw.length) {
        const c = raw[j];
        if (inQuote) { if (c === inQuote) inQuote = ''; }
        else if (c === '"' || c === "'") { inQuote = c; }
        else if (c === '>') { j++; break; }
        j++;
      }
      const chunk = raw.slice(i, j);
      const sl = curLine; curLine += countNL(chunk);
      out.appendChild(sq(colorizeTag(chunk), sl, chunk));
      i = j; continue;
    }
    if (raw[i] === '&') {
      const end = raw.indexOf(';', i);
      if (end !== -1 && end - i < 16) {
        out.appendChild(createStyledSpan('xt-entity', raw.slice(i, end + 1)));
        i = end + 1; continue;
      }
    }
    let j = i + 1;
    while (j < raw.length && raw[j] !== '<' && raw[j] !== '&') j++;
    const text = raw.slice(i, j);
    curLine += countNL(text);
    appendPlainText(out, text);
    i = j;
  }
  return out;
}

function colorizeAttrs(chunk: string): DocumentFragment {
  const frag = document.createDocumentFragment();
  const attrPattern = /([\w:\-\.]+)(\s*=\s*)(["'])((?:[^"'\\]|\\.)*)(\3)|([\w:\-\.]+)|(\s+)/g;
  let match: RegExpExecArray | null;

  while ((match = attrPattern.exec(chunk)) !== null) {
    const aName = match[1];
    const eq = match[2];
    const q = match[3];
    const val = match[4];
    const bare = match[6];
    const ws = match[7];

    if (ws) {
      appendPlainText(frag, ws);
      continue;
    }
    if (bare) {
      appendStyledText(frag, 'xt-attr', bare);
      continue;
    }
    if (aName) {
      appendStyledText(frag, 'xt-attr', aName);
      appendStyledText(frag, 'xt-punct', eq ?? '');
      appendStyledText(frag, 'xt-punct', q ?? '');
      appendStyledText(frag, 'xt-val', val ?? '');
      appendStyledText(frag, 'xt-punct', q ?? '');
    }
  }

  return frag;
}

function colorizeTag(tag: string): DocumentFragment {
  const frag = document.createDocumentFragment();
  const closeM = tag.match(/^(<\/)([\w:\-\.]+)(>)$/);
  if (closeM) {
    appendStyledText(frag, 'xt-punct', closeM[1]);
    appendStyledText(frag, 'xt-tag', closeM[2]);
    appendStyledText(frag, 'xt-punct', closeM[3]);
    return frag;
  }

  let i = 1;
  appendStyledText(frag, 'xt-punct', '<');
  if (tag[i] === '/') {
    appendStyledText(frag, 'xt-punct', '/');
    i++;
  }
  const nameStart = i;
  while (i < tag.length && !/[ \/>\n\t]/.test(tag[i])) i++;
  appendStyledText(frag, 'xt-tag', tag.slice(nameStart, i));
  const tail = tag.slice(i);
  const isSelfClose = tail.trimEnd().endsWith('/>');
  const inner = tail.slice(0, tail.lastIndexOf(isSelfClose ? '/>' : '>'));
  frag.appendChild(colorizeAttrs(inner));
  appendStyledText(frag, 'xt-punct', isSelfClose ? '/>' : '>');
  return frag;
}

// ── DOM refs ─────────────────────────────────────────────────────────────────
const xmlInput        = document.getElementById('xml-input')         as HTMLTextAreaElement;
const xmlLineHlWrap   = document.getElementById('xml-line-hl-wrap')!;
const xmlLineHlInner  = document.getElementById('xml-line-hl-inner')!;
const xmlHlCode     = document.getElementById('xml-hl-code')!;
const xmlHl         = document.getElementById('xml-hl')!;
const analyzeBtn    = document.getElementById('analyze-btn')     as HTMLButtonElement;
const analyzingInd  = document.getElementById('analyzing-indicator')!;
const charCount     = document.getElementById('char-count')!;
const topbarTitle   = document.getElementById('topbar-title')!;
const topbarPills   = document.getElementById('topbar-pills')!;
const resultsHdr    = document.getElementById('results-hdr')!;
const resultsHdrLbl = document.getElementById('results-hdr-label')!;
const filterBar     = document.getElementById('filter-bar')!;
const loadingState  = document.getElementById('loading-state')!;
const emptyState    = document.getElementById('results-empty')!;
const issueList     = document.getElementById('issue-list')!;
const cleanState    = document.getElementById('clean-state')!;
const xmlTooltip    = document.getElementById('xml-tooltip')!;

// ── Per-line issue state (set after analysis, cleared on new run) ─────────────
let currentLineIssueMap: Map<number, Issue[]> = new Map();

// ── Tooltip ───────────────────────────────────────────────────────────────────
const SEV_COLOR: Record<string, string> = {
  error: '#ef4444', warning: '#f59e0b', info: '#818cf8',
};

function showTooltip(clientX: number, clientY: number, issues: Issue[]) {
  const topSev = issues[0].severity;
  const color  = SEV_COLOR[topSev] ?? '#8aa7c8';
  xmlTooltip.replaceChildren(...issues.map(createTooltipRow));
  xmlTooltip.style.borderColor = color;
  xmlTooltip.style.display = 'block';
  // Position: prefer above cursor, flip below if too close to top
  const TIP_W = 320;
  const TIP_OFFSET = 12;
  let left = clientX - TIP_W / 2;
  left = Math.max(8, Math.min(left, window.innerWidth - TIP_W - 8));
  xmlTooltip.style.left = `${left}px`;
  xmlTooltip.style.top  = '0px'; // measure after display
  const h = xmlTooltip.offsetHeight;
  const top = clientY - h - TIP_OFFSET;
  xmlTooltip.style.top = `${top < 8 ? clientY + TIP_OFFSET : top}px`;
}

function hideTooltip() {
  xmlTooltip.style.display = 'none';
}

// ── Highlight + scroll sync ───────────────────────────────────────────────────
function updateHighlight() {
  xmlHlCode.replaceChildren(
    highlightXml(xmlInput.value, currentLineIssueMap.size ? currentLineIssueMap : undefined),
    document.createTextNode('\n'),
  );
}

xmlInput.addEventListener('input', () => {
  updateHighlight();
  const len = xmlInput.value.length;
  charCount.textContent = len > 0 ? `${len.toLocaleString()} chars` : '';
  scheduleAnalysis();
});

// Paste: fire immediately after value is updated
xmlInput.addEventListener('paste', () => {
  setTimeout(() => {
    updateHighlight();
    const len = xmlInput.value.length;
    charCount.textContent = len > 0 ? `${len.toLocaleString()} chars` : '';
    scheduleAnalysis(0);
  }, 0);
});

let analyzeTimer: ReturnType<typeof setTimeout> | null = null;
function scheduleAnalysis(delay = 600) {
  if (analyzeTimer) clearTimeout(analyzeTimer);
  analyzeTimer = setTimeout(() => { analyzeTimer = null; runAnalysis(); }, delay);
}

xmlInput.addEventListener('scroll', () => {
  xmlHl.scrollTop  = xmlInput.scrollTop;
  xmlHl.scrollLeft = xmlInput.scrollLeft;
  xmlLineHlWrap.scrollTop  = xmlInput.scrollTop;
  xmlLineHlWrap.scrollLeft = xmlInput.scrollLeft;
});

xmlInput.addEventListener('mousemove', (e: MouseEvent) => {
  if (!currentLineIssueMap.size) return;
  const line = Math.floor((e.offsetY + xmlInput.scrollTop - PAD_TOP) / LINE_H) + 1;
  const issues = currentLineIssueMap.get(line);
  if (!issues) { hideTooltip(); return; }
  // Only show tooltip for active (not filtered-out) severities
  const visible = issues.filter(iss => activeFilters.has(iss.severity));
  if (!visible.length) { hideTooltip(); return; }
  showTooltip(e.clientX, e.clientY, visible);
});

xmlInput.addEventListener('mouseleave', hideTooltip);

// ── Line highlights ───────────────────────────────────────────────────────────
const LINE_H  = 12.5 * 1.65; // matches CSS font/line-height
const PAD_TOP = 16;           // matches editor padding

function applyLineHighlights(lineMap: Map<number, string>) {
  xmlLineHlInner.replaceChildren();
  const totalLines = xmlInput.value.split('\n').length;
  xmlLineHlInner.style.height = `${PAD_TOP + totalLines * LINE_H + PAD_TOP}px`;
  for (const [line, sev] of lineMap) {
    const div = document.createElement('div');
    div.className = `xl-line ${sev}`;
    div.dataset.sev = sev;
    div.style.top = `${PAD_TOP + (line - 1) * LINE_H}px`;
    xmlLineHlInner.appendChild(div);
  }
}

// ── Filter state ──────────────────────────────────────────────────────────────
const activeFilters = new Set(['error', 'warning', 'info']);
filterBar.querySelectorAll<HTMLButtonElement>('.filter-btn').forEach(btn => {
  btn.addEventListener('click', () => {
    const sev = btn.dataset.sev!;
    if (activeFilters.has(sev)) { activeFilters.delete(sev); btn.classList.remove('active'); }
    else                        { activeFilters.add(sev);    btn.classList.add('active'); }
    applyFilters();
  });
});
function applyFilters() {
  issueList.querySelectorAll<HTMLElement>('.issue-card').forEach(el => {
    el.classList.toggle('hidden', !activeFilters.has(el.dataset.sev ?? ''));
  });
  issueList.querySelectorAll<HTMLElement>('.issue-group').forEach(grp => {
    const visible = grp.querySelectorAll('.issue-card:not(.hidden)').length > 0;
    (grp as HTMLElement).style.display = visible ? '' : 'none';
  });
  xmlLineHlInner.querySelectorAll<HTMLElement>('.xl-line').forEach(el => {
    (el as HTMLElement).style.display = activeFilters.has(el.dataset.sev ?? '') ? 'block' : 'none';
  });
}

// ── Analyze ───────────────────────────────────────────────────────────────────
analyzeBtn.addEventListener('click', () => runAnalysis());

async function runAnalysis() {
  const xml = xmlInput.value.trim();
  if (!xml) return;
  analyzeBtn.disabled        = true;
  analyzingInd.style.display = 'inline';
  loadingState.style.display = 'flex';
  emptyState.style.display   = 'none';
  issueList.style.display    = 'none';
  cleanState.style.display   = 'none';
  topbarPills.replaceChildren();
  resultsHdr.style.display   = 'none';
  filterBar.classList.remove('visible');
  currentLineIssueMap = new Map();
  updateHighlight(); // clear squiggles while re-analyzing
  hideTooltip();

  let result: ValidationResult;
  try {
    result = await validateVast(xml);
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : String(e);
    loadingState.style.display = 'none';
    issueList.style.display    = 'block';
    showErrorMessage(issueList, msg);
    analyzeBtn.disabled        = false;
    analyzingInd.style.display = 'none';
    return;
  }
  analyzeBtn.disabled        = false;
  analyzingInd.style.display = 'none';
  loadingState.style.display = 'none';
  renderResults(result);
}

function renderResults(result: ValidationResult) {
  const { errors, warnings, infos } = result.summary;
  const total = errors + warnings + infos;

  topbarPills.replaceChildren(createTopbarPills(result));

  topbarTitle.replaceChildren();
  if (total > 0) {
    topbarTitle.appendChild(document.createTextNode('Found '));
    const strong = createNode('strong');
    strong.textContent = `${total} issue${total !== 1 ? 's' : ''}`;
    topbarTitle.appendChild(strong);
  } else {
    const strong = createNode('strong');
    strong.textContent = 'No issues found';
    topbarTitle.appendChild(strong);
  }

  resultsHdr.style.display  = 'flex';
  resultsHdrLbl.textContent = total > 0 ? `${total} issue${total !== 1 ? 's' : ''}` : 'Results';

  if (total === 0) {
    cleanState.style.display = 'flex'; issueList.style.display = 'none';
    filterBar.classList.remove('visible');
    currentLineIssueMap = new Map();
    updateHighlight();
    applyLineHighlights(new Map());
    return;
  }

  filterBar.classList.add('visible');
  const grouped: Record<string, typeof result.issues> = { error: [], warning: [], info: [] };
  for (const iss of result.issues) (grouped[iss.severity] ?? grouped['info']).push(iss);
  issueList.replaceChildren(...(['error', 'warning', 'info'] as const)
    .filter(sev => grouped[sev].length > 0)
    .map(sev => createIssueGroup(sev, grouped[sev])));

  issueList.style.display  = 'block';
  cleanState.style.display = 'none';

  // Build per-line issue map — highest severity first, drives squiggles + tooltips
  const sevOrder: Record<string, number> = { error: 3, warning: 2, info: 1 };
  currentLineIssueMap = new Map();
  for (const iss of result.issues) {
    if (!iss.line) continue;
    if (!currentLineIssueMap.has(iss.line)) currentLineIssueMap.set(iss.line, []);
    currentLineIssueMap.get(iss.line)!.push(iss);
  }
  for (const arr of currentLineIssueMap.values()) {
    arr.sort((a, b) => (sevOrder[b.severity] ?? 0) - (sevOrder[a.severity] ?? 0));
  }
  updateHighlight(); // re-render with squiggles

  // Background line highlights (top sev per line)
  const bgLineMap = new Map<number, string>();
  for (const [line, arr] of currentLineIssueMap) bgLineMap.set(line, arr[0].severity);
  applyLineHighlights(bgLineMap);
  applyFilters();
}

// ── Load XML from session storage (passed from popup) ─────────────────────────
async function init() {
  validateVast('<VAST/>').catch(() => {}); // pre-warm WASM

  try {
    const stored = await chrome.storage.session.get('paste_xml');
    const xml = stored['paste_xml'] as string | undefined;
    if (xml) {
      xmlInput.value = xml;
      updateHighlight();
      charCount.textContent = `${xml.length.toLocaleString()} chars`;
      await chrome.storage.session.remove('paste_xml');
      runAnalysis();
    }
  } catch { /* standalone open */ }
}

init();
