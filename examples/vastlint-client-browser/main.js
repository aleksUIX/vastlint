import "./style.css";

import { createVastSession, selectResolvedAdMediaFile } from "vastlint-client";

const fixtureXml = `<?xml version="1.0" encoding="UTF-8"?>
<VAST version="4.2">
  <Ad id="browser-demo-ad">
    <InLine>
      <AdSystem>vastlint browser demo</AdSystem>
      <AdTitle>Browser Session Review</AdTitle>
      <Impression><![CDATA[https://track.example.com/impression]]></Impression>
      <Creatives>
        <Creative>
          <Linear>
            <Duration>00:00:30</Duration>
            <TrackingEvents>
              <Tracking event="creativeView"><![CDATA[https://track.example.com/creative-view]]></Tracking>
              <Tracking event="start"><![CDATA[https://track.example.com/start]]></Tracking>
            </TrackingEvents>
            <VideoClicks>
              <ClickThrough><![CDATA[https://click.example.com/landing]]></ClickThrough>
              <ClickTracking><![CDATA[https://track.example.com/click]]></ClickTracking>
            </VideoClicks>
            <MediaFiles>
              <MediaFile delivery="progressive" type="video/mp4" width="1280" height="720" bitrate="1400"><![CDATA[https://cdn.example.com/browser-demo.mp4]]></MediaFile>
              <MediaFile delivery="streaming" type="application/vnd.apple.mpegurl" width="1920" height="1080" bitrate="2400"><![CDATA[https://cdn.example.com/browser-demo.m3u8]]></MediaFile>
            </MediaFiles>
          </Linear>
        </Creative>
        <Creative>
          <CompanionAds>
            <Companion id="hero-companion" width="300" height="250" adSlotId="sidebar-right">
              <StaticResource creativeType="image/png"><![CDATA[https://cdn.example.com/companion.png]]></StaticResource>
              <TrackingEvents>
                <Tracking event="creativeView"><![CDATA[https://track.example.com/companion/view]]></Tracking>
              </TrackingEvents>
              <CompanionClickThrough><![CDATA[https://click.example.com/companion]]></CompanionClickThrough>
              <CompanionClickTracking><![CDATA[https://track.example.com/companion/click]]></CompanionClickTracking>
            </Companion>
          </CompanionAds>
        </Creative>
      </Creatives>
    </InLine>
  </Ad>
</VAST>`;

const app = document.querySelector("#app");

const state = {
  xml: fixtureXml,
  session: null,
  unsubscribe: null,
  snapshot: null,
  networkLog: [],
  busy: false,
  error: null,
};

function formatJson(value) {
  return JSON.stringify(value, null, 2);
}

function mockFetch(url) {
  state.networkLog = [
    {
      at: new Date().toISOString(),
      url: String(url),
    },
    ...state.networkLog,
  ].slice(0, 12);
  render();

  return Promise.resolve(new Response("", { status: 204, statusText: "No Content" }));
}

function teardownSession() {
  if (state.unsubscribe) {
    state.unsubscribe();
    state.unsubscribe = null;
  }
}

function mountSession(xml) {
  teardownSession();
  state.xml = xml;
  state.error = null;
  state.busy = false;
  state.networkLog = [];
  state.session = createVastSession({
    source: {
      kind: "xml",
      xml,
      label: "browser-demo.xml",
    },
    fetch: mockFetch,
  });
  state.snapshot = state.session.getSnapshot();
  state.unsubscribe = state.session.subscribe((snapshot) => {
    state.snapshot = snapshot;
    render();
  });
  render();
}

async function runAction(action) {
  if (!state.session || state.busy) {
    return;
  }

  state.busy = true;
  state.error = null;
  render();

  try {
    await action();
  } catch (error) {
    state.error = error instanceof Error ? error.message : String(error);
  } finally {
    state.busy = false;
    render();
  }
}

function createStat(label, value, accent = false) {
  return `<div class="stat ${accent ? "stat--accent" : ""}"><span>${label}</span><strong>${value}</strong></div>`;
}

