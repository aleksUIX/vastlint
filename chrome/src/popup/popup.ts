/**
 * Popup script — reads cached badge data from chrome.storage.session,
 * provides copy / focus / scan actions, and paste-to-tester on vastlint.org.
 */

const SITE_URL = 'https://vastlint.org';
const TESTER_URL = 'https://vastlint.org/tester/';
const SIMID_STUDIO_URL = 'https://iab-tech-lab-vast-tester.vastlint.org/';
/** Match vastlint-infra share.ts: fragments longer than this are unusable. */
const MAX_FRAGMENT_CHARS = 16_000;

interface VastEntry { label: string; version: string | null; errors: number; warnings: number; infos: number; }
interface TabData   { errors: number; warnings: number; infos: number; vasts: VastEntry[]; }

const SEV_COLOR: Record<string, string> = { error: '#ef5350', warning: '#ffb74d', info: '#63b3ed' };
// (kept for potential future use)

function createNode<K extends keyof HTMLElementTagNameMap>(tag: K, className?: string): HTMLElementTagNameMap[K] {
  const el = document.createElement(tag);
  if (className) el.className = className;
  return el;
}

function renderFileAccessMessage(container: HTMLElement) {
  const strong = createNode('strong');
  strong.textContent = 'File access needed';

  const xmlCode = createNode('code');
  xmlCode.textContent = '.xml';

  const bold = createNode('b');
  bold.textContent = 'Allow access to file URLs';

  const chromeCode = createNode('code');
  chromeCode.textContent = 'chrome://extensions';

  container.replaceChildren(
    strong,
    document.createTextNode(' To validate local '),
    xmlCode,
    document.createTextNode(' files, enable '),
    bold,
    document.createTextNode(' for this extension:\n\n'),
    chromeCode,
    document.createTextNode(' → VAST lint → Details → toggle on'),
  );
}

function createVastRow(v: VastEntry): HTMLDivElement {
  const hasErr  = v.errors > 0;
  const hasWarn = v.warnings > 0;
  const dot  = hasErr ? 'dot-err' : hasWarn ? 'dot-warn' : 'dot-ok';

  const row = createNode('div', 'vast-row');
  row.dataset.label = v.label;

  row.appendChild(createNode('span', `vast-dot ${dot}`));

  const label = createNode('span', 'vast-label');
  label.textContent = v.label;
  row.appendChild(label);

  if (v.version) {
    const ver = createNode('span', 'vast-ver');
    ver.textContent = `VAST ${v.version}`;
    row.appendChild(ver);
  }

  const pills = createNode('span', 'vast-pills');
  if (v.errors > 0) {
    const pill = createNode('span', 'pill pill-err');
    pill.textContent = `${v.errors}E`;
    pills.appendChild(pill);
  }
  if (v.warnings > 0) {
    const pill = createNode('span', 'pill pill-warn');
    pill.textContent = `${v.warnings}W`;
    pills.appendChild(pill);
  }
  if (v.infos > 0) {
    const pill = createNode('span', 'pill pill-info');
    pill.textContent = `${v.infos}I`;
    pills.appendChild(pill);
  }
  if (!hasErr && !hasWarn) {
    const pill = createNode('span', 'pill pill-ok');
    pill.textContent = '✓';
    pills.appendChild(pill);
  }
  row.appendChild(pills);

  const focusBtn = createNode('button', 'focus-btn');
  focusBtn.dataset.label = v.label;
  focusBtn.textContent = 'focus';
  row.appendChild(focusBtn);

  const copyOneBtn = createNode('button', 'copy-one-btn');
  copyOneBtn.dataset.label = v.label;
  copyOneBtn.title = 'Copy annotated VAST';
  copyOneBtn.textContent = 'copy';
  row.appendChild(copyOneBtn);

  return row;
}

