import { fix, fixWithOptions, inspectDocument, validate, validateWithOptions } from "vastlint";

import { buildResolvedState } from "./resolved.js";
import { buildTrackingPlan, createEmptyTrackingPlan, expandTrackingUrl, selectTrackingTargets } from "./tracking.js";

import type {
  VastAdSelector,
  VastSession,
  VastSessionEvent,
  VastTrackOptions,
  VastTrackableEvent,
  VastTrackingPlan,
  VastTrackingTarget,
  VastTrackingDispatchResult,
  VastTrackingState,
  VastSessionOptions,
  VastResolvedAd,
  VastResolutionSummary,
  VastSessionSnapshot,
  VastWrapperHop,
} from "./types.js";

type BaseSessionSnapshot = Omit<VastSessionSnapshot, "tracking" | "resolvedAd" | "resolvedAds">;

function createTimestamp(): string {
  return new Date().toISOString();
}

function defaultMaxWrapperDepth(options: VastSessionOptions): number {
  const configuredDepth = options.maxWrapperDepth ?? options.validateOptions?.max_wrapper_depth ?? 5;
  return configuredDepth > 0 ? configuredDepth : 5;
}

function resolveWrapperUri(wrapperUri: string, currentUrl: string | null): string {
  if (currentUrl) {
    return new URL(wrapperUri, currentUrl).toString();
  }

  return new URL(wrapperUri).toString();
}

function buildResolutionSummary(wrapperChain: VastWrapperHop[], stoppedReason: string | null): VastResolutionSummary {
  const totals = wrapperChain.reduce(
    (summary, hop) => {
      if (!hop.validation) {
        return summary;
      }

      return {
        totalErrors: summary.totalErrors + hop.validation.summary.errors,
        totalWarnings: summary.totalWarnings + hop.validation.summary.warnings,
        totalInfos: summary.totalInfos + hop.validation.summary.infos,
      };
    },
    {
      totalErrors: 0,
      totalWarnings: 0,
      totalInfos: 0,
    },
  );

  const lastHop = wrapperChain[wrapperChain.length - 1] ?? null;
  const resolved = lastHop?.adType === "InLine";

  return {
    hopCount: wrapperChain.length,
    resolved,
    chainValid: totals.totalErrors === 0,
    totalErrors: totals.totalErrors,
    totalWarnings: totals.totalWarnings,
    totalInfos: totals.totalInfos,
    stoppedReason,
  };
}

function cloneTrackingState(tracking: VastTrackingState): VastTrackingState {
  return {
    plan: {
      impressions: tracking.plan.impressions.map((target) => ({ ...target })),
      errors: tracking.plan.errors.map((target) => ({ ...target })),
      clickTrackings: tracking.plan.clickTrackings.map((target) => ({ ...target })),
      clickThroughs: tracking.plan.clickThroughs.map((target) => ({ ...target })),
      events: tracking.plan.events.map((target) => ({ ...target })),
    },
    history: tracking.history.map((entry) => ({ ...entry })),
  };
}

function cloneTrackingPlan(plan: VastTrackingPlan): VastTrackingPlan {
  return {
    impressions: plan.impressions.map((target) => ({ ...target })),
    errors: plan.errors.map((target) => ({ ...target })),
    clickTrackings: plan.clickTrackings.map((target) => ({ ...target })),
    clickThroughs: plan.clickThroughs.map((target) => ({ ...target })),
    events: plan.events.map((target) => ({ ...target })),
  };
}

function cloneResolvedAd(resolvedAd: VastResolvedAd | null): VastResolvedAd | null {
  if (!resolvedAd) {
    return null;
  }

  return {
    ...resolvedAd,
    mediaFiles: resolvedAd.mediaFiles.map((mediaFile) => ({ ...mediaFile })),
    companions: resolvedAd.companions.map((companion) => ({
      ...companion,
      resources: companion.resources.map((resource) => ({ ...resource })),
      clickTrackingUrls: [...companion.clickTrackingUrls],
      trackingEvents: Object.fromEntries(
        Object.entries(companion.trackingEvents).map(([event, urls]) => [event, [...urls]]),
      ),
    })),
    icons: resolvedAd.icons.map((icon) => ({
      ...icon,
      resources: icon.resources.map((resource) => ({ ...resource })),
      clickTrackingUrls: [...icon.clickTrackingUrls],
      viewTrackingUrls: [...icon.viewTrackingUrls],
    })),
    universalAdIds: resolvedAd.universalAdIds.map((universalAdId) => ({ ...universalAdId })),
    categories: resolvedAd.categories.map((category) => ({ ...category })),
    adVerifications: resolvedAd.adVerifications.map((verification) => ({
      ...verification,
      resources: verification.resources.map((resource) => ({ ...resource })),
    })),
    adPod: { ...resolvedAd.adPod },
    impressionUrls: [...resolvedAd.impressionUrls],
    errorUrls: [...resolvedAd.errorUrls],
    clickTrackingUrls: [...resolvedAd.clickTrackingUrls],
    clickThroughUrls: [...resolvedAd.clickThroughUrls],
    trackingPlan: cloneTrackingPlan(resolvedAd.trackingPlan),
    trackingEvents: Object.fromEntries(
      Object.entries(resolvedAd.trackingEvents).map(([event, urls]) => [event, [...urls]]),
    ),
  };
}

