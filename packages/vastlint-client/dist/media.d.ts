import type { VastMediaFile, VastMediaSelectionCandidate, VastMediaSelectionOptions, VastMediaSelectionResult, VastResolvedAd } from "./types.js";
export declare function rankMediaFiles(mediaFiles: readonly VastMediaFile[], options?: VastMediaSelectionOptions): VastMediaSelectionCandidate[];
export declare function selectMediaFile(mediaFiles: readonly VastMediaFile[], options?: VastMediaSelectionOptions): VastMediaSelectionResult;
export declare function selectResolvedAdMediaFile(resolvedAd: VastResolvedAd | null, options?: VastMediaSelectionOptions): VastMediaSelectionResult;
