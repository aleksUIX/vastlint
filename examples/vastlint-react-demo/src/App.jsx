import { useState } from "react";

import { useVastPlayback, useVastSession, useVastTracker } from "vastlint-react";

const fixtureXml = `<?xml version="1.0" encoding="UTF-8"?>
<VAST version="4.2">
  <Ad id="react-demo-ad">
    <InLine>
      <AdSystem>vastlint react demo</AdSystem>
      <AdTitle>React Hook Review</AdTitle>
      <Impression><![CDATA[https://track.example.com/react/impression]]></Impression>
      <Creatives>
        <Creative>
          <Linear>
            <Duration>00:00:24</Duration>
            <TrackingEvents>
              <Tracking event="creativeView"><![CDATA[https://track.example.com/react/creative-view]]></Tracking>
              <Tracking event="start"><![CDATA[https://track.example.com/react/start]]></Tracking>
              <Tracking event="firstQuartile"><![CDATA[https://track.example.com/react/first-quartile]]></Tracking>
              <Tracking event="midpoint"><![CDATA[https://track.example.com/react/midpoint]]></Tracking>
              <Tracking event="thirdQuartile"><![CDATA[https://track.example.com/react/third-quartile]]></Tracking>
              <Tracking event="complete"><![CDATA[https://track.example.com/react/complete]]></Tracking>
              <Tracking event="pause"><![CDATA[https://track.example.com/react/pause]]></Tracking>
              <Tracking event="resume"><![CDATA[https://track.example.com/react/resume]]></Tracking>
            </TrackingEvents>
            <VideoClicks>
              <ClickThrough><![CDATA[https://click.example.com/react/landing]]></ClickThrough>
              <ClickTracking><![CDATA[https://track.example.com/react/click]]></ClickTracking>
            </VideoClicks>
            <MediaFiles>
              <MediaFile delivery="progressive" type="video/mp4" width="1280" height="720" bitrate="1500"><![CDATA[https://cdn.example.com/react-demo.mp4]]></MediaFile>
            </MediaFiles>
          </Linear>
        </Creative>
        <Creative>
          <CompanionAds>
            <Companion id="react-companion" width="300" height="250" adSlotId="right-rail">
              <StaticResource creativeType="image/png"><![CDATA[https://cdn.example.com/react-companion.png]]></StaticResource>
              <TrackingEvents>
                <Tracking event="creativeView"><![CDATA[https://track.example.com/react/companion-view]]></Tracking>
              </TrackingEvents>
              <CompanionClickThrough><![CDATA[https://click.example.com/react/companion]]></CompanionClickThrough>
              <CompanionClickTracking><![CDATA[https://track.example.com/react/companion-click]]></CompanionClickTracking>
            </Companion>
          </CompanionAds>
        </Creative>
      </Creatives>
    </InLine>
  </Ad>
</VAST>`;

async function mockFetch() {
  return new Response("", { status: 204, statusText: "No Content" });
}

const sessionOptions = {
  source: {
    kind: "xml",
    xml: fixtureXml,
    label: "react-demo.xml",
  },
  fetch: mockFetch,
  autoLoad: false,
  autoValidate: false,
};

const playbackOptions = {
  autoInitialize: false,
  mediaSelection: {
    supportedMimeTypes: ["video/mp4"],
    preferredMimeTypes: ["video/mp4"],
  },
};

