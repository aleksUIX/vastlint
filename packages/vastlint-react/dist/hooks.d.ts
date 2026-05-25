import type { UseVastAnnotationsOptions, UseVastPlaybackOptions, UseVastPlaybackQueueOptions, UseVastSessionOptions, UseVastTrackerOptions, VastAnnotationModel, VastPlaybackHookResult, VastPlaybackQueueHookResult, VastSessionHookResult, VastTrackerHookResult } from "./types.js";
export declare function useVastSession(options: UseVastSessionOptions): VastSessionHookResult;
export declare function useVastAnnotations(_options: UseVastAnnotationsOptions): VastAnnotationModel;
export declare function useVastPlayback(options: UseVastPlaybackOptions): VastPlaybackHookResult;
export declare function useVastPlaybackQueue(options: UseVastPlaybackQueueOptions): VastPlaybackQueueHookResult;
export declare function useVastTracker(options: UseVastTrackerOptions): VastTrackerHookResult;
