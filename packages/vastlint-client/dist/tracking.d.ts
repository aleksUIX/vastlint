import type { VastTrackableEvent, VastTrackingPlan, VastTrackingTarget, VastWrapperHop } from "./types.js";
type TrackingSourceHop = Pick<VastWrapperHop, "index" | "url" | "xml">;
export declare function createEmptyTrackingPlan(): VastTrackingPlan;
export declare function buildTrackingPlan(hops: readonly TrackingSourceHop[]): VastTrackingPlan;
export declare function selectTrackingTargets(plan: VastTrackingPlan, event: VastTrackableEvent, offset?: string): VastTrackingTarget[];
export declare function expandTrackingUrl(url: string, macros?: Record<string, string | number>): string;
export {};
