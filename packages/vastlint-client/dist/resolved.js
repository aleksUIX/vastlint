import { buildTrackingPlan, createEmptyTrackingPlan } from "./tracking.js";
const XML_ENTITIES = {
    amp: "&",
    apos: "'",
    gt: ">",
    lt: "<",
    quot: '"',
};
function decodeXmlEntities(value) {
    return value.replace(/&(#x?[0-9a-fA-F]+|amp|apos|gt|lt|quot);/g, (_match, entity) => {
        if (entity in XML_ENTITIES) {
            return XML_ENTITIES[entity];
        }
        if (entity.startsWith("#x")) {
            return String.fromCodePoint(Number.parseInt(entity.slice(2), 16));
        }
        if (entity.startsWith("#")) {
            return String.fromCodePoint(Number.parseInt(entity.slice(1), 10));
        }
        return `&${entity};`;
    });
}
function cleanXmlText(value) {
    const withoutCdata = value.replace(/<!\[CDATA\[([\s\S]*?)\]\]>/g, "$1");
    return decodeXmlEntities(withoutCdata).trim();
}
function extractAttribute(rawAttributes, attributeName) {
    const match = new RegExp(`${attributeName}=("([^"]*)"|'([^']*)')`, "i").exec(rawAttributes);
    if (!match) {
        return "";
    }
    return cleanXmlText(match[2] ?? match[3] ?? "");
}
function extractSkipOffset(xml) {
    const match = /<Linear\b([^>]*)>/i.exec(xml);
    if (!match) {
        return null;
    }
    return extractAttribute(match[1] ?? "", "skipoffset") || null;
}
function uniqueUrls(values) {
    return [...new Set(values.filter((value) => value.length > 0))];
}
function groupTrackingEvents(plan) {
    const grouped = {};
    for (const target of plan.events) {
        const key = String(target.event);
        grouped[key] ??= [];
        grouped[key].push(target.url);
    }
    return Object.fromEntries(Object.entries(grouped).map(([event, urls]) => [event, uniqueUrls(urls)]));
}
function cloneMediaFiles(mediaFiles) {
    return mediaFiles.map((mediaFile) => ({ ...mediaFile }));
}
function uniqueValues(values) {
    return [...new Set(values.filter((value) => value.length > 0))];
}
function mergeTrackingEventMaps(...eventMaps) {
    const grouped = {};
    for (const eventMap of eventMaps) {
        for (const [event, urls] of Object.entries(eventMap)) {
            grouped[event] ??= [];
            grouped[event].push(...urls);
        }
    }
    return Object.fromEntries(Object.entries(grouped).map(([event, urls]) => [event, uniqueValues(urls)]));
}
function collectTagTexts(xml, tagName) {
    const values = [];
    const pattern = new RegExp(`<${tagName}\\b[^>]*>([\\s\\S]*?)<\\/${tagName}>`, "gi");
    for (const match of xml.matchAll(pattern)) {
        const value = cleanXmlText(match[1] ?? "");
        if (value) {
            values.push(value);
        }
    }
    return values;
}
function collectTrackingEventMap(xml) {
    const grouped = {};
    const pattern = /<Tracking\b([^>]*)>([\s\S]*?)<\/Tracking>/gi;
    for (const match of xml.matchAll(pattern)) {
        const rawAttributes = match[1] ?? "";
        const event = extractAttribute(rawAttributes, "event");
        const url = cleanXmlText(match[2] ?? "");
        if (!event || !url) {
            continue;
        }
        grouped[event] ??= [];
        grouped[event].push(url);
    }
    return Object.fromEntries(Object.entries(grouped).map(([event, urls]) => [event, uniqueValues(urls)]));
}
function collectAdSegments(xml) {
    const segments = [];
    const pattern = /<Ad\b([^>]*)>([\s\S]*?)<\/Ad>/gi;
    let documentIndex = 0;
    for (const match of xml.matchAll(pattern)) {
        segments.push({
            xml: match[0] ?? "",
            rawAttributes: match[1] ?? "",
            body: match[2] ?? "",
            documentIndex,
        });
        documentIndex += 1;
    }
    return segments;
}
function extractMediaFiles(xml) {
    const mediaFiles = [];
    const pattern = /<MediaFile\b([^>]*)>([\s\S]*?)<\/MediaFile>/gi;
    for (const match of xml.matchAll(pattern)) {
        const rawAttributes = match[1] ?? "";
        const url = cleanXmlText(match[2] ?? "");
        if (!url) {
            continue;
        }
        mediaFiles.push({
            url,
            mimeType: extractAttribute(rawAttributes, "type"),
            delivery: extractAttribute(rawAttributes, "delivery"),
            width: extractAttribute(rawAttributes, "width"),
            height: extractAttribute(rawAttributes, "height"),
            bitrate: extractAttribute(rawAttributes, "bitrate"),
        });
    }
    return mediaFiles;
}
function extractTrackingSurface(xml) {
    const trackingEvents = collectTrackingEventMap(xml);
    const viewable = collectTagTexts(xml, "Viewable");
    const notViewable = collectTagTexts(xml, "NotViewable");
    const viewUndetermined = collectTagTexts(xml, "ViewUndetermined");
    return {
        impressionUrls: uniqueValues(collectTagTexts(xml, "Impression")),
        errorUrls: uniqueValues(collectTagTexts(xml, "Error")),
        clickTrackingUrls: uniqueValues([
            ...collectTagTexts(xml, "ClickTracking"),
            ...collectTagTexts(xml, "CompanionClickTracking"),
            ...collectTagTexts(xml, "IconClickTracking"),
            ...collectTagTexts(xml, "NonLinearClickTracking"),
        ]),
        clickThroughUrls: uniqueValues([
            ...collectTagTexts(xml, "ClickThrough"),
            ...collectTagTexts(xml, "CompanionClickThrough"),
            ...collectTagTexts(xml, "IconClickThrough"),
            ...collectTagTexts(xml, "NonLinearClickThrough"),
        ]),
        trackingEvents: mergeTrackingEventMaps(trackingEvents, viewable.length ? { viewable } : {}, notViewable.length ? { notViewable } : {}, viewUndetermined.length ? { viewUndetermined } : {}),
    };
}
function mergeTrackingSurfaces(...surfaces) {
    return {
        impressionUrls: uniqueValues(surfaces.flatMap((surface) => surface.impressionUrls)),
        errorUrls: uniqueValues(surfaces.flatMap((surface) => surface.errorUrls)),
        clickTrackingUrls: uniqueValues(surfaces.flatMap((surface) => surface.clickTrackingUrls)),
        clickThroughUrls: uniqueValues(surfaces.flatMap((surface) => surface.clickThroughUrls)),
        trackingEvents: mergeTrackingEventMaps(...surfaces.map((surface) => surface.trackingEvents)),
    };
}
function extractAdType(xml) {
    if (/<Wrapper\b/i.test(xml)) {
        return "Wrapper";
    }
    if (/<InLine\b/i.test(xml)) {
        return "InLine";
    }
    return "Unknown";
}
function extractSequence(rawAttributes) {
    const sequenceValue = extractAttribute(rawAttributes, "sequence");
    if (!sequenceValue) {
        return null;
    }
    const parsed = Number.parseInt(sequenceValue, 10);
    return Number.isFinite(parsed) ? parsed : null;
}
function sortAdSegments(segments) {
    return [...segments].sort((left, right) => {
        const leftSequence = extractSequence(left.rawAttributes);
        const rightSequence = extractSequence(right.rawAttributes);
        if (leftSequence !== null && rightSequence !== null && leftSequence !== rightSequence) {
            return leftSequence - rightSequence;
        }
        if (leftSequence !== null && rightSequence === null) {
            return -1;
        }
        if (leftSequence === null && rightSequence !== null) {
            return 1;
        }
        return left.documentIndex - right.documentIndex;
    });
}
function buildAdTrackingPlan(wrapperChain, finalSegmentXml) {
    const hops = [
        ...wrapperChain.slice(0, -1).map((hop) => ({
            index: hop.index,
            url: hop.url,
            xml: hop.xml,
        })),
        {
            index: wrapperChain[wrapperChain.length - 1]?.index ?? 0,
            url: wrapperChain[wrapperChain.length - 1]?.url ?? null,
            xml: finalSegmentXml,
        },
    ];
    return buildTrackingPlan(hops);
}
function collectOpenCloseElements(xml, tagName) {
    const elements = [];
    const pattern = new RegExp(`<${tagName}\\b([^>]*)>([\\s\\S]*?)<\\/${tagName}>`, "gi");
    for (const match of xml.matchAll(pattern)) {
        elements.push({
            rawAttributes: match[1] ?? "",
            body: match[2] ?? "",
        });
    }
    return elements;
}
function collectSelfClosingOrOpenCloseElements(xml, tagName) {
    const elements = [];
    const pattern = new RegExp(`<${tagName}\\b([^>]*?)(?:>([\\s\\S]*?)<\\/${tagName}>|\\s*\\/>)`, "gi");
    for (const match of xml.matchAll(pattern)) {
        elements.push({
            rawAttributes: match[1] ?? "",
            body: match[2] ?? "",
        });
    }
    return elements;
}
function collectCreativeResources(xml) {
    const resources = [];
    const resourceTags = [
        { tagName: "StaticResource", kind: "static" },
        { tagName: "IFrameResource", kind: "iframe" },
        { tagName: "HTMLResource", kind: "html" },
    ];
    for (const { tagName, kind } of resourceTags) {
        const pattern = new RegExp(`<${tagName}\\b([^>]*)>([\\s\\S]*?)<\\/${tagName}>`, "gi");
        for (const match of xml.matchAll(pattern)) {
            const rawAttributes = match[1] ?? "";
            const content = cleanXmlText(match[2] ?? "");
            if (!content) {
                continue;
            }
            resources.push({
                kind,
                content,
                creativeType: extractAttribute(rawAttributes, "creativeType") || null,
                xmlEncoded: extractAttribute(rawAttributes, "xmlEncoded") || null,
            });
        }
    }
    return resources;
}
function extractUniversalAdIds(xml) {
    const universalAdIds = [];
    const creativePattern = /<Creative\b([^>]*)>([\s\S]*?)<\/Creative>/gi;
    let creativeIndex = 0;
    for (const creativeMatch of xml.matchAll(creativePattern)) {
        const creativeAttributes = creativeMatch[1] ?? "";
        const creativeBody = creativeMatch[2] ?? "";
        const creativeId = extractAttribute(creativeAttributes, "id") || null;
        for (const universalAdId of collectSelfClosingOrOpenCloseElements(creativeBody, "UniversalAdId")) {
            const idRegistry = extractAttribute(universalAdId.rawAttributes, "idRegistry") || null;
            const idValue = extractAttribute(universalAdId.rawAttributes, "idValue") || null;
            const value = cleanXmlText(universalAdId.body) || idValue || "";
            if (!value && !idRegistry && !idValue) {
                continue;
            }
            universalAdIds.push({
                creativeId,
                creativeIndex,
                idRegistry,
                idValue,
                value,
            });
        }
        creativeIndex += 1;
    }
    return universalAdIds;
}
function extractCategories(xml) {
    return collectOpenCloseElements(xml, "Category")
        .map((category) => ({
        authority: extractAttribute(category.rawAttributes, "authority") || null,
        value: cleanXmlText(category.body),
    }))
        .filter((category) => category.value.length > 0 || category.authority !== null);
}
function extractVerificationResources(xml) {
    const resources = [];
    const resourceTags = [
        { tagName: "JavaScriptResource", kind: "javascript" },
        { tagName: "ExecutableResource", kind: "executable" },
    ];
    for (const { tagName, kind } of resourceTags) {
        for (const resource of collectOpenCloseElements(xml, tagName)) {
            const url = cleanXmlText(resource.body);
            if (!url) {
                continue;
            }
            resources.push({
                kind,
                url,
                apiFramework: extractAttribute(resource.rawAttributes, "apiFramework") || null,
                mimeType: extractAttribute(resource.rawAttributes, "type") || null,
                browserOptional: extractAttribute(resource.rawAttributes, "browserOptional") || null,
            });
        }
    }
    return resources;
}
function extractAdVerifications(xml) {
    return collectOpenCloseElements(xml, "Verification")
        .map((verification) => ({
        vendor: extractAttribute(verification.rawAttributes, "vendor") || null,
        resources: extractVerificationResources(verification.body),
        verificationParameters: collectTagTexts(verification.body, "VerificationParameters")[0] ?? null,
    }))
        .filter((verification) => verification.vendor !== null
        || verification.resources.length > 0
        || verification.verificationParameters !== null);
}
function extractAdPodMetadata(xml) {
    const adMatch = /<Ad\b([^>]*)>/i.exec(xml);
    const rawAttributes = adMatch?.[1] ?? "";
    const adId = extractAttribute(rawAttributes, "id") || null;
    const sequenceValue = extractAttribute(rawAttributes, "sequence");
    const sequence = sequenceValue ? Number.parseInt(sequenceValue, 10) : Number.NaN;
    return {
        adId,
        sequence: Number.isFinite(sequence) ? sequence : null,
        adType: extractAttribute(rawAttributes, "adType") || null,
        adServingId: collectTagTexts(xml, "AdServingId")[0] ?? null,
        isAdPod: Number.isFinite(sequence),
    };
}
function extractCompanions(xml) {
    const companions = [];
    const pattern = /<Companion\b([^>]*)>([\s\S]*?)<\/Companion>/gi;
    for (const match of xml.matchAll(pattern)) {
        const rawAttributes = match[1] ?? "";
        const body = match[2] ?? "";
        companions.push({
            id: extractAttribute(rawAttributes, "id") || null,
            width: extractAttribute(rawAttributes, "width"),
            height: extractAttribute(rawAttributes, "height"),
            assetWidth: extractAttribute(rawAttributes, "assetWidth") || null,
            assetHeight: extractAttribute(rawAttributes, "assetHeight") || null,
            expandedWidth: extractAttribute(rawAttributes, "expandedWidth") || null,
            expandedHeight: extractAttribute(rawAttributes, "expandedHeight") || null,
            apiFramework: extractAttribute(rawAttributes, "apiFramework") || null,
            adSlotId: extractAttribute(rawAttributes, "adSlotId") || null,
            pxratio: extractAttribute(rawAttributes, "pxratio") || null,
            renderingMode: extractAttribute(rawAttributes, "renderingMode") || null,
            language: extractAttribute(rawAttributes, "lang") || extractAttribute(rawAttributes, "language") || null,
            resources: collectCreativeResources(body),
            clickThroughUrl: collectTagTexts(body, "CompanionClickThrough")[0] ?? null,
            clickTrackingUrls: uniqueValues(collectTagTexts(body, "CompanionClickTracking")),
            trackingEvents: collectTrackingEventMap(body),
        });
    }
    return companions;
}
function extractIcons(xml) {
    const icons = [];
    const pattern = /<Icon\b([^>]*)>([\s\S]*?)<\/Icon>/gi;
    for (const match of xml.matchAll(pattern)) {
        const rawAttributes = match[1] ?? "";
        const body = match[2] ?? "";
        icons.push({
            program: extractAttribute(rawAttributes, "program") || null,
            width: extractAttribute(rawAttributes, "width"),
            height: extractAttribute(rawAttributes, "height"),
            xPosition: extractAttribute(rawAttributes, "xPosition") || null,
            yPosition: extractAttribute(rawAttributes, "yPosition") || null,
            offset: extractAttribute(rawAttributes, "offset") || null,
            duration: extractAttribute(rawAttributes, "duration") || null,
            apiFramework: extractAttribute(rawAttributes, "apiFramework") || null,
            pxratio: extractAttribute(rawAttributes, "pxratio") || null,
            resources: collectCreativeResources(body),
            clickThroughUrl: collectTagTexts(body, "IconClickThrough")[0] ?? null,
            clickTrackingUrls: uniqueValues(collectTagTexts(body, "IconClickTracking")),
            viewTrackingUrls: uniqueValues(collectTagTexts(body, "IconViewTracking")),
        });
    }
    return icons;
}
function buildResolvedAdFromSegment(segment, wrapperChain, lastHop, stoppedReason) {
    const mediaFiles = extractMediaFiles(segment.xml);
    const companions = extractCompanions(segment.xml);
    const icons = extractIcons(segment.xml);
    const universalAdIds = extractUniversalAdIds(segment.xml);
    const categories = extractCategories(segment.xml);
    const adVerifications = extractAdVerifications(segment.xml);
    const adPod = extractAdPodMetadata(segment.xml);
    const trackingPlan = buildAdTrackingPlan(wrapperChain, segment.xml);
    const adType = extractAdType(segment.xml);
    return {
        resolved: adType === "InLine",
        finalHopIndex: lastHop.index,
        finalUrl: lastHop.url,
        adType,
        adSystem: collectTagTexts(segment.xml, "AdSystem")[0] ?? "",
        adTitle: collectTagTexts(segment.xml, "AdTitle")[0] ?? "",
        duration: collectTagTexts(segment.xml, "Duration")[0] ?? "",
        skipOffset: extractSkipOffset(segment.xml),
        mediaFiles: cloneMediaFiles(mediaFiles),
        companions,
        icons,
        universalAdIds,
        categories,
        adVerifications,
        adPod,
        impressionUrls: uniqueUrls(trackingPlan.impressions.map((target) => target.url)),
        errorUrls: uniqueUrls(trackingPlan.errors.map((target) => target.url)),
        clickTrackingUrls: uniqueUrls(trackingPlan.clickTrackings.map((target) => target.url)),
        clickThroughUrls: uniqueUrls(trackingPlan.clickThroughs.map((target) => target.url)),
        clickThroughUrl: trackingPlan.clickThroughs[trackingPlan.clickThroughs.length - 1]?.url ?? null,
        trackingPlan,
        trackingEvents: groupTrackingEvents(trackingPlan),
        companionCount: companions.length,
        wrapperHopCount: Math.max(0, lastHop.index),
        stoppedReason,
    };
}
function emptyResolvedAd(stoppedReason) {
    return {
        resolved: false,
        finalHopIndex: null,
        finalUrl: null,
        adType: "Unknown",
        adSystem: "",
        adTitle: "",
        duration: "",
        skipOffset: null,
        mediaFiles: [],
        companions: [],
        icons: [],
        universalAdIds: [],
        categories: [],
        adVerifications: [],
        adPod: {
            adId: null,
            sequence: null,
            adType: null,
            adServingId: null,
            isAdPod: false,
        },
        impressionUrls: [],
        errorUrls: [],
        clickTrackingUrls: [],
        clickThroughUrls: [],
        clickThroughUrl: null,
        trackingPlan: createEmptyTrackingPlan(),
        trackingEvents: {},
        companionCount: 0,
        wrapperHopCount: 0,
        stoppedReason,
    };
}
export function buildResolvedState(wrapperChain, resolution) {
    if (wrapperChain.length === 0) {
        return {
            resolvedAd: null,
            resolvedAds: [],
        };
    }
    const lastHop = wrapperChain[wrapperChain.length - 1] ?? null;
    if (!lastHop) {
        return {
            resolvedAd: null,
            resolvedAds: [],
        };
    }
    const stoppedReason = resolution?.stoppedReason ?? null;
    const adSegments = sortAdSegments(collectAdSegments(lastHop.xml));
    const resolvedAds = adSegments.map((segment) => buildResolvedAdFromSegment(segment, wrapperChain, lastHop, stoppedReason));
    return {
        resolvedAd: resolvedAds[0] ?? emptyResolvedAd(stoppedReason),
        resolvedAds,
    };
}
export function buildResolvedAd(wrapperChain, resolution) {
    return buildResolvedState(wrapperChain, resolution).resolvedAd;
}
