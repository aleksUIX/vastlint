import assert from "node:assert/strict";
import test from "node:test";

import React, { act } from "react";
import { JSDOM } from "jsdom";
import { createRoot } from "react-dom/client";

import { createVastSession } from "vastlint-client";

import { useVastPlayback, useVastPlaybackQueue } from "../dist/index.js";

globalThis.IS_REACT_ACT_ENVIRONMENT = true;

function defineGlobal(name, value) {
  Object.defineProperty(globalThis, name, {
    configurable: true,
    writable: true,
    value,
  });
}

function installDomGlobals(window) {
  defineGlobal("window", window);
  defineGlobal("document", window.document);
  defineGlobal("navigator", window.navigator);
  defineGlobal("HTMLElement", window.HTMLElement);
  defineGlobal("Node", window.Node);
  defineGlobal("Event", window.Event);
  defineGlobal("CustomEvent", window.CustomEvent);
  defineGlobal("requestAnimationFrame", (callback) => setTimeout(() => callback(Date.now()), 0));
  defineGlobal("cancelAnimationFrame", (id) => clearTimeout(id));
}

const playbackFixture = `<?xml version="1.0" encoding="UTF-8"?>
<VAST version="4.2">
  <Ad id="playback-1">
    <InLine>
      <AdSystem>Playback Demo</AdSystem>
      <AdTitle>Playback Fixture</AdTitle>
      <Impression><![CDATA[https://track.example.com/impression]]></Impression>
      <Creatives>
        <Creative>
          <Linear>
            <Duration>00:00:20</Duration>
            <TrackingEvents>
              <Tracking event="creativeView"><![CDATA[https://track.example.com/creative-view]]></Tracking>
              <Tracking event="start"><![CDATA[https://track.example.com/start]]></Tracking>
            </TrackingEvents>
            <MediaFiles>
              <MediaFile delivery="progressive" type="video/mp4" width="1280" height="720"><![CDATA[https://cdn.example.com/video.mp4]]></MediaFile>
            </MediaFiles>
          </Linear>
        </Creative>
      </Creatives>
    </InLine>
  </Ad>
</VAST>`;

const podPlaybackFixture = `<?xml version="1.0" encoding="UTF-8"?>
<VAST version="4.2">
  <Ad id="pod-a" sequence="1">
    <InLine>
      <AdSystem>Queue Alpha</AdSystem>
      <AdTitle>Queue Alpha</AdTitle>
      <Impression><![CDATA[https://queue.example.com/alpha/impression]]></Impression>
      <Creatives>
        <Creative>
          <Linear>
            <Duration>00:00:10</Duration>
            <TrackingEvents>
              <Tracking event="creativeView"><![CDATA[https://queue.example.com/alpha/creative-view]]></Tracking>
              <Tracking event="start"><![CDATA[https://queue.example.com/alpha/start]]></Tracking>
            </TrackingEvents>
            <MediaFiles>
              <MediaFile delivery="progressive" type="video/mp4" width="1280" height="720"><![CDATA[https://cdn.example.com/alpha.mp4]]></MediaFile>
            </MediaFiles>
          </Linear>
        </Creative>
      </Creatives>
    </InLine>
  </Ad>
  <Ad id="pod-b" sequence="2">
    <InLine>
      <AdSystem>Queue Beta</AdSystem>
      <AdTitle>Queue Beta</AdTitle>
      <Impression><![CDATA[https://queue.example.com/beta/impression]]></Impression>
      <Creatives>
        <Creative>
          <Linear>
            <Duration>00:00:15</Duration>
            <TrackingEvents>
              <Tracking event="creativeView"><![CDATA[https://queue.example.com/beta/creative-view]]></Tracking>
              <Tracking event="start"><![CDATA[https://queue.example.com/beta/start]]></Tracking>
            </TrackingEvents>
            <MediaFiles>
              <MediaFile delivery="progressive" type="video/mp4" width="1920" height="1080"><![CDATA[https://cdn.example.com/beta.mp4]]></MediaFile>
            </MediaFiles>
          </Linear>
        </Creative>
      </Creatives>
    </InLine>
  </Ad>
</VAST>`;

