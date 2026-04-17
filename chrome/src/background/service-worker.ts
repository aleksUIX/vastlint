/**
 * Service worker (MV3 background script).
 *
 * Responsibilities:
 *  - Receive UPDATE_BADGE messages from the content script
 *  - Store per-tab totals + per-VAST breakdown in chrome.storage.session
 *  - Update the action badge text/colour on the tab
 */

interface VastEntry { label: string; version: string | null; errors: number; warnings: number; infos: number; }
interface TabData   { errors: number; warnings: number; infos: number; vasts: VastEntry[]; }

chrome.runtime.onMessage.addListener((msg, sender) => {
  if (msg.type !== 'UPDATE_BADGE') return;
  const tabId = sender.tab?.id;
  if (tabId == null) return;

  // Content script sends a batch of all VASTs found in this scan
  const incoming = msg.vasts as VastEntry[];
  if (!incoming?.length) return;

  const key = `tab_${tabId}`;
  chrome.storage.session.get(key).then(stored => {
    const prev = (stored[key] ?? { errors: 0, warnings: 0, infos: 0, vasts: [] }) as TabData;
    const next: TabData = {
      errors:   prev.errors   + incoming.reduce((s, v) => s + v.errors,   0),
      warnings: prev.warnings + incoming.reduce((s, v) => s + v.warnings, 0),
      infos:    prev.infos    + incoming.reduce((s, v) => s + v.infos,    0),
      vasts:    [...prev.vasts, ...incoming],
    };
    chrome.storage.session.set({ [key]: next });
    setBadge(tabId, next.errors, next.warnings);
  });
});

// Clear per-tab data when the tab navigates away
chrome.tabs.onUpdated.addListener((tabId, changeInfo) => {
  if (changeInfo.status === 'loading') {
    chrome.storage.session.remove(`tab_${tabId}`);
    chrome.action.setBadgeText({ text: '', tabId });
  }
});

function setBadge(tabId: number, errors: number, warnings: number) {
  if (errors > 0) {
    chrome.action.setBadgeText({ text: String(errors), tabId });
    chrome.action.setBadgeBackgroundColor({ color: '#e53935', tabId });
  } else if (warnings > 0) {
    chrome.action.setBadgeText({ text: String(warnings), tabId });
    chrome.action.setBadgeBackgroundColor({ color: '#f4a000', tabId });
  } else {
    chrome.action.setBadgeText({ text: '✓', tabId });
    chrome.action.setBadgeBackgroundColor({ color: '#43a047', tabId });
  }
}
