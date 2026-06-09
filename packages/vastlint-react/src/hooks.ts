import { startTransition, useDeferredValue, useEffect, useMemo, useReducer, useRef } from "react";
import { createVastPlaybackController, createVastPlaybackQueueController, createVastSession, selectTrackingTargets } from "vastlint-client";

import type { ValidateOptions, ValidationResult } from "vastlint";
import type {
  VastPlaybackController as RuntimeVastPlaybackController,
  VastPlaybackControllerOptions,
  VastPlaybackQueueController as RuntimeVastPlaybackQueueController,
  VastPlaybackQueueControllerOptions,
  VastSession as RuntimeVastSession,
  VastSessionOptions,
  VastTrackableEvent,
} from "vastlint-client";

import type {
  VastAnnotation,
  UseVastAnnotationsOptions,
  UseVastPlaybackOptions,
  UseVastPlaybackQueueOptions,
  UseVastSessionOptions,
  UseVastTrackerOptions,
  VastAnnotationModel,
  VastPlaybackHookResult,
  VastPlaybackQueueHookResult,
  VastSessionHookResult,
  VastTrackerHookResult,
} from "./types.js";

interface SessionConfig {
  sourceKind: VastSessionOptions["source"]["kind"];
  sourceValue: string;
  label: string | null;
  requestRef: RequestInit | null;
  fetchRef: VastSessionOptions["fetch"] | null;
  timeoutMs: number | null;
  maxWrapperDepth: number | null;
  validateOptionsKey: string;
}

interface TrackedSession {
  config: SessionConfig;
  session: RuntimeVastSession;
}

interface PlaybackConfig {
  sessionRef: RuntimeVastSession;
  autoResolve: boolean;
  mediaSelectionKey: string;
}

interface TrackedPlayback {
  config: PlaybackConfig;
  controller: RuntimeVastPlaybackController;
}

interface PlaybackQueueConfig {
  sessionRef: RuntimeVastSession;
  fetchRef: VastPlaybackQueueControllerOptions["fetch"] | null;
  autoResolve: boolean;
  mediaSelectionKey: string;
}

interface TrackedPlaybackQueue {
  config: PlaybackQueueConfig;
  controller: RuntimeVastPlaybackQueueController;
}

function useSubscribedSnapshot<TSnapshot>(store: {
  getSnapshot(): TSnapshot;
  subscribe(listener: (snapshot: TSnapshot) => void): () => void;
}): TSnapshot {
  const [, forceRender] = useReducer((value: number) => value + 1, 0);
  const storeRef = useRef(store);
  const snapshotRef = useRef(store.getSnapshot());

  if (!Object.is(storeRef.current, store)) {
    storeRef.current = store;
    snapshotRef.current = store.getSnapshot();
  }

  useEffect(() => {
    snapshotRef.current = store.getSnapshot();
    let isSynchronousSubscription = true;
    const unsubscribe = store.subscribe((nextSnapshot) => {
      if (isSynchronousSubscription) {
        return;
      }

      snapshotRef.current = nextSnapshot;
      forceRender();
    });

    isSynchronousSubscription = false;
    return unsubscribe;
  }, [store]);

  return snapshotRef.current;
}

function createEmptyAnnotationModel(): VastAnnotationModel {
  return {
    annotations: [],
    byLine: new Map(),
    byIssueId: new Map(),
  };
}

function countXmlLines(xml: string): number {
  if (!xml) {
    return 0;
  }

  return xml.split(/\r\n?|\n/).length;
}

function buildVastAnnotationModel(xml: string, validation: ValidationResult | null): VastAnnotationModel {
  if (!validation || validation.issues.length === 0) {
    return createEmptyAnnotationModel();
  }

  const maxLine = countXmlLines(xml);
  const annotations: VastAnnotation[] = validation.issues.map((issue, index) => {
    const line = issue.line !== null && issue.line >= 1 && issue.line <= maxLine ? issue.line : null;
    const col = line === null ? null : issue.col;

    return {
      id: `${issue.id}:${issue.path ?? "document"}:${line ?? 0}:${col ?? 0}:${index}`,
      issueId: issue.id,
      severity: issue.severity,
      message: issue.message,
      path: issue.path,
      line,
      col,
    };
  });

  const byLine = new Map<number, VastAnnotation[]>();
  const byIssueId = new Map<string, VastAnnotation[]>();

  for (const annotation of annotations) {
    if (annotation.line !== null) {
      const lineAnnotations = byLine.get(annotation.line) ?? [];
      lineAnnotations.push(annotation);
      byLine.set(annotation.line, lineAnnotations);
    }

    const issueAnnotations = byIssueId.get(annotation.issueId) ?? [];
    issueAnnotations.push(annotation);
    byIssueId.set(annotation.issueId, issueAnnotations);
  }

  return {
    annotations,
    byLine,
    byIssueId,
  };
}

