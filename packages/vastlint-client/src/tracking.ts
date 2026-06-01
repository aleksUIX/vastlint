import type {
  VastTrackableEvent,
  VastTrackingPlan,
  VastTrackingTarget,
  VastWrapperHop,
} from "./types.js";

type TrackingSourceHop = Pick<VastWrapperHop, "index" | "url" | "xml">;

const XML_ENTITIES: Record<string, string> = {
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
] as const;

const CLICK_THROUGH_TAGS = [
  "ClickThrough",
  "CompanionClickThrough",
  "IconClickThrough",
  "NonLinearClickThrough",
] as const;

const VIEWABILITY_TAGS = [
  ["Viewable", "viewable"],
  ["NotViewable", "notViewable"],
  ["ViewUndetermined", "viewUndetermined"],
] as const;

export function createEmptyTrackingPlan(): VastTrackingPlan {
  return {
    impressions: [],
    errors: [],
    clickTrackings: [],
    clickThroughs: [],
    events: [],
  };
}

function normalizeWhitespace(value: string): string {
  return value.replace(/\s+/g, " ").trim();
}

function decodeXmlEntities(value: string): string {
  return value.replace(/&(#x?[0-9a-fA-F]+|amp|apos|gt|lt|quot);/g, (_match, entity: string) => {
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

function stripCdataSections(value: string): string {
  let output = "";
  let cursor = 0;

  while (cursor < value.length) {
    const start = value.indexOf("<![CDATA[", cursor);
    if (start === -1) {
      output += value.slice(cursor);
      break;
    }

    output += value.slice(cursor, start);

    const contentStart = start + "<![CDATA[".length;
    const end = value.indexOf("]]>", contentStart);
    if (end === -1) {
      output += value.slice(start);
      break;
    }

    output += value.slice(contentStart, end);
    cursor = end + 3;
  }

  return output;
}

function cleanXmlText(value: string, collapseWhitespace = false): string {
  const withoutCdata = stripCdataSections(value);
  const decoded = decodeXmlEntities(withoutCdata);
  return collapseWhitespace ? normalizeWhitespace(decoded) : decoded.trim();
}

function extractAttribute(rawAttributes: string, attributeName: string): string {
  const match = new RegExp(`${attributeName}=("([^"]*)"|'([^']*)')`, "i").exec(rawAttributes);
  if (!match) {
    return "";
  }

  return cleanXmlText(match[2] ?? match[3] ?? "", false);
}

function buildTarget(
  kind: VastTrackingTarget["kind"],
  event: VastTrackingTarget["event"],
  rawValue: string,
  hop: TrackingSourceHop,
  offset: string | null = null,
): VastTrackingTarget | null {
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

function collectTagTargets(
  xml: string,
  tagName: string,
  kind: VastTrackingTarget["kind"],
  event: VastTrackingTarget["event"],
  hop: TrackingSourceHop,
): VastTrackingTarget[] {
  const targets: VastTrackingTarget[] = [];
  const pattern = new RegExp(`<${tagName}\\b[^>]*>([\\s\\S]*?)<\\/${tagName}>`, "gi");

  for (const match of xml.matchAll(pattern)) {
    const target = buildTarget(kind, event, match[1] ?? "", hop);
    if (target) {
      targets.push(target);
    }
  }

  return targets;
}

function collectTrackingEvents(xml: string, hop: TrackingSourceHop): VastTrackingTarget[] {
  const targets: VastTrackingTarget[] = [];
  const pattern = /<Tracking\b([^>]*)>([\s\S]*?)<\/Tracking>/gi;

  for (const match of xml.matchAll(pattern)) {
    const rawAttributes = match[1] ?? "";
    const event = extractAttribute(rawAttributes, "event") as VastTrackableEvent;
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

export function buildTrackingPlan(hops: readonly TrackingSourceHop[]): VastTrackingPlan {
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

export function selectTrackingTargets(
  plan: VastTrackingPlan,
  event: VastTrackableEvent,
  offset?: string,
): VastTrackingTarget[] {
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

function randomCacheBustingValue(): string {
  return Math.floor(Math.random() * 100000000)
    .toString()
    .padStart(8, "0");
}

export function expandTrackingUrl(url: string, macros?: Record<string, string | number>): string {
  const values: Record<string, string> = {
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