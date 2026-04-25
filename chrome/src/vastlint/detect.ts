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
// across coloured <span> elements; HTML entities (&lt; &gt; &amp;) are decoded
// by the HTML parser and stored as raw characters in text nodes.
//
// Extraction pipeline
// ───────────────────
// 1. Query for <span> elements with inline color styles that carry a VAST
//    element name — these are the "tag name" spans emitted by virtually every
//    HTML syntax highlighter.
// 2. Walk up from each such span until we find the nearest ancestor whose
//    textContent contains a complete VAST document (opening <VAST…> tag +
//    closing </VAST>).  textContent is used here because it works on off-screen
//    elements without triggering layout reflow, and HTML entities are already
//    decoded in text nodes.
// 3. Deduplicate — keep only the deepest (most specific) container per VAST blob.
// 4. Extract the XML by walking the container's DOM tree, concatenating text
//    node content, and inserting newlines at block-level element boundaries
//    (DIV, P, BR, …).  This gives a properly line-structured XML string without
//    any HTML markup — no reliance on innerText (which requires layout) and no
//    extra UI-chrome text from outside the container.

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
 * Block-level HTML tags that should introduce a newline when reconstructing
 * text from a syntax-highlighted HTML tree.
 */
const BLOCK_TAGS = new Set([
  'DIV','P','BR','LI','TR','TD','TH','SECTION','ARTICLE','HEADER','FOOTER',
  'MAIN','NAV','ASIDE','BLOCKQUOTE','PRE','H1','H2','H3','H4','H5','H6',
]);

/**
 * Strip all HTML from a syntax-highlighted XML container and return a
 * plain-text reconstruction with proper line breaks.
 *
 * Strategy
 * ────────
 * Walk every DOM node inside `el`:
 *   • TEXT node  → append its text content to the current line buffer.
 *                  The browser already decoded HTML entities (&lt; → <),
 *                  so we get raw XML characters for free.
 *   • ELEMENT    → if it's a block-level tag, flush the current line buffer
 *                  and start a new line.
 *
 * This mirrors what `innerText` does but without triggering layout reflow and
 * without including any text that lives *outside* our target element (e.g.
 * UI chrome that happens to be a parent).
 */
export function extractHtmlRenderedVast(el: Element): string | null {
  const lines: string[] = [];
  let cur = '';

  function walk(node: Node): void {
    if (node.nodeType === Node.TEXT_NODE) {
      cur += node.textContent ?? '';
      return;
    }
    if (node.nodeType !== Node.ELEMENT_NODE) return;

    const tag = (node as Element).tagName;
    const isBlock = BLOCK_TAGS.has(tag);

    // Flush accumulated text as a new line when we enter a new block element
    if (isBlock && cur.length > 0) {
      lines.push(cur);
      cur = '';
    }

    for (const child of node.childNodes) walk(child);

    // Flush again when we leave the block element (handles inline content
    // that follows the last text node inside the block)
    if (isBlock && cur.length > 0) {
      lines.push(cur);
      cur = '';
    }
  }

  walk(el);
  if (cur.length > 0) lines.push(cur); // trailing content

  // Join lines and trim blank lines that arise from empty <div> rows
  const xml = lines
    .map(l => l) // keep raw — don't strip internal whitespace
    .filter(l => l.trim().length > 0)
    .join('\n')
    .trim();

  if (!VAST_SIGNATURE_RE.test(xml)) return null;
  return xml;
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

    // Step 2 — walk up the DOM to find the nearest ancestor whose reconstructed
    // text looks like a complete VAST document.
    // We use textContent (not innerText) because:
    //   a) it works even when elements are off-screen / not laid out
    //   b) it's synchronous and doesn't trigger reflow
    // HTML entities in span text nodes are already decoded by the browser's
    // HTML parser, so textContent gives us the raw < > & characters.
    let el: Element | null = span.parentElement;
    while (el && el !== root) {
      const text = el.textContent ?? '';
      if (
        VAST_SIGNATURE_RE.test(text) &&
        /<\/VAST>/i.test(text)
      ) {
        containerCandidates.add(el);
        break; // stop at the first (tightest) qualifying ancestor
      }
      el = el.parentElement;
    }
  }

  // Step 3 — deduplicate: discard any container that is an ancestor of a
  // deeper candidate so we emit only the tightest container per VAST.
  const all = Array.from(containerCandidates);
  return all.filter(c => !all.some(other => other !== c && c.contains(other)));
}