function serializeValidateOptions(validateOptions?: ValidateOptions): string {
  if (!validateOptions) {
    return "";
  }

  const sortedRuleOverrides = validateOptions.rule_overrides
    ? Object.fromEntries(
        Object.entries(validateOptions.rule_overrides).sort(([left], [right]) => left.localeCompare(right)),
      )
    : undefined;

  return JSON.stringify({
    wrapper_depth: validateOptions.wrapper_depth ?? null,
    max_wrapper_depth: validateOptions.max_wrapper_depth ?? null,
    rule_overrides: sortedRuleOverrides ?? null,
  });
}

function serializeMediaSelectionOptions(
  mediaSelection?: VastPlaybackControllerOptions["mediaSelection"] | VastPlaybackQueueControllerOptions["mediaSelection"],
): string {
  if (!mediaSelection) {
    return "";
  }

  return JSON.stringify({
    supportedMimeTypes: mediaSelection.supportedMimeTypes ?? null,
    preferredMimeTypes: mediaSelection.preferredMimeTypes ?? null,
    preferredDelivery: mediaSelection.preferredDelivery ?? null,
    targetBitrate: mediaSelection.targetBitrate ?? null,
    maxBitrate: mediaSelection.maxBitrate ?? null,
    targetWidth: mediaSelection.targetWidth ?? null,
    targetHeight: mediaSelection.targetHeight ?? null,
  });
}

function createSessionConfig(options: VastSessionOptions): SessionConfig {
  return {
    sourceKind: options.source.kind,
    sourceValue: options.source.kind === "xml" ? options.source.xml : options.source.url,
    label: options.source.label ?? null,
    requestRef: options.source.kind === "url" ? (options.source.request ?? null) : null,
    fetchRef: options.fetch ?? null,
    timeoutMs: options.timeoutMs ?? null,
    maxWrapperDepth: options.maxWrapperDepth ?? null,
    validateOptionsKey: serializeValidateOptions(options.validateOptions),
  };
}

function createPlaybackConfig(options: VastPlaybackControllerOptions): PlaybackConfig {
  return {
    sessionRef: options.session,
    autoResolve: options.autoResolve !== false,
    mediaSelectionKey: serializeMediaSelectionOptions(options.mediaSelection),
  };
}

function createPlaybackQueueConfig(options: VastPlaybackQueueControllerOptions): PlaybackQueueConfig {
  return {
    sessionRef: options.session,
    fetchRef: options.fetch ?? null,
    autoResolve: options.autoResolve !== false,
    mediaSelectionKey: serializeMediaSelectionOptions(options.mediaSelection),
  };
}

function hasSessionConfigChanged(previous: SessionConfig, next: SessionConfig): boolean {
  return (
    previous.sourceKind !== next.sourceKind
    || previous.sourceValue !== next.sourceValue
    || previous.label !== next.label
    || !Object.is(previous.requestRef, next.requestRef)
    || !Object.is(previous.fetchRef, next.fetchRef)
    || previous.timeoutMs !== next.timeoutMs
    || previous.maxWrapperDepth !== next.maxWrapperDepth
    || previous.validateOptionsKey !== next.validateOptionsKey
  );
}

function hasPlaybackConfigChanged(previous: PlaybackConfig, next: PlaybackConfig): boolean {
  return (
    !Object.is(previous.sessionRef, next.sessionRef)
    || previous.autoResolve !== next.autoResolve
    || previous.mediaSelectionKey !== next.mediaSelectionKey
  );
}

function hasPlaybackQueueConfigChanged(previous: PlaybackQueueConfig, next: PlaybackQueueConfig): boolean {
  return (
    !Object.is(previous.sessionRef, next.sessionRef)
    || !Object.is(previous.fetchRef, next.fetchRef)
    || previous.autoResolve !== next.autoResolve
    || previous.mediaSelectionKey !== next.mediaSelectionKey
  );
}

async function runAutoAction(
  session: RuntimeVastSession,
  autoLoad: boolean,
  autoValidate: boolean,
) {
  if (autoValidate) {
    await session.validate();
    return session.getSnapshot();
  }

  if (autoLoad) {
    return session.load();
  }

  return session.getSnapshot();
}

