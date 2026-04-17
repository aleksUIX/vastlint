/**
 * vastlint-extension — content script entry point
 *
 * Strategy:
 *  1. Scan the DOM for candidate VAST XML blobs:
 *     - Text nodes / <pre> / <textarea> whose content looks like VAST XML
 *     - Inline <script type="text/xml"> or similar
 *     - <video> ad tag attributes (data-vast, data-ad-tag-uri, etc.)
 *     - Text responses already rendered as plain text (Content-Type: text/xml pages)
 *  2. De-duplicate by XML hash so we don't lint the same payload twice.
 *  3. Run vastlint WASM validate() on each candidate.
 *  4. Inject a collapsible overlay panel adjacent to the element.
 *  5. Set the extension badge text to the total error count via messaging.
 */

import { validateVast } from '../vastlint/validator';
import { renderPanel } from './panel';
import { VAST_SIGNATURE_RE } from '../vastlint/detect';
import type { ValidationResult } from '../types/vastlint';

// ─── State — declared first so the IIFE can use it ───────────────────────────

const MAX_VASTS = 10;

/** Content-hash dedup: hashes of XML blobs that are currently tracked or already dismissed. */
const seen = new Set<string>();

/**
 * Primary results store, keyed by anchor Element.
 * Re-keying by element (not hash) is what lets us handle streaming/char-by-char
 * updates: we can detect "same element, new content" and refresh in place.
 */
const lastResults = new Map<Element, { hash: string; xml: string; result: ValidationResult; label: string }>();

/** Stable label counter — never resets mid-session so labels don't collide after evictions. */
let vastCounter = 0;

/** Last-seen URL — used to detect SPA navigations inside scanDOM. */
let currentUrl = location.href;

/** Set to true when the user disables vastlint on this host. Checked before every scan. */
let siteDisabled = false;

// ─── Initialise WASM then scan ────────────────────────────────────────────────

(async () => {
  try {
    // Check disabled-hosts list before doing anything
    const host = location.hostname;
    const stored = await chrome.storage.sync.get('disabledHosts');
    const disabled: string[] = stored.disabledHosts ?? [];
    if (disabled.includes(host)) {
      siteDisabled = true;
      return; // don't scan, don't observe
    }

    await import('../vastlint/validator'); // trigger WASM init

    // Special path: full-page XML document (e.g. file://….xml or a text/xml response).
    // Re-fetch the raw source so we preserve original formatting and avoid
    // XMLSerializer injecting xmlns/namespace noise.
    if (document.contentType === 'text/xml' || document.contentType === 'application/xml') {
      let src: string;
      try {
        const resp = await fetch(location.href);
        src = await resp.text();
      } catch {
        src = new XMLSerializer().serializeToString(document);
      }
      if (VAST_SIGNATURE_RE.test(src)) {
        const anchor = document.documentElement;
        const key = simpleHash(src);
        if (!seen.has(key) && !lastResults.has(anchor)) {
          seen.add(key);
          lintAndRenderAll([{ xml: src, anchor }]);
        }
      }
      window.addEventListener('pagehide', clearAll);
      return; // no further DOM scanning needed on a pure XML page
    }

    scanDOM();
    observeMutations();
    hookNavigation();
  } catch (e) {
    console.error('[vastlint] failed to initialise', e);
  }
})();

// ─── DOM scanning ─────────────────────────────────────────────────────────────

