import type { FixResult, ValidateOptions, ValidationResult } from "vastlint";
export type VastSessionStatus = "idle" | "loading" | "ready" | "validating" | "resolved" | "fixing" | "error";
export type VastSessionEventType = "session:created" | "source:loading" | "source:loaded" | "validate:started" | "validate:completed" | "fix:started" | "fix:completed" | "resolve:started" | "resolve:hop" | "resolve:completed" | "track:started" | "track:completed" | "session:error";
export type VastAdType = "Wrapper" | "InLine" | "Unknown";
export type VastTrackableEvent = "impression" | "error" | "clickTracking" | "viewable" | "notViewable" | "viewUndetermined" | "creativeView" | "start" | "firstQuartile" | "midpoint" | "thirdQuartile" | "complete" | "mute" | "unmute" | "pause" | "resume" | "rewind" | "skip" | "playerExpand" | "playerCollapse" | "acceptInvitationLinear" | "closeLinear" | "close" | "progress" | "fullscreen" | "exitFullscreen" | (string & {});
export type VastTrackingEntryKind = "impression" | "error" | "clickTracking" | "clickThrough" | "event";
export interface VastTrackingTarget {
    kind: VastTrackingEntryKind;
    event: VastTrackableEvent | "clickThrough";
    url: string;
    hopIndex: number;
    sourceUrl: string | null;
    offset: string | null;
}
export interface VastTrackingPlan {
    impressions: VastTrackingTarget[];
    errors: VastTrackingTarget[];
    clickTrackings: VastTrackingTarget[];
    clickThroughs: VastTrackingTarget[];
    events: VastTrackingTarget[];
}
export interface VastTrackingDispatchResult {
    event: VastTrackableEvent;
    url: string;
    resolvedUrl: string;
    hopIndex: number;
    sourceUrl: string | null;
    offset: string | null;
    ok: boolean;
    status: number | null;
    dispatchedAt: string;
    error: string | null;
}
export interface VastTrackingState {
    plan: VastTrackingPlan;
    history: VastTrackingDispatchResult[];
}
export interface VastTrackOptions {
    macros?: Record<string, string | number>;
    offset?: string;
    dedupe?: boolean;
}
export interface VastAdIndexSelector {
    adIndex: number;
}
export interface VastAdIdSelector {
    adId: string;
}
export interface VastAdSequenceSelector {
    sequence: number;
}
export type VastAdSelector = number | VastAdIndexSelector | VastAdIdSelector | VastAdSequenceSelector;
export interface VastXmlSource {
    kind: "xml";
    xml: string;
    label?: string;
}
export interface VastUrlSource {
    kind: "url";
    url: string;
    request?: RequestInit;
    label?: string;
}
export type VastSessionSource = VastXmlSource | VastUrlSource;
export interface VastSessionEvent {
    type: VastSessionEventType;
    timestamp: string;
    detail?: Record<string, unknown>;
}
export interface VastMediaFile {
    url: string;
    mimeType: string;
    delivery: string;
    width: string;
    height: string;
    bitrate: string;
}
export type VastCreativeResourceKind = "static" | "iframe" | "html";
export interface VastCreativeResource {
    kind: VastCreativeResourceKind;
    content: string;
    creativeType: string | null;
    xmlEncoded: string | null;
}
export interface VastCompanionAd {
    id: string | null;
    width: string;
    height: string;
    assetWidth: string | null;
    assetHeight: string | null;
    expandedWidth: string | null;
    expandedHeight: string | null;
    apiFramework: string | null;
    adSlotId: string | null;
    pxratio: string | null;
    renderingMode: string | null;
    language: string | null;
    resources: VastCreativeResource[];
    clickThroughUrl: string | null;
    clickTrackingUrls: string[];
    trackingEvents: Record<string, string[]>;
}
export interface VastIcon {
    program: string | null;
    width: string;
    height: string;
    xPosition: string | null;
    yPosition: string | null;
    offset: string | null;
    duration: string | null;
    apiFramework: string | null;
    pxratio: string | null;
    resources: VastCreativeResource[];
    clickThroughUrl: string | null;
    clickTrackingUrls: string[];
    viewTrackingUrls: string[];
}
export interface VastUniversalAdId {
    creativeId: string | null;
    creativeIndex: number;
    idRegistry: string | null;
    idValue: string | null;
    value: string;
}
export interface VastCategory {
    authority: string | null;
    value: string;
}
export type VastVerificationResourceKind = "javascript" | "executable";
export interface VastVerificationResource {
    kind: VastVerificationResourceKind;
    url: string;
    apiFramework: string | null;
    mimeType: string | null;
    browserOptional: string | null;
}
export interface VastAdVerification {
    vendor: string | null;
    resources: VastVerificationResource[];
    verificationParameters: string | null;
}
export interface VastAdPodMetadata {
    adId: string | null;
    sequence: number | null;
    adType: string | null;
    adServingId: string | null;
    isAdPod: boolean;
}
export interface VastMediaSelectionOptions {
    supportedMimeTypes?: string[];
    preferredMimeTypes?: string[];
    preferredDelivery?: string[];
    targetBitrate?: number;
    maxBitrate?: number;
    targetWidth?: number;
    targetHeight?: number;
}
export interface VastMediaSelectionCandidate {
    mediaFile: VastMediaFile;
    score: number;
    reasons: string[];
}
export interface VastMediaSelectionResult {
    selected: VastMediaFile | null;
    candidates: VastMediaSelectionCandidate[];
}
export interface VastWrapperHop {
    index: number;
    source: VastSessionSource;
    url: string | null;
    xml: string;
    fetchedAt: string;
    fetchMs: number;
    adType: VastAdType;
    adSystem: string;
    adTitle: string;
    duration: string;
    impressionCount: number;
    trackingEventCount: number;
    companionCount: number;
    mediaFiles: VastMediaFile[];
    wrapperUri: string | null;
    validation: ValidationResult | null;
}
export interface VastResolutionSummary {
    hopCount: number;
    resolved: boolean;
    chainValid: boolean;
    totalErrors: number;
    totalWarnings: number;
    totalInfos: number;
    stoppedReason: string | null;
}
export interface VastResolvedAd {
    resolved: boolean;
    finalHopIndex: number | null;
    finalUrl: string | null;
    adType: VastAdType;
    adSystem: string;
    adTitle: string;
    duration: string;
    skipOffset: string | null;
    mediaFiles: VastMediaFile[];
    companions: VastCompanionAd[];
    icons: VastIcon[];
    universalAdIds: VastUniversalAdId[];
    categories: VastCategory[];
    adVerifications: VastAdVerification[];
    adPod: VastAdPodMetadata;
    impressionUrls: string[];
    errorUrls: string[];
    clickTrackingUrls: string[];
    clickThroughUrls: string[];
    clickThroughUrl: string | null;
    trackingPlan: VastTrackingPlan;
    trackingEvents: Record<string, string[]>;
    companionCount: number;
    wrapperHopCount: number;
    stoppedReason: string | null;
}
export interface VastSessionSnapshot {
    status: VastSessionStatus;
    source: VastSessionSource;
    xml: string | null;
    rootXml: string | null;
    validation: ValidationResult | null;
    fixed: FixResult | null;
    wrapperChain: VastWrapperHop[];
    resolution: VastResolutionSummary | null;
    tracking: VastTrackingState;
    resolvedAd: VastResolvedAd | null;
    resolvedAds: VastResolvedAd[];
    events: VastSessionEvent[];
    error: Error | null;
}
export interface VastSessionOptions {
    source: VastSessionSource;
    validateOptions?: ValidateOptions;
    fetch?: typeof globalThis.fetch;
    timeoutMs?: number;
    maxWrapperDepth?: number;
}
export interface VastSession {
    load(): Promise<VastSessionSnapshot>;
    validate(): Promise<ValidationResult>;
    fix(): Promise<FixResult>;
    resolve(): Promise<VastSessionSnapshot>;
    getTracking(): VastTrackingState;
    getAdTrackingTargets(adSelector: VastAdSelector, event: VastTrackableEvent, options?: Pick<VastTrackOptions, "offset">): VastTrackingTarget[];
    track(event: VastTrackableEvent, options?: VastTrackOptions): Promise<VastTrackingDispatchResult[]>;
    trackAd(adSelector: VastAdSelector, event: VastTrackableEvent, options?: VastTrackOptions): Promise<VastTrackingDispatchResult[]>;
    getSnapshot(): VastSessionSnapshot;
    subscribe(listener: (snapshot: VastSessionSnapshot) => void): () => void;
}
export type VastPlaybackStatus = "idle" | "ready" | "playing" | "paused" | "ended" | "error";
export type VastPlaybackViewability = "viewable" | "notViewable" | "viewUndetermined";
export interface VastPlaybackMilestones {
    start: boolean;
    firstQuartile: boolean;
    midpoint: boolean;
    thirdQuartile: boolean;
    complete: boolean;
}
export interface VastPlaybackSnapshot {
    status: VastPlaybackStatus;
    session: VastSessionSnapshot;
    resolvedAd: VastResolvedAd | null;
    mediaSelection: VastMediaSelectionResult;
    currentTimeSec: number;
    durationSec: number | null;
    impressionTracked: boolean;
    muted: boolean;
    fullscreen: boolean;
    skipped: boolean;
    viewability: VastPlaybackViewability | null;
    milestones: VastPlaybackMilestones;
    clickThroughUrl: string | null;
    error: string | null;
}
export interface VastPlaybackControllerOptions {
    session: VastSession;
    mediaSelection?: VastMediaSelectionOptions;
    autoResolve?: boolean;
}
export interface VastPlaybackClickResult {
    clickThroughUrl: string | null;
    tracking: VastTrackingDispatchResult[];
    snapshot: VastPlaybackSnapshot;
}
export interface VastPlaybackQueueItem {
    adIndex: number;
    resolvedAd: VastResolvedAd;
    mediaSelection: VastMediaSelectionResult;
    currentTimeSec: number;
    durationSec: number | null;
    impressionTracked: boolean;
    skipped: boolean;
    milestones: VastPlaybackMilestones;
    status: VastPlaybackStatus;
    clickThroughUrl: string | null;
    error: string | null;
}
export interface VastPlaybackQueueSnapshot {
    status: VastPlaybackStatus;
    session: VastSessionSnapshot;
    resolvedAds: VastResolvedAd[];
    items: VastPlaybackQueueItem[];
    currentAdIndex: number | null;
    currentItem: VastPlaybackQueueItem | null;
    hasNext: boolean;
    muted: boolean;
    fullscreen: boolean;
    viewability: VastPlaybackViewability | null;
    error: string | null;
}
export interface VastPlaybackQueueControllerOptions {
    session: VastSession;
    mediaSelection?: VastMediaSelectionOptions;
    autoResolve?: boolean;
    fetch?: typeof globalThis.fetch;
}
export interface VastPlaybackQueueClickResult {
    clickThroughUrl: string | null;
    tracking: VastTrackingDispatchResult[];
    snapshot: VastPlaybackQueueSnapshot;
}
export interface VastPlaybackQueueController {
    initialize(): Promise<VastPlaybackQueueSnapshot>;
    start(): Promise<VastPlaybackQueueSnapshot>;
    pause(): Promise<VastPlaybackQueueSnapshot>;
    resume(): Promise<VastPlaybackQueueSnapshot>;
    updateProgress(currentTimeSec: number, durationSec?: number): Promise<VastPlaybackQueueSnapshot>;
    completeCurrent(): Promise<VastPlaybackQueueSnapshot>;
    next(): Promise<VastPlaybackQueueSnapshot>;
    setMuted(muted: boolean): Promise<VastPlaybackQueueSnapshot>;
    setFullscreen(fullscreen: boolean): Promise<VastPlaybackQueueSnapshot>;
    setViewability(viewability: VastPlaybackViewability): Promise<VastPlaybackQueueSnapshot>;
    click(options?: VastTrackOptions): Promise<VastPlaybackQueueClickResult>;
    skip(): Promise<VastPlaybackQueueSnapshot>;
    signalError(options?: VastTrackOptions): Promise<VastPlaybackQueueSnapshot>;
    getSnapshot(): VastPlaybackQueueSnapshot;
    subscribe(listener: (snapshot: VastPlaybackQueueSnapshot) => void): () => void;
    dispose(): void;
}
export interface VastPlaybackController {
    initialize(): Promise<VastPlaybackSnapshot>;
    start(): Promise<VastPlaybackSnapshot>;
    pause(): Promise<VastPlaybackSnapshot>;
    resume(): Promise<VastPlaybackSnapshot>;
    updateProgress(currentTimeSec: number, durationSec?: number): Promise<VastPlaybackSnapshot>;
    complete(): Promise<VastPlaybackSnapshot>;
    setMuted(muted: boolean): Promise<VastPlaybackSnapshot>;
    setFullscreen(fullscreen: boolean): Promise<VastPlaybackSnapshot>;
    setViewability(viewability: VastPlaybackViewability): Promise<VastPlaybackSnapshot>;
    click(options?: VastTrackOptions): Promise<VastPlaybackClickResult>;
    skip(): Promise<VastPlaybackSnapshot>;
    signalError(options?: VastTrackOptions): Promise<VastPlaybackSnapshot>;
    getSnapshot(): VastPlaybackSnapshot;
    subscribe(listener: (snapshot: VastPlaybackSnapshot) => void): () => void;
    dispose(): void;
}
