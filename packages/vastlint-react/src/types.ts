import type {
  VastPlaybackController,
  VastPlaybackControllerOptions,
  VastPlaybackSnapshot,
  VastPlaybackQueueController,
  VastPlaybackQueueControllerOptions,
  VastPlaybackQueueSnapshot,
  VastAdSelector,
  VastResolvedAd,
  VastSession,
  VastSessionOptions,
  VastSessionSnapshot,
  VastTrackOptions,
  VastTrackableEvent,
  VastTrackingDispatchResult,
  VastTrackingState,
  VastTrackingTarget,
} from "vastlint-client";
import type { FixResult, ValidationResult } from "vastlint";

export interface UseVastSessionOptions extends VastSessionOptions {
  autoLoad?: boolean;
  autoValidate?: boolean;
}

export interface VastAnnotation {
  id: string;
  issueId: string;
  severity: "error" | "warning" | "info";
  message: string;
  path: string | null;
  line: number | null;
  col: number | null;
}

export interface VastAnnotationModel {
  annotations: VastAnnotation[];
  byLine: Map<number, VastAnnotation[]>;
  byIssueId: Map<string, VastAnnotation[]>;
}

export interface UseVastAnnotationsOptions {
  xml: string;
  validation: ValidationResult | null;
}

export interface UseVastTrackerOptions {
  session: VastSession;
}

export interface UseVastPlaybackOptions extends VastPlaybackControllerOptions {
  autoInitialize?: boolean;
}

export interface UseVastPlaybackQueueOptions extends VastPlaybackQueueControllerOptions {
  autoInitialize?: boolean;
}

export interface VastTrackerHookResult {
  session: VastSession;
  tracking: VastTrackingState;
  resolvedAd: VastResolvedAd | null;
  resolvedAds: VastResolvedAd[];
  availableEvents: VastTrackableEvent[];
  clickThroughUrl: string | null;
  clickThroughUrls: string[];
  track(
    event: VastTrackableEvent,
    options?: VastTrackOptions,
  ): Promise<VastTrackingDispatchResult[]>;
  trackAd(
    adSelector: VastAdSelector,
    event: VastTrackableEvent,
    options?: VastTrackOptions,
  ): Promise<VastTrackingDispatchResult[]>;
  getTargets(event: VastTrackableEvent, options?: Pick<VastTrackOptions, "offset">): VastTrackingTarget[];
  getAdTargets(
    adSelector: VastAdSelector,
    event: VastTrackableEvent,
    options?: Pick<VastTrackOptions, "offset">,
  ): VastTrackingTarget[];
  hasTargets(event: VastTrackableEvent, options?: Pick<VastTrackOptions, "offset">): boolean;
  hasAdTargets(
    adSelector: VastAdSelector,
    event: VastTrackableEvent,
    options?: Pick<VastTrackOptions, "offset">,
  ): boolean;
}

export interface VastSessionHookResult {
  session: VastSession;
  snapshot: VastSessionSnapshot;
  load(): Promise<VastSessionSnapshot>;
  reload(): Promise<VastSessionSnapshot>;
  validate(): Promise<ValidationResult>;
  fix(): Promise<FixResult>;
  resolve(): Promise<VastSessionSnapshot>;
  track(
    event: VastTrackableEvent,
    options?: VastTrackOptions,
  ): Promise<VastTrackingDispatchResult[]>;
  trackAd(
    adSelector: VastAdSelector,
    event: VastTrackableEvent,
    options?: VastTrackOptions,
  ): Promise<VastTrackingDispatchResult[]>;
}

export interface VastPlaybackHookResult extends Pick<
  VastPlaybackController,
  | "initialize"
  | "start"
  | "pause"
  | "resume"
  | "updateProgress"
  | "complete"
  | "setMuted"
  | "setFullscreen"
  | "setViewability"
  | "click"
  | "skip"
  | "signalError"
> {
  controller: VastPlaybackController;
  snapshot: VastPlaybackSnapshot;
}

export interface VastPlaybackQueueHookResult extends Pick<
  VastPlaybackQueueController,
  | "initialize"
  | "start"
  | "pause"
  | "resume"
  | "updateProgress"
  | "completeCurrent"
  | "next"
  | "setMuted"
  | "setFullscreen"
  | "setViewability"
  | "click"
  | "skip"
  | "signalError"
> {
  controller: VastPlaybackQueueController;
  snapshot: VastPlaybackQueueSnapshot;
}