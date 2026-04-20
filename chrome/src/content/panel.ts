/**
 * VAST Lint overlay renderer.
 *
 * Two annotation modes, switchable via the summary bar:
 *
 *  INLINE  — per-line badges pinned to the right gutter. One badge per line,
 *            fixed-width column so all badges left-align. Hover → tooltip with
 *            all issues for that line. Tooltip always renders on top (z-index
 *            managed via JS mouseenter so it's never occluded by sibling rows).
 *
 *  PANEL   — a draggable floating box listing every issue. Works as a movable
 *            overlay so the user can position it anywhere on screen.
 *
 * The original anchor element is never modified.
 */

import type { ValidationResult, Issue } from '../types/vastlint';

const SEV_COLOR: Record<string, string> = {
  error:   '#e53935',
  warning: '#f4a000',
  info:    '#1e88e5',
};
const SEV_ICON: Record<string, string> = {
  error: '✕', warning: '⚠', info: 'ℹ',
};

type ViewMode = 'inline' | 'panel';

// ─── Public entry point ───────────────────────────────────────────────────────

export function renderPanel(result: ValidationResult, anchor: Element, origToFmt?: Map<number, number>, pathToFmt?: Map<string, number>): void {
  const el = anchor as HTMLElement & { _vlCleanup?: () => void };
  el._vlCleanup?.();

  const isCode = anchor.tagName === 'PRE'
    || anchor.tagName === 'TEXTAREA'
    || anchor === document.documentElement;

  if (isCode) {
    renderOverlay(result, anchor as HTMLElement, origToFmt, pathToFmt);
  } else {
    renderFloatingPanel(result, null);
  }
}

// ─── Shared helpers ───────────────────────────────────────────────────────────

/**
 * Create an element in the HTML namespace.
 * Using document.createElement() on an XML document (e.g. a raw VAST tag
 * opened directly in the browser) produces an XML-namespace element that does
 * NOT support attachShadow(), causing the overlay to silently fail. Using
 * createElementNS with the XHTML namespace works in both HTML and XML docs.
 */
function htmlEl<K extends keyof HTMLElementTagNameMap>(tag: K): HTMLElementTagNameMap[K] {
  return document.createElementNS('http://www.w3.org/1999/xhtml', tag) as HTMLElementTagNameMap[K];
}

function escHtml(s: string): string {
  return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;');
}

function buildLineIssues(result: ValidationResult) {
  const map = new Map<number, Issue[]>();
  for (const iss of result.issues) {
    const ln = iss.line ?? 1;
    if (!map.has(ln)) map.set(ln, []);
    map.get(ln)!.push(iss);
  }
  return map;
}

function summaryHTML(result: ValidationResult) {
  const { errors, warnings, infos } = result.summary;
  return [
    errors   ? `<span class="e">${errors} error${errors !== 1 ? 's' : ''}</span>` : '',
    warnings ? `<span class="w">${warnings} warning${warnings !== 1 ? 's' : ''}</span>` : '',
    infos    ? `<span class="i">${infos} info</span>` : '',
    !errors && !warnings && !infos ? '<span class="ok">✓ Valid</span>' : '',
  ].filter(Boolean).join('<span class="sep"> · </span>');
}

function issueListHTML(issues: Issue[]) {
  return issues.map(iss => {
    const color = SEV_COLOR[iss.severity];
    const icon  = SEV_ICON[iss.severity];
    const meta  = [
      iss.path     ? `<code>${escHtml(iss.path)}</code>` : '',
      iss.line     ? `line ${iss.line}` : '',
      iss.spec_ref ? `<em>${escHtml(iss.spec_ref)}</em>` : '',
    ].filter(Boolean).join(' · ');
    const idLink = `<a class="issue-id-link" href="https://vastlint.org/docs/rules/${encodeURIComponent(iss.id)}" target="_blank" rel="noopener">[${escHtml(iss.id)}]</a>`;
    return `<div class="issue" data-sev="${escHtml(iss.severity)}">
      <span class="issue-icon" style="color:${color}">${icon}</span>
      <div>
        <div>${idLink} <span class="issue-msg">${escHtml(iss.message)}</span></div>
        ${meta ? `<div class="issue-meta">${meta}</div>` : ''}
      </div>
    </div>`;
  }).join('');
}

// ─── Main overlay (code mode) ─────────────────────────────────────────────────

