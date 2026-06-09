import { useEffect, useRef, useState } from "react";
import {
  createVastPlaybackController,
  createVastSession,
  selectResolvedAdMediaFile,
  selectTrackingTargets,
} from "vastlint-client";
import { useVastPlayback, useVastSession, useVastTracker } from "vastlint-react";

const fixtureXml = `<?xml version="1.0" encoding="UTF-8"?>
<VAST version="4.2">
  <Ad id="scratch-runtime-ad">
    <InLine>
      <AdSystem>vastlint scratch app</AdSystem>
      <AdTitle>Client and Hook Runtime Walkthrough</AdTitle>
      <Impression><![CDATA[https://track.example.com/scratch/impression]]></Impression>
      <Creatives>
        <Creative>
          <Linear>
            <Duration>00:00:24</Duration>
            <TrackingEvents>
              <Tracking event="creativeView"><![CDATA[https://track.example.com/scratch/creative-view]]></Tracking>
              <Tracking event="start"><![CDATA[https://track.example.com/scratch/start]]></Tracking>
              <Tracking event="firstQuartile"><![CDATA[https://track.example.com/scratch/first-quartile]]></Tracking>
              <Tracking event="midpoint"><![CDATA[https://track.example.com/scratch/midpoint]]></Tracking>
              <Tracking event="thirdQuartile"><![CDATA[https://track.example.com/scratch/third-quartile]]></Tracking>
              <Tracking event="complete"><![CDATA[https://track.example.com/scratch/complete]]></Tracking>
              <Tracking event="pause"><![CDATA[https://track.example.com/scratch/pause]]></Tracking>
              <Tracking event="resume"><![CDATA[https://track.example.com/scratch/resume]]></Tracking>
            </TrackingEvents>
            <VideoClicks>
              <ClickThrough><![CDATA[https://click.example.com/scratch/landing]]></ClickThrough>
              <ClickTracking><![CDATA[https://track.example.com/scratch/click]]></ClickTracking>
            </VideoClicks>
            <MediaFiles>
              <MediaFile delivery="progressive" type="video/mp4" width="1280" height="720" bitrate="1400"><![CDATA[https://cdn.example.com/scratch-runtime.mp4]]></MediaFile>
              <MediaFile delivery="streaming" type="application/vnd.apple.mpegurl" width="1920" height="1080" bitrate="2400"><![CDATA[https://cdn.example.com/scratch-runtime.m3u8]]></MediaFile>
            </MediaFiles>
          </Linear>
        </Creative>
        <Creative>
          <CompanionAds>
            <Companion id="scratch-companion" width="300" height="250" adSlotId="sidebar-right">
              <StaticResource creativeType="image/png"><![CDATA[https://cdn.example.com/scratch-companion.png]]></StaticResource>
              <TrackingEvents>
                <Tracking event="creativeView"><![CDATA[https://track.example.com/scratch/companion-view]]></Tracking>
              </TrackingEvents>
              <CompanionClickThrough><![CDATA[https://click.example.com/scratch/companion]]></CompanionClickThrough>
              <CompanionClickTracking><![CDATA[https://track.example.com/scratch/companion-click]]></CompanionClickTracking>
            </Companion>
          </CompanionAds>
        </Creative>
      </Creatives>
    </InLine>
  </Ad>
</VAST>`;

const mediaSelection = {
  supportedMimeTypes: ["video/mp4", "application/vnd.apple.mpegurl"],
  preferredMimeTypes: ["video/mp4", "application/vnd.apple.mpegurl"],
  preferredDelivery: ["progressive", "streaming"],
  targetWidth: 1280,
  targetHeight: 720,
};

const playbackAdvanceSeconds = 6;

function formatJson(value) {
  return JSON.stringify(value, null, 2);
}

function createMockFetch(runtime, setNetworkLog) {
  return async function mockFetch(url) {
    setNetworkLog((current) => [
      {
        at: new Date().toISOString(),
        runtime,
        url: String(url),
      },
      ...current,
    ].slice(0, 12));

    return new Response(null, { status: 204, statusText: "No Content" });
  };
}

