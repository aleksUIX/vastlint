import type { VastResolvedAd, VastResolutionSummary, VastWrapperHop } from "./types.js";
export interface VastResolvedState {
    resolvedAd: VastResolvedAd | null;
    resolvedAds: VastResolvedAd[];
}
export declare function buildResolvedState(wrapperChain: readonly VastWrapperHop[], resolution: VastResolutionSummary | null): VastResolvedState;
export declare function buildResolvedAd(wrapperChain: readonly VastWrapperHop[], resolution: VastResolutionSummary | null): VastResolvedAd | null;
