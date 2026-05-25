import { selectResolvedAdMediaFile } from "./media.js";
import { expandTrackingUrl } from "./tracking.js";
const PROGRESS_MILESTONES = [
    ["firstQuartile", 0.25],
    ["midpoint", 0.5],
    ["thirdQuartile", 0.75],
    ["complete", 1],
];
function createTimestamp() {
    return new Date().toISOString();
}
function toError(value) {
    if (value instanceof Error) {
        return value;
    }
    return new Error(typeof value === "string" ? value : "Unknown vastlint-client playback queue error.");
}
function parseDurationToSeconds(value) {
    const trimmed = value.trim();
    if (!trimmed) {
        return null;
    }
    const parts = trimmed.split(":");
    if (parts.length !== 3) {
        return null;
    }
    const hours = Number.parseInt(parts[0] ?? "", 10);
    const minutes = Number.parseInt(parts[1] ?? "", 10);
    const seconds = Number.parseFloat(parts[2] ?? "");
    if (![hours, minutes, seconds].every(Number.isFinite)) {
        return null;
    }
    return hours * 3600 + minutes * 60 + seconds;
}
function createMilestones() {
    return {
        start: false,
        firstQuartile: false,
        midpoint: false,
        thirdQuartile: false,
        complete: false,
    };
}
function clonePlaybackMilestones(milestones) {
    return { ...milestones };
}
function cloneMediaSelection(item) {
    return {
        selected: item.selected ? { ...item.selected } : null,
        candidates: item.candidates.map((candidate) => ({
            mediaFile: { ...candidate.mediaFile },
            score: candidate.score,
            reasons: [...candidate.reasons],
        })),
    };
}
function buildAdKey(resolvedAd) {
    return [
        resolvedAd.finalUrl ?? "",
        resolvedAd.adTitle,
        resolvedAd.duration,
        resolvedAd.adPod.adId ?? "",
        String(resolvedAd.adPod.sequence ?? ""),
        ...resolvedAd.mediaFiles.map((mediaFile) => mediaFile.url),
    ].join("::");
}
function buildQueueItem(resolvedAd, adIndex, options) {
    return {
        adIndex,
        resolvedAd,
        mediaSelection: selectResolvedAdMediaFile(resolvedAd, options.mediaSelection),
        currentTimeSec: 0,
        durationSec: parseDurationToSeconds(resolvedAd.duration),
        impressionTracked: false,
        skipped: false,
        milestones: createMilestones(),
        status: resolvedAd.resolved ? "ready" : "idle",
        clickThroughUrl: resolvedAd.clickThroughUrl ?? null,
        error: null,
    };
}
function syncQueueItem(existing, resolvedAd, adIndex, options) {
    if (!existing) {
        return buildQueueItem(resolvedAd, adIndex, options);
    }
    return {
        ...existing,
        adIndex,
        resolvedAd,
        mediaSelection: selectResolvedAdMediaFile(resolvedAd, options.mediaSelection),
        durationSec: existing.durationSec ?? parseDurationToSeconds(resolvedAd.duration),
        clickThroughUrl: resolvedAd.clickThroughUrl ?? null,
    };
}
function cloneQueueItem(item) {
    return {
        ...item,
        mediaSelection: cloneMediaSelection(item.mediaSelection),
        milestones: clonePlaybackMilestones(item.milestones),
    };
}
function buildQueueItems(resolvedAds, options, previousItems = []) {
    const previousByKey = new Map(previousItems.map((item) => [buildAdKey(item.resolvedAd), item]));
    return resolvedAds.map((resolvedAd, adIndex) => syncQueueItem(previousByKey.get(buildAdKey(resolvedAd)), resolvedAd, adIndex, options));
}
function normalizeQueueSnapshot(next) {
    const currentAdIndex = next.currentAdIndex !== null && next.currentAdIndex >= 0 && next.currentAdIndex < next.items.length
        ? next.currentAdIndex
        : next.items.length > 0
            ? 0
            : null;
    const currentItem = currentAdIndex !== null ? next.items[currentAdIndex] ?? null : null;
    const status = next.session.error
        ? "error"
        : currentItem?.status ?? (next.items.length > 0 ? "ready" : "idle");
    return {
        ...next,
        resolvedAds: [...next.session.resolvedAds],
        currentAdIndex,
        currentItem,
        hasNext: currentAdIndex !== null ? currentAdIndex + 1 < next.items.length : false,
        status,
        error: next.session.error?.message ?? currentItem?.error ?? null,
    };
}
function cloneQueueSnapshot(snapshot, session) {
    const sessionSnapshot = session.getSnapshot();
    const currentAdIndex = snapshot.currentAdIndex !== null && snapshot.currentAdIndex < snapshot.items.length
        ? snapshot.currentAdIndex
        : snapshot.items.length > 0
            ? 0
            : null;
    return normalizeQueueSnapshot({
        ...snapshot,
        session: sessionSnapshot,
        resolvedAds: [...sessionSnapshot.resolvedAds],
        items: snapshot.items.map((item) => cloneQueueItem(item)),
        currentAdIndex,
        currentItem: currentAdIndex !== null ? cloneQueueItem(snapshot.items[currentAdIndex] ?? snapshot.currentItem ?? snapshot.items[0]) : null,
    });
}
function selectTrackingUrls(resolvedAd, event) {
    if (event === "impression") {
        return [...resolvedAd.impressionUrls];
    }
    if (event === "error") {
        return [...resolvedAd.errorUrls];
    }
    if (event === "clickTracking") {
        return [...resolvedAd.clickTrackingUrls];
    }
    return [...(resolvedAd.trackingEvents[event] ?? [])];
}
export function createVastPlaybackQueueController(options) {
    const listeners = new Set();
    const dispatchedTrackingKeys = new Set();
    let snapshot = normalizeQueueSnapshot({
        status: "idle",
        session: options.session.getSnapshot(),
        resolvedAds: options.session.getSnapshot().resolvedAds,
        items: buildQueueItems(options.session.getSnapshot().resolvedAds, options),
        currentAdIndex: options.session.getSnapshot().resolvedAds.length > 0 ? 0 : null,
        currentItem: null,
        hasNext: false,
        muted: false,
        fullscreen: false,
        viewability: null,
        error: options.session.getSnapshot().error?.message ?? null,
    });
    const notify = () => {
        const current = cloneQueueSnapshot(snapshot, options.session);
        for (const listener of listeners) {
            listener(current);
        }
    };
    const setSnapshot = (next) => {
        snapshot = normalizeQueueSnapshot(next);
        notify();
    };
    const sessionUnsubscribe = options.session.subscribe((sessionSnapshot) => {
        const currentKey = snapshot.currentItem ? buildAdKey(snapshot.currentItem.resolvedAd) : null;
        const items = buildQueueItems(sessionSnapshot.resolvedAds, options, snapshot.items);
        const nextIndex = currentKey
            ? items.findIndex((item) => buildAdKey(item.resolvedAd) === currentKey)
            : snapshot.currentAdIndex ?? (items.length > 0 ? 0 : null);
        setSnapshot({
            ...snapshot,
            session: sessionSnapshot,
            resolvedAds: sessionSnapshot.resolvedAds,
            items,
            currentAdIndex: typeof nextIndex === "number" && nextIndex >= 0 ? nextIndex : items.length > 0 ? 0 : null,
            currentItem: null,
        });
    });
    const updateCurrentItem = (mutator) => {
        if (snapshot.currentAdIndex === null) {
            throw new Error("No active VAST ad is available in the playback queue.");
        }
        const currentItem = snapshot.items[snapshot.currentAdIndex];
        if (!currentItem) {
            throw new Error("The current VAST playback queue item is unavailable.");
        }
        const nextItem = mutator(currentItem);
        const items = snapshot.items.map((item, index) => (index === snapshot.currentAdIndex ? nextItem : item));
        setSnapshot({
            ...snapshot,
            items,
            currentItem: nextItem,
        });
        return nextItem;
    };
    const runAction = async (action) => {
        try {
            return await action();
        }
        catch (error) {
            const nextError = toError(error);
            if (snapshot.currentAdIndex !== null) {
                updateCurrentItem((item) => ({
                    ...item,
                    status: "error",
                    error: nextError.message,
                }));
            }
            else {
                setSnapshot({
                    ...snapshot,
                    error: nextError.message,
                });
            }
            throw nextError;
        }
    };
    const ensureQueueReady = async () => {
        if (snapshot.items.length === 0) {
            if (options.autoResolve === false) {
                throw new Error("VAST playback queue requires a resolved session when autoResolve is false.");
            }
            await options.session.resolve();
        }
        if (snapshot.currentAdIndex === null || !snapshot.currentItem) {
            throw new Error("No resolved VAST ad is available in the playback queue.");
        }
        if (!snapshot.currentItem.resolvedAd.resolved) {
            throw new Error("The current VAST queue item does not resolve to an inline playable ad.");
        }
        return snapshot.currentItem;
    };
    const dispatchCurrentEvent = async (event, trackOptions = {}, defaultDedupe = true) => {
        const currentItem = await ensureQueueReady();
        const urls = selectTrackingUrls(currentItem.resolvedAd, event);
        const dedupe = trackOptions.dedupe ?? defaultDedupe;
        const fetchImpl = options.fetch ?? globalThis.fetch;
        if (typeof fetchImpl !== "function") {
            throw new Error("No fetch implementation is available for VAST playback queue tracking dispatch.");
        }
        const filteredUrls = dedupe
            ? urls.filter((url) => !dispatchedTrackingKeys.has(`${buildAdKey(currentItem.resolvedAd)}:${event}:${url}`))
            : urls;
        const results = await Promise.all(filteredUrls.map(async (url) => {
            const resolvedUrl = expandTrackingUrl(url, trackOptions.macros);
            try {
                const response = await fetchImpl(resolvedUrl, { method: "GET" });
                return {
                    event,
                    url,
                    resolvedUrl,
                    hopIndex: currentItem.resolvedAd.finalHopIndex ?? 0,
                    sourceUrl: currentItem.resolvedAd.finalUrl,
                    offset: null,
                    ok: response.ok,
                    status: response.status,
                    dispatchedAt: createTimestamp(),
                    error: null,
                };
            }
            catch (dispatchError) {
                return {
                    event,
                    url,
                    resolvedUrl,
                    hopIndex: currentItem.resolvedAd.finalHopIndex ?? 0,
                    sourceUrl: currentItem.resolvedAd.finalUrl,
                    offset: null,
                    ok: false,
                    status: null,
                    dispatchedAt: createTimestamp(),
                    error: toError(dispatchError).message,
                };
            }
        }));
        for (const url of filteredUrls) {
            dispatchedTrackingKeys.add(`${buildAdKey(currentItem.resolvedAd)}:${event}:${url}`);
        }
        return results;
    };
    const ensureStarted = async () => {
        const currentItem = await ensureQueueReady();
        if (!currentItem.impressionTracked) {
            await dispatchCurrentEvent("impression", { dedupe: true });
        }
        if (!currentItem.milestones.start) {
            await dispatchCurrentEvent("creativeView", { dedupe: true });
            await dispatchCurrentEvent("start", { dedupe: true });
        }
        updateCurrentItem((item) => ({
            ...item,
            impressionTracked: true,
            milestones: {
                ...item.milestones,
                start: true,
            },
            status: item.milestones.complete || item.skipped ? "ended" : "playing",
            error: null,
        }));
    };
    return {
        async initialize() {
            return runAction(async () => {
                await ensureQueueReady();
                return cloneQueueSnapshot(snapshot, options.session);
            });
        },
        async start() {
            return runAction(async () => {
                await ensureStarted();
                return cloneQueueSnapshot(snapshot, options.session);
            });
        },
        async pause() {
            return runAction(async () => {
                const currentItem = await ensureQueueReady();
                if (currentItem.status !== "playing") {
                    return cloneQueueSnapshot(snapshot, options.session);
                }
                await dispatchCurrentEvent("pause", { dedupe: false }, false);
                updateCurrentItem((item) => ({
                    ...item,
                    status: "paused",
                    error: null,
                }));
                return cloneQueueSnapshot(snapshot, options.session);
            });
        },
        async resume() {
            return runAction(async () => {
                const currentItem = await ensureQueueReady();
                if (!currentItem.milestones.start) {
                    await ensureStarted();
                    return cloneQueueSnapshot(snapshot, options.session);
                }
                if (currentItem.status !== "paused") {
                    return cloneQueueSnapshot(snapshot, options.session);
                }
                await dispatchCurrentEvent("resume", { dedupe: false }, false);
                updateCurrentItem((item) => ({
                    ...item,
                    status: "playing",
                    error: null,
                }));
                return cloneQueueSnapshot(snapshot, options.session);
            });
        },
        async updateProgress(currentTimeSec, durationSec) {
            return runAction(async () => {
                const currentItem = await ensureQueueReady();
                if (!Number.isFinite(currentTimeSec) || currentTimeSec < 0) {
                    throw new Error("Playback progress time must be a finite, non-negative number.");
                }
                if (currentTimeSec > 0 && !currentItem.milestones.start) {
                    await ensureStarted();
                }
                const activeItem = snapshot.currentItem;
                if (!activeItem) {
                    throw new Error("The current VAST playback queue item is unavailable.");
                }
                const nextDuration = durationSec ?? activeItem.durationSec ?? parseDurationToSeconds(activeItem.resolvedAd.duration);
                const nextMilestones = clonePlaybackMilestones(activeItem.milestones);
                if (nextDuration && nextDuration > 0) {
                    const progress = currentTimeSec / nextDuration;
                    for (const [event, threshold] of PROGRESS_MILESTONES) {
                        if (nextMilestones[event] || progress < threshold) {
                            continue;
                        }
                        await dispatchCurrentEvent(event, { dedupe: true });
                        nextMilestones[event] = true;
                    }
                }
                updateCurrentItem((item) => ({
                    ...item,
                    currentTimeSec,
                    durationSec: nextDuration,
                    milestones: nextMilestones,
                    status: nextMilestones.complete
                        ? "ended"
                        : item.status === "paused"
                            ? "paused"
                            : item.milestones.start || currentTimeSec > 0
                                ? "playing"
                                : item.status,
                    error: null,
                }));
                return cloneQueueSnapshot(snapshot, options.session);
            });
        },
        async completeCurrent() {
            return runAction(async () => {
                await ensureStarted();
                const currentItem = snapshot.currentItem;
                if (!currentItem || currentItem.milestones.complete) {
                    return cloneQueueSnapshot(snapshot, options.session);
                }
                await dispatchCurrentEvent("complete", { dedupe: true });
                updateCurrentItem((item) => ({
                    ...item,
                    currentTimeSec: item.durationSec ?? item.currentTimeSec,
                    milestones: {
                        ...item.milestones,
                        complete: true,
                    },
                    status: "ended",
                    error: null,
                }));
                return cloneQueueSnapshot(snapshot, options.session);
            });
        },
        async next() {
            return runAction(async () => {
                await ensureQueueReady();
                if (snapshot.currentAdIndex === null || snapshot.currentAdIndex + 1 >= snapshot.items.length) {
                    return cloneQueueSnapshot(snapshot, options.session);
                }
                setSnapshot({
                    ...snapshot,
                    currentAdIndex: snapshot.currentAdIndex + 1,
                    currentItem: null,
                    error: null,
                });
                return cloneQueueSnapshot(snapshot, options.session);
            });
        },
        async setMuted(muted) {
            return runAction(async () => {
                await ensureQueueReady();
                if (snapshot.muted === muted) {
                    return cloneQueueSnapshot(snapshot, options.session);
                }
                if (snapshot.currentItem?.milestones.start) {
                    await dispatchCurrentEvent(muted ? "mute" : "unmute", { dedupe: false }, false);
                }
                setSnapshot({
                    ...snapshot,
                    muted,
                });
                return cloneQueueSnapshot(snapshot, options.session);
            });
        },
        async setFullscreen(fullscreen) {
            return runAction(async () => {
                await ensureQueueReady();
                if (snapshot.fullscreen === fullscreen) {
                    return cloneQueueSnapshot(snapshot, options.session);
                }
                if (snapshot.currentItem?.milestones.start) {
                    await dispatchCurrentEvent(fullscreen ? "fullscreen" : "exitFullscreen", { dedupe: false }, false);
                }
                setSnapshot({
                    ...snapshot,
                    fullscreen,
                });
                return cloneQueueSnapshot(snapshot, options.session);
            });
        },
        async setViewability(viewability) {
            return runAction(async () => {
                await ensureQueueReady();
                if (snapshot.viewability === viewability) {
                    return cloneQueueSnapshot(snapshot, options.session);
                }
                if (snapshot.currentItem?.milestones.start) {
                    await dispatchCurrentEvent(viewability, { dedupe: false }, false);
                }
                setSnapshot({
                    ...snapshot,
                    viewability,
                });
                return cloneQueueSnapshot(snapshot, options.session);
            });
        },
        async click(trackOptions = {}) {
            return runAction(async () => {
                const currentItem = await ensureQueueReady();
                const tracking = await dispatchCurrentEvent("clickTracking", {
                    ...trackOptions,
                    dedupe: false,
                }, false);
                return {
                    clickThroughUrl: currentItem.clickThroughUrl,
                    tracking,
                    snapshot: cloneQueueSnapshot(snapshot, options.session),
                };
            });
        },
        async skip() {
            return runAction(async () => {
                const currentItem = await ensureQueueReady();
                if (currentItem.skipped || currentItem.milestones.complete) {
                    return cloneQueueSnapshot(snapshot, options.session);
                }
                await dispatchCurrentEvent("skip", { dedupe: true });
                updateCurrentItem((item) => ({
                    ...item,
                    skipped: true,
                    status: "ended",
                    error: null,
                }));
                return cloneQueueSnapshot(snapshot, options.session);
            });
        },
        async signalError(trackOptions = {}) {
            return runAction(async () => {
                await ensureQueueReady();
                await dispatchCurrentEvent("error", {
                    ...trackOptions,
                    dedupe: false,
                }, false);
                updateCurrentItem((item) => ({
                    ...item,
                    status: "error",
                    error: "Playback error signaled.",
                }));
                return cloneQueueSnapshot(snapshot, options.session);
            });
        },
        getSnapshot() {
            return cloneQueueSnapshot(snapshot, options.session);
        },
        subscribe(listener) {
            listeners.add(listener);
            listener(cloneQueueSnapshot(snapshot, options.session));
            return () => {
                listeners.delete(listener);
            };
        },
        dispose() {
            sessionUnsubscribe();
            listeners.clear();
        },
    };
}
