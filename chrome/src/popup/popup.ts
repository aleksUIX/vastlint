/**
 * Popup script — reads cached badge data from chrome.storage.session,
 * provides copy / focus / scan actions, and a paste-to-analyze panel.
 */

interface VastEntry { label: string; version: string | null; errors: number; warnings: number; infos: number; }
interface TabData   { errors: number; warnings: number; infos: number; vasts: VastEntry[]; }

const SEV_COLOR: Record<string, string> = { error: '#ef5350', warning: '#ffb74d', info: '#63b3ed' };
// (kept for potential future use)

function escHtml(s: string): string {
  return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
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

  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  if (!tab?.id) {
    status.textContent = 'No active tab.';
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
      siteToggle.title = `vastlint is OFF on ${host} — click to enable`;
    } else {
      siteToggle.classList.add('on');
      siteToggle.classList.remove('disabled-state');
      toggleLabel.textContent = 'on';
      siteToggle.title = `vastlint is ON on ${host} — click to disable`;
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
      list.innerHTML = data.vasts.map((v) => {
        const hasErr  = v.errors   > 0;
        const hasWarn = v.warnings > 0;
        const dot  = hasErr  ? 'dot-err'
                   : hasWarn ? 'dot-warn'
                   :           'dot-ok';
        const pills = [
          v.errors   > 0 ? `<span class="pill pill-err">${v.errors}E</span>`   : '',
          v.warnings > 0 ? `<span class="pill pill-warn">${v.warnings}W</span>` : '',
          v.infos    > 0 ? `<span class="pill pill-info">${v.infos}I</span>`    : '',
          !hasErr && !hasWarn ? `<span class="pill pill-ok">✓</span>` : '',
        ].filter(Boolean).join('');
        const ver = v.version ? `<span class="vast-ver">VAST ${escHtml(v.version)}</span>` : '';
        return `<div class="vast-row" data-label="${escHtml(v.label)}">
          <span class="vast-dot ${dot}"></span>
          <span class="vast-label">${escHtml(v.label)}</span>
          ${ver}
          <span class="vast-pills">${pills}</span>
          <button class="focus-btn" data-label="${escHtml(v.label)}">focus</button>
          <button class="copy-one-btn" data-label="${escHtml(v.label)}" title="Copy annotated VAST">copy</button>
        </div>`;
      }).join('');

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
      status.innerHTML = `
        <strong>File access needed</strong>
        To validate local <code>.xml</code> files, enable
        <b>Allow access to file URLs</b> for this extension:<br><br>
        <code>chrome://extensions</code> → VAST lint → Details → toggle on
      `;
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

  // Auto-open tab on paste — short delay so it doesn't feel abrupt
  textarea.addEventListener('paste', () => {
    // value isn't updated yet during 'paste', wait one tick for DOM then delay
    setTimeout(async () => {
      const xml = textarea.value.trim();
      if (!xml) return;
      try {
        await chrome.storage.session.set({ paste_xml: xml });
        pasteSection.classList.add('analyzing');
        await new Promise(r => setTimeout(r, 700));
        await chrome.tabs.create({ url: chrome.runtime.getURL('analysis.html') });
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
      await chrome.storage.session.set({ paste_xml: xml });
      await chrome.tabs.create({ url: chrome.runtime.getURL('analysis.html') });
      window.close();
    } catch (e) {
      openBtn.disabled = false;
      openBtn.textContent = '↗ Analyze in new tab';
    }
  });
}

init().catch(console.error);

