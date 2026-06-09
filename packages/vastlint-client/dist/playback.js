import { selectResolvedAdMediaFile } from "./media.js";
const PROGRESS_MILESTONES = [
    ["firstQuartile", 0.25],
    ["midpoint", 0.5],
    ["thirdQuartile", 0.75],
    ["complete", 1],
];
function toError(value) {
    if (value instanceof Error) {
        return value;
    }
    return new Error(typeof value === "string" ? value : "Unknown vastlint-client playback error.");
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
function cloneMediaSelection(snapshot) {
    return {
        selected: snapshot.selected ? { ...snapshot.selected } : null,
        candidates: snapshot.candidates.map((candidate) => ({
            mediaFile: { ...candidate.mediaFile },
            score: candidate.score,
            reasons: [...candidate.reasons],
        })),
    };
}
function buildAdKey(resolvedAd) {
    if (!resolvedAd) {
        return "none";
    }
    return [
        resolvedAd.finalUrl ?? "",
        resolvedAd.adTitle,
        resolvedAd.duration,
        resolvedAd.adPod.adId ?? "",
        String(resolvedAd.adPod.sequence ?? ""),
        ...resolvedAd.mediaFiles.map((mediaFile) => mediaFile.url),
    ].join("::");
}
function buildBaseSnapshot(sessionSnapshot, options) {
    const resolvedAd = sessionSnapshot.resolvedAd;
    return {
        status: sessionSnapshot.error ? "error" : resolvedAd ? "ready" : "idle",
        session: sessionSnapshot,
        resolvedAd,
        mediaSelection: selectResolvedAdMediaFile(resolvedAd, options.mediaSelection),
        currentTimeSec: 0,
        durationSec: parseDurationToSeconds(resolvedAd?.duration ?? ""),
        impressionTracked: false,
        muted: false,
        fullscreen: false,
        skipped: false,
        viewability: null,
        milestones: createMilestones(),
        clickThroughUrl: resolvedAd?.clickThroughUrl ?? null,
        error: sessionSnapshot.error?.message ?? null,
    };
}
function clonePlaybackSnapshot(snapshot, session) {
    const sessionSnapshot = session.getSnapshot();
    return {
        ...snapshot,
        session: sessionSnapshot,
        resolvedAd: sessionSnapshot.resolvedAd,
        mediaSelection: cloneMediaSelection(snapshot.mediaSelection),
        milestones: clonePlaybackMilestones(snapshot.milestones),
    };
}
function normalizeStatus(currentStatus, sessionSnapshot, resolvedAd) {
    if (sessionSnapshot.error) {
        return "error";
    }
    if (!resolvedAd) {
        return "idle";
    }
    if (currentStatus === "idle") {
        return "ready";
    }
    return currentStatus;
}
export function createVastPlaybackController(options) {
    const listeners = new Set();
    let snapshot = buildBaseSnapshot(options.session.getSnapshot(), options);
    const notify = () => {
        const current = clonePlaybackSnapshot(snapshot, options.session);
        for (const listener of listeners) {
            listener(current);
        }
    };
    const deriveSnapshotFromSession = (sessionSnapshot) => {
        const resolvedAd = sessionSnapshot.resolvedAd;
        const adChanged = buildAdKey(snapshot.resolvedAd) !== buildAdKey(resolvedAd);
        return adChanged
            ? {
                ...buildBaseSnapshot(sessionSnapshot, options),
                muted: snapshot.muted,
                fullscreen: snapshot.fullscreen,
                viewability: snapshot.viewability,
            }
            : {
                ...snapshot,
                session: sessionSnapshot,
                resolvedAd,
                mediaSelection: selectResolvedAdMediaFile(resolvedAd, options.mediaSelection),
                durationSec: snapshot.durationSec ?? parseDurationToSeconds(resolvedAd?.duration ?? ""),
                clickThroughUrl: resolvedAd?.clickThroughUrl ?? null,
                error: sessionSnapshot.error?.message ?? snapshot.error,
                status: normalizeStatus(snapshot.status, sessionSnapshot, resolvedAd),
            };
    };
    const sessionUnsubscribe = options.session.subscribe((sessionSnapshot) => {
        snapshot = deriveSnapshotFromSession(sessionSnapshot);
        notify();
    });
    const setSnapshot = (next) => {
        snapshot = next;
        notify();
    };
    const runAction = async (action) => {
        try {
            return await action();
        }
        catch (error) {
            const nextError = toError(error);
            setSnapshot({
                ...snapshot,
                status: "error",
                error: nextError.message,
            });
            throw nextError;
        }
    };
    const ensureReady = async () => {
        snapshot = deriveSnapshotFromSession(options.session.getSnapshot());
        if (!snapshot.resolvedAd) {
            if (options.autoResolve === false) {
                throw new Error("VAST playback controller requires a resolved session when autoResolve is false.");
            }
            await options.session.resolve();
            snapshot = deriveSnapshotFromSession(options.session.getSnapshot());
        }
        if (!snapshot.resolvedAd || !snapshot.resolvedAd.resolved) {
            throw new Error("No resolved inline VAST ad is available for playback.");
        }
        if (snapshot.status !== "ready" || snapshot.error !== null) {
            setSnapshot({
                ...snapshot,
                status: "ready",
                error: null,
            });
        }
        return snapshot.resolvedAd;
    };
    const ensureStarted = async () => {
        await ensureReady();
        if (!snapshot.impressionTracked) {
            await options.session.track("impression", { dedupe: true });
        }
        if (!snapshot.milestones.start) {
            await options.session.track("creativeView", { dedupe: true });
            await options.session.track("start", { dedupe: true });
        }
        setSnapshot({
            ...snapshot,
            status: snapshot.milestones.complete || snapshot.skipped ? "ended" : "playing",
            impressionTracked: true,
            milestones: {
                ...snapshot.milestones,
                start: true,
            },
            error: null,
        });
    };
    const dispatchLifecycleEvent = async (event, optionsOverride) => {
        await options.session.track(event, optionsOverride);
    };
    return {
        async initialize() {
            return runAction(async () => {
                await ensureReady();
                return clonePlaybackSnapshot(snapshot, options.session);
            });
        },
        async start() {
            return runAction(async () => {
                await ensureStarted();
                return clonePlaybackSnapshot(snapshot, options.session);
            });
        },
        async pause() {
            return runAction(async () => {
                await ensureReady();
                if (snapshot.status !== "playing") {
                    return clonePlaybackSnapshot(snapshot, options.session);
                }
                await dispatchLifecycleEvent("pause", { dedupe: false });
                setSnapshot({
                    ...snapshot,
                    status: "paused",
                    error: null,
                });
                return clonePlaybackSnapshot(snapshot, options.session);
            });
        },
        async resume() {
            return runAction(async () => {
                await ensureReady();
                if (!snapshot.milestones.start) {
                    await ensureStarted();
                    return clonePlaybackSnapshot(snapshot, options.session);
                }
                if (snapshot.status !== "paused") {
                    return clonePlaybackSnapshot(snapshot, options.session);
                }
                await dispatchLifecycleEvent("resume", { dedupe: false });
                setSnapshot({
                    ...snapshot,
                    status: "playing",
                    error: null,
                });
                return clonePlaybackSnapshot(snapshot, options.session);
            });
        },
        async updateProgress(currentTimeSec, durationSec) {
            return runAction(async () => {
                await ensureReady();
                if (!Number.isFinite(currentTimeSec) || currentTimeSec < 0) {
                    throw new Error("Playback progress time must be a finite, non-negative number.");
                }
                const nextDuration = durationSec ?? snapshot.durationSec ?? parseDurationToSeconds(snapshot.resolvedAd?.duration ?? "");
                if (currentTimeSec > 0 && !snapshot.milestones.start) {
                    await ensureStarted();
                }
                const nextMilestones = clonePlaybackMilestones(snapshot.milestones);
                if (nextDuration && nextDuration > 0) {
                    const progress = currentTimeSec / nextDuration;
                    for (const [event, threshold] of PROGRESS_MILESTONES) {
                        if (nextMilestones[event] || progress < threshold) {
                            continue;
                        }
                        await dispatchLifecycleEvent(event, { dedupe: true });
                        nextMilestones[event] = true;
                    }
                }
                setSnapshot({
                    ...snapshot,
                    currentTimeSec,
                    durationSec: nextDuration,
                    milestones: nextMilestones,
                    status: nextMilestones.complete
                        ? "ended"
                        : snapshot.status === "paused"
                            ? "paused"
                            : snapshot.milestones.start || currentTimeSec > 0
                                ? "playing"
                                : snapshot.status,
                    error: null,
                });
                return clonePlaybackSnapshot(snapshot, options.session);
            });
        },
        async complete() {
            return runAction(async () => {
                await ensureStarted();
                if (snapshot.milestones.complete) {
                    return clonePlaybackSnapshot(snapshot, options.session);
                }
                await dispatchLifecycleEvent("complete", { dedupe: true });
                setSnapshot({
                    ...snapshot,
                    currentTimeSec: snapshot.durationSec ?? snapshot.currentTimeSec,
                    milestones: {
                        ...snapshot.milestones,
                        complete: true,
                    },
                    status: "ended",
                    error: null,
                });
                return clonePlaybackSnapshot(snapshot, options.session);
            });
        },
        async setMuted(muted) {
            return runAction(async () => {
                await ensureReady();
                if (snapshot.muted === muted) {
                    return clonePlaybackSnapshot(snapshot, options.session);
                }
                if (snapshot.milestones.start) {
                    await dispatchLifecycleEvent(muted ? "mute" : "unmute", { dedupe: false });
                }
                setSnapshot({
                    ...snapshot,
                    muted,
                    error: null,
                });
                return clonePlaybackSnapshot(snapshot, options.session);
            });
        },
        async setFullscreen(fullscreen) {
            return runAction(async () => {
                await ensureReady();
                if (snapshot.fullscreen === fullscreen) {
                    return clonePlaybackSnapshot(snapshot, options.session);
                }
                if (snapshot.milestones.start) {
                    await dispatchLifecycleEvent(fullscreen ? "fullscreen" : "exitFullscreen", { dedupe: false });
                }
                setSnapshot({
                    ...snapshot,
                    fullscreen,
                    error: null,
                });
                return clonePlaybackSnapshot(snapshot, options.session);
            });
        },
        async setViewability(viewability) {
            return runAction(async () => {
                await ensureReady();
                if (snapshot.viewability === viewability) {
                    return clonePlaybackSnapshot(snapshot, options.session);
                }
                if (snapshot.milestones.start) {
                    await dispatchLifecycleEvent(viewability, { dedupe: false });
                }
                setSnapshot({
                    ...snapshot,
                    viewability,
                    error: null,
                });
                return clonePlaybackSnapshot(snapshot, options.session);
            });
        },
        async click(trackOptions = {}) {
            return runAction(async () => {
                await ensureReady();
                const tracking = await options.session.track("clickTracking", {
                    ...trackOptions,
                    dedupe: false,
                });
                setSnapshot({
                    ...snapshot,
                    clickThroughUrl: snapshot.resolvedAd?.clickThroughUrl ?? null,
                    error: null,
                });
                return {
                    clickThroughUrl: snapshot.clickThroughUrl,
                    tracking,
                    snapshot: clonePlaybackSnapshot(snapshot, options.session),
                };
            });
        },
        async skip() {
            return runAction(async () => {
                await ensureReady();
                if (snapshot.skipped || snapshot.milestones.complete) {
                    return clonePlaybackSnapshot(snapshot, options.session);
                }
                await dispatchLifecycleEvent("skip", { dedupe: true });
                setSnapshot({
                    ...snapshot,
                    skipped: true,
                    status: "ended",
                    error: null,
                });
                return clonePlaybackSnapshot(snapshot, options.session);
            });
        },
        async signalError(trackOptions = {}) {
            return runAction(async () => {
                await ensureReady();
                await options.session.track("error", {
                    ...trackOptions,
                    dedupe: false,
                });
                setSnapshot({
                    ...snapshot,
                    status: "error",
                    error: "Playback error signaled.",
                });
                return clonePlaybackSnapshot(snapshot, options.session);
            });
        },
        getSnapshot() {
            return clonePlaybackSnapshot(snapshot, options.session);
        },
        subscribe(listener) {
            listeners.add(listener);
            listener(clonePlaybackSnapshot(snapshot, options.session));
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