function render() {
  const snapshot = state.snapshot;
  const resolvedAd = snapshot?.resolvedAd ?? null;
  const selectedMedia = resolvedAd
    ? selectResolvedAdMediaFile(resolvedAd, {
        supportedMimeTypes: ["video/mp4", "application/vnd.apple.mpegurl"],
        preferredMimeTypes: ["video/mp4", "application/vnd.apple.mpegurl"],
        preferredDelivery: ["progressive", "streaming"],
        targetWidth: 1280,
        targetHeight: 720,
      })
    : { selected: null, candidates: [] };

  const validation = snapshot?.validation?.summary;
  const companions = resolvedAd?.companions ?? [];
  const companionTargets = resolvedAd
    ? state.session.getCompanionTrackingTargets(0, { companionId: "hero-companion" }, "clickTracking")
    : [];

  app.innerHTML = `
    <main class="shell">
      <section class="hero card">
        <div>
          <p class="eyebrow">vastlint-client browser demo</p>
          <h1>Review session resolve, media choice, and companion tracking in one page.</h1>
          <p class="lede">This demo runs entirely against fixture XML in the browser. Tracking dispatch is intercepted by a mock fetch so you can review emitted URLs without leaving the page.</p>
        </div>
        <div class="hero__stats">
          ${createStat("status", snapshot?.status ?? "idle", true)}
          ${createStat("errors", validation?.errors ?? 0)}
          ${createStat("warnings", validation?.warnings ?? 0)}
          ${createStat("companions", companions.length)}
        </div>
      </section>

      <section class="layout">
        <article class="card panel panel--editor">
          <div class="panel__header">
            <div>
              <p class="eyebrow">Fixture XML</p>
              <h2>Edit and recreate the session</h2>
            </div>
            <button class="ghost" data-action="reset">Apply XML</button>
          </div>
          <textarea id="xml-input" spellcheck="false">${state.xml.replace(/</g, "&lt;")}</textarea>
        </article>

        <article class="card panel">
          <div class="panel__header">
            <div>
              <p class="eyebrow">Session controls</p>
              <h2>Core runtime actions</h2>
            </div>
          </div>
          <div class="button-grid">
            <button data-action="validate">Validate</button>
            <button data-action="resolve">Resolve</button>
            <button data-action="track-impression">Track impression</button>
            <button data-action="track-companion">Track companion click</button>
          </div>
          <div class="status-strip">
            <span>${state.busy ? "Running…" : "Ready"}</span>
            <span>${state.error ? `Error: ${state.error}` : "No client-side errors"}</span>
          </div>
          <pre class="code-block">${formatJson({
            source: snapshot?.source ?? null,
            resolution: snapshot?.resolution ?? null,
            trackingHistory: snapshot?.tracking.history ?? [],
          })}</pre>
        </article>

        <article class="card panel">
          <div class="panel__header">
            <div>
              <p class="eyebrow">Resolved output</p>
              <h2>What playback consumers receive</h2>
            </div>
          </div>
          <div class="detail-list">
            <div><span>ad title</span><strong>${resolvedAd?.adTitle ?? "—"}</strong></div>
            <div><span>duration</span><strong>${resolvedAd?.duration ?? "—"}</strong></div>
            <div><span>media chosen</span><strong>${selectedMedia.selected?.url ?? "—"}</strong></div>
            <div><span>click through</span><strong>${resolvedAd?.clickThroughUrl ?? "—"}</strong></div>
          </div>
          <pre class="code-block">${formatJson({
            resolvedAd,
            selectedMedia,
            companionClickTargets: companionTargets,
          })}</pre>
        </article>

        <article class="card panel">
          <div class="panel__header">
            <div>
              <p class="eyebrow">Mock network log</p>
              <h2>Tracking URLs dispatched by the session</h2>
            </div>
          </div>
          <ol class="network-log">
            ${state.networkLog.map((entry) => `<li><time>${entry.at}</time><span>${entry.url}</span></li>`).join("") || "<li class=\"network-log__empty\">No tracking requests yet.</li>"}
          </ol>
        </article>
      </section>
    </main>
  `;

  app.querySelector("[data-action='reset']")?.addEventListener("click", () => {
    const nextXml = app.querySelector("#xml-input")?.value ?? state.xml;
    mountSession(nextXml);
  });
  app.querySelector("[data-action='validate']")?.addEventListener("click", () => runAction(() => state.session.validate()));
  app.querySelector("[data-action='resolve']")?.addEventListener("click", () => runAction(() => state.session.resolve()));
  app.querySelector("[data-action='track-impression']")?.addEventListener("click", () => runAction(() => state.session.track("impression")));
  app.querySelector("[data-action='track-companion']")?.addEventListener("click", () => runAction(() => state.session.trackCompanion(0, { companionId: "hero-companion" }, "clickTracking")));
}

mountSession(fixtureXml);