function createPixelResponse() {
  return {
    ok: true,
    status: 204,
    statusText: "No Content",
    async text() {
      return "";
    },
  };
}

async function mountHook(useHook) {
  let latestResult = null;
  const dom = new JSDOM("<!doctype html><html><body></body></html>");
  installDomGlobals(dom.window);

  const container = dom.window.document.createElement("div");
  dom.window.document.body.append(container);
  const root = createRoot(container);

  function Probe() {
    latestResult = useHook();
    return null;
  }

  await act(async () => {
    root.render(React.createElement(Probe));
  });

  return {
    getLatest() {
      assert.ok(latestResult);
      return latestResult;
    },
    async unmount() {
      await act(async () => {
        root.unmount();
      });

      dom.window.close();
    },
  };
}

test("useVastPlayback initializes and updates playback snapshot state", async () => {
  const trackingCalls = [];
  const session = createVastSession({
    source: { kind: "xml", xml: playbackFixture },
    fetch: async (url) => {
      trackingCalls.push(String(url));
      return createPixelResponse();
    },
  });
  const mediaSelection = { supportedMimeTypes: ["video/mp4"] };
  const options = {
    session,
    autoInitialize: false,
    mediaSelection,
  };

  const mounted = await mountHook(() => useVastPlayback(options));

  assert.equal(mounted.getLatest().snapshot.status, "idle");

  await act(async () => {
    await mounted.getLatest().initialize();
  });

  assert.equal(mounted.getLatest().snapshot.status, "ready");
  assert.equal(mounted.getLatest().snapshot.resolvedAd?.adTitle, "Playback Fixture");
  assert.equal(mounted.getLatest().snapshot.mediaSelection.selected?.url, "https://cdn.example.com/video.mp4");

  await act(async () => {
    await mounted.getLatest().start();
  });

  assert.equal(mounted.getLatest().snapshot.status, "playing");
  assert.deepEqual(trackingCalls, [
    "https://track.example.com/impression",
    "https://track.example.com/creative-view",
    "https://track.example.com/start",
  ]);

  const playbackController = mounted.getLatest().controller;
  let playbackDisposeCalls = 0;
  const disposePlaybackController = playbackController.dispose.bind(playbackController);
  playbackController.dispose = () => {
    playbackDisposeCalls += 1;
    disposePlaybackController();
  };

  await mounted.unmount();
  assert.equal(playbackDisposeCalls, 1);
});

test("useVastPlaybackQueue initializes and updates pod playback snapshot state", async () => {
  const trackingCalls = [];
  const session = createVastSession({
    source: { kind: "xml", xml: podPlaybackFixture },
  });
  const mediaSelection = { supportedMimeTypes: ["video/mp4"] };
  const queueFetch = async (url) => {
    trackingCalls.push(String(url));
    return createPixelResponse();
  };
  const options = {
    session,
    autoInitialize: false,
    mediaSelection,
    fetch: queueFetch,
  };

  const mounted = await mountHook(() => useVastPlaybackQueue(options));

  assert.equal(mounted.getLatest().snapshot.status, "idle");

  await act(async () => {
    await mounted.getLatest().initialize();
  });

  assert.equal(mounted.getLatest().snapshot.status, "ready");
  assert.equal(mounted.getLatest().snapshot.items.length, 2);
  assert.equal(mounted.getLatest().snapshot.currentAdIndex, 0);
  assert.equal(mounted.getLatest().snapshot.currentItem?.resolvedAd.adTitle, "Queue Alpha");

  await act(async () => {
    await mounted.getLatest().start();
  });

  assert.equal(mounted.getLatest().snapshot.currentItem?.status, "playing");
  assert.deepEqual(trackingCalls, [
    "https://queue.example.com/alpha/impression",
    "https://queue.example.com/alpha/creative-view",
    "https://queue.example.com/alpha/start",
  ]);

  const playbackQueueController = mounted.getLatest().controller;
  let playbackQueueDisposeCalls = 0;
  const disposePlaybackQueueController = playbackQueueController.dispose.bind(playbackQueueController);
  playbackQueueController.dispose = () => {
    playbackQueueDisposeCalls += 1;
    disposePlaybackQueueController();
  };

  await mounted.unmount();
  assert.equal(playbackQueueDisposeCalls, 1);
});