function scanDOM() {
  if (siteDisabled) return;

  // ── SPA navigation detection ────────────────────────────────────────────────
  // content scripts live in an isolated world, so patching history.pushState
  // has no effect on the page's main world. Instead we check the URL on every
  // MutationObserver tick (which fires whenever the SPA updates the DOM after
  // a route change) — the cheapest reliable approach.
  if (location.href !== currentUrl) {
    currentUrl = location.href;
    clearAll(); // remove all overlays + reset state
    // fall through — immediately scan the new page content
  }

  const candidates = collectCandidates(document.body);

  const toRelint:  Candidate[] = []; // existing anchor whose XML changed (streaming refresh)
  const toAdd:     Candidate[] = []; // brand-new VAST not yet seen

  for (const c of candidates) {
    const existing = lastResults.get(c.anchor);
    const newHash  = simpleHash(c.xml);

    if (existing) {
      // Same anchor — check if content changed (char-by-char streaming case)
      if (existing.hash !== newHash) {
        // Evict old hash so it doesn't block future scans
        seen.delete(existing.hash);
        seen.add(newHash);
        toRelint.push(c);
      }
      // else: identical content, nothing to do
    } else {
      // Brand-new anchor
      if (seen.has(newHash)) continue;           // duplicate content in another element
      if (lastResults.size + toAdd.length >= MAX_VASTS) continue; // hard cap
      seen.add(newHash);
      toAdd.push(c);
    }
  }

  const all = [...toRelint, ...toAdd];
  if (all.length === 0) return;
  lintAndRenderAll(all);
}

function observeMutations() {
  // Two debounce timers:
  //   structural  — new child nodes added (e.g. page dynamically injects a VAST blob)
  //                 short delay so we pick it up quickly
  //   streaming   — characterData mutations (e.g. typing animation / mock IDE)
  //                 longer delay so we only lint once the content has settled,
  //                 not on every individual keystroke/character
  let structuralTimer = 0;
  let streamingTimer  = 0;

  const STRUCTURAL_DELAY_MS = 200;
  const STREAMING_DELAY_MS  = 800;

  const observer = new MutationObserver(mutations => {
    let hasStructural = false;
    let hasStreaming   = false;

    for (const m of mutations) {
      if (m.type === 'childList' && m.addedNodes.length) hasStructural = true;
      if (m.type === 'characterData') hasStreaming = true;
    }

    if (hasStructural) {
      clearTimeout(structuralTimer);
      structuralTimer = window.setTimeout(() => scanDOM(), STRUCTURAL_DELAY_MS);
    }

    if (hasStreaming) {
      // Reset the streaming timer on every character — only fires after typing stops
      clearTimeout(streamingTimer);
      streamingTimer = window.setTimeout(() => scanDOM(), STREAMING_DELAY_MS);
    }
  });

  observer.observe(document.body, {
    childList: true,
    subtree: true,
    characterData: true,
  });
}

// ─── Navigation cleanup ───────────────────────────────────────────────────────

function clearAll() {
  // Sync currentUrl so scanDOM's URL-change check doesn't double-fire
  currentUrl = location.href;

  // Call each anchor's cleanup function to remove overlays + cancel rAF loops
  for (const anchor of lastResults.keys()) {
    const el = anchor as HTMLElement & { _vlCleanup?: () => void };
    el._vlCleanup?.();
  }
  // Belt-and-suspenders: remove any stray overlay hosts by attribute
  document.querySelectorAll('[data-vastlint-overlay],[data-vastlint-panel]').forEach(el => el.remove());

  seen.clear();
  lastResults.clear();
  // Don't reset vastCounter — labels stay unique across navigations within the same context lifetime

  try {
    chrome.runtime.sendMessage({ type: 'UPDATE_BADGE', vasts: [] });
  } catch { /* extension context may be gone */ }
}

/** Re-scan after a SPA navigation — DOM needs a tick to settle. */
function onNavigation() {
  clearAll();
  // Wait one animation frame for the SPA to update the DOM
  requestAnimationFrame(() => scanDOM());
}

function hookNavigation() {
  // 'vastlint:nav' is fired by nav-hook.js (world: "MAIN") which patches
  // history.pushState, replaceState, and popstate in the page's main world.
  // This is the reliable path for all SPA navigations.
  window.addEventListener('vastlint:nav', onNavigation);

  // Belt-and-suspenders: also handle popstate directly in case the main-world
  // script hasn't fired yet. onNavigation is idempotent (clearAll is safe to
  // call twice — the second call is a no-op since lastResults is already empty).
  window.addEventListener('popstate', onNavigation);

  // Real full-page unloads
  window.addEventListener('pagehide', clearAll);
}

