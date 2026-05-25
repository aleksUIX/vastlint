import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

import {
  createVastPlaybackController,
  createVastPlaybackQueueController,
  createVastSession,
} from "../dist/index.js";

const fixturesDir = path.resolve(
  import.meta.dirname,
  "..",
  "..",
  "..",
  "crates",
  "vastlint-core",
  "tests",
  "fixtures",
);

const metadataFixture = `<?xml version="1.0" encoding="UTF-8"?>
<VAST version="4.2">
  <Ad id="pod-1" sequence="2" adType="audio">
    <InLine>
      <AdSystem>Demo</AdSystem>
      <AdTitle>Metadata Fixture</AdTitle>
      <AdServingId>550e8400-e29b-41d4-a716-446655440000</AdServingId>
      <Category authority="https://www.iab.com/guidelines/taxonomy">IAB-1</Category>
      <Impression><![CDATA[https://metrics.example.com/impression]]></Impression>
      <Creatives>
        <Creative id="creative-1">
          <UniversalAdId idRegistry="ad-id.org">ABCD1234</UniversalAdId>
          <Linear>
            <Duration>00:00:15</Duration>
            <MediaFiles>
              <MediaFile delivery="progressive" type="audio/mpeg" width="0" height="0"><![CDATA[https://cdn.example.com/ad.mp3]]></MediaFile>
            </MediaFiles>
          </Linear>
        </Creative>
      </Creatives>
      <AdVerifications>
        <Verification vendor="company.com-omid">
          <JavaScriptResource apiFramework="omid"><![CDATA[https://verify.example.com/omid.js]]></JavaScriptResource>
          <ExecutableResource apiFramework="omid" type="application/x-omid-binary"><![CDATA[https://verify.example.com/omid.bin]]></ExecutableResource>
          <VerificationParameters><![CDATA[{"key":"value"}]]></VerificationParameters>
        </Verification>
      </AdVerifications>
    </InLine>
  </Ad>
</VAST>`;

const playbackFixture = `<?xml version="1.0" encoding="UTF-8"?>
<VAST version="4.2">
  <Ad id="playback-1">
    <InLine>
      <AdSystem>Playback Demo</AdSystem>
      <AdTitle>Playback Fixture</AdTitle>
      <Error><![CDATA[https://track.example.com/error?code=%%ERRORCODE%%]]></Error>
      <Impression><![CDATA[https://track.example.com/impression]]></Impression>
      <Creatives>
        <Creative>
          <Linear>
            <Duration>00:00:20</Duration>
            <TrackingEvents>
              <Tracking event="creativeView"><![CDATA[https://track.example.com/creative-view]]></Tracking>
              <Tracking event="start"><![CDATA[https://track.example.com/start]]></Tracking>
              <Tracking event="firstQuartile"><![CDATA[https://track.example.com/first-quartile]]></Tracking>
              <Tracking event="midpoint"><![CDATA[https://track.example.com/midpoint]]></Tracking>
              <Tracking event="thirdQuartile"><![CDATA[https://track.example.com/third-quartile]]></Tracking>
              <Tracking event="complete"><![CDATA[https://track.example.com/complete]]></Tracking>
              <Tracking event="pause"><![CDATA[https://track.example.com/pause]]></Tracking>
              <Tracking event="resume"><![CDATA[https://track.example.com/resume]]></Tracking>
              <Tracking event="mute"><![CDATA[https://track.example.com/mute]]></Tracking>
              <Tracking event="unmute"><![CDATA[https://track.example.com/unmute]]></Tracking>
              <Tracking event="fullscreen"><![CDATA[https://track.example.com/fullscreen]]></Tracking>
              <Tracking event="exitFullscreen"><![CDATA[https://track.example.com/exit-fullscreen]]></Tracking>
              <Tracking event="skip"><![CDATA[https://track.example.com/skip]]></Tracking>
            </TrackingEvents>
            <VideoClicks>
              <ClickThrough><![CDATA[https://click.example.com/landing]]></ClickThrough>
              <ClickTracking><![CDATA[https://track.example.com/click]]></ClickTracking>
            </VideoClicks>
            <MediaFiles>
              <MediaFile delivery="progressive" type="video/mp4" width="1280" height="720"><![CDATA[https://cdn.example.com/video.mp4]]></MediaFile>
            </MediaFiles>
          </Linear>
        </Creative>
      </Creatives>
      <ViewableImpression>
        <Viewable><![CDATA[https://track.example.com/viewable]]></Viewable>
      </ViewableImpression>
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
              <Tracking event="complete"><![CDATA[https://queue.example.com/alpha/complete]]></Tracking>
            </TrackingEvents>
            <VideoClicks>
              <ClickThrough><![CDATA[https://queue.example.com/alpha/clickthrough]]></ClickThrough>
              <ClickTracking><![CDATA[https://queue.example.com/alpha/click]]></ClickTracking>
            </VideoClicks>
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
      <Error><![CDATA[https://queue.example.com/beta/error?code=%%ERRORCODE%%]]></Error>
      <Creatives>
        <Creative>
          <Linear>
            <Duration>00:00:15</Duration>
            <TrackingEvents>
              <Tracking event="creativeView"><![CDATA[https://queue.example.com/beta/creative-view]]></Tracking>
              <Tracking event="start"><![CDATA[https://queue.example.com/beta/start]]></Tracking>
              <Tracking event="skip"><![CDATA[https://queue.example.com/beta/skip]]></Tracking>
            </TrackingEvents>
            <VideoClicks>
              <ClickThrough><![CDATA[https://queue.example.com/beta/clickthrough]]></ClickThrough>
              <ClickTracking><![CDATA[https://queue.example.com/beta/click]]></ClickTracking>
            </VideoClicks>
            <MediaFiles>
              <MediaFile delivery="progressive" type="video/mp4" width="1920" height="1080"><![CDATA[https://cdn.example.com/beta.mp4]]></MediaFile>
            </MediaFiles>
          </Linear>
        </Creative>
      </Creatives>
    </InLine>
  </Ad>
</VAST>`;

