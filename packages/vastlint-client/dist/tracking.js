const XML_ENTITIES = {
    amp: "&",
    apos: "'",
    gt: ">",
    lt: "<",
    quot: '"',
};
const CLICK_TRACKING_TAGS = [
    "ClickTracking",
    "CompanionClickTracking",
    "IconClickTracking",
    "NonLinearClickTracking",
];
const CLICK_THROUGH_TAGS = [
    "ClickThrough",
    "CompanionClickThrough",
    "IconClickThrough",
    "NonLinearClickThrough",
];
const VIEWABILITY_TAGS = [
    ["Viewable", "viewable"],
    ["NotViewable", "notViewable"],
    ["ViewUndetermined", "viewUndetermined"],
];
export function createEmptyTrackingPlan() {
    return {
        impressions: [],
        errors: [],
        clickTrackings: [],
        clickThroughs: [],
        events: [],
    };
}
function normalizeWhitespace(value) {
    return value.replace(/\s+/g, " ").trim();
}
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
function cleanXmlText(value, collapseWhitespace = false) {
    const withoutCdata = value.replace(/<!\[CDATA\[([\s\S]*?)\]\]>/g, "$1");
    const decoded = decodeXmlEntities(withoutCdata);
    return collapseWhitespace ? normalizeWhitespace(decoded) : decoded.trim();
}
function extractAttribute(rawAttributes, attributeName) {
    const match = new RegExp(`${attributeName}=("([^"]*)"|'([^']*)')`, "i").exec(rawAttributes);
    if (!match) {
        return "";
    }
    return cleanXmlText(match[2] ?? match[3] ?? "", false);
}
function buildTarget(kind, event, rawValue, hop, offset = null) {
    const url = cleanXmlText(rawValue, false);
    if (!url) {
        return null;
    }
    return {
        kind,
        event,
        url,
        hopIndex: hop.index,
        sourceUrl: hop.url,
        offset,
    };
}
function collectTagTargets(xml, tagName, kind, event, hop) {
    const targets = [];
    const pattern = new RegExp(`<${tagName}\\b[^>]*>([\\s\\S]*?)<\\/${tagName}>`, "gi");
    for (const match of xml.matchAll(pattern)) {
        const target = buildTarget(kind, event, match[1] ?? "", hop);
        if (target) {
            targets.push(target);
        }
    }
    return targets;
}
function collectTrackingEvents(xml, hop) {
    const targets = [];
    const pattern = /<Tracking\b([^>]*)>([\s\S]*?)<\/Tracking>/gi;
    for (const match of xml.matchAll(pattern)) {
        const rawAttributes = match[1] ?? "";
        const event = extractAttribute(rawAttributes, "event");
        if (!event) {
            continue;
        }
        const offset = extractAttribute(rawAttributes, "offset") || null;
        const target = buildTarget("event", event, match[2] ?? "", hop, offset);
        if (target) {
            targets.push(target);
        }
    }
    return targets;
}
export function buildTrackingPlan(hops) {
    const plan = createEmptyTrackingPlan();
    for (const hop of hops) {
        plan.impressions.push(...collectTagTargets(hop.xml, "Impression", "impression", "impression", hop));
        plan.errors.push(...collectTagTargets(hop.xml, "Error", "error", "error", hop));
        for (const tagName of CLICK_TRACKING_TAGS) {
            plan.clickTrackings.push(...collectTagTargets(hop.xml, tagName, "clickTracking", "clickTracking", hop));
        }
        for (const tagName of CLICK_THROUGH_TAGS) {
            plan.clickThroughs.push(...collectTagTargets(hop.xml, tagName, "clickThrough", "clickThrough", hop));
        }
        for (const [tagName, eventName] of VIEWABILITY_TAGS) {
            plan.events.push(...collectTagTargets(hop.xml, tagName, "event", eventName, hop));
        }
        plan.events.push(...collectTrackingEvents(hop.xml, hop));
    }
    return plan;
}
export function selectTrackingTargets(plan, event, offset) {
    if (event === "impression") {
        return [...plan.impressions];
    }
    if (event === "error") {
        return [...plan.errors];
    }
    if (event === "clickTracking") {
        return [...plan.clickTrackings];
    }
    return plan.events.filter((target) => {
        if (target.event !== event) {
            return false;
        }
        if (offset === undefined) {
            return true;
        }
        return target.offset === offset;
    });
}
function randomCacheBustingValue() {
    return Math.floor(Math.random() * 100000000)
        .toString()
        .padStart(8, "0");
}
export function expandTrackingUrl(url, macros) {
    const values = {
        CACHEBUSTING: randomCacheBustingValue(),
        TIMESTAMP: new Date().toISOString(),
    };
    for (const [key, value] of Object.entries(macros ?? {})) {
        values[key.toUpperCase()] = String(value);
    }
    return url.replace(/\[([A-Z0-9_]+)\]|%%([A-Z0-9_]+)%%/gi, (match, bracketName, legacyName) => {
        const key = String(bracketName ?? legacyName ?? "").toUpperCase();
        const replacement = values[key];
        return replacement === undefined ? match : encodeURIComponent(replacement);
    });
}