function Stat({ label, value, accent = false }) {
  return (
    <div className={accent ? "stat stat--accent" : "stat"}>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function DataCard({ eyebrow, title, children }) {
  return (
    <article className="card panel">
      <div className="panel__header">
        <div>
          <p className="eyebrow">{eyebrow}</p>
          <h2>{title}</h2>
        </div>
      </div>
      {children}
    </article>
  );
}

function formatHistoryEntry(entry) {
  return `${entry.event} -> ${entry.url} (${entry.status ?? "n/a"})`;
}

export default function App() {
  const [error, setError] = useState(null);
  const session = useVastSession(sessionOptions);
  const tracker = useVastTracker({ session: session.session });
  const playback = useVastPlayback({ session: session.session, ...playbackOptions });

  const resolvedAd = session.snapshot.resolvedAd;
  const history = tracker.tracking.history;
  const companions = tracker.companions;
  const companionTargetCount = resolvedAd
    ? tracker.getCompanionTargets(0, { companionId: "react-companion" }, "clickTracking").length
    : 0;

  async function run(action) {
    try {
      setError(null);
      await action();
    } catch (nextError) {
      setError(nextError instanceof Error ? nextError.message : String(nextError));
    }
  }

  async function advancePlayback() {
    const nextTime = Math.min(playback.snapshot.currentTimeSec + 8, playback.snapshot.durationSec ?? 24);
    await playback.updateProgress(nextTime, playback.snapshot.durationSec ?? 24);
  }

  return (
    <main className="shell">
      <section className="hero card">
        <div>
          <p className="eyebrow">vastlint-react demo</p>
          <h1>Inspect how the hooks compose around one shared VAST session.</h1>
          <p className="lede">
            This demo wires <code>useVastSession</code>, <code>useVastTracker</code>, and <code>useVastPlayback</code>
            together against one fixture ad. Tracking uses a mock fetch, so every dispatch is reviewable in-session.
          </p>
        </div>
        <div className="hero__stats">
          <Stat label="session" value={session.snapshot.status} accent />
          <Stat label="playback" value={playback.snapshot.status} />
          <Stat label="history" value={history.length} />
          <Stat label="companions" value={companions.length} />
        </div>
      </section>

      <section className="grid">
        <DataCard eyebrow="Session hook" title="Lifecycle and resolve state">
          <div className="button-row">
            <button onClick={() => run(() => session.validate())}>Validate</button>
            <button onClick={() => run(() => session.resolve())}>Resolve</button>
            <button className="ghost" onClick={() => run(() => session.reload())}>Reload</button>
          </div>
          <div className="detail-list">
            <div><span>ad title</span><strong>{resolvedAd?.adTitle ?? "—"}</strong></div>
            <div><span>duration</span><strong>{resolvedAd?.duration ?? "—"}</strong></div>
            <div><span>media files</span><strong>{resolvedAd?.mediaFiles.length ?? 0}</strong></div>
            <div><span>companions</span><strong>{resolvedAd?.companions.length ?? 0}</strong></div>
          </div>
          <pre className="code-block">{JSON.stringify({
            validation: session.snapshot.validation?.summary ?? null,
            resolution: session.snapshot.resolution,
            events: session.snapshot.events.slice(-6),
          }, null, 2)}</pre>
        </DataCard>

        <DataCard eyebrow="Tracker hook" title="Companion and ad dispatch">
          <div className="button-row">
            <button onClick={() => run(() => tracker.track("impression"))}>Track impression</button>
            <button onClick={() => run(() => tracker.trackCompanion(0, { companionId: "react-companion" }, "clickTracking"))}>Track companion click</button>
          </div>
          <div className="detail-list">
            <div><span>available events</span><strong>{tracker.availableEvents.slice(0, 6).join(", ") || "—"}</strong></div>
            <div><span>click through</span><strong>{tracker.clickThroughUrl ?? "—"}</strong></div>
            <div><span>companion slot</span><strong>{companions[0]?.adSlotId ?? "—"}</strong></div>
            <div><span>companion targets</span><strong>{companionTargetCount}</strong></div>
          </div>
          <ol className="event-log">
            {history.length === 0 ? <li className="event-log__empty">No tracking requests yet.</li> : null}
            {history.slice().reverse().map((entry, index) => (
              <li key={`${entry.url}-${entry.dispatchedAt}-${index}`}>{formatHistoryEntry(entry)}</li>
            ))}
          </ol>
        </DataCard>

        <DataCard eyebrow="Playback hook" title="Simulate player control">
          <div className="button-grid">
            <button onClick={() => run(() => playback.initialize())}>Initialize</button>
            <button onClick={() => run(() => playback.start())}>Start</button>
            <button onClick={() => run(() => playback.pause())}>Pause</button>
            <button onClick={() => run(() => playback.resume())}>Resume</button>
            <button onClick={() => run(() => advancePlayback())}>Advance 8s</button>
            <button onClick={() => run(() => playback.click())}>Click</button>
            <button className="ghost" onClick={() => run(() => playback.complete())}>Complete</button>
          </div>
          <div className="detail-list">
            <div><span>current time</span><strong>{playback.snapshot.currentTimeSec.toFixed(1)}s</strong></div>
            <div><span>duration</span><strong>{playback.snapshot.durationSec ?? "—"}</strong></div>
            <div><span>viewability</span><strong>{playback.snapshot.viewability ?? "—"}</strong></div>
            <div><span>click through</span><strong>{playback.snapshot.clickThroughUrl ?? "—"}</strong></div>
          </div>
          <pre className="code-block">{JSON.stringify(playback.snapshot.milestones, null, 2)}</pre>
        </DataCard>

        <DataCard eyebrow="Fixture" title="The XML wired into the hooks">
          <pre className="code-block code-block--tall">{fixtureXml}</pre>
        </DataCard>
      </section>

      <footer className="footer card">
        <p>{error ?? "No hook-level errors. Use the buttons to drive the runtime manually."}</p>
      </footer>
    </main>
  );
}