function cloneResolvedAds(resolvedAds: readonly VastResolvedAd[]): VastResolvedAd[] {
  return resolvedAds.map((resolvedAd) => cloneResolvedAd(resolvedAd)).filter((resolvedAd): resolvedAd is VastResolvedAd => resolvedAd !== null);
}

function cloneSnapshot(snapshot: VastSessionSnapshot): VastSessionSnapshot {
  return {
    ...snapshot,
    events: [...snapshot.events],
    tracking: cloneTrackingState(snapshot.tracking),
    resolvedAd: cloneResolvedAd(snapshot.resolvedAd),
    resolvedAds: cloneResolvedAds(snapshot.resolvedAds),
    wrapperChain: snapshot.wrapperChain.map((hop) => ({
      ...hop,
      mediaFiles: hop.mediaFiles.map((mediaFile) => ({ ...mediaFile })),
    })),
  };
}

function toError(value: unknown): Error {
  if (value instanceof Error) {
    return value;
  }

  return new Error(typeof value === "string" ? value : "Unknown vastlint-client error.");
}

export function createVastSession(options: VastSessionOptions): VastSession {
  const listeners = new Set<(snapshot: VastSessionSnapshot) => void>();
  const fetchRequest = options.source.kind === "url" ? options.source.request : undefined;
  const trackingHistory: VastTrackingDispatchResult[] = [];
  const dispatchedTrackingKeys = new Set<string>();

  const stripTracking = (current: BaseSessionSnapshot | VastSessionSnapshot): BaseSessionSnapshot => {
    if ("tracking" in current) {
      const { tracking: _tracking, resolvedAd: _resolvedAd, resolvedAds: _resolvedAds, ...base } = current;
      return base;
    }

    return current;
  };

  const buildTrackingState = (current: BaseSessionSnapshot): VastTrackingState => {
    const trackingHops = current.wrapperChain.length
      ? current.wrapperChain.map((hop) => ({
          index: hop.index,
          url: hop.url,
          xml: hop.xml,
        }))
      : current.xml
        ? [
            {
              index: 0,
              url: current.source.kind === "url" ? current.source.url : null,
              xml: current.xml,
            },
          ]
        : [];

    return {
      plan: trackingHops.length ? buildTrackingPlan(trackingHops) : createEmptyTrackingPlan(),
      history: trackingHistory.map((entry) => ({ ...entry })),
    };
  };

  const withTracking = (next: BaseSessionSnapshot | VastSessionSnapshot): VastSessionSnapshot => {
    const base = stripTracking(next);
    const tracking = buildTrackingState(base);
    const resolvedState = buildResolvedState(base.wrapperChain, base.resolution);
    return {
      ...base,
      tracking,
      resolvedAd: resolvedState.resolvedAd,
      resolvedAds: resolvedState.resolvedAds,
    };
  };

  let snapshot: VastSessionSnapshot = withTracking({
    status: "idle",
    source: options.source,
    xml: options.source.kind === "xml" ? options.source.xml : null,
    rootXml: options.source.kind === "xml" ? options.source.xml : null,
    validation: null,
    fixed: null,
    wrapperChain: [],
    resolution: null,
    resolvedAds: [],
    events: [],
    error: null,
  });

  const notify = () => {
    const current = cloneSnapshot(snapshot);
    for (const listener of listeners) {
      listener(current);
    }
  };

  const setSnapshot = (next: VastSessionSnapshot) => {
    snapshot = withTracking(next);
    notify();
  };

  const emit = (type: VastSessionEvent["type"], detail?: Record<string, unknown>) => {
    const event: VastSessionEvent = detail
      ? { type, timestamp: createTimestamp(), detail }
      : { type, timestamp: createTimestamp() };

    snapshot = withTracking({
      ...snapshot,
      events: [...snapshot.events, event],
    });
    notify();
  };

  const setError = (error: unknown) => {
    const nextError = toError(error);
    snapshot = withTracking({
      ...snapshot,
      status: "error",
      error: nextError,
    });
    emit("session:error", { message: nextError.message });
  };

  const buildTrackingDispatchKey = (event: string, url: string, hopIndex: number, offset: string | null) =>
    `${event}:${hopIndex}:${offset ?? ""}:${url}`;

  const getScopedDispatchKey = (
    scope: string | null,
    event: string,
    url: string,
    hopIndex: number,
    offset: string | null,
  ) => {
    const base = buildTrackingDispatchKey(event, url, hopIndex, offset);
    return scope ? `${scope}:${base}` : base;
  };

  const dispatchTrackingTargets = async (
    event: VastTrackableEvent,
    availableTargets: readonly VastTrackingTarget[],
    trackOptions: VastTrackOptions,
    scope: string | null,
  ): Promise<VastTrackingDispatchResult[]> => {
    const dedupe = trackOptions.dedupe ?? true;
    const targets = dedupe
      ? availableTargets.filter(
          (target) => !dispatchedTrackingKeys.has(
            getScopedDispatchKey(scope, event, target.url, target.hopIndex, target.offset),
          ),
        )
      : [...availableTargets];

    if (targets.length === 0) {
      emit("track:completed", {
        event,
        adIndex: scope?.startsWith("ad:") ? Number.parseInt(scope.slice(3), 10) : null,
        requested: availableTargets.length,
        dispatched: 0,
        succeeded: 0,
        failed: 0,
      });
      return [];
    }

    const fetchImpl = options.fetch ?? globalThis.fetch;
    if (typeof fetchImpl !== "function") {
      throw new Error("No fetch implementation is available for VAST tracking dispatch.");
    }

    const results = await Promise.all(
      targets.map(async (target) => {
        const resolvedUrl = expandTrackingUrl(target.url, trackOptions.macros);

        try {
          const response = await fetchImpl(resolvedUrl, { method: "GET" });
          return {
            event,
            url: target.url,
            resolvedUrl,
            hopIndex: target.hopIndex,
            sourceUrl: target.sourceUrl,
            offset: target.offset,
            ok: response.ok,
            status: response.status,
            dispatchedAt: createTimestamp(),
            error: null,
          } satisfies VastTrackingDispatchResult;
        } catch (dispatchError) {
          return {
            event,
            url: target.url,
            resolvedUrl,
            hopIndex: target.hopIndex,
            sourceUrl: target.sourceUrl,
            offset: target.offset,
            ok: false,
            status: null,
            dispatchedAt: createTimestamp(),
            error: toError(dispatchError).message,
          } satisfies VastTrackingDispatchResult;
        }
      }),
    );

    for (const target of targets) {
      dispatchedTrackingKeys.add(getScopedDispatchKey(scope, event, target.url, target.hopIndex, target.offset));
    }

    trackingHistory.push(...results);
    setSnapshot(snapshot);

    const succeeded = results.filter((result) => result.ok).length;
    emit("track:completed", {
      event,
      adIndex: scope?.startsWith("ad:") ? Number.parseInt(scope.slice(3), 10) : null,
      requested: availableTargets.length,
      dispatched: results.length,
      succeeded,
      failed: results.length - succeeded,
    });

    return results;
  };

  const getResolvedAdAtIndex = (adIndex: number): { adIndex: number; resolvedAd: VastResolvedAd } => {
    if (!Number.isInteger(adIndex) || adIndex < 0) {
      throw new Error(`Ad index must be a non-negative integer, got ${String(adIndex)}.`);
    }

    const resolvedAd = snapshot.resolvedAds[adIndex] ?? null;
    if (!resolvedAd) {
      throw new Error(`Resolved ad at index ${String(adIndex)} is unavailable. Call resolve() first and use a valid ad index.`);
    }

    return {
      adIndex,
      resolvedAd,
    };
  };

  const findResolvedAd = (
    predicate: (resolvedAd: VastResolvedAd) => boolean,
    description: string,
  ): { adIndex: number; resolvedAd: VastResolvedAd } => {
    let match: { adIndex: number; resolvedAd: VastResolvedAd } | null = null;

    for (const [adIndex, resolvedAd] of snapshot.resolvedAds.entries()) {
      if (!predicate(resolvedAd)) {
        continue;
      }

      if (match) {
        throw new Error(`Resolved ad selector ${description} matched multiple ads. Use adIndex to disambiguate.`);
      }

      match = {
        adIndex,
        resolvedAd,
      };
    }

    if (!match) {
      throw new Error(`Resolved ad for ${description} is unavailable. Call resolve() first and use a valid selector.`);
    }

    return match;
  };

  const describeAdSelector = (adSelector: VastAdSelector): string => {
    if (typeof adSelector === "number") {
      return `adIndex ${String(adSelector)}`;
    }

    if ("adIndex" in adSelector) {
      return `adIndex ${String(adSelector.adIndex)}`;
    }

    if ("adId" in adSelector) {
      return `adId '${adSelector.adId}'`;
    }

    return `sequence ${String(adSelector.sequence)}`;
  };

  const buildTrackAdStartedDetail = (
    adSelector: VastAdSelector,
    event: VastTrackableEvent,
    offset: string | null,
  ): Record<string, unknown> => {
    const detail: Record<string, unknown> = {
      event,
      offset,
    };

    if (typeof adSelector === "number") {
      detail.adIndex = adSelector;
      return detail;
    }

    if ("adIndex" in adSelector) {
      detail.adIndex = adSelector.adIndex;
      return detail;
    }

    if ("adId" in adSelector) {
      detail.adId = adSelector.adId;
      return detail;
    }

    detail.sequence = adSelector.sequence;
    return detail;
  };

  const getResolvedAdSelection = (adSelector: VastAdSelector): { adIndex: number; resolvedAd: VastResolvedAd } => {
    if (typeof adSelector === "number") {
      return getResolvedAdAtIndex(adSelector);
    }

    if ("adIndex" in adSelector) {
      return getResolvedAdAtIndex(adSelector.adIndex);
    }

    if ("adId" in adSelector) {
      const adId = adSelector.adId.trim();
      if (!adId) {
        throw new Error("Ad selector adId must be a non-empty string.");
      }

      return findResolvedAd((resolvedAd) => resolvedAd.adPod.adId === adId, describeAdSelector({ adId }));
    }

    const sequence = adSelector.sequence;
    if (!Number.isInteger(sequence) || sequence < 1) {
      throw new Error(`Ad selector sequence must be a positive integer, got ${String(sequence)}.`);
    }

    return findResolvedAd((resolvedAd) => resolvedAd.adPod.sequence === sequence, describeAdSelector({ sequence }));
  };

  const buildHop = (
    xml: string,
    index: number,
    source: VastWrapperHop["source"],
    url: string | null,
    fetchMs: number,
    validation: VastWrapperHop["validation"],
  ): VastWrapperHop => {
    const meta = inspectDocument(xml);

    return {
      index,
      source,
      url,
      xml,
      fetchedAt: createTimestamp(),
      fetchMs,
      adType: meta.adType,
      adSystem: meta.adSystem,
      adTitle: meta.adTitle,
      duration: meta.duration,
      impressionCount: meta.impressionCount,
      trackingEventCount: meta.trackingEventCount,
      companionCount: meta.companionCount,
      mediaFiles: meta.mediaFiles,
      wrapperUri: meta.wrapperUri,
      validation,
    };
  };

  const validateXmlAtDepth = (xml: string, wrapperDepth: number) => {
    const maxWrapperDepth = defaultMaxWrapperDepth(options);
    const validateOptions = {
      ...options.validateOptions,
      wrapper_depth: wrapperDepth,
      max_wrapper_depth: maxWrapperDepth,
    };

    if (!options.validateOptions && wrapperDepth === 0 && maxWrapperDepth === 5) {
      return validate(xml);
    }

    return validateWithOptions(xml, validateOptions);
  };

  const fetchXmlFromUrl = async (url: string): Promise<{ xml: string; fetchMs: number }> => {
    const fetchImpl = options.fetch ?? globalThis.fetch;
    if (typeof fetchImpl !== "function") {
      throw new Error("No fetch implementation is available for URL-backed VAST sessions.");
    }

    const controller = typeof AbortController === "function" && options.timeoutMs ? new AbortController() : null;
    const timeoutId = controller
      ? setTimeout(() => controller.abort(new Error(`Timed out fetching VAST URL after ${options.timeoutMs} ms`)), options.timeoutMs)
      : null;

    const startedAt = Date.now();

    try {
      const resolvedSignal = controller?.signal ?? fetchRequest?.signal ?? null;
      const requestInit: RequestInit = resolvedSignal
        ? {
            ...(fetchRequest ?? {}),
            signal: resolvedSignal,
          }
        : { ...(fetchRequest ?? {}) };

      const response = await fetchImpl(url, requestInit);
      if (!response.ok) {
        throw new Error(`Failed to load VAST URL: ${response.status} ${response.statusText}`);
      }

      return {
        xml: await response.text(),
        fetchMs: Date.now() - startedAt,
      };
    } finally {
      if (timeoutId) {
        clearTimeout(timeoutId);
      }
    }
  };

  const loadXml = async (): Promise<string> => {
    if (snapshot.xml) {
      return snapshot.xml;
    }

    emit("source:loading", {
      sourceKind: snapshot.source.kind,
      label: snapshot.source.label ?? null,
    });

    snapshot = {
      ...snapshot,
      status: "loading",
      error: null,
    };
    notify();

    try {
      let xml = "";
      let fetchMs = 0;

      if (snapshot.source.kind === "xml") {
        xml = snapshot.source.xml;
      } else {
        const fetched = await fetchXmlFromUrl(snapshot.source.url);
        xml = fetched.xml;
        fetchMs = fetched.fetchMs;
      }

      const nextSnapshot: VastSessionSnapshot = {
        ...snapshot,
        status: "ready",
        xml,
        rootXml: xml,
        wrapperChain: [
          buildHop(
            xml,
            0,
            snapshot.source,
            snapshot.source.kind === "url" ? snapshot.source.url : null,
            fetchMs,
            null,
          ),
        ],
        resolution: null,
      };

      setSnapshot(nextSnapshot);
      emit("source:loaded", { bytes: xml.length, hops: nextSnapshot.wrapperChain.length });

      return xml;
    } catch (error) {
      setError(error);
      throw toError(error);
    }
  };

  const runValidation = async () => {
    emit("validate:started", {
      hasOptions: Boolean(options.validateOptions),
    });

    snapshot = {
      ...snapshot,
      status: "validating",
      error: null,
    };
    notify();

    try {
      const xml = await loadXml();
      const result = validateXmlAtDepth(xml, 0);

      const wrapperChain = snapshot.wrapperChain.length
        ? snapshot.wrapperChain.map((hop, index) => (index === 0 ? { ...hop, validation: result } : hop))
        : [
            {
              ...buildHop(xml, 0, snapshot.source, snapshot.source.kind === "url" ? snapshot.source.url : null, 0, null),
              validation: result,
            },
          ];

      setSnapshot({
        ...snapshot,
        status: "ready",
        validation: result,
        rootXml: snapshot.rootXml ?? xml,
        wrapperChain,
      });
      emit("validate:completed", {
        errors: result.summary.errors,
        warnings: result.summary.warnings,
        infos: result.summary.infos,
      });

      return result;
    } catch (error) {
      setError(error);
      throw toError(error);
    }
  };

  emit("session:created", {
    sourceKind: snapshot.source.kind,
    timeoutMs: options.timeoutMs ?? null,
    maxWrapperDepth: options.maxWrapperDepth ?? null,
  });

  const resolveSession = async () => {
    emit("resolve:started", {
      strategy: "wrapper-chain",
      maxWrapperDepth: defaultMaxWrapperDepth(options),
    });

    snapshot = {
      ...snapshot,
      status: "loading",
      error: null,
    };
    notify();

    try {
      const rootXml = await loadXml();
      const rootUrl = snapshot.source.kind === "url" ? snapshot.source.url : null;
      const wrapperChain: VastWrapperHop[] = [];
      const seenUrls = new Set<string>(rootUrl ? [rootUrl] : []);
      const maxWrapperDepth = defaultMaxWrapperDepth(options);

      let currentXml = rootXml;
      let currentUrl = rootUrl;
      let currentSource = snapshot.source;
      let currentFetchMs = snapshot.wrapperChain[0]?.fetchMs ?? 0;
      let stoppedReason: string | null = "resolved";

      for (let hopIndex = 0; hopIndex < maxWrapperDepth; hopIndex++) {
        const validation = validateXmlAtDepth(currentXml, hopIndex);
        const hop = buildHop(
          currentXml,
          hopIndex,
          currentSource,
          currentUrl,
          currentFetchMs,
          validation,
        );

        wrapperChain.push(hop);
        emit("resolve:hop", {
          index: hopIndex,
          url: currentUrl,
          adType: hop.adType,
          wrapperUri: hop.wrapperUri,
          errors: validation.summary.errors,
          warnings: validation.summary.warnings,
        });

        if (hop.adType !== "Wrapper") {
          stoppedReason = hop.adType === "InLine" ? "resolved" : "parse_error: Could not determine ad type";
          break;
        }

        if (!hop.wrapperUri) {
          stoppedReason = "parse_error: Wrapper has no VASTAdTagURI";
          break;
        }

        let nextUrl: string;
        try {
          nextUrl = resolveWrapperUri(hop.wrapperUri, currentUrl);
        } catch {
          stoppedReason = `parse_error: Could not resolve wrapper URI '${hop.wrapperUri}'`;
          break;
        }

        if (seenUrls.has(nextUrl)) {
          stoppedReason = `parse_error: Circular wrapper chain detected at ${nextUrl}`;
          break;
        }
        seenUrls.add(nextUrl);

        if (hopIndex + 1 >= maxWrapperDepth) {
          stoppedReason = "max_depth";
          break;
        }

        const fetched = await fetchXmlFromUrl(nextUrl);
        currentXml = fetched.xml;
        currentUrl = nextUrl;
        currentSource = { kind: "url", url: nextUrl };
        currentFetchMs = fetched.fetchMs;
      }

      const lastHop = wrapperChain[wrapperChain.length - 1] ?? null;
      const resolution = buildResolutionSummary(wrapperChain, stoppedReason);

      setSnapshot({
        ...snapshot,
        status: "resolved",
        xml: lastHop?.xml ?? rootXml,
        rootXml,
        validation: lastHop?.validation ?? null,
        wrapperChain,
        resolution,
      });
      emit("resolve:completed", {
        hopCount: resolution.hopCount,
        resolved: resolution.resolved,
        stoppedReason: resolution.stoppedReason,
        totalErrors: resolution.totalErrors,
        totalWarnings: resolution.totalWarnings,
      });

      return cloneSnapshot(snapshot);
    } catch (error) {
      setError(error);
      throw toError(error);
    }
  };

  return {
    async load() {
      await loadXml();
      return cloneSnapshot(snapshot);
    },

    async validate() {
      return runValidation();
    },

    async fix() {
      emit("fix:started");

      snapshot = {
        ...snapshot,
        status: "fixing",
        error: null,
      };
      notify();

      try {
        const xml = await loadXml();
        const result = options.validateOptions
          ? fixWithOptions(xml, options.validateOptions)
          : fix(xml);

        setSnapshot({
          ...snapshot,
          status: "ready",
          fixed: result,
        });
        emit("fix:completed", {
          applied: result.applied.length,
          remaining: result.remaining.length,
        });

        return result;
      } catch (error) {
        setError(error);
        throw toError(error);
      }
    },

    getTracking() {
      return cloneTrackingState(snapshot.tracking);
    },

    getAdTrackingTargets(adSelector, event, trackOptions) {
      const { resolvedAd } = getResolvedAdSelection(adSelector);
      return selectTrackingTargets(resolvedAd.trackingPlan, event, trackOptions?.offset);
    },

    async track(event, trackOptions: VastTrackOptions = {}) {
      emit("track:started", {
        event,
        offset: trackOptions.offset ?? null,
      });

      try {
        await loadXml();

        const availableTargets = selectTrackingTargets(snapshot.tracking.plan, event, trackOptions.offset);
        return dispatchTrackingTargets(event, availableTargets, trackOptions, null);
      } catch (error) {
        setError(error);
        throw toError(error);
      }
    },

    async trackAd(adSelector, event, trackOptions: VastTrackOptions = {}) {
      emit("track:started", buildTrackAdStartedDetail(adSelector, event, trackOptions.offset ?? null));

      try {
        if (snapshot.resolvedAds.length === 0) {
          await resolveSession();
        }

        const { adIndex, resolvedAd } = getResolvedAdSelection(adSelector);
        const availableTargets = selectTrackingTargets(resolvedAd.trackingPlan, event, trackOptions.offset);
        return dispatchTrackingTargets(event, availableTargets, trackOptions, `ad:${String(adIndex)}`);
      } catch (error) {
        setError(error);
        throw toError(error);
      }
    },

    async resolve() {
      return resolveSession();
    },

    getSnapshot() {
      return cloneSnapshot(snapshot);
    },

    subscribe(listener) {
      listeners.add(listener);
      listener(cloneSnapshot(snapshot));
      return () => {
        listeners.delete(listener);
      };
    },
  };
}