function toBase64Url(bytes: Uint8Array): string {
  let bin = '';
  for (let i = 0; i < bytes.length; i++) bin += String.fromCharCode(bytes[i]);
  return btoa(bin).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

async function deflateRaw(text: string): Promise<Uint8Array> {
  const stream = new Blob([text]).stream().pipeThrough(new CompressionStream('deflate-raw'));
  return new Uint8Array(await new Response(stream).arrayBuffer());
}

/** Tester share URL: `?url=` for http(s) tags, `#vast=` blob for pasted XML. */
async function testerUrlForInput(input: string): Promise<string> {
  const trimmed = input.trim();
  if (/^https?:\/\//i.test(trimmed)) {
    return `${TESTER_URL}?url=${encodeURIComponent(trimmed)}`;
  }
  if (typeof CompressionStream === 'undefined') {
    throw new Error('unsupported');
  }
  const blob = toBase64Url(await deflateRaw(trimmed));
  if (blob.length > MAX_FRAGMENT_CHARS) {
    throw new Error('too-large');
  }
  return `${TESTER_URL}#vast=${blob}`;
}

async function openUrl(url: string) {
  await chrome.tabs.create({ url });
}

async function openSite() {
  await openUrl(SITE_URL);
}

async function openTester(input: string) {
  try {
    const url = await testerUrlForInput(input);
    await chrome.tabs.create({ url });
  } catch {
    // Huge tags (or missing CompressionStream) stay in the local analysis tab.
    await chrome.storage.session.set({ paste_xml: input.trim() });
    await chrome.tabs.create({ url: chrome.runtime.getURL('analysis.html') });
  }
}

async function init() {
  const status   = document.getElementById('status')!;
  const statsEl  = document.getElementById('stats')!;
  const errCount = document.getElementById('err-count')!;
  const warnCount= document.getElementById('warn-count')!;
  const infoCount= document.getElementById('info-count')!;
  const copyBtn  = document.getElementById('copy-btn') as HTMLButtonElement;
  const scanBtn  = document.getElementById('scan-btn') as HTMLButtonElement;
  const siteToggle = document.getElementById('site-toggle') as HTMLButtonElement;
  const toggleLabel = siteToggle.querySelector('.toggle-label') as HTMLElement;
  const websiteBtn = document.getElementById('website-btn') as HTMLButtonElement | null;
  const websiteLink = document.getElementById('website-link') as HTMLAnchorElement | null;
  const analysisStudioBtn = document.getElementById('analysis-studio-btn') as HTMLButtonElement | null;
  const simidStudioBtn = document.getElementById('simid-studio-btn') as HTMLButtonElement | null;
  const versionEl = document.getElementById('extension-version');

  if (versionEl) {
    versionEl.textContent = `v${chrome.runtime.getManifest().version}`;
  }

  websiteBtn?.addEventListener('click', async () => {
    await openSite();
    window.close();
  });
  websiteLink?.addEventListener('click', async (e) => {
    e.preventDefault();
    await openSite();
    window.close();
  });
  analysisStudioBtn?.addEventListener('click', async () => {
    await openUrl(TESTER_URL);
    window.close();
  });
  simidStudioBtn?.addEventListener('click', async () => {
    await openUrl(SIMID_STUDIO_URL);
    window.close();
  });

  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  if (!tab?.id) {
    status.textContent = 'No active tab.';
    initPasteAnalyzer();
    return;
  }

  // ── Site disable toggle ────────────────────────────────────────────────────
  const host = new URL(tab.url ?? 'about:blank').hostname;
  const syncData = await chrome.storage.sync.get('disabledHosts');
  let disabledHosts: string[] = syncData.disabledHosts ?? [];
  let siteDisabled = disabledHosts.includes(host);

  function applyToggleState() {
    if (siteDisabled) {
      siteToggle.classList.remove('on');
      siteToggle.classList.add('disabled-state');
      toggleLabel.textContent = 'off';
      siteToggle.title = `VASTlint is OFF on ${host} — click to enable`;
    } else {
      siteToggle.classList.add('on');
      siteToggle.classList.remove('disabled-state');
      toggleLabel.textContent = 'on';
      siteToggle.title = `VASTlint is ON on ${host} — click to disable`;
    }
  }
  applyToggleState();

  siteToggle.addEventListener('click', async () => {
    siteDisabled = !siteDisabled;

    if (siteDisabled) {
      disabledHosts = [...new Set([...disabledHosts, host])];
    } else {
      disabledHosts = disabledHosts.filter(h => h !== host);
    }
    await chrome.storage.sync.set({ disabledHosts });
    applyToggleState();

    const msgType = siteDisabled ? 'DISABLE_SITE' : 'ENABLE_SITE';
    try {
      await chrome.tabs.sendMessage(tab.id!, { type: msgType });
    } catch { /* content script may not be active */ }

    // Clear badge immediately when disabling
    if (siteDisabled) {
      await chrome.storage.session.remove(`tab_${tab.id}`);
      await chrome.action.setBadgeText({ text: '', tabId: tab.id! });
    }

    // Close and let the user see the result (badge cleared / page re-scanned)
    window.close();
  });

  const key    = `tab_${tab.id}`;
  const stored = await chrome.storage.session.get(key);
  const data   = stored[key] as TabData | undefined;

  if (data) {
    status.style.display = 'none';
    statsEl.style.display = 'block';
    errCount.textContent  = String(data.errors);
    warnCount.textContent = String(data.warnings);
    infoCount.textContent = String(data.infos ?? 0);
    copyBtn.style.display = 'inline-flex';

    // green "all clear" styling when counts are zero
    const statErr  = document.getElementById('stat-err')!;
    const statWarn = document.getElementById('stat-warn')!;
    if (data.errors   === 0) statErr.classList.add('zero');
    if (data.warnings === 0) statWarn.classList.add('zero');

    // Per-VAST breakdown — only shown when there are 2+ tags
    if (data.vasts && data.vasts.length > 1) {
      const breakdown = document.getElementById('breakdown')!;
      breakdown.style.display = 'block';
      const list = document.getElementById('vast-list')!;
      list.replaceChildren(...data.vasts.map(createVastRow));

      // Focus buttons
      list.querySelectorAll<HTMLButtonElement>('.focus-btn').forEach(btn => {
        btn.addEventListener('click', async () => {
          const label = btn.dataset.label!;
          try {
            await chrome.tabs.sendMessage(tab.id!, { type: 'FOCUS_VAST', label });
          } catch { /* tab may not have content script */ }
          window.close();
        });
      });

      // Per-tag copy buttons
      list.querySelectorAll<HTMLButtonElement>('.copy-one-btn').forEach(btn => {
        btn.addEventListener('click', async () => {
          const label = btn.dataset.label!;
          btn.disabled = true;
          btn.textContent = '…';
          try {
            const response = await chrome.tabs.sendMessage(tab.id!, { type: 'COPY_ANNOTATED_ONE', label }) as
              { ok: boolean; text?: string; reason?: string };
            if (response.ok && response.text) {
              await navigator.clipboard.writeText(response.text);
              btn.textContent = '✓';
              setTimeout(() => { btn.disabled = false; btn.textContent = 'copy'; }, 1500);
            } else {
              btn.textContent = '✕';
              setTimeout(() => { btn.disabled = false; btn.textContent = 'copy'; }, 1500);
            }
          } catch {
            btn.textContent = '✕';
            setTimeout(() => { btn.disabled = false; btn.textContent = 'copy'; }, 1500);
          }
        });
      });
    }
  } else {
    // Check if we're on a file:// XML page — content scripts can't run there
    // unless the user enables "Allow access to file URLs" in chrome://extensions
    const url = tab.url ?? '';
    const isFileXml = url.startsWith('file://') && /\.xml(\?|#|$)/i.test(url);

    if (isFileXml) {
      renderFileAccessMessage(status);
    }
    // else leave default "No VAST XML detected" message
  }

  // ── Copy annotated VAST ────────────────────────────────────────────────────
  copyBtn.addEventListener('click', async () => {
    copyBtn.disabled = true;
    copyBtn.textContent = 'Copying…';

    try {
      const response = await chrome.tabs.sendMessage(tab.id!, { type: 'COPY_ANNOTATED' }) as
        { ok: boolean; count?: number; text?: string; reason?: string };

      if (response.ok && response.text) {
        await navigator.clipboard.writeText(response.text);
      copyBtn.textContent = `✓ Copied${response.count && response.count > 1 ? ` (${response.count})` : ''}`;
        copyBtn.classList.add('success');
      } else {
        copyBtn.textContent = `✕ ${response.reason ?? 'Failed'}`;
        copyBtn.classList.add('error');
      }
    } catch {
      copyBtn.textContent = '✕ Could not reach page';
      copyBtn.classList.add('error');
    }

    setTimeout(() => {
      copyBtn.disabled = false;
      copyBtn.textContent = '📋 Copy VAST';
      copyBtn.classList.remove('success', 'error');
    }, 2500);
  });

  // ── Scan Now ───────────────────────────────────────────────────────────────
  scanBtn.addEventListener('click', async () => {
    scanBtn.disabled = true;
    scanBtn.textContent = '⟳ Scanning…';
    try {
      await chrome.tabs.sendMessage(tab.id!, { type: 'SCAN_NOW' });
      // Give the content script a moment to lint and send UPDATE_BADGE,
      // then re-open the popup with fresh data
      setTimeout(() => window.close(), 600);
    } catch {
      scanBtn.textContent = '✕ Failed';
      setTimeout(() => {
        scanBtn.disabled = false;
        scanBtn.textContent = '⟳ Scan page';
      }, 1500);
    }
  });

  // ── Paste & Analyze ────────────────────────────────────────────────────────
  initPasteAnalyzer();
}

function initPasteAnalyzer() {
  const textarea     = document.getElementById('paste-input')    as HTMLTextAreaElement;
  const clearBtn     = document.getElementById('paste-clear')    as HTMLButtonElement;
  const openBtn      = document.getElementById('paste-open')     as HTMLButtonElement;
  const pasteSection = document.getElementById('paste-section')  as HTMLElement;

  function updateBtn() {
    const has = textarea.value.trim().length > 0;
    textarea.classList.toggle('has-content', has);
    clearBtn.style.display = has ? 'inline' : 'none';
    openBtn.disabled = !has;
  }

  textarea.addEventListener('input', updateBtn);

  // Auto-open the hosted tester on paste. Short delay so it doesn't feel abrupt
  textarea.addEventListener('paste', () => {
    // value isn't updated yet during 'paste', wait one tick for DOM then delay
    setTimeout(async () => {
      const xml = textarea.value.trim();
      if (!xml) return;
      try {
        pasteSection.classList.add('analyzing');
        await new Promise(r => setTimeout(r, 700));
        await openTester(xml);
        window.close();
      } catch {
        pasteSection.classList.remove('analyzing');
        updateBtn(); // fall back to manual button
      }
    }, 0);
  });

  clearBtn.addEventListener('click', () => {
    textarea.value = '';
    updateBtn();
  });

  openBtn.addEventListener('click', async () => {
    const xml = textarea.value.trim();
    if (!xml) return;
    openBtn.disabled = true;
    openBtn.textContent = 'Opening…';
    try {
      await openTester(xml);
      window.close();
    } catch {
      openBtn.disabled = false;
      openBtn.textContent = 'Open tester';
    }
  });
}

init().catch(console.error);