// ─── Candidate collection ─────────────────────────────────────────────────────

interface Candidate {
  xml: string;
  /** Element to anchor the overlay panel to */
  anchor: Element;
}

function collectCandidates(root: Element | Document): Candidate[] {
  const results: Candidate[] = [];

  // 1. <pre> or <textarea> containing raw XML
  const prelike = (root instanceof Element ? root : document).querySelectorAll<HTMLElement>(
    'pre, textarea, [data-vast], [data-ad-tag], [data-ad-tag-uri]'
  );
  for (const el of prelike) {
    const text = el.tagName === 'TEXTAREA'
      ? (el as HTMLTextAreaElement).value
      : el.textContent ?? '';
    if (VAST_SIGNATURE_RE.test(text)) {
      results.push({ xml: text.trim(), anchor: el });
    }
  }

  // 2. <script type="text/xml"> or similar embedded blobs
  const scripts = (root instanceof Element ? root : document).querySelectorAll<HTMLScriptElement>(
    'script[type="text/xml"], script[type="application/xml"]'
  );
  for (const el of scripts) {
    const text = el.textContent ?? '';
    if (VAST_SIGNATURE_RE.test(text)) {
      results.push({ xml: text.trim(), anchor: el });
    }
  }

  // 3. Arbitrary text nodes whose entire content looks like VAST
  //    (catches pages that just dump XML without any wrapper element)
  const walker = document.createTreeWalker(
    root instanceof Element ? root : document.body,
    NodeFilter.SHOW_TEXT,
    null
  );
  let node: Node | null;
  while ((node = walker.nextNode())) {
    const text = (node as Text).textContent ?? '';
    if (text.length > 100 && VAST_SIGNATURE_RE.test(text)) {
      const parent = (node as Text).parentElement;
      if (parent && !['PRE', 'TEXTAREA', 'SCRIPT', 'STYLE'].includes(parent.tagName)) {
        results.push({ xml: text.trim(), anchor: parent });
      }
    }
  }

  return results;
}

// ─── Lint & render ────────────────────────────────────────────────────────────

interface Candidate { xml: string; anchor: Element; }

/** Lint all candidates in parallel, render overlays, then send ONE batched badge message. */
async function lintAndRenderAll(candidates: Candidate[]) {
  const results = await Promise.all(
    candidates.map(async ({ xml, anchor }) => {
      const result = await validateVast(xml);
      renderPanel(result, anchor);

      const adIdMatch = xml.match(/<Ad\b[^>]*\bid=["']([^"']+)["']/i);
      // Reuse existing label if this is a refresh of an already-tracked element
      const existing = lastResults.get(anchor);
      const label = existing?.label ?? (adIdMatch ? adIdMatch[1] : `VAST #${++vastCounter}`);
      lastResults.set(anchor, { hash: simpleHash(xml), xml, result, label });

      return {
        label,
        version: result.version ?? null,
        errors:   result.summary.errors,
        warnings: result.summary.warnings,
        infos:    result.summary.infos ?? 0,
      };
    })
  );

  try {
    chrome.runtime.sendMessage({ type: 'UPDATE_BADGE', vasts: results });
  } catch {
    // Extension context invalidated (e.g. after reload) — silently ignore
  }
}

// ─── Message handler (requests from popup) ────────────────────────────────────

chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  // ── Disable on this host ───────────────────────────────────────────────────
  if (msg.type === 'DISABLE_SITE') {
    siteDisabled = true;
    clearAll();
    sendResponse({ ok: true });
    return true;
  }

  // ── Re-enable on this host ─────────────────────────────────────────────────
  if (msg.type === 'ENABLE_SITE') {
    siteDisabled = false;
    scanDOM();
    sendResponse({ ok: true });
    return true;
  }

  // ── Force re-scan ──────────────────────────────────────────────────────────
  if (msg.type === 'SCAN_NOW') {
    // Evict all existing hashes so scanDOM treats everything as fresh
    seen.clear();
    // Keep lastResults entries so streaming-refresh logic still works per-anchor
    scanDOM();
    sendResponse({ ok: true });
    return true;
  }

  // ── Focus / scroll to a specific VAST ──────────────────────────────────────
  if (msg.type === 'FOCUS_VAST') {
    let found = false;
    for (const [anchor, entry] of lastResults) {
      if (entry.label === msg.label) {
        anchor.scrollIntoView({ behavior: 'smooth', block: 'center' });
        const el = anchor as HTMLElement;
        const prev = el.style.outline;
        el.style.outline = '2px solid #63b3ed';
        el.style.outlineOffset = '4px';
        setTimeout(() => { el.style.outline = prev; el.style.outlineOffset = ''; }, 1500);
        found = true;
        break;
      }
    }
    sendResponse({ ok: found });
    return true;
  }

  // ── Copy annotated XML for a single VAST by label ─────────────────────────
  if (msg.type === 'COPY_ANNOTATED_ONE') {
    let found: { xml: string; result: ValidationResult } | undefined;
    for (const entry of lastResults.values()) {
      if (entry.label === msg.label) { found = entry; break; }
    }
    if (!found) {
      sendResponse({ ok: false, reason: 'VAST not found.' });
      return true;
    }
    const text = buildAnnotatedXml(found.xml, found.result);
    sendResponse({ ok: true, text });
    return true;
  }

  if (msg.type !== 'COPY_ANNOTATED') return;

  // Collect annotated text for all detected VAST blobs on the page
  const parts: string[] = [];
  for (const { xml, result } of lastResults.values()) {
    parts.push(buildAnnotatedXml(xml, result));
  }

  if (parts.length === 0) {
    sendResponse({ ok: false, reason: 'No VAST found on this page yet.' });
    return;
  }

  // Return the text to the popup — the popup writes to clipboard while it has focus
  sendResponse({ ok: true, count: parts.length, text: parts.join('\n\n<!-- ─── next VAST ─── -->\n\n') });
  return true;
});

/**
 * Rebuild the XML with inline <!-- vastlint: … --> comments injected before
 * each line that has issues, plus a header summary block.
 */
function buildAnnotatedXml(xml: string, result: ValidationResult): string {
  // Build line → issues map
  const lineIssues = new Map<number, typeof result.issues>();
  for (const iss of result.issues) {
    const ln = iss.line ?? 1;
    if (!lineIssues.has(ln)) lineIssues.set(ln, []);
    lineIssues.get(ln)!.push(iss);
  }

  const lines = xml.split('\n');
  const out: string[] = [];

  // Header summary
  const { errors, warnings, infos } = result.summary;
  out.push(`<!-- vastlint: ${errors} error(s), ${warnings} warning(s), ${infos} info(s) — VAST ${result.version ?? '?'} -->`);

  for (let i = 0; i < lines.length; i++) {
    const ln = i + 1;
    const issues = lineIssues.get(ln);
    if (issues) {
      for (const iss of issues) {
        const sev = iss.severity.toUpperCase().padEnd(7);
        const loc = iss.path ? ` @ ${iss.path}` : '';
        out.push(`<!-- vastlint ${sev} [${iss.id}]${loc}: ${iss.message} -->`);
      }
    }
    out.push(lines[i]);
  }

  return out.join('\n');
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

function simpleHash(s: string): string {
  let h = 0;
  for (let i = 0; i < Math.min(s.length, 4096); i++) {
    h = (Math.imul(31, h) + s.charCodeAt(i)) | 0;
  }
  return String(h);
}