function renderOverlay(result: ValidationResult, anchor: HTMLElement, origToFmt?: Map<number, number>, pathToFmt?: Map<string, number>) {
  const lineIssues = buildLineIssues(result);

  // The anchor may live inside an iframe (e.g. our XML viewer). We must inject
  // the overlay host into the same document so that position:fixed resolves
  // correctly relative to that viewport, not the outer XML document.
  const ownerDoc = (anchor.ownerDocument ?? document) as Document;
  const ownerWin = (ownerDoc.defaultView ?? window) as Window & typeof globalThis;

  const host = ownerDoc.createElementNS('http://www.w3.org/1999/xhtml', 'div') as HTMLDivElement;
  host.setAttribute('data-vastlint-overlay', 'true');
  // pointer-events:none on the host; individual children opt in
  host.style.cssText = 'all:initial;position:fixed;top:0;left:0;width:0;height:0;pointer-events:none;';
  (ownerDoc.body ?? ownerDoc.documentElement).appendChild(host);

  const shadow = host.attachShadow({ mode: 'open' });

  shadow.innerHTML = `
    <style>
      * { box-sizing:border-box; }

      /* ── Summary / mode bar ─────────────────────────────── */
      #bar {
        position:fixed; pointer-events:all;
        display:flex; align-items:center; gap:8px;
        padding:6px 14px; border-radius:0 0 8px 8px;
        background:#0d0d14; color:#eee;
        font:600 13px/1 system-ui,sans-serif;
        box-shadow:0 3px 14px rgba(0,0,0,.7), 0 1px 3px rgba(0,0,0,.5);
        border: 1px solid rgba(255,255,255,.08); border-top:none;
        white-space:nowrap; z-index:2147483646;
      }
      #bar .logo { font-weight:600; opacity:.4; font-size:11px; margin-left:4px; letter-spacing:.03em; }
      #bar .sep  { opacity:.2; font-weight:400; }
      #bar .e    { color:#ff6b6b; font-weight:700; }
      #bar .w    { color:#ffd166; font-weight:700; }
      #bar .i    { color:#74b9ff; font-weight:700; }
      #bar .ok   { color:#55efc4; font-weight:700; }
      .mode-btn {
        padding:4px 10px; border-radius:5px; font-size:12px; font-weight:700;
        cursor:pointer; border:1px solid rgba(255,255,255,.18); color:#999;
        background:rgba(255,255,255,.05); line-height:1.4;
        transition: background .12s, color .12s, border-color .12s;
      }
      .mode-btn:hover { background:rgba(255,255,255,.1); color:#ddd; border-color:rgba(255,255,255,.3); }
      .mode-btn.active { background:rgba(255,255,255,.18); color:#fff; border-color:rgba(255,255,255,.5); }
      .sev-btn {
        padding:4px 8px; border-radius:5px; font-size:12px; font-weight:700;
        cursor:pointer; border:1px solid rgba(255,255,255,.15); color:#666;
        background:rgba(255,255,255,.04); line-height:1.4; min-width:28px; text-align:center;
        transition: background .12s, color .12s, border-color .12s;
      }
      .sev-btn:hover { background:rgba(255,255,255,.1); color:#bbb; border-color:rgba(255,255,255,.3); }
      .sev-btn.active { background:rgba(255,255,255,.14); color:#fff; border-color:rgba(255,255,255,.4); }
      /* When filter is off, hide matching rows/issues */
      #layers.hide-error   .highlight[data-sev="error"],
      #layers.hide-error   .squiggle[data-sev="error"],
      #layers.hide-error   .inline-label[data-sev="error"],
      #layers.hide-warning .highlight[data-sev="warning"],
      #layers.hide-warning .squiggle[data-sev="warning"],
      #layers.hide-warning .inline-label[data-sev="warning"],
      #layers.hide-info    .highlight[data-sev="info"],
      #layers.hide-info    .squiggle[data-sev="info"],
      #layers.hide-info    .inline-label[data-sev="info"] { display:none !important; }
      #float-body.hide-error   .issue[data-sev="error"],
      #float-body.hide-warning .issue[data-sev="warning"],
      #float-body.hide-info    .issue[data-sev="info"] { display:none !important; }
      .bar-close { opacity:.5; font-size:14px; cursor:pointer; margin-left:4px; transition:opacity .12s; }
      .bar-close:hover { opacity:1; }
      #bar-drag {
        font-size:16px; color:#fff; opacity:.35; cursor:grab; user-select:none;
        padding:0 2px; letter-spacing:-1px; transition:opacity .12s;
      }
      #bar-drag:hover { opacity:.8; }
      #bar-drag:active { cursor:grabbing; opacity:1; }
      .ver-badge {
        font-size:11px; color:#7ec8e3; opacity:.8;
        border:1px solid rgba(126,200,227,.3); border-radius:4px;
        padding:2px 7px; line-height:1.4; font-weight:700;
      }

      /* ── Full-line highlight band ────────────────────────── */
      .highlight {
        position:fixed; pointer-events:all; cursor:default;
        background:color-mix(in srgb,var(--c) 28%,transparent);
        border-left:3px solid color-mix(in srgb,var(--c) 90%,transparent);
      }

      /* ── Squiggly underline ──────────────────────────────── */
      .squiggle {
        position:fixed; pointer-events:none; z-index:1;
        height:4px; overflow:visible;
      }

      /* ── Inline label (after line text) ────────────────────── */
      .inline-label {
        position:fixed; pointer-events:none;
        display:flex; align-items:center;
        font:700 13px/1 'JetBrains Mono','Fira Code','Consolas',monospace;
        color:#ffffff;
        white-space:nowrap;
        padding:0 12px 0 10px;
        background: color-mix(in srgb,var(--c) 90%, #000 10%);
        border-left: 3px solid var(--c);
      }

      /* Tooltip — pointer events enabled so rule links are clickable */
      .tooltip {
        position:fixed;
        pointer-events:auto;
        background:#1a1a2e; color:#eee;
        border:1px solid #2e2e4e; border-radius:6px;
        padding:8px 10px;
        font:400 12px/1.5 system-ui,sans-serif;
        /* let it size to content, cap at viewport width */
        width:max-content;
        max-width:min(420px, calc(100vw - 16px));
        box-shadow:0 8px 32px rgba(0,0,0,.7);
        white-space:normal;
        display:none;
        z-index:2147483647;
      }
      .tip-issue { display:flex; gap:6px; padding:4px 0; border-bottom:1px solid #2a2a3e; }
      .tip-issue:last-child { border-bottom:none; padding-bottom:0; }
      .tip-icon     { flex-shrink:0; font-weight:700; margin-top:1px; }
      .tip-id-link  { font-size:10px; opacity:.7; color:#7ec8e3; text-decoration:none; }
      .tip-id-link:hover { text-decoration:underline; opacity:1; }
      .tip-msg   { color:#fff; font-weight:600; font-size:11px; }
      .tip-meta  { color:#666; font-size:10px; margin-top:2px; }
      .tip-meta code { color:#7ec8e3; font-family:monospace; word-break:break-all; }
      .tip-meta em   { font-style:normal; color:#555; }

      /* ── Floating panel ─────────────────────────────────── */
      #float {
        position:fixed; pointer-events:all;
        background:#1a1a2e; color:#eee; border:1px solid #2e2e4e;
        border-radius:8px; box-shadow:0 8px 32px rgba(0,0,0,.6);
        font:400 12px/1.5 system-ui,sans-serif;
        width:360px; max-height:480px;
        display:flex; flex-direction:column;
        z-index:2147483646;
        display:none;
      }
      #float-hdr {
        display:flex; align-items:center; gap:6px;
        padding:7px 10px; background:#0f0f1e; border-radius:8px 8px 0 0;
        cursor:grab; user-select:none; flex-shrink:0;
        border-bottom:1px solid #2e2e4e;
        font-weight:600; font-size:12px;
      }
      #float-hdr:active { cursor:grabbing; }
      #float-hdr .logo  { font-weight:400; opacity:.35; font-size:10px; margin-left:auto; }
      #float-hdr .close { opacity:.45; cursor:pointer; font-size:12px; margin-left:4px; }
      #float-body {
        overflow-y:auto; flex:1; padding:4px 0;
        scrollbar-width:thin; scrollbar-color:#2e2e4e #1a1a2e;
      }
      .issue { display:flex; gap:8px; padding:6px 10px; border-bottom:1px solid #1e1e30; font-size:11px; line-height:1.4; }
      .issue:last-child { border-bottom:none; }
      .issue-icon { flex-shrink:0; font-weight:700; margin-top:1px; }
      .issue-id-link { font-size:10px; color:#7ec8e3; opacity:.8; text-decoration:none; }
      .issue-id-link:hover { text-decoration:underline; opacity:1; }
      .issue-msg  { color:#fff; font-weight:600; }
      .issue-meta { color:#666; font-size:10px; margin-top:2px; }
      .issue-meta code { color:#7ec8e3; font-family:monospace; }
      .issue-meta em   { font-style:normal; color:#555; }
    </style>

    <div id="bar">
      <span id="bar-drag" title="Drag to move">⠿</span>
      <span class="sep">·</span>
      ${summaryHTML(result)}
      ${result.version ? `<span class="ver-badge">VAST ${escHtml(result.version)}</span>` : ''}
      <span class="sep">·</span>
      <button class="sev-btn active" id="flt-e" data-sev="error"   title="Toggle errors">E</button>
      <button class="sev-btn active" id="flt-w" data-sev="warning" title="Toggle warnings">W</button>
      <button class="sev-btn active" id="flt-i" data-sev="info"    title="Toggle infos">I</button>
      <span class="sep">·</span>
      <button class="mode-btn active" id="btn-inline" title="Inline line annotations">inline</button>
      <button class="mode-btn"        id="btn-panel"  title="Floating panel">panel</button>
      <span class="logo">vastlint</span>
      <span class="bar-close" id="bar-close" title="Hide">✕</span>
    </div>

    <div id="layers"></div>

    <div id="float">
      <div id="float-hdr">
        ${summaryHTML(result)}
        ${result.version ? `<span style="opacity:.5;font-size:10px;font-weight:400">VAST ${result.version}</span>` : ''}
        <span class="logo">vastlint</span>
        <span class="close" id="float-close">✕</span>
      </div>
      <div id="float-body">
        ${result.issues.length === 0
          ? '<div style="padding:10px;color:#55efc4">✓ No issues found</div>'
          : issueListHTML(result.issues)}
      </div>
    </div>

    <div class="tooltip" id="tip"></div>
  `;

  const bar        = shadow.getElementById('bar')!;
  const layers     = shadow.getElementById('layers')!;
  const btnInline  = shadow.getElementById('btn-inline') as HTMLButtonElement;
  const btnPanel   = shadow.getElementById('btn-panel')  as HTMLButtonElement;
  const barClose   = shadow.getElementById('bar-close')!;
  const barDrag    = shadow.getElementById('bar-drag')!;
  const floatEl    = shadow.getElementById('float')      as HTMLElement;
  const floatHdr   = shadow.getElementById('float-hdr')!;
  const floatClose = shadow.getElementById('float-close')!;
  const floatBody  = shadow.getElementById('float-body') as HTMLElement;
  const tip        = shadow.getElementById('tip')        as HTMLElement;

  // ── Build inline annotations ───────────────────────────────────────────────
  const inlineLabels: HTMLElement[] = [];
  const highlights:   HTMLElement[] = [];
  const squiggles:    HTMLElement[] = [];

  for (const [ln, issues] of lineIssues) {
    const worstSev   = issues.find(x => x.severity === 'error')?.severity
      ?? issues.find(x => x.severity === 'warning')?.severity
      ?? issues[0]?.severity ?? 'info';
    const worstColor = SEV_COLOR[worstSev];
    const worstIcon  = SEV_ICON[worstSev];
    const n = issues.length;
    const label = n === 1 ? `${worstIcon} ${issues[0].id}` : `${worstIcon} ${n} issues`;

    // Store the first available XPath path for XML-doc positioning
    const issuePath = issues.find(x => x.path)?.path ?? '';

    const highlight = htmlEl('div');
    highlight.className = 'highlight';
    highlight.style.setProperty('--c', worstColor);
    highlight.dataset.ln    = String(ln);
    highlight.dataset.sev   = worstSev;
    highlight.dataset.xpath = issuePath;
    layers.appendChild(highlight);
    highlights.push(highlight);

    const squiggle = htmlEl('div');
    squiggle.className = 'squiggle';
    squiggle.style.setProperty('--c', worstColor);
    squiggle.dataset.ln    = String(ln);
    squiggle.dataset.sev   = worstSev;
    squiggle.dataset.xpath = issuePath;
    squiggle.dataset.color = worstColor;
    layers.appendChild(squiggle);
    squiggles.push(squiggle);

    const inlineLabel = htmlEl('div');
    inlineLabel.className = 'inline-label';
    inlineLabel.dataset.ln  = String(ln);
    inlineLabel.dataset.sev = worstSev;
    inlineLabel.style.setProperty('--c', worstColor);
    inlineLabel.textContent = label;
    layers.appendChild(inlineLabel);
    inlineLabels.push(inlineLabel);

    // Tooltip: show on mouseenter, positioned below the badge, always on top
    const tipRows = issues.map(iss => {
      const color = SEV_COLOR[iss.severity];
      const icon  = SEV_ICON[iss.severity];
      const meta  = [
        iss.path     ? `<code>${escHtml(iss.path)}</code>` : '',
        iss.line     ? `line ${iss.line}` : '',
        iss.spec_ref ? `<em>${escHtml(iss.spec_ref)}</em>` : '',
      ].filter(Boolean).join(' · ');
      const idLink = `<a class="tip-id-link" href="https://vastlint.org/docs/rules/${encodeURIComponent(iss.id)}" target="_blank" rel="noopener">[${escHtml(iss.id)}]</a>`;
      return `<div class="tip-issue">
        <span class="tip-icon" style="color:${color}">${icon}</span>
        <div>
          <div>${idLink} <span class="tip-msg">${escHtml(iss.message)}</span></div>
          ${meta ? `<div class="tip-meta">${meta}</div>` : ''}
        </div>
      </div>`;
    }).join('');

    function showTip(anchorRect: DOMRect) {
      tip.innerHTML = tipRows;
      tip.style.display = 'block';

      const MARGIN = 8;
      const tipH   = tip.offsetHeight || 120;
      // Prefer below the line, flip above if near viewport bottom
      if (ownerWin.innerHeight - anchorRect.bottom < tipH + MARGIN && anchorRect.top > tipH + MARGIN) {
        tip.style.top    = '';
        tip.style.bottom = `${ownerWin.innerHeight - anchorRect.top + 4}px`;
      } else {
        tip.style.bottom = '';
        tip.style.top    = `${anchorRect.bottom + 4}px`;
      }
      // Horizontal: anchor to left of line/badge, clamp inside viewport
      tip.style.right = '';
      const tipW    = tip.offsetWidth || 300;
      const ideal   = anchorRect.left;
      const maxLeft = ownerWin.innerWidth - tipW - MARGIN;
      tip.style.left = `${Math.max(MARGIN, Math.min(ideal, maxLeft))}px`;
    }
    function hideTip() {
      setTimeout(() => {
        if (!tip.matches(':hover')) tip.style.display = 'none';
      }, 120);
    }

    // Hover on the highlight band (whole line) — VS Code-style
    highlight.addEventListener('mouseenter', () => showTip(highlight.getBoundingClientRect()));
    highlight.addEventListener('mouseleave', hideTip);

    tip.addEventListener('mouseleave', () => { tip.style.display = 'none'; });
  }

  // ── Mode switching ─────────────────────────────────────────────────────────
  let mode: ViewMode = 'inline';

  function setMode(m: ViewMode) {
    mode = m;
    btnInline.classList.toggle('active', m === 'inline');
    btnPanel.classList.toggle('active',  m === 'panel');
    layers.style.display   = m === 'inline' ? '' : 'none';
    tip.style.display      = 'none';
    floatEl.style.display  = m === 'panel'  ? 'flex' : 'none';
    // Position float near the top-right of the anchor on first open
    if (m === 'panel' && !floatEl.dataset.positioned) {
      const rect = anchor.getBoundingClientRect();
      floatEl.style.top  = `${Math.max(8, rect.top)}px`;
      floatEl.style.left = `${Math.min(rect.right + 12, window.innerWidth - 376)}px`;
      floatEl.dataset.positioned = '1';
    }
  }

  btnInline.addEventListener('click', () => setMode('inline'));
  btnPanel.addEventListener('click',  () => setMode('panel'));

  // ── Severity filter toggles ────────────────────────────────────────────────
  (['error', 'warning', 'info'] as const).forEach(sev => {
    const btn = shadow.getElementById(`flt-${sev[0]}`) as HTMLButtonElement | null;
    if (!btn) return;
    btn.addEventListener('click', () => {
      const hide = `hide-${sev}`;
      const active = btn.classList.toggle('active');
      if (active) {
        layers.classList.remove(hide);
        floatBody?.classList.remove(hide);
      } else {
        layers.classList.add(hide);
        floatBody?.classList.add(hide);
      }
    });
  });

  // ── Bar hide ───────────────────────────────────────────────────────────────
  barClose.addEventListener('click', () => {
    layers.style.display = 'none';
    floatEl.style.display = 'none';
    tip.style.display = 'none';
    bar.style.display = 'none';
  });

  // ── Float close ────────────────────────────────────────────────────────────
  floatClose.addEventListener('click', () => setMode('inline'));

  // ── Drag the bar ───────────────────────────────────────────────────────────
  let barDragging = false, barOffX = 0, barOffY = 0, barPinned = false;

  barDrag.addEventListener('mousedown', e => {
    barDragging = true;
    barPinned   = true;
    const r = bar.getBoundingClientRect();
    barOffX = e.clientX - r.left;
    barOffY = e.clientY - r.top;
    e.preventDefault();
  });
  shadow.addEventListener('mousemove', e => {
    if (!barDragging) return;
    const me = e as MouseEvent;
    bar.style.left = `${me.clientX - barOffX}px`;
    bar.style.top  = `${me.clientY - barOffY}px`;
  });
  shadow.addEventListener('mouseup', () => { barDragging = false; });
  ownerDoc.addEventListener('mouseup', () => { barDragging = false; });

  // ── Drag the floating panel ────────────────────────────────────────────────
  let dragOffX = 0, dragOffY = 0, dragging = false;

  floatHdr.addEventListener('mousedown', e => {
    dragging = true;
    const r = floatEl.getBoundingClientRect();
    dragOffX = e.clientX - r.left;
    dragOffY = e.clientY - r.top;
    e.preventDefault();
  });
  // Listen on shadow root so mousemove/up work even when pointer leaves the header
  shadow.addEventListener('mousemove', e => {
    if (!dragging) return;
    const me = e as MouseEvent;
    floatEl.style.left = `${me.clientX - dragOffX}px`;
    floatEl.style.top  = `${me.clientY - dragOffY}px`;
    floatEl.style.right = '';
  });
  shadow.addEventListener('mouseup', () => { dragging = false; });
  // Also handle when pointer leaves the shadow root entirely
  ownerDoc.addEventListener('mouseup', () => { dragging = false; });

  // ── Layout: position bar, strips and rows to track the anchor ─────────────

  /**
   * Convert a vastlint path (/VAST/Ad[0]/Wrapper) to a standard XPath
   * expression (/VAST/Ad[1]/Wrapper). vastlint uses 0-based indices; XPath
   * uses 1-based.  Also strips any attribute selector at the end ([@delivery]).
   */
  function vastPathToXPath(path: string): string {
    return path
      .replace(/\[@[^\]]*\]/g, '')               // drop [@attr] segments
      .replace(/\[(\d+)\]/g, (_, n) => `[${parseInt(n, 10) + 1}]`);
  }

  /**
   * On a full-page XML document Chrome exposes the actual XML DOM, so we can
   * use document.evaluate() to find the exact node for each issue path and
   * read its real bounding rect — far more accurate than line-number pixel math.
   */
  const isXmlDoc = document.contentType === 'text/xml' || document.contentType === 'application/xml';

  // Cache element lookups within a layout pass (same path → same node)
  const xpathCache = new Map<string, Element | null>();
  function elementForPath(path: string): Element | null {
    if (!path) return null;
    if (xpathCache.has(path)) return xpathCache.get(path)!;
    try {
      const xp = vastPathToXPath(path);
      const res = document.evaluate(xp, document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null);
      const el = res.singleNodeValue as Element | null;
      xpathCache.set(path, el);
      return el;
    } catch {
      xpathCache.set(path, null);
      return null;
    }
  }

  function layout() {
    // Invalidate cache each layout tick so we pick up any DOM changes
    xpathCache.clear();

    const rect = anchor.getBoundingClientRect();

    // On a pure XML document, document.documentElement has no CSS layout box —
    // its width/height may be 0. Fall back to the viewport as the content area.
    const contentRect = (rect.width === 0 || rect.height === 0)
      ? new DOMRect(0, 0, ownerWin.innerWidth, ownerWin.innerHeight)
      : rect;

    const cs = ownerWin.getComputedStyle(anchor);
    let defaultLh = parseFloat(cs.lineHeight);
    if (isNaN(defaultLh)) defaultLh = parseFloat(cs.fontSize) * 1.5 || 18;
    const pt = parseFloat(cs.paddingTop)  || 0;
    const pl = parseFloat(cs.paddingLeft) || 0;

    if (!barPinned) {
      bar.style.top  = `${Math.max(0, contentRect.top - 28)}px`;
      bar.style.left = `${contentRect.left + 16}px`;
    }

    for (let i = 0; i < highlights.length; i++) {
      const inlineLabel = inlineLabels[i];
      const highlight   = highlights[i];
      const squiggle    = squiggles[i];

      // ── Determine lineY and lh ───────────────────────────────────────────
      let lineY: number;
      let lh = defaultLh;

      const ln = parseInt(highlight.dataset.ln!, 10);
      const issuePath = highlight.dataset.xpath ?? '';

      // ── Resolve which formatted line span to highlight ───────────────────
      // Priority 1: exact path match via pathToFmt.
      //   vastlint omits [0] for elements that happen to be unique children.
      //   Normalize by adding [0] to every segment that lacks an index.
      //   Our formatter always emits [n], so /Wrapper becomes /Wrapper[0] etc.
      // Priority 2: data-orig exact match (works when source XML is not minified).
      // Priority 3: origToFmt fuzzy ±5.
      // Priority 4: treat ln as 1-based index directly.

      function normalisePath(p: string): string {
        return p.replace(/\/([A-Za-z][\w:.-]*)(?!\[)/g, '/$1[0]');
      }

      let lineSpan: HTMLElement | null = null;

      if (issuePath && pathToFmt) {
        const norm = normalisePath(issuePath);
        const fmtIdx = pathToFmt.get(norm);
        if (fmtIdx !== undefined) {
          lineSpan = anchor.querySelector(`.ln[data-ln="${fmtIdx}"]`) as HTMLElement | null;
        }
        // Fallback: also try the DOM attribute (handles path → span directly)
        if (!lineSpan) {
          lineSpan = anchor.querySelector(`.ln[data-path="${norm.replace(/"/g, '\\"')}"]`) as HTMLElement | null;
        }
      }

      if (!lineSpan) {
        lineSpan = anchor.querySelector(`.ln[data-orig="${ln}"]`) as HTMLElement | null;
      }

      // For XML viewer: if we have origToFmt but NO path resolved the line,
      // only use the raw line number if the source is NOT minified.
      // "Minified" = origToFmt has very few distinct keys relative to total lines
      // (i.e., nearly everything is on line 1). Heuristic: if origToFmt exists but
      // the max raw line across all entries is <= 5, the source is minified and
      // raw line numbers are useless — pin the issue to the document root instead.
      const isMiniXml = origToFmt && (() => {
        let maxLine = 0;
        for (const k of origToFmt.keys()) if (k > maxLine) maxLine = k;
        return maxLine <= 5;
      })();

      if (!lineSpan && origToFmt && !isMiniXml) {
        let fmtIdx = origToFmt.get(ln);
        if (fmtIdx === undefined) {
          for (let d = 1; d <= 5 && fmtIdx === undefined; d++) {
            fmtIdx = origToFmt.get(ln - d) ?? origToFmt.get(ln + d);
          }
        }
        if (fmtIdx !== undefined) {
          lineSpan = anchor.querySelector(`.ln[data-ln="${fmtIdx}"]`) as HTMLElement | null;
        }
      }

      // Last resort for non-XML pages: treat ln as 1-based line index directly
      if (!lineSpan && !origToFmt) {
        lineSpan = anchor.querySelector(`.ln[data-ln="${ln - 1}"]`) as HTMLElement | null;
      }

      // If still no span (minified XML, no path), pin to the document root line (ln=0)
      if (!lineSpan && origToFmt) {
        lineSpan = anchor.querySelector('.ln[data-ln="0"]') as HTMLElement | null;
      }

      if (lineSpan) {
        const sr = lineSpan.getBoundingClientRect();
        if (sr.height > 0) {
          lineY = sr.top;
          // Extend height to cover attr-continuation lines (spans with no data-orig)
          let bottomY = sr.bottom;
          let next = parseInt(lineSpan.dataset.ln!, 10) + 1;
          while (true) {
            const ns = anchor.querySelector(`.ln[data-ln="${next}"]`) as HTMLElement | null;
            if (!ns || ns.dataset.orig) break;
            const nr = ns.getBoundingClientRect();
            if (nr.height === 0) break;
            bottomY = nr.bottom;
            next++;
          }
          lh = bottomY - sr.top;
        } else {
          lineY = contentRect.top + pt + (ln - 1) * defaultLh;
        }
      } else if (isXmlDoc && issuePath) {
        const el = elementForPath(issuePath);
        const er = el?.getBoundingClientRect();
        if (er && er.height > 2) {
          lineY = er.top;
          lh    = er.height;
        } else {
          lineY = contentRect.top + pt + (ln - 1) * defaultLh;
        }
      } else {
        lineY = contentRect.top + pt + (ln - 1) * lh;
      }

      const inView = lineY + lh > 0 && lineY < ownerWin.innerHeight;
      const show   = inView && mode === 'inline';
      inlineLabel.style.display = highlight.style.display = squiggle.style.display = show ? '' : 'none';
      if (!show) continue;

      // Full-line highlight band
      highlight.style.left   = `${contentRect.left}px`;
      highlight.style.top    = `${lineY}px`;
      highlight.style.width  = `${contentRect.width}px`;
      highlight.style.height = `${lh}px`;

      // Squiggly underline — from text left to text right
      const innerSpan = lineSpan?.querySelector('span');
      const innerRect = innerSpan?.getBoundingClientRect();
      const textLeft  = innerRect ? innerRect.left  : contentRect.left + pl + 8;
      const textRight = innerRect ? innerRect.right : contentRect.left + pl + 200;

      // Squiggly underline disabled
      squiggle.style.backgroundImage  = '';
      squiggle.style.backgroundRepeat = '';
      squiggle.style.backgroundSize   = '';
      squiggle.style.left  = `${textLeft}px`;
      squiggle.style.top   = `${lineY + lh - 6}px`;
      squiggle.style.width = `${Math.max(0, textRight - textLeft)}px`;

      // Inline label — pinned to the right edge of the content area
      inlineLabel.style.right  = `${Math.max(8, ownerWin.innerWidth - (contentRect.left + contentRect.width))}px`;
      inlineLabel.style.left   = '';
      inlineLabel.style.top    = `${lineY}px`;
      inlineLabel.style.height = `${lh}px`;
    }
  }

  let rafId = 0;
  function tick() { layout(); rafId = requestAnimationFrame(tick); }
  rafId = requestAnimationFrame(tick);

  (anchor as HTMLElement & { _vlCleanup?: () => void })._vlCleanup = () => {
    cancelAnimationFrame(rafId);
    ownerDoc.removeEventListener('mouseup', () => { dragging = false; });
    host.remove();
  };
}

