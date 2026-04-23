/**
 * VAST XML detection helpers.
 *
 * A string is a VAST candidate if it contains the opening tag of a VAST root
 * element (with or without a namespace / version attribute).
 * We intentionally keep this fast and loose — false positives are filtered out
 * by the WASM parser which will simply return a validation error.
 */

// Matches the opening of a <VAST …> tag (case-insensitive, with optional BOM/whitespace)
export const VAST_SIGNATURE_RE = /(?:<\?xml[^>]*>[\s\S]{0,200})?<VAST[\s>\/]/i;

// ─── HTML-rendered VAST XML detection ────────────────────────────────────────
//
// Many ad-tech debug UIs (Publica, SpringServe, etc.) render VAST XML as
// syntax-highlighted HTML rather than raw text.  The XML tokens are split
// across coloured <span> elements and HTML entities (&lt; &gt; &amp;) appear
// decoded in the underlying text nodes.  `element.innerText` reconstructs the
// raw XML faithfully because:
//   • block-level <div> rows produce newlines
//   • text nodes already carry the decoded characters
//
// Detection strategy
// ─────────────────
// 1. Query for <span> elements that carry inline color styles — these are the
//    syntax-highlighting spans produced by virtually every HTML code renderer.
// 2. Among those spans, look for ones whose text content matches a VAST element
//    name (VAST, InLine, Wrapper, Impression …).  These are the "orange tag"
//    spans in Publica-style UIs, but any syntax highlighter that colours XML
//    element names differently will match.
// 3. Walk up from each matching span until we reach a container whose
//    innerText starts with a VAST document (matches VAST_SIGNATURE_RE) and
//    contains the closing </VAST> tag.
// 4. Deduplicate — keep only the deepest (most specific) container so we
//    don't emit the same VAST blob for every ancestor wrapper.

/** VAST element names we use as anchors when scanning coloured spans. */
const HTML_VAST_TAG_RE = /^(VAST|Ad|InLine|Wrapper|Impression|Creatives|Creative|Linear|NonLinear|Companion|MediaFiles|MediaFile|TrackingEvents|Tracking|VideoClicks|ClickThrough|ClickTracking|Extensions|Extension|AdSystem|AdTitle|Description|Error|Duration|AdServingId|Verification|AdVerifications)$/;

/**
 * Heuristic: does this element appear to be a syntax-highlighted XML block?
 * Requires at least `minSpans` child <span> elements with inline color styles.
 */
export function isHtmlSyntaxHighlighted(el: Element, minSpans = 4): boolean {
  let count = 0;
  for (const span of el.querySelectorAll('span[style]')) {
    if ((span as HTMLElement).style.color) {
      if (++count >= minSpans) return true;
    }
  }
  return false;
}

/**
 * Extract the raw VAST XML text from an element that displays it as
 * syntax-highlighted HTML.  Returns `null` when the element's innerText does
 * not look like a VAST document.
 */
export function extractHtmlRenderedVast(el: Element): string | null {
  const text = (el as HTMLElement).innerText ?? el.textContent ?? '';
  if (!VAST_SIGNATURE_RE.test(text)) return null;
  return text.trim();
}

/**
 * Scan `root` for elements that render VAST XML as syntax-highlighted HTML
 * and return the tightest container per unique VAST blob found.
 *
 * Works across ad-tech UIs that share the common pattern of colouring XML
 * tokens via inline `style="color: …"` on <span> elements.
 */
export function findHtmlRenderedVastContainers(root: Element): Element[] {
  // Step 1 — find all colour-styled spans that hold a VAST element name.
  const colorSpans = root.querySelectorAll<HTMLElement>('span[style]');
  const containerCandidates = new Set<Element>();

  for (const span of colorSpans) {
    if (!span.style.color) continue;
    const name = (span.textContent ?? '').trim();
    if (!HTML_VAST_TAG_RE.test(name)) continue;

    // Step 2 — walk up the DOM to find the nearest ancestor whose innerText
    // looks like a complete VAST document.
    let el: Element | null = span.parentElement;
    while (el && el !== root) {
      // Skip tiny containers — a full VAST doc is at least a few hundred chars.
      if ((el as HTMLElement).offsetHeight !== undefined) {
        const text = (el as HTMLElement).innerText ?? '';
        if (
          text.length > 200 &&
          VAST_SIGNATURE_RE.test(text) &&
          /<\/VAST>/i.test(text)
        ) {
          containerCandidates.add(el);
          break; // stop at the first (tightest) qualifying ancestor
        }
      }
      el = el.parentElement;
    }
  }

  // Step 3 — deduplicate: discard any container that is an ancestor of a
  // deeper candidate so we emit only the tightest container per VAST.
  const all = Array.from(containerCandidates);
  return all.filter(c => !all.some(other => other !== c && c.contains(other)));
}
