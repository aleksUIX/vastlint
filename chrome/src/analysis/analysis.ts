/**
 * Full-page VAST analysis tab.
 * Receives XML via chrome.storage.session key 'paste_xml' (set by popup)
 * or lets the user paste directly into the editor.
 */

import { validateVast } from '../vastlint/validator';
import type { ValidationResult, Issue } from '../types/vastlint';

function esc(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

// ── XML Syntax Highlighter ───────────────────────────────────────────────────
function countNL(s: string): number {
  let n = 0; for (let i = 0; i < s.length; i++) if (s[i] === '\n') n++; return n;
}

function highlightXml(raw: string, lineIssues?: Map<number, Issue[]>): string {
  let out = '';
  let i = 0;
  let curLine = 1;
  const squiggled = new Set<number>();

  function sq(html: string, startLine: number, chunk: string): string {
    if (!lineIssues || squiggled.has(startLine) || chunk.includes('\n')) return html;
    const issues = lineIssues.get(startLine);
    if (!issues) return html;
    squiggled.add(startLine);
    const sev = issues[0].severity;
    return `<span class="sq sq-${sev}">${html}</span>`;
  }

  while (i < raw.length) {
    if (raw.startsWith('<!--', i)) {
      const end = raw.indexOf('-->', i + 4);
      const chunk = end === -1 ? raw.slice(i) : raw.slice(i, end + 3);
      const sl = curLine; curLine += countNL(chunk);
      out += sq(`<span class="xt-comment">${esc(chunk)}</span>`, sl, chunk);
      i += chunk.length; continue;
    }
    if (raw.startsWith('<![CDATA[', i)) {
      const end = raw.indexOf(']]>', i + 9);
      const chunk = end === -1 ? raw.slice(i) : raw.slice(i, end + 3);
      const sl = curLine; curLine += countNL(chunk);
      out += sq(`<span class="xt-cdata">${esc(chunk)}</span>`, sl, chunk);
      i += chunk.length; continue;
    }
    if (raw.startsWith('<?', i)) {
      const end = raw.indexOf('?>', i + 2);
      const chunk = end === -1 ? raw.slice(i) : raw.slice(i, end + 2);
      const sl = curLine; curLine += countNL(chunk);
      out += sq(`<span class="xt-pi">${esc(chunk)}</span>`, sl, chunk);
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
      out += sq(colorizeTag(chunk), sl, chunk);
      i = j; continue;
    }
    if (raw[i] === '&') {
      const end = raw.indexOf(';', i);
      if (end !== -1 && end - i < 16) {
        out += `<span class="xt-entity">${esc(raw.slice(i, end + 1))}</span>`;
        i = end + 1; continue;
      }
    }
    let j = i + 1;
    while (j < raw.length && raw[j] !== '<' && raw[j] !== '&') j++;
    const text = raw.slice(i, j);
    curLine += countNL(text);
    out += esc(text);
    i = j;
  }
  return out;
}

function p(s: string) { return s ? `<span class="xt-punct">${esc(s)}</span>` : ''; }
function t(s: string) { return s ? `<span class="xt-tag">${esc(s)}</span>` : ''; }

function colorizeAttrs(chunk: string): string {
  return chunk.replace(
    /([\w:\-\.]+)(\s*=\s*)(["'])((?:[^"'\\]|\\.)*)(\3)|([\w:\-\.]+)|(\s+)/g,
    (_, aName, eq, q, val, _q2, bare, ws) => {
      if (ws)   return esc(ws);
      if (bare) return `<span class="xt-attr">${esc(bare)}</span>`;
      if (aName) return `<span class="xt-attr">${esc(aName)}</span>`
               + p(eq ?? '')
               + p(q)
               + `<span class="xt-val">${esc(val ?? '')}</span>`
               + p(q);
      return '';
    }
  );
}

function colorizeTag(tag: string): string {
  const closeM = tag.match(/^(<\/)([\w:\-\.]+)(>)$/);
  if (closeM) return p(closeM[1]) + t(closeM[2]) + p(closeM[3]);
  let i = 1; let out = p('<');
  if (tag[i] === '/') { out += p('/'); i++; }
  const nameStart = i;
  while (i < tag.length && !/[ \/>\n\t]/.test(tag[i])) i++;
  out += t(tag.slice(nameStart, i));
  const tail = tag.slice(i);
  const isSelfClose = tail.trimEnd().endsWith('/>');
  const inner = tail.slice(0, tail.lastIndexOf(isSelfClose ? '/>' : '>'));
  out += colorizeAttrs(inner);
  out += p(isSelfClose ? '/>' : '>');
  return out;
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
  xmlTooltip.innerHTML = issues.map(iss => `
    <div class="tip-row">
      <div class="tip-meta">
        <a class="tip-id" href="https://vastlint.org/docs/rules/${encodeURIComponent(iss.id)}"
           target="_blank" rel="noopener">${esc(iss.id)}</a>
        ${iss.line ? `<span class="tip-line">line ${iss.line}</span>` : ''}
      </div>
      <div class="tip-msg">${esc(iss.message)}</div>
    </div>`).join('');
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
  xmlHlCode.innerHTML = highlightXml(xmlInput.value, currentLineIssueMap.size ? currentLineIssueMap : undefined) + '\n';
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
  xmlLineHlInner.innerHTML = '';
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
  topbarPills.innerHTML      = '';
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
    issueList.innerHTML = `<div style="padding:20px;color:#ef5350;font-size:13px;"><strong>⚠ Validation error</strong><br><br>${esc(msg)}</div>`;
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

  topbarPills.innerHTML = [
    errors   > 0 ? `<span class="tpill tpill-err">${errors}E</span>`    : '',
    warnings > 0 ? `<span class="tpill tpill-warn">${warnings}W</span>` : '',
    infos    > 0 ? `<span class="tpill tpill-info">${infos}I</span>`     : '',
    total === 0  ? `<span class="tpill tpill-ok">✓ Clean</span>`        : '',
    result.version ? `<span class="ver-pill">VAST ${esc(result.version)}</span>` : '',
  ].filter(Boolean).join('');

  topbarTitle.innerHTML = total > 0
    ? `Found <strong>${total} issue${total !== 1 ? 's' : ''}</strong>`
    : '<strong>No issues found</strong>';

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
  const groupCount: Record<string, string> = {
    error: 'error', warning: 'warning', info: 'advisory',
  };

  issueList.innerHTML = (['error', 'warning', 'info'] as const)
    .filter(sev => grouped[sev].length > 0)
    .map(sev => {
      const n = grouped[sev].length;
      const items = grouped[sev].map(iss => {
        const pathShort = iss.path ? iss.path.split('/').slice(-2).join('/') : null;
        const meta = [
          `<a class="iss-id" href="https://vastlint.org/docs/rules/${encodeURIComponent(iss.id)}" target="_blank" rel="noopener">${esc(iss.id)}</a>`,
          pathShort ? `<span class="iss-path" title="${esc(iss.path ?? '')}">${esc(pathShort)}</span>` : '',
          iss.line  ? `<span class="iss-line">line ${iss.line}</span>` : '',
        ].filter(Boolean).join('');
        return `<div class="issue-card sev-${iss.severity}" data-sev="${iss.severity}">
          <div class="iss-meta">${meta}</div>
          <div class="iss-msg">${esc(iss.message)}</div>
          ${iss.spec_ref ? `<div class="iss-spec">${esc(iss.spec_ref)}</div>` : ''}
        </div>`;
      }).join('');
      return `<div class="issue-group">
        <div class="issue-group-hdr">
          <span class="sev-badge sev-badge-${sev}">${sev.toUpperCase()}</span>
          <span class="group-count">${n} ${groupCount[sev]}${n !== 1 ? 's' : ''}</span>
        </div>
        <div class="issue-group-items">${items}</div>
      </div>`;
    }).join('');

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