function getSelectedMedia(resolvedAd) {
  return resolvedAd
    ? selectResolvedAdMediaFile(resolvedAd, mediaSelection)
    : { selected: null, candidates: [] };
}

function Stat({ label, value, accent = false }) {
  return (
    <div className={accent ? "stat stat--accent" : "stat"}>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function Panel({ eyebrow, title, children, className = "" }) {
  const panelClassName = className ? `card panel ${className}` : "card panel";

  return (
    <article className={panelClassName}>
      <div className="panel__header">
        <div>
          <p className="eyebrow">{eyebrow}</p>
          <h3>{title}</h3>
        </div>
      </div>
      {children}
    </article>
  );
}

function NetworkLog({ entries }) {
  return (
    <ol className="network-log">
      {entries.length === 0 ? <li className="network-log__empty">No tracking requests yet.</li> : null}
      {entries.map((entry, index) => (
        <li key={`${entry.runtime}-${entry.at}-${entry.url}-${index}`}>
          <time>{entry.at}</time>
          <strong>{entry.runtime}</strong>
          <span>{entry.url}</span>
        </li>
      ))}
    </ol>
  );
}

function RuntimeFrame({ eyebrow, title, description, stats, children, footer }) {
  return (
    <section className="runtime card">
      <header className="runtime__header">
        <div>
          <p className="eyebrow">{eyebrow}</p>
          <h2>{title}</h2>
          <p className="lede lede--compact">{description}</p>
        </div>
        <div className="hero__stats">{stats}</div>
      </header>
      <div className="runtime__grid">{children}</div>
      <footer className="runtime__footer">{footer}</footer>
    </section>
  );
}

function ClientRuntime({ xml }) {
  const sessionRef = useRef(null);
  const playbackRef = useRef(null);
  const [sessionSnapshot, setSessionSnapshot] = useState(null);
  const [playbackSnapshot, setPlaybackSnapshot] = useState(null);
  const [networkLog, setNetworkLog] = useState([]);
  const [error, setError] = useState(null);

  useEffect(() => {
    setError(null);
    setNetworkLog([]);

    const session = createVastSession({
      source: {
        kind: "xml",
        xml,
        label: "scratch-client.xml",
      },
      fetch: createMockFetch("vastlint-client", setNetworkLog),
    });

    const playback = createVastPlaybackController({
      session,
      autoResolve: false,
      mediaSelection,
    });

    sessionRef.current = session;
    playbackRef.current = playback;
    setSessionSnapshot(session.getSnapshot());
    setPlaybackSnapshot(playback.getSnapshot());

    const unsubscribeSession = session.subscribe((nextSnapshot) => {
      setSessionSnapshot(nextSnapshot);
    });
    const unsubscribePlayback = playback.subscribe((nextSnapshot) => {
      setPlaybackSnapshot(nextSnapshot);
    });

    return () => {
      sessionRef.current = null;
      playbackRef.current = null;
      unsubscribeSession();
      unsubscribePlayback();
      playback.dispose();
    };
  }, [xml]);

  const resolvedAd = sessionSnapshot?.resolvedAd ?? null;
  const selectedMedia = getSelectedMedia(resolvedAd);
  const startTargets = sessionSnapshot ? selectTrackingTargets(sessionSnapshot.tracking.plan, "start") : [];
  const companionTargets = resolvedAd && sessionRef.current
    ? sessionRef.current.getCompanionTrackingTargets(0, { companionId: "scratch-companion" }, "clickTracking")
    : [];

  async function run(action) {
    try {
      setError(null);
      await action();
    } catch (nextError) {
      setError(nextError instanceof Error ? nextError.message : String(nextError));
    }
  }

  async function advancePlayback() {
    const snapshot = playbackRef.current?.getSnapshot();
    if (!snapshot) {
      return;
    }

    const durationSec = snapshot.durationSec ?? 24;
    const nextTime = Math.min(snapshot.currentTimeSec + playbackAdvanceSeconds, durationSec);
    await playbackRef.current.updateProgress(nextTime, durationSec);
  }

  return (
    <RuntimeFrame
      eyebrow="Framework-agnostic runtime"
      title="vastlint-client"
      description="You create the session, own the subscriptions, and wire playback yourself. The pane exposes the direct runtime surface without any React-specific wrapper."
      stats={[
        <Stat key="session" label="session" value={sessionSnapshot?.status ?? "idle"} accent />,
        <Stat key="playback" label="playback" value={playbackSnapshot?.status ?? "idle"} />,
        <Stat key="history" label="history" value={sessionSnapshot?.tracking.history.length ?? 0} />,
        <Stat key="targets" label="start targets" value={startTargets.length} />,
      ]}
      footer={error ?? "Resolve the session first, then drive playback. autoResolve is disabled here so the dependency stays explicit."}
    >
      <Panel eyebrow="Imports" title="Direct APIs used here">
        <pre className="code-block code-block--compact">{`import {
  createVastSession,
  createVastPlaybackController,
  selectResolvedAdMediaFile,
  selectTrackingTargets,
} from "vastlint-client";`}</pre>
      </Panel>

      <Panel eyebrow="Session" title="Manual lifecycle control">
        <div className="button-grid button-grid--two">
          <button onClick={() => run(() => sessionRef.current?.validate())}>Validate</button>
          <button onClick={() => run(() => sessionRef.current?.resolve())}>Resolve</button>
          <button onClick={() => run(() => sessionRef.current?.track("impression"))}>Track impression</button>
          <button onClick={() => run(() => sessionRef.current?.track("start"))}>Track start</button>
        </div>
        <div className="detail-list">
          <div><span>ad title</span><strong>{resolvedAd?.adTitle ?? "—"}</strong></div>
          <div><span>duration</span><strong>{resolvedAd?.duration ?? "—"}</strong></div>
          <div><span>selected media</span><strong>{selectedMedia.selected?.url ?? "—"}</strong></div>
          <div><span>companion targets</span><strong>{companionTargets.length}</strong></div>
        </div>
        <pre className="code-block">{formatJson({
          validation: sessionSnapshot?.validation?.summary ?? null,
          resolution: sessionSnapshot?.resolution,
          recentEvents: sessionSnapshot?.events.slice(-6) ?? [],
        })}</pre>
      </Panel>

      <Panel eyebrow="Playback" title="Manual playback controller">
        <div className="button-grid">
          <button onClick={() => run(() => playbackRef.current?.initialize())}>Initialize</button>
          <button onClick={() => run(() => playbackRef.current?.start())}>Start</button>
          <button onClick={() => run(() => playbackRef.current?.pause())}>Pause</button>
          <button onClick={() => run(() => playbackRef.current?.resume())}>Resume</button>
          <button onClick={() => run(() => advancePlayback())}>Advance 6s</button>
          <button onClick={() => run(() => playbackRef.current?.click())}>Click</button>
          <button className="ghost" onClick={() => run(() => playbackRef.current?.complete())}>Complete</button>
        </div>
        <pre className="code-block">{formatJson({
          status: playbackSnapshot?.status ?? "idle",
          currentTimeSec: playbackSnapshot?.currentTimeSec ?? 0,
          durationSec: playbackSnapshot?.durationSec ?? null,
          clickThroughUrl: playbackSnapshot?.clickThroughUrl ?? null,
          milestones: playbackSnapshot?.milestones ?? null,
        })}</pre>
      </Panel>

      <Panel eyebrow="Inspectors" title="Pure selectors before dispatch" className="panel--span">
        <pre className="code-block">{formatJson({
          selectedMedia,
          startTargets,
          companionClickTargets: companionTargets,
          trackingHistory: sessionSnapshot?.tracking.history ?? [],
        })}</pre>
      </Panel>

      <Panel eyebrow="Mock network" title="Requests emitted by the runtime" className="panel--span">
        <NetworkLog entries={networkLog} />
      </Panel>
    </RuntimeFrame>
  );
}

function HooksRuntime({ xml }) {
  const [networkLog, setNetworkLog] = useState([]);
  const [error, setError] = useState(null);
  const fetchRef = useRef(null);

  if (fetchRef.current === null) {
    fetchRef.current = createMockFetch("vastlint-react", setNetworkLog);
  }

  const session = useVastSession({
    source: {
      kind: "xml",
      xml,
      label: "scratch-hooks.xml",
    },
    fetch: fetchRef.current,
    autoLoad: false,
    autoValidate: false,
  });
  const tracker = useVastTracker({ session: session.session });
  const playback = useVastPlayback({
    session: session.session,
    autoInitialize: false,
    mediaSelection,
  });

  const resolvedAd = session.snapshot.resolvedAd;
  const hookCompanionTargets = resolvedAd
    ? tracker.getCompanionTargets(0, { companionId: "scratch-companion" }, "clickTracking")
    : [];

  async function run(action) {
    try {
      setError(null);
      await action();
    } catch (nextError) {
      setError(nextError instanceof Error ? nextError.message : String(nextError));
    }
  }

  async function advancePlayback() {
    const durationSec = playback.snapshot.durationSec ?? 24;
    const nextTime = Math.min(playback.snapshot.currentTimeSec + playbackAdvanceSeconds, durationSec);
    await playback.updateProgress(nextTime, durationSec);
  }

  return (
    <RuntimeFrame
      eyebrow="React wrappers"
      title="vastlint-react"
      description="The hooks keep the same underlying runtime but package subscriptions and imperative methods into a React-friendly shape. This pane keeps the default auto-resolve path enabled."
      stats={[
        <Stat key="session" label="session" value={session.snapshot.status} accent />,
        <Stat key="playback" label="playback" value={playback.snapshot.status} />,
        <Stat key="history" label="history" value={tracker.tracking.history.length} />,
        <Stat key="events" label="events" value={tracker.availableEvents.length} />,
      ]}
      footer={error ?? "The hook results map straight to the same session and playback operations, but React owns the subscription plumbing and keeps playback on the default auto-resolve happy path."}
    >
      <Panel eyebrow="Imports" title="Hook APIs used here">
        <pre className="code-block code-block--compact">{`import {
  useVastSession,
  useVastTracker,
  useVastPlayback,
} from "vastlint-react";`}</pre>
      </Panel>

      <Panel eyebrow="Session hook" title="Session + tracker composition">
        <div className="button-grid button-grid--two">
          <button onClick={() => run(() => session.validate())}>Validate</button>
          <button onClick={() => run(() => session.resolve())}>Resolve</button>
          <button onClick={() => run(() => tracker.track("impression"))}>Track impression</button>
          <button onClick={() => run(() => tracker.track("start"))}>Track start</button>
        </div>
        <div className="detail-list">
          <div><span>ad title</span><strong>{resolvedAd?.adTitle ?? "—"}</strong></div>
          <div><span>duration</span><strong>{resolvedAd?.duration ?? "—"}</strong></div>
          <div><span>available events</span><strong>{tracker.availableEvents.slice(0, 5).join(", ") || "—"}</strong></div>
          <div><span>companion targets</span><strong>{hookCompanionTargets.length}</strong></div>
        </div>
        <pre className="code-block">{formatJson({
          validation: session.snapshot.validation?.summary ?? null,
          resolution: session.snapshot.resolution,
          recentHistory: tracker.tracking.history.slice(-6),
        })}</pre>
      </Panel>

      <Panel eyebrow="Playback hook" title="React-friendly playback methods">
        <div className="button-grid">
          <button onClick={() => run(() => playback.initialize())}>Initialize</button>
          <button onClick={() => run(() => playback.start())}>Start</button>
          <button onClick={() => run(() => playback.pause())}>Pause</button>
          <button onClick={() => run(() => playback.resume())}>Resume</button>
          <button onClick={() => run(() => advancePlayback())}>Advance 6s</button>
          <button onClick={() => run(() => playback.click())}>Click</button>
          <button className="ghost" onClick={() => run(() => playback.complete())}>Complete</button>
        </div>
        <pre className="code-block">{formatJson({
          status: playback.snapshot.status,
          currentTimeSec: playback.snapshot.currentTimeSec,
          durationSec: playback.snapshot.durationSec,
          clickThroughUrl: playback.snapshot.clickThroughUrl,
          milestones: playback.snapshot.milestones,
          mediaSelection: playback.snapshot.mediaSelection,
        })}</pre>
      </Panel>

      <Panel eyebrow="Hook selectors" title="Wrapped tracker helpers" className="panel--span">
        <pre className="code-block">{formatJson({
          startTargets: tracker.getTargets("start"),
          companionClickTargets: hookCompanionTargets,
          clickThroughUrl: tracker.clickThroughUrl,
          playbackSelection: playback.snapshot.mediaSelection,
        })}</pre>
      </Panel>

      <Panel eyebrow="Mock network" title="Requests emitted by the hooks" className="panel--span">
        <NetworkLog entries={networkLog} />
      </Panel>
    </RuntimeFrame>
  );
}

export default function App() {
  const [draftXml, setDraftXml] = useState(fixtureXml);
  const [appliedXml, setAppliedXml] = useState(fixtureXml);
  const [revision, setRevision] = useState(0);

  function applyXml() {
    setAppliedXml(draftXml);
    setRevision((current) => current + 1);
  }

  function resetFixture() {
    setDraftXml(fixtureXml);
    setAppliedXml(fixtureXml);
    setRevision((current) => current + 1);
  }

  return (
    <main className="shell">
      <section className="hero card">
        <div>
          <p className="eyebrow">vastlint client + react scratch</p>
          <h1>One XML editor, two runtimes: direct controllers on one side and React hooks on the other.</h1>
          <p className="lede">
            This scratch app keeps the XML source shared, then remounts a raw <code>vastlint-client</code> runtime and a
            <code> vastlint-react</code> runtime so you can compare how the APIs line up.
          </p>
        </div>
        <div className="hero__stats">
          <Stat label="shared source" value="1 XML fixture" accent />
          <Stat label="client layer" value="manual subscriptions" />
          <Stat label="react layer" value="hooks + snapshots" />
          <Stat label="playback mode" value="split resolve modes" />
        </div>
      </section>

      <section className="workspace">
        <section className="editor-column">
          <Panel eyebrow="Shared editor" title="Apply the same VAST to both runtimes" className="panel--sticky">
            <div className="button-grid button-grid--two">
              <button onClick={applyXml}>Apply XML to both panes</button>
              <button className="ghost" onClick={resetFixture}>Reset fixture</button>
            </div>
            <div className="detail-list detail-list--stacked">
              <div><span>vastlint-client</span><strong>Create the session and playback controller yourself.</strong></div>
              <div><span>vastlint-react</span><strong>Use hooks that subscribe to the same runtime primitives and keep playback on the default auto-resolve path.</strong></div>
              <div><span>comparison rule</span><strong>Both panes run against the same applied XML and mock tracking transport, but the raw client pane keeps resolve explicit.</strong></div>
            </div>
            <textarea
              aria-label="VAST XML editor"
              spellCheck="false"
              value={draftXml}
              onChange={(event) => setDraftXml(event.target.value)}
            />
          </Panel>
        </section>

        <section className="runtime-stack">
          <ClientRuntime key={`client-${revision}`} xml={appliedXml} />
          <HooksRuntime key={`hooks-${revision}`} xml={appliedXml} />
        </section>
      </section>
    </main>
  );
}