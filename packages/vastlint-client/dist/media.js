function normalizeValue(value) {
    return value.trim().toLowerCase();
}
function parseOptionalNumber(value) {
    const parsed = Number.parseInt(value, 10);
    return Number.isFinite(parsed) ? parsed : null;
}
function buildPreferredIndex(values) {
    return new Map((values ?? []).map((value, index) => [normalizeValue(value), index]));
}
function sortCandidates(left, right) {
    return right.score - left.score || left.mediaFile.url.localeCompare(right.mediaFile.url);
}
function scoreDistance(target, actual, weight) {
    if (target === undefined || actual === null) {
        return 0;
    }
    return Math.max(0, weight - Math.abs(target - actual));
}
function scoreMediaFile(mediaFile, options, preferredMimeTypes, preferredDelivery) {
    const reasons = [];
    let score = 0;
    const mimeType = normalizeValue(mediaFile.mimeType);
    const delivery = normalizeValue(mediaFile.delivery);
    const width = parseOptionalNumber(mediaFile.width);
    const height = parseOptionalNumber(mediaFile.height);
    const bitrate = parseOptionalNumber(mediaFile.bitrate);
    const supportedMimeTypes = (options.supportedMimeTypes ?? []).map(normalizeValue);
    if (supportedMimeTypes.length > 0) {
        if (!mimeType || !supportedMimeTypes.includes(mimeType)) {
            return null;
        }
        score += 400;
        reasons.push(`supported MIME type ${mimeType}`);
    }
    const preferredMimeIndex = preferredMimeTypes.get(mimeType);
    if (preferredMimeIndex !== undefined) {
        score += 300 - preferredMimeIndex * 25;
        reasons.push(`preferred MIME type ${mimeType}`);
    }
    else if (mimeType.startsWith("video/")) {
        score += 75;
        reasons.push(`video MIME type ${mimeType}`);
    }
    const preferredDeliveryIndex = preferredDelivery.get(delivery);
    if (preferredDeliveryIndex !== undefined) {
        score += 120 - preferredDeliveryIndex * 15;
        reasons.push(`preferred delivery ${delivery}`);
    }
    else if (delivery) {
        score += 20;
        reasons.push(`delivery ${delivery}`);
    }
    if (bitrate !== null) {
        score += 25;
        reasons.push(`declares bitrate ${bitrate}`);
    }
    if (width !== null && height !== null) {
        score += 25;
        reasons.push(`declares dimensions ${width}x${height}`);
    }
    if (options.maxBitrate !== undefined && bitrate !== null) {
        if (bitrate > options.maxBitrate) {
            score -= 200;
            reasons.push(`bitrate ${bitrate} exceeds max ${options.maxBitrate}`);
        }
        else {
            score += 40;
            reasons.push(`bitrate ${bitrate} is within max ${options.maxBitrate}`);
        }
    }
    const bitrateScore = scoreDistance(options.targetBitrate, bitrate, 160);
    if (bitrateScore > 0 && bitrate !== null && options.targetBitrate !== undefined) {
        score += bitrateScore;
        reasons.push(`bitrate ${bitrate} is close to target ${options.targetBitrate}`);
    }
    const widthScore = scoreDistance(options.targetWidth, width, 120);
    if (widthScore > 0 && width !== null && options.targetWidth !== undefined) {
        score += widthScore;
        reasons.push(`width ${width} is close to target ${options.targetWidth}`);
    }
    const heightScore = scoreDistance(options.targetHeight, height, 120);
    if (heightScore > 0 && height !== null && options.targetHeight !== undefined) {
        score += heightScore;
        reasons.push(`height ${height} is close to target ${options.targetHeight}`);
    }
    if (mediaFile.url) {
        score += 5;
    }
    return {
        mediaFile: { ...mediaFile },
        score,
        reasons,
    };
}
export function rankMediaFiles(mediaFiles, options = {}) {
    const preferredMimeTypes = buildPreferredIndex(options.preferredMimeTypes);
    const preferredDelivery = buildPreferredIndex(options.preferredDelivery);
    return mediaFiles
        .map((mediaFile) => scoreMediaFile(mediaFile, options, preferredMimeTypes, preferredDelivery))
        .filter((candidate) => candidate !== null)
        .sort(sortCandidates);
}
export function selectMediaFile(mediaFiles, options = {}) {
    const candidates = rankMediaFiles(mediaFiles, options);
    return {
        selected: candidates[0]?.mediaFile ?? null,
        candidates,
    };
}
export function selectResolvedAdMediaFile(resolvedAd, options = {}) {
    return selectMediaFile(resolvedAd?.mediaFiles ?? [], options);
}