export function useVastSession(options: UseVastSessionOptions): VastSessionHookResult {
  const { autoLoad = true, autoValidate = false, ...sessionOptions } = options;
  const [, forceRender] = useReducer((value: number) => value + 1, 0);
  const trackedSessionRef = useRef<TrackedSession | null>(null);
  const autoStartedSessionsRef = useRef(new WeakSet<RuntimeVastSession>());

  const sessionConfig = createSessionConfig(sessionOptions);

  if (
    !trackedSessionRef.current
    || hasSessionConfigChanged(trackedSessionRef.current.config, sessionConfig)
  ) {
    trackedSessionRef.current = {
      config: sessionConfig,
      session: createVastSession(sessionOptions),
    };
  }

  const getCurrentSession = () => trackedSessionRef.current!.session;

  const replaceSession = () => {
    const nextSession = createVastSession(sessionOptions);
    trackedSessionRef.current = {
      config: sessionConfig,
      session: nextSession,
    };
    startTransition(() => {
      forceRender();
    });
    return nextSession;
  };

  const session = getCurrentSession();
  const snapshot = useSubscribedSnapshot(session);

  useEffect(() => {
    if ((!autoLoad && !autoValidate) || autoStartedSessionsRef.current.has(session)) {
      return;
    }

    autoStartedSessionsRef.current.add(session);

    void runAutoAction(session, autoLoad, autoValidate).catch(() => {
      // Session state already captures errors for consumers.
    });
  }, [session, autoLoad, autoValidate]);

  return {
    session,
    snapshot,
    load() {
      return getCurrentSession().load();
    },
    reload() {
      const nextSession = replaceSession();
      autoStartedSessionsRef.current.add(nextSession);
      return runAutoAction(nextSession, true, autoValidate);
    },
    validate() {
      return getCurrentSession().validate();
    },
    fix() {
      return getCurrentSession().fix();
    },
    resolve() {
      return getCurrentSession().resolve();
    },
    track(event, trackOptions) {
      return getCurrentSession().track(event, trackOptions);
    },
    trackAd(adSelector, event, trackOptions) {
      return getCurrentSession().trackAd(adSelector, event, trackOptions);
    },
  };
}

export function useVastAnnotations(_options: UseVastAnnotationsOptions): VastAnnotationModel {
  const deferredXml = useDeferredValue(_options.xml);
  const deferredValidation = useDeferredValue(_options.validation);

  return useMemo(
    () => buildVastAnnotationModel(deferredXml, deferredValidation),
    [deferredXml, deferredValidation],
  );
}

export function useVastPlayback(options: UseVastPlaybackOptions): VastPlaybackHookResult {
  const { autoInitialize = true, ...controllerOptions } = options;
  const trackedPlaybackRef = useRef<TrackedPlayback | null>(null);
  const autoInitializedControllersRef = useRef(new WeakSet<RuntimeVastPlaybackController>());
  const disposedControllersRef = useRef(new WeakSet<RuntimeVastPlaybackController>());

  const playbackConfig = createPlaybackConfig(controllerOptions);

  if (
    !trackedPlaybackRef.current
    || disposedControllersRef.current.has(trackedPlaybackRef.current.controller)
    || hasPlaybackConfigChanged(trackedPlaybackRef.current.config, playbackConfig)
  ) {
    trackedPlaybackRef.current = {
      config: playbackConfig,
      controller: createVastPlaybackController(controllerOptions),
    };
  }

  const getCurrentController = () => trackedPlaybackRef.current!.controller;
  const controller = getCurrentController();
  const snapshot = useSubscribedSnapshot(controller);

  useEffect(() => {
    return () => {
      disposedControllersRef.current.add(controller);
      controller.dispose();
    };
  }, [controller]);

  useEffect(() => {
    if (!autoInitialize || autoInitializedControllersRef.current.has(controller)) {
      return;
    }

    autoInitializedControllersRef.current.add(controller);

    void controller.initialize().catch(() => {
      // Controller state already captures errors for consumers.
    });
  }, [controller, autoInitialize]);

  return {
    controller,
    snapshot,
    initialize() {
      return getCurrentController().initialize();
    },
    start() {
      return getCurrentController().start();
    },
    pause() {
      return getCurrentController().pause();
    },
    resume() {
      return getCurrentController().resume();
    },
    updateProgress(currentTimeSec, durationSec) {
      return getCurrentController().updateProgress(currentTimeSec, durationSec);
    },
    complete() {
      return getCurrentController().complete();
    },
    setMuted(muted) {
      return getCurrentController().setMuted(muted);
    },
    setFullscreen(fullscreen) {
      return getCurrentController().setFullscreen(fullscreen);
    },
    setViewability(viewability) {
      return getCurrentController().setViewability(viewability);
    },
    click(trackOptions) {
      return getCurrentController().click(trackOptions);
    },
    skip() {
      return getCurrentController().skip();
    },
    signalError(trackOptions) {
      return getCurrentController().signalError(trackOptions);
    },
  };
}