// ─── Non-code anchor: just show the floating panel directly ──────────────────

function renderFloatingPanel(result: ValidationResult, anchor: Element | null) {
  const host = htmlEl('div');
  host.setAttribute('data-vastlint-panel', 'true');
  host.style.cssText = 'all:initial;position:fixed;top:0;left:0;width:0;height:0;pointer-events:none;';
  (document.body ?? document.documentElement).appendChild(host);

  const shadow = host.attachShadow({ mode: 'open' });
  const { errors, warnings } = result.summary;
  const badgeColor = errors > 0 ? SEV_COLOR.error : warnings > 0 ? SEV_COLOR.warning : '#43a047';

  shadow.innerHTML = `
    <style>
      * { box-sizing:border-box; }
      #float {
        position:fixed; pointer-events:all;
        background:#1a1a2e; color:#eee; border:1px solid #2e2e4e;
        border-radius:8px; box-shadow:0 8px 32px rgba(0,0,0,.6);
        font:400 12px/1.5 system-ui,sans-serif;
        width:360px; max-height:480px; top:60px; right:16px;
        display:flex; flex-direction:column; z-index:2147483647;
      }
      #hdr {
        display:flex; align-items:center; gap:6px;
        padding:7px 10px; background:#0f0f1e; border-radius:8px 8px 0 0;
        cursor:grab; user-select:none; flex-shrink:0;
        border-bottom:1px solid #2e2e4e; font-weight:600; font-size:12px;
      }
      #hdr:active { cursor:grabbing; }
      #hdr .logo  { font-weight:400; opacity:.35; font-size:10px; margin-left:auto; }
      #body { overflow-y:auto; flex:1; padding:4px 0; scrollbar-width:thin; scrollbar-color:#2e2e4e #1a1a2e; }
      .issue { display:flex; gap:8px; padding:6px 10px; border-bottom:1px solid #1e1e30; font-size:11px; line-height:1.4; }
      .issue:last-child { border-bottom:none; }
      .issue-icon { flex-shrink:0; font-weight:700; margin-top:1px; }
      .issue-id-link { font-size:10px; color:#7ec8e3; opacity:.8; text-decoration:none; }
      .issue-id-link:hover { text-decoration:underline; opacity:1; }
      .issue-msg  { color:#fff; font-weight:600; }
      .issue-meta { color:#666; font-size:10px; margin-top:2px; }
      .issue-meta code { color:#7ec8e3; font-family:monospace; }
      .issue-meta em   { font-style:normal; color:#555; }
    </style>
    <div id="float">
      <div id="hdr">
        ${summaryHTML(result)}
        ${result.version ? `<span style="opacity:.5;font-size:10px;font-weight:400">VAST ${result.version}</span>` : ''}
        <span class="logo">vastlint</span>
      </div>
      <div id="body">
        ${result.issues.length === 0
          ? '<div style="padding:10px;color:#55efc4">✓ No issues found</div>'
          : issueListHTML(result.issues)}
      </div>
    </div>`;

  const floatEl = shadow.getElementById('float')   as HTMLElement;
  const hdr     = shadow.getElementById('hdr')!;
  let dragOffX = 0, dragOffY = 0, dragging = false;
  hdr.addEventListener('mousedown', e => {
    dragging = true;
    const r = floatEl.getBoundingClientRect();
    dragOffX = e.clientX - r.left; dragOffY = e.clientY - r.top;
    e.preventDefault();
  });
  shadow.addEventListener('mousemove', e => {
    if (!dragging) return;
    const me = e as MouseEvent;
    floatEl.style.left = `${me.clientX - dragOffX}px`;
    floatEl.style.top  = `${me.clientY - dragOffY}px`;
    floatEl.style.right = '';
  });
  const stopDrag = () => { dragging = false; };
  shadow.addEventListener('mouseup', stopDrag);
  document.addEventListener('mouseup', stopDrag);

  if (anchor) {
    (anchor as HTMLElement & { _vlCleanup?: () => void })._vlCleanup = () => {
      document.removeEventListener('mouseup', stopDrag);
      host.remove();
    };
  }
}