const companionFixture = `<?xml version="1.0" encoding="UTF-8"?>
<VAST version="4.2">
  <Ad id="companion-ad">
    <InLine>
      <AdSystem>Companion Demo</AdSystem>
      <AdTitle>Companion Fixture</AdTitle>
      <Creatives>
        <Creative>
          <CompanionAds>
            <Companion id="companion-slot" width="300" height="250" adSlotId="sidebar">
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

function readFixture(name) {
  return fs.readFileSync(path.join(fixturesDir, name), "utf8");
}

function createXmlResponse(xml) {
  return {
    ok: true,
    status: 200,
    statusText: "OK",
    async text() {
      return xml;
    },
  };
}

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

test("resolve follows a wrapper hop and exposes merged playback state", async () => {
  const fetchCalls = [];
  const fixtureMap = new Map([
    ["http://localhost:18765/valid_2.0.xml", readFixture("valid_2.0.xml")],
  ]);

  const session = createVastSession({
    source: { kind: "xml", xml: readFixture("wrapper_to_inline.xml") },
    fetch: async (url) => {
      fetchCalls.push(String(url));
      const xml = fixtureMap.get(String(url));
      if (!xml) {
        throw new Error(`unexpected fetch ${String(url)}`);
      }

      return createXmlResponse(xml);
    },
  });

  const snapshot = await session.resolve();
  assert.equal(fetchCalls.length, 1);
  assert.equal(fetchCalls[0], "http://localhost:18765/valid_2.0.xml");
  assert.equal(snapshot.wrapperChain.length, 2);
  assert.equal(snapshot.resolution?.resolved, true);
  assert.equal(snapshot.resolvedAd?.resolved, true);
  assert.equal(snapshot.resolvedAd?.wrapperHopCount, 1);
  assert.equal(snapshot.resolvedAd?.adTitle, "Test Ad");
  assert.deepEqual(snapshot.resolvedAd?.impressionUrls, [
    "https://example.com/wrapper_impression",
    "https://example.com/impression",
  ]);
  assert.equal(snapshot.resolvedAd?.mediaFiles.length, 1);
  assert.equal(snapshot.resolvedAd?.mediaFiles[0]?.url, "https://example.com/video.mp4");
});

test("resolvedAd extracts verification and universal ID metadata from core fixtures", async () => {
  const session = createVastSession({
    source: { kind: "xml", xml: readFixture("valid_4.3_with_verification.xml") },
  });

  const snapshot = await session.resolve();
  const resolvedAd = snapshot.resolvedAd;

  assert.ok(resolvedAd);
  assert.equal(resolvedAd.universalAdIds.length, 1);
  assert.deepEqual(resolvedAd.universalAdIds[0], {
    creativeId: null,
    creativeIndex: 0,
    idRegistry: "ad-id.org",
    idValue: null,
    value: "TEST-1234",
  });
  assert.equal(resolvedAd.adVerifications.length, 1);
  assert.equal(resolvedAd.adVerifications[0]?.vendor, "iabtechlab.com-omid");
  assert.equal(resolvedAd.adVerifications[0]?.resources.length, 1);
  assert.deepEqual(resolvedAd.adVerifications[0]?.resources[0], {
    kind: "javascript",
    url: "https://verification.example.com/omid.js",
    apiFramework: "omid",
    mimeType: null,
    browserOptional: "false",
  });
  assert.equal(resolvedAd.adPod.adServingId, "TEST-SERVING-ID-001");
});

test("resolvedAd metadata fixture exposes categories, ad verification resources, and ad-pod fields", async () => {
  const session = createVastSession({ source: { kind: "xml", xml: metadataFixture } });
  const snapshot = await session.resolve();
  const resolvedAd = snapshot.resolvedAd;

  assert.ok(resolvedAd);
  assert.equal(resolvedAd.categories.length, 1);
  assert.deepEqual(resolvedAd.categories[0], {
    authority: "https://www.iab.com/guidelines/taxonomy",
    value: "IAB-1",
  });
  assert.equal(resolvedAd.adVerifications.length, 1);
  assert.equal(resolvedAd.adVerifications[0]?.resources.length, 2);
  assert.deepEqual(resolvedAd.adPod, {
    adId: "pod-1",
    sequence: 2,
    adType: "audio",
    adServingId: "550e8400-e29b-41d4-a716-446655440000",
    isAdPod: true,
  });
});

test("resolvedAds keeps pod creatives separated by sequence", async () => {
  const session = createVastSession({
    source: { kind: "xml", xml: readFixture("warn_mixed_vendor_pod.xml") },
  });

  const snapshot = await session.resolve();

  assert.equal(snapshot.resolvedAds.length, 2);
  assert.equal(snapshot.resolvedAd?.adTitle, "CTV Pod Slot 1");
  assert.deepEqual(snapshot.resolvedAds.map((ad) => ad.adTitle), [
    "CTV Pod Slot 1",
    "CTV Pod Slot 2",
  ]);
  assert.deepEqual(snapshot.resolvedAds.map((ad) => ad.universalAdIds[0]?.value ?? null), [
    "ALPHA-1234",
    "BETA-5678",
  ]);
  assert.deepEqual(snapshot.resolvedAds.map((ad) => ad.adPod.adServingId), [
    "POD-SLOT-1",
    "POD-SLOT-2",
  ]);
  assert.deepEqual(snapshot.resolvedAds.map((ad) => ad.mediaFiles[0]?.url ?? null), [
    "https://cdn.alpha.example.com/pod-slot-1.mp4",
    "https://cdn.beta.example.com/pod-slot-2.mp4",
  ]);
  assert.deepEqual(snapshot.resolvedAds.map((ad) => ad.impressionUrls), [
    ["https://metrics.alpha.example.com/imp?slot=1"],
    ["http://metrics.beta.example.com/imp?slot=2"],
  ]);
  assert.deepEqual(snapshot.resolvedAds.map((ad) => [...new Set(ad.trackingPlan.impressions.map((target) => target.url))]), [
    ["https://metrics.alpha.example.com/imp?slot=1"],
    ["http://metrics.beta.example.com/imp?slot=2"],
  ]);
});

test("session exposes selector-scoped tracking targets and dispatch", async () => {
  const trackingCalls = [];

  const session = createVastSession({
    source: {
      kind: "xml",
      xml: `<?xml version="1.0" encoding="UTF-8"?>
