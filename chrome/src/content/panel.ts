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

export function renderPanel(result: ValidationResult, anchor: Element): void {
  const el = anchor as HTMLElement & { _vlCleanup?: () => void };
  el._vlCleanup?.();

  const isCode = anchor.tagName === 'PRE'
    || anchor.tagName === 'TEXTAREA'
    || anchor === document.documentElement;

  if (isCode) {
    renderOverlay(result, anchor as HTMLElement);
  } else {
    renderFloatingPanel(result, null); // non-code: go straight to floating panel
  }
}

// ─── Shared helpers ───────────────────────────────────────────────────────────

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

function renderOverlay(result: ValidationResult, anchor: HTMLElement) {
  const lineIssues = buildLineIssues(result);

  const host = document.createElement('div');
  host.setAttribute('data-vastlint-overlay', 'true');
  // pointer-events:none on the host; individual children opt in
  host.style.cssText = 'all:initial;position:fixed;top:0;left:0;width:0;height:0;pointer-events:none;';
  (document.body ?? document.documentElement).appendChild(host);

  const shadow = host.attachShadow({ mode: 'open' });

  shadow.innerHTML = `
    <style>
      * { box-sizing:border-box; }

      /* ── Summary / mode bar ─────────────────────────────── */
      #bar {
        position:fixed; pointer-events:all;
        display:flex; align-items:center; gap:6px;
        padding:4px 10px; border-radius:0 0 6px 6px;
        background:#1a1a2e; color:#eee;
        font:600 12px/1 system-ui,sans-serif;
        box-shadow:0 2px 8px rgba(0,0,0,.5);
        white-space:nowrap; z-index:2147483646;
      }
      #bar .logo { font-weight:400; opacity:.35; font-size:10px; margin-left:2px; }
      #bar .sep  { opacity:.3; font-weight:400; }
      #bar .e    { color:#ff6b6b; }
      #bar .w    { color:#ffd166; }
      #bar .i    { color:#74b9ff; }
      #bar .ok   { color:#55efc4; }
      .mode-btn {
        padding:2px 7px; border-radius:3px; font-size:10px; font-weight:700;
        cursor:pointer; border:1px solid rgba(255,255,255,.2); color:#aaa;
        background:transparent; line-height:1.4;
      }
      .mode-btn.active { background:rgba(255,255,255,.15); color:#fff; border-color:rgba(255,255,255,.4); }
      .sev-btn {
        padding:2px 5px; border-radius:3px; font-size:10px; font-weight:700;
        cursor:pointer; border:1px solid rgba(255,255,255,.15); color:#666;
        background:transparent; line-height:1.4; min-width:18px;
        transition: background .12s, color .12s;
      }
      .sev-btn.active { background:rgba(255,255,255,.12); color:#ccc; border-color:rgba(255,255,255,.35); }
      /* When filter is off, hide matching rows/issues */
      #layers.hide-error   .row[data-sev="error"],
      #layers.hide-warning .row[data-sev="warning"],
      #layers.hide-info    .row[data-sev="info"] { display:none !important; }
      #float-body.hide-error   .issue[data-sev="error"],
      #float-body.hide-warning .issue[data-sev="warning"],
      #float-body.hide-info    .issue[data-sev="info"] { display:none !important; }
      .bar-close { opacity:.45; font-size:11px; cursor:pointer; margin-left:2px; }
      .ver-badge {
        font-size:10px; color:#7ec8e3; opacity:.6;
        border:1px solid rgba(126,200,227,.2); border-radius:3px;
        padding:1px 5px; line-height:1.4; font-weight:600;
      }

      /* ── Gutter strip ───────────────────────────────────── */
      .strip { position:fixed; pointer-events:none; width:3px; border-radius:0 2px 2px 0; }

      /* ── Inline badge rows ──────────────────────────────── */
      .row { position:fixed; pointer-events:all; display:flex; align-items:center; }

      .badge {
        display:inline-flex; align-items:center; gap:3px;
        padding:2px 7px; border-radius:3px;
        font:700 10px/1.4 system-ui,sans-serif;
        background:color-mix(in srgb,var(--c) 18%,#1a1a2e);
        color:var(--c); border:1px solid color-mix(in srgb,var(--c) 40%,transparent);
        cursor:default; white-space:nowrap;
        box-shadow:0 1px 4px rgba(0,0,0,.35);
        min-width:128px; max-width:240px; justify-content:flex-start;
        overflow:hidden; text-overflow:ellipsis;
      }
      /* on hover expand badge to show full text — use a non-layout property so nothing shifts */
      .row:hover .badge {
        background:var(--c); color:#fff;
        max-width:none;
        overflow:visible;
        position:relative; z-index:1;
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
  const floatEl    = shadow.getElementById('float')      as HTMLElement;
  const floatHdr   = shadow.getElementById('float-hdr')!;
  const floatClose = shadow.getElementById('float-close')!;
  const floatBody  = shadow.getElementById('float-body') as HTMLElement;
  const tip        = shadow.getElementById('tip')        as HTMLElement;

  // ── Build inline strips + rows ─────────────────────────────────────────────
  const strips: HTMLElement[] = [];
  const rows:   HTMLElement[] = [];

  for (const [ln, issues] of lineIssues) {
    const worstSev   = issues.find(x => x.severity === 'error')?.severity
      ?? issues.find(x => x.severity === 'warning')?.severity
      ?? issues[0]?.severity ?? 'info';
    const worstColor = SEV_COLOR[worstSev];
    const worstIcon  = SEV_ICON[worstSev];
    const n = issues.length;
    const label = n === 1 ? `${worstIcon} ${issues[0].id}` : `${worstIcon} ${n} issues`;

    const strip = document.createElement('div');
    strip.className = 'strip';
    strip.style.background = SEV_COLOR[worstSev];
    strip.dataset.ln = String(ln);
    layers.appendChild(strip);
    strips.push(strip);

    const row = document.createElement('div');
    row.className = 'row';
    row.dataset.ln  = String(ln);
    row.dataset.sev = worstSev;
    row.style.setProperty('--c', worstColor);
    row.innerHTML = `<span class="badge">${escHtml(label)}</span>`;
    layers.appendChild(row);
    rows.push(row);

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

    row.addEventListener('mouseenter', () => {
      tip.innerHTML = tipRows;
      tip.style.display = 'block';

      const rb      = row.getBoundingClientRect();
      const MARGIN  = 8;

      // Vertical: prefer below badge, flip above if near viewport bottom
      const tipH = tip.offsetHeight || 120;
      if (window.innerHeight - rb.bottom < tipH + MARGIN && rb.top > tipH + MARGIN) {
        tip.style.top    = '';
        tip.style.bottom = `${window.innerHeight - rb.top + 4}px`;
      } else {
        tip.style.bottom = '';
        tip.style.top    = `${rb.bottom + 4}px`;
      }

      // Horizontal: anchor to badge left, clamp so it never overflows right edge
      tip.style.right = '';
      const tipW    = tip.offsetWidth || 300;
      const ideal   = rb.left;
      const maxLeft = window.innerWidth - tipW - MARGIN;
      tip.style.left = `${Math.max(MARGIN, Math.min(ideal, maxLeft))}px`;
    });
    row.addEventListener('mouseleave', () => {
      // Delay hide so user can move mouse into the tooltip to click links
      setTimeout(() => {
        if (!tip.matches(':hover')) tip.style.display = 'none';
      }, 120);
    });
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
  document.addEventListener('mouseup', () => { dragging = false; });

  // ── Layout: position bar, strips and rows to track the anchor ─────────────
  function layout() {
    const rect = anchor.getBoundingClientRect();
    if (rect.width === 0) return;

    const cs = window.getComputedStyle(anchor);
    let lh   = parseFloat(cs.lineHeight);
    if (isNaN(lh)) lh = parseFloat(cs.fontSize) * 1.5 || 19.5;
    const pt = parseFloat(cs.paddingTop)  || 0;
    const pl = parseFloat(cs.paddingLeft) || 0;

    bar.style.top  = `${Math.max(0, rect.top - 28)}px`;
    bar.style.left = `${rect.left}px`;

    const BADGE_GAP = 6;
    for (let i = 0; i < strips.length; i++) {
      const strip = strips[i];
      const row   = rows[i];
      const ln    = parseInt(strip.dataset.ln!, 10);
      const lineY = rect.top + pt + (ln - 1) * lh;
      const inView = lineY + lh > rect.top && lineY < rect.bottom;

      strip.style.display = row.style.display = inView && mode === 'inline' ? '' : 'none';
      if (!inView || mode !== 'inline') continue;

      strip.style.left   = `${rect.left + pl - 4}px`;
      strip.style.top    = `${lineY}px`;
      strip.style.height = `${lh}px`;

      row.style.top    = `${lineY}px`;
      row.style.right  = `${window.innerWidth - rect.right - BADGE_GAP}px`;
      row.style.height = `${lh}px`;
    }
  }

  let rafId = 0;
  function tick() { layout(); rafId = requestAnimationFrame(tick); }
  rafId = requestAnimationFrame(tick);

  (anchor as HTMLElement & { _vlCleanup?: () => void })._vlCleanup = () => {
    cancelAnimationFrame(rafId);
    document.removeEventListener('mouseup', () => { dragging = false; });
    host.remove();
  };
}

// ─── Non-code anchor: just show the floating panel directly ──────────────────

function renderFloatingPanel(result: ValidationResult, anchor: Element | null) {
  const host = document.createElement('div');
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