export function useVastPlaybackQueue(options: UseVastPlaybackQueueOptions): VastPlaybackQueueHookResult {
  const { autoInitialize = true, ...controllerOptions } = options;
  const trackedPlaybackQueueRef = useRef<TrackedPlaybackQueue | null>(null);
  const autoInitializedControllersRef = useRef(new WeakSet<RuntimeVastPlaybackQueueController>());
  const disposedControllersRef = useRef(new WeakSet<RuntimeVastPlaybackQueueController>());

  const playbackQueueConfig = createPlaybackQueueConfig(controllerOptions);

  if (
    !trackedPlaybackQueueRef.current
    || disposedControllersRef.current.has(trackedPlaybackQueueRef.current.controller)
    || hasPlaybackQueueConfigChanged(trackedPlaybackQueueRef.current.config, playbackQueueConfig)
  ) {
    trackedPlaybackQueueRef.current = {
      config: playbackQueueConfig,
      controller: createVastPlaybackQueueController(controllerOptions),
    };
  }

  const getCurrentController = () => trackedPlaybackQueueRef.current!.controller;
  const controller = getCurrentController();
  const snapshot = useSubscribedSnapshot(controller);

  useEffect(() => {
    return () => {
      disposedControllersRef.current.add(controller);
      controller.dispose();
    };
  }, [controller]);

  useEffect(() => {
    if (!autoInitialize || autoInitializedControllersRef.current.has(controller)) {
      return;
    }

    autoInitializedControllersRef.current.add(controller);

    void controller.initialize().catch(() => {
      // Controller state already captures errors for consumers.
    });
  }, [controller, autoInitialize]);

  return {
    controller,
    snapshot,
    initialize() {
      return getCurrentController().initialize();
    },
    start() {
      return getCurrentController().start();
    },
    pause() {
      return getCurrentController().pause();
    },
    resume() {
      return getCurrentController().resume();
    },
    updateProgress(currentTimeSec, durationSec) {
      return getCurrentController().updateProgress(currentTimeSec, durationSec);
    },
    completeCurrent() {
      return getCurrentController().completeCurrent();
    },
    next() {
      return getCurrentController().next();
    },
    setMuted(muted) {
      return getCurrentController().setMuted(muted);
    },
    setFullscreen(fullscreen) {
      return getCurrentController().setFullscreen(fullscreen);
    },
    setViewability(viewability) {
      return getCurrentController().setViewability(viewability);
    },
    click(trackOptions) {
      return getCurrentController().click(trackOptions);
    },
    skip() {
      return getCurrentController().skip();
    },
    signalError(trackOptions) {
      return getCurrentController().signalError(trackOptions);
    },
  };
}

export function useVastTracker(options: UseVastTrackerOptions): VastTrackerHookResult {
  const { session } = options;
  const snapshot = useSubscribedSnapshot(session);

  const availableEvents = useMemo(() => {
    const events = new Set<VastTrackableEvent>();

    if (snapshot.tracking.plan.impressions.length > 0) {
      events.add("impression");
    }

    if (snapshot.tracking.plan.errors.length > 0) {
      events.add("error");
    }

    if (snapshot.tracking.plan.clickTrackings.length > 0) {
      events.add("clickTracking");
    }

    for (const target of snapshot.tracking.plan.events) {
      events.add(target.event as VastTrackableEvent);
    }

    return [...events].sort((left, right) => left.localeCompare(right));
  }, [snapshot.tracking]);

  return {
    session,
    tracking: snapshot.tracking,
    resolvedAd: snapshot.resolvedAd,
    resolvedAds: snapshot.resolvedAds,
    companions: snapshot.resolvedAd?.companions ?? [],
    availableEvents,
    clickThroughUrl: snapshot.resolvedAd?.clickThroughUrl ?? null,
    clickThroughUrls: snapshot.resolvedAd?.clickThroughUrls ?? [],
    getAdCompanions(adSelector) {
      return session.getAdCompanions(adSelector);
    },
    track(event, trackOptions) {
      return session.track(event, trackOptions);
    },
    trackAd(adSelector, event, trackOptions) {
      return session.trackAd(adSelector, event, trackOptions);
    },
    trackCompanion(adSelector, companionSelector, event, trackOptions) {
      return session.trackCompanion(adSelector, companionSelector, event, trackOptions);
    },
    getTargets(event, trackOptions) {
      return selectTrackingTargets(snapshot.tracking.plan, event, trackOptions?.offset);
    },
    getAdTargets(adSelector, event, trackOptions) {
      return session.getAdTrackingTargets(adSelector, event, trackOptions);
    },
    getCompanionTargets(adSelector, companionSelector, event) {
      return session.getCompanionTrackingTargets(adSelector, companionSelector, event);
    },
    hasTargets(event, trackOptions) {
      return selectTrackingTargets(snapshot.tracking.plan, event, trackOptions?.offset).length > 0;
    },
    hasAdTargets(adSelector, event, trackOptions) {
      return session.getAdTrackingTargets(adSelector, event, trackOptions).length > 0;
    },
    hasCompanionTargets(adSelector, companionSelector, event) {
      return session.getCompanionTrackingTargets(adSelector, companionSelector, event).length > 0;
    },
  };
}