<VAST version="4.2">
  <Ad id="ad-a" sequence="1">
    <InLine>
      <AdSystem>Scoped Alpha</AdSystem>
      <AdTitle>Scoped Alpha</AdTitle>
      <Impression><![CDATA[https://track.example.com/shared-impression]]></Impression>
      <Impression><![CDATA[https://track.example.com/alpha-impression]]></Impression>
      <Creatives>
        <Creative>
          <Linear>
            <Duration>00:00:10</Duration>
            <TrackingEvents>
              <Tracking event="start"><![CDATA[https://track.example.com/alpha-start]]></Tracking>
            </TrackingEvents>
            <MediaFiles>
              <MediaFile delivery="progressive" type="video/mp4" width="1280" height="720"><![CDATA[https://cdn.example.com/alpha.mp4]]></MediaFile>
            </MediaFiles>
          </Linear>
        </Creative>
      </Creatives>
    </InLine>
  </Ad>
  <Ad id="ad-b" sequence="2">
    <InLine>
      <AdSystem>Scoped Beta</AdSystem>
      <AdTitle>Scoped Beta</AdTitle>
      <Impression><![CDATA[https://track.example.com/shared-impression]]></Impression>
      <Impression><![CDATA[https://track.example.com/beta-impression]]></Impression>
      <Error><![CDATA[https://track.example.com/beta-error?code=%%ERRORCODE%%]]></Error>
      <Creatives>
        <Creative>
          <Linear>
            <Duration>00:00:12</Duration>
            <TrackingEvents>
              <Tracking event="start"><![CDATA[https://track.example.com/beta-start]]></Tracking>
            </TrackingEvents>
            <MediaFiles>
              <MediaFile delivery="progressive" type="video/mp4" width="1280" height="720"><![CDATA[https://cdn.example.com/beta.mp4]]></MediaFile>
            </MediaFiles>
          </Linear>
        </Creative>
      </Creatives>
    </InLine>
  </Ad>
</VAST>`,
    },
    fetch: async (url) => {
      trackingCalls.push(String(url));
      return createPixelResponse();
    },
  });

  await session.resolve();

  assert.deepEqual(
    session.getAdTrackingTargets({ adId: "ad-b" }, "impression").map((target) => target.url),
    [
      "https://track.example.com/shared-impression",
      "https://track.example.com/beta-impression",
    ],
  );
  assert.deepEqual(
    session.getAdTrackingTargets({ sequence: 2 }, "start").map((target) => target.url),
    ["https://track.example.com/beta-start"],
  );

  await session.trackAd({ sequence: 1 }, "impression");
  await session.trackAd({ adId: "ad-b" }, "impression");
  await session.trackAd({ adId: "ad-b" }, "start");
  await session.trackAd({ adIndex: 1 }, "error", { macros: { ERRORCODE: 901 } });

  assert.deepEqual(trackingCalls, [
    "https://track.example.com/shared-impression",
    "https://track.example.com/alpha-impression",
    "https://track.example.com/shared-impression",
    "https://track.example.com/beta-impression",
    "https://track.example.com/beta-start",
    "https://track.example.com/beta-error?code=901",
  ]);
});

test("session exposes companion helpers and companion-scoped tracking dispatch", async () => {
  const trackingCalls = [];

  const session = createVastSession({
    source: { kind: "xml", xml: companionFixture },
    fetch: async (url) => {
      trackingCalls.push(String(url));
      return createPixelResponse();
    },
  });

  const snapshot = await session.resolve();
  assert.equal(snapshot.resolvedAd?.companions.length, 1);
  assert.equal(snapshot.resolvedAd?.companions[0]?.clickThroughUrl, "https://click.example.com/companion");

  const companions = session.getAdCompanions({ adId: "companion-ad" });
  assert.equal(companions.length, 1);
  assert.equal(companions[0]?.adSlotId, "sidebar");

  const targets = session.getCompanionTrackingTargets(
    { adId: "companion-ad" },
    { companionId: "companion-slot" },
    "creativeView",
  );
  assert.deepEqual(targets, [
    {
      kind: "event",
      event: "creativeView",
      url: "https://track.example.com/companion/view",
      hopIndex: 0,
      sourceUrl: null,
      offset: null,
    },
  ]);

  const tracking = await session.trackCompanion(0, { adSlotId: "sidebar" }, "clickTracking");
  assert.equal(tracking.length, 1);
  assert.equal(tracking[0]?.url, "https://track.example.com/companion/click");
  assert.deepEqual(trackingCalls, ["https://track.example.com/companion/click"]);
});

test("playback controller selects media and dispatches lifecycle tracking", async () => {
  const trackingCalls = [];

  const session = createVastSession({
    source: { kind: "xml", xml: playbackFixture },
    fetch: async (url) => {
      trackingCalls.push(String(url));
      return createPixelResponse();
    },
  });

  const controller = createVastPlaybackController({
    session,
    mediaSelection: { supportedMimeTypes: ["video/mp4"] },
  });

  const readySnapshot = await controller.initialize();
  assert.equal(readySnapshot.status, "ready");
  assert.equal(readySnapshot.mediaSelection.selected?.url, "https://cdn.example.com/video.mp4");

  await controller.start();
  await controller.pause();
  await controller.resume();
  await controller.setMuted(true);
  await controller.setMuted(false);
  await controller.setFullscreen(true);
  await controller.setFullscreen(false);
  await controller.setViewability("viewable");
  await controller.updateProgress(5, 20);
  await controller.updateProgress(10, 20);
  await controller.updateProgress(15, 20);
  const clickResult = await controller.click();
  const completeSnapshot = await controller.complete();
  const errorSnapshot = await controller.signalError({ macros: { ERRORCODE: 901 } });

  assert.equal(clickResult.clickThroughUrl, "https://click.example.com/landing");
  assert.equal(completeSnapshot.status, "ended");
  assert.equal(errorSnapshot.status, "error");

  assert.deepEqual(trackingCalls, [
    "https://track.example.com/impression",
    "https://track.example.com/creative-view",
    "https://track.example.com/start",
    "https://track.example.com/pause",
    "https://track.example.com/resume",
    "https://track.example.com/mute",
    "https://track.example.com/unmute",
    "https://track.example.com/fullscreen",
    "https://track.example.com/exit-fullscreen",
    "https://track.example.com/viewable",
    "https://track.example.com/first-quartile",
    "https://track.example.com/midpoint",
    "https://track.example.com/third-quartile",
    "https://track.example.com/click",
    "https://track.example.com/complete",
    "https://track.example.com/error?code=901",
  ]);

  controller.dispose();
});

test("playback queue controller advances through pod ads with per-ad tracking", async () => {
  const trackingCalls = [];

  const session = createVastSession({
    source: { kind: "xml", xml: podPlaybackFixture },
  });

  const queue = createVastPlaybackQueueController({
    session,
    mediaSelection: { supportedMimeTypes: ["video/mp4"] },
    fetch: async (url) => {
      trackingCalls.push(String(url));
      return createPixelResponse();
    },
  });

  const readySnapshot = await queue.initialize();
  assert.equal(readySnapshot.currentAdIndex, 0);
  assert.equal(readySnapshot.items.length, 2);
  assert.equal(readySnapshot.currentItem?.resolvedAd.adTitle, "Queue Alpha");

  const firstStarted = await queue.start();
  assert.equal(firstStarted.currentItem?.status, "playing");

  const firstComplete = await queue.completeCurrent();
  assert.equal(firstComplete.currentItem?.status, "ended");

  const secondReady = await queue.next();
  assert.equal(secondReady.currentAdIndex, 1);
  assert.equal(secondReady.currentItem?.resolvedAd.adTitle, "Queue Beta");

  await queue.start();
  const clickResult = await queue.click();
  assert.equal(clickResult.clickThroughUrl, "https://queue.example.com/beta/clickthrough");

  const skipped = await queue.skip();
  assert.equal(skipped.currentItem?.status, "ended");
  assert.equal(skipped.currentItem?.skipped, true);

  const errorSnapshot = await queue.signalError({ macros: { ERRORCODE: 901 } });
  assert.equal(errorSnapshot.currentItem?.status, "error");

  assert.deepEqual(trackingCalls, [
    "https://queue.example.com/alpha/impression",
    "https://queue.example.com/alpha/creative-view",
    "https://queue.example.com/alpha/start",
    "https://queue.example.com/alpha/complete",
    "https://queue.example.com/beta/impression",
    "https://queue.example.com/beta/creative-view",
    "https://queue.example.com/beta/start",
    "https://queue.example.com/beta/click",
    "https://queue.example.com/beta/skip",
    "https://queue.example.com/beta/error?code=901",
  ]);

  queue.dispose();
});