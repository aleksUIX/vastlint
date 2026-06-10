/**
 * xml-viewer.ts — pretty-printing XML viewer in an iframe.
 * Reformats XML at 2-space indent per level (matching Chrome's native viewer),
 * then syntax-highlights line by line.
 *
 * Also builds origToFmt: Map<origLine1Based, formattedLineIdx0Based> so that
 * the linting overlay can map validator-reported line numbers (from raw XML)
 * to the correct line in the formatted output.
 */

// ─── XML pretty-printer ───────────────────────────────────────────────────────

interface Token {
  type: 'open' | 'close' | 'self' | 'text' | 'cdata' | 'comment' | 'pi' | 'doctype';
  raw: string;
  tag?: string;
  attrs?: string;
  /** 1-based line number of this token in the original XML source. */
  origLine: number;
}

function tokenise(xml: string): Token[] {
  const tokens: Token[] = [];
  const re = /<\?[\s\S]*?\?>|<!DOCTYPE[^>]*>|<!--[\s\S]*?-->|<!\[CDATA\[[\s\S]*?\]\]>|<\/[\w:.-]+\s*>|<[\w:.-][^>]*\/?>|[^<]+/g;
  let curLine = 1;
  let lastEnd = 0;
  let m: RegExpExecArray | null;
  while ((m = re.exec(xml)) !== null) {
    // Count newlines in the gap before this token to advance curLine
    const gap = xml.slice(lastEnd, m.index);
    for (const ch of gap) if (ch === '\n') curLine++;
    const origLine = curLine;
    // Count newlines inside the token itself
    for (const ch of m[0]) if (ch === '\n') curLine++;
    lastEnd = m.index + m[0].length;

    const raw = m[0];
    if (!raw.trim()) { tokens.push({ type: 'text', raw, origLine }); continue; }
    if (raw.startsWith('<?'))          { tokens.push({ type: 'pi',      raw, origLine }); continue; }
    if (raw.startsWith('<!DOCTYPE'))   { tokens.push({ type: 'doctype', raw, origLine }); continue; }
    if (raw.startsWith('<!--'))        { tokens.push({ type: 'comment', raw, origLine }); continue; }
    if (raw.startsWith('<![CDATA['))   { tokens.push({ type: 'cdata',   raw, origLine }); continue; }
    if (raw.startsWith('</')) {
      tokens.push({ type: 'close', raw, origLine, tag: raw.match(/<\/([\w:.-]+)/)?.[1] ?? '' }); continue;
    }
    if (raw.startsWith('<')) {
      const self     = raw.endsWith('/>');
      const tagMatch = raw.match(/^<([\w:.-]+)([\s\S]*)>$/);
      const tag      = tagMatch?.[1] ?? '';
      const attrs    = tagMatch?.[2].trim().replace(/\/$/, '').trim() ?? '';
      tokens.push({ type: self ? 'self' : 'open', raw, origLine, tag, attrs }); continue;
    }
    tokens.push({ type: 'text', raw, origLine });
  }
  return tokens;
}

function formatTag(tag: string, attrsRaw: string, self: boolean): string {
  if (!attrsRaw) return self ? `<${tag}/>` : `<${tag}>`;
  return `<${tag} ${attrsRaw}${self ? '/>' : '>'}`;
}

function prettyPrint(xml: string): {
  text: string;
  origToFmt: Map<number, number>;
  fmtToOrig: Map<number, number>;
  fmtToPath: Map<number, string>;
} {
  const IND = '  ';
  const tokens = tokenise(xml);
  const lines: string[] = [];
  const origToFmt = new Map<number, number>();
  const fmtToOrig = new Map<number, number>();
  // fmtToPath: first formatted line of each open/self token → its vastlint-style path
  const fmtToPath = new Map<number, string>();

  // Path stack. Each frame: path to this element, count of element-children pushed so far.
  const stack: Array<{ path: string; childCount: number }> = [];

  function pushLine(text: string, origLine: number) {
    const fmtIdx = lines.length;
    if (!origToFmt.has(origLine)) origToFmt.set(origLine, fmtIdx);
    fmtToOrig.set(fmtIdx, origLine);
    lines.push(text);
  }

  function pathForChild(tag: string): string {
    if (stack.length === 0) return `/${tag}`;
    const parent = stack[stack.length - 1];
    return `${parent.path}/${tag}[${parent.childCount}]`;
  }

  function allocateChild(tag: string): string {
    const p = pathForChild(tag);
    if (stack.length > 0) stack[stack.length - 1].childCount++;
    return p;
  }

  let depth = 0;

  for (const tok of tokens) {
    const ind = IND.repeat(depth);
    switch (tok.type) {
      case 'pi':
      case 'doctype':
        pushLine(ind + tok.raw.trim(), tok.origLine);
        break;
      case 'comment':
        pushLine(ind + tok.raw.trim(), tok.origLine);
        break;
      case 'cdata': {
        pushLine(ind + tok.raw.trim(), tok.origLine);
        break;
      }
      case 'open': {
        const elemPath = allocateChild(tok.tag!);
        const firstFmtIdx = lines.length;
        pushLine(ind + formatTag(tok.tag!, tok.attrs!, false), tok.origLine);
        fmtToPath.set(firstFmtIdx, elemPath);
        stack.push({ path: elemPath, childCount: 0 });
        depth++;
        break;
      }
      case 'self': {
        const elemPath = allocateChild(tok.tag!);
        const firstFmtIdx = lines.length;
        pushLine(ind + formatTag(tok.tag!, tok.attrs!, true), tok.origLine);
        fmtToPath.set(firstFmtIdx, elemPath);
        break;
      }
      case 'close': {
        depth = Math.max(0, depth - 1);
        stack.pop();
        const closeInd = IND.repeat(depth);
        // Merge <Foo></Foo> onto one line when the previous line opened it
        if (lines.length > 0) {
          const prev = lines[lines.length - 1];
          if (new RegExp(`^\\s*<${tok.tag}(\\s[^>]*)?>$`).test(prev)) {
            lines[lines.length - 1] = prev + `</${tok.tag}>`;
            break;
          }
        }
        pushLine(closeInd + `</${tok.tag}>`, tok.origLine);
        break;
      }
      case 'text': {
        const t = tok.raw.trim();
        if (!t) break;
        pushLine(ind + t, tok.origLine);
        break;
      }
    }
  }
  return { text: lines.join('\n'), origToFmt, fmtToOrig, fmtToPath };
}

// ─── Syntax highlighter (line-by-line, post-format) ──────────────────────────

function esc(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

function highlightLine(line: string): string {
  const e = esc(line);
  if (/&lt;!\[CDATA\[/.test(e) || /\]\]&gt;/.test(e) ||
      (!/&lt;/.test(e) && !/&gt;/.test(e) && line.trim().length > 0 &&
       line.trim().startsWith('https'))) {
    return `<span class="cdata">${e}</span>`;
  }
  if (/&lt;!--/.test(e) || /--&gt;/.test(e)) return `<span class="comment">${e}</span>`;
  if (/&lt;!DOCTYPE/i.test(e))               return `<span class="comment">${e}</span>`;
  if (/^(\s*)&lt;\?/.test(e))               return `<span class="pi">${e}</span>`;
  if (/^(\s*)&lt;\//.test(e)) {
    return e.replace(/(&lt;\/)([\w:.-]+)(&gt;)/, '&lt;<span class="ct">/$2</span>&gt;');
  }
  if (/^(\s*)&lt;[\w]/.test(e)) {
    return e
      .replace(/^(\s*)(&lt;)([\w:.-]+)/, '$1&lt;<span class="t">$3</span>')
      .replace(/([\w:.-]+)(=)(&quot;[^&]*&quot;)/g,
               '<span class="an">$1</span>$2<span class="av">$3</span>')
      .replace(/(\/>|&gt;)$/, '<span class="t">$1</span>');
  }
  // Attribute continuation line (wrapped attrs)
  if (/^\s+[\w:.-]+=&quot;/.test(e)) {
    return e.replace(/([\w:.-]+)(=)(&quot;[^&]*&quot;)/g,
                     '<span class="an">$1</span>$2<span class="av">$3</span>');
  }
  // CDATA content line (no angle brackets, indented)
  if (line.length > 0 && !/^</.test(line.trim()) && /^\s/.test(line)) {
    return `<span class="cdata">${e}</span>`;
  }
  return e;
}

function htmlToFragment(html: string): DocumentFragment {
  const parsed = new DOMParser().parseFromString(`<body>${html}</body>`, 'text/html');
  const frag = document.createDocumentFragment();
  frag.append(...Array.from(parsed.body.childNodes).map((node) => document.importNode(node, true)));
  return frag;
}

// ─── Main export ─────────────────────────────────────────────────────────────

export function injectXmlViewer(src: string): { pre: HTMLPreElement; origToFmt: Map<number, number>; pathToFmt: Map<string, number> } {
  const { text: formatted, origToFmt, fmtToOrig, fmtToPath } = prettyPrint(src);
  const lines = formatted.split('\n');
  const pad   = String(lines.length).length;

  // Invert fmtToPath so panel.ts can do O(1) path → fmtIdx lookup
  const pathToFmt = new Map<string, number>();
  for (const [fmtIdx, path] of fmtToPath) pathToFmt.set(path, fmtIdx);

  // ── Create iframe ─────────────────────────────────────────────────────────
  const iframe = document.createElementNS('http://www.w3.org/1999/xhtml', 'iframe') as HTMLIFrameElement;
  iframe.style.cssText = 'all:initial;position:fixed;inset:0;width:100vw;height:100vh;border:none;z-index:2147483640';
  (document.body ?? document.documentElement).appendChild(iframe);

  const iDoc = iframe.contentDocument!;
  iDoc.open();
  iDoc.write(`<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><title>VAST XML \u2014 VASTlint</title>
<style>
*,*::before,*::after{box-sizing:border-box;margin:0;padding:0}
html,body{background:#0d0d17;color:#cdd6f4;height:100%;overflow:hidden}
#root{display:flex;flex-direction:column;height:100vh;
  font:13px/1.6 'JetBrains Mono','Fira Code','Cascadia Code','Consolas',monospace}
#toolbar{flex-shrink:0;display:flex;align-items:center;gap:8px;padding:5px 16px;
  background:#11111f;border-bottom:1px solid #1e1e35;
  font-family:system-ui,sans-serif;font-size:12px;color:#888}
.logo{color:#7ec8e3;font-weight:700;font-size:13px}
.url{opacity:.4;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-size:11px;flex:1}
#btn-native{margin-left:auto;flex-shrink:0;padding:3px 10px;border-radius:4px;border:1px solid #2e2e4e;
  background:#1a1a2e;color:#888;font-family:system-ui,sans-serif;font-size:11px;
  cursor:pointer;transition:border-color .15s,color .15s}
#btn-native:hover{border-color:#7ec8e3;color:#cdd6f4}
#scroll{flex:1;overflow:auto}
#wrap{display:flex;padding:12px 0}
#gutter{flex-shrink:0;padding:0 14px 0 16px;text-align:right;color:#2e2e4e;
  user-select:none;border-right:1px solid #1e1e35;white-space:pre;
  line-height:1.6;font-size:13px;display:flex;flex-direction:column}
.gln{display:block;line-height:1.6}
#code{flex:1;margin:0;padding:0 8px 0 0;white-space:pre;line-height:1.6;
  font-size:13px;background:transparent;tab-size:2}
.ln{display:block;white-space:pre}
.t  {color:#88b4e8}.ct {color:#88b4e8}.an {color:#c98de8}.av {color:#6abf69}
.comment{color:#5a6070;font-style:italic}.cdata{color:#5a7080}.pi{color:#e88}
</style></head><body>
<div id="root">
  <div id="toolbar">
    <span class="logo">VASTlint</span>
    <span style="opacity:.3">&middot;</span>
    <span class="url" id="url-bar"></span>
    <button id="btn-native">&#x2715; Native view</button>
  </div>
  <div id="scroll"><div id="wrap">
    <div id="gutter"></div>
    <pre id="code"></pre>
  </div></div>
</div>
</body></html>`);
  iDoc.close();

  const iWin   = iframe.contentWindow!;
  const code   = iDoc.getElementById('code')!;
  const gutter = iDoc.getElementById('gutter')!;
  const urlBar = iDoc.getElementById('url-bar')!;
  urlBar.textContent = iWin.location.href;

  iDoc.getElementById('btn-native')!.addEventListener('click', () => iframe.remove());

  // ── Build flat line list — no folding, one line = one span ────────────────
  for (let i = 0; i < lines.length; i++) {
    const gln = iDoc.createElement('span');
    gln.className = 'gln';
    gln.textContent = String(i + 1).padStart(pad, ' ');
    gutter.appendChild(gln);

    const ln = iDoc.createElement('span');
    ln.className = 'ln';
    ln.dataset['ln'] = String(i);
    if (fmtToOrig.has(i)) ln.dataset['orig'] = String(fmtToOrig.get(i));
    if (fmtToPath.has(i)) ln.dataset['path'] = fmtToPath.get(i)!;
    const span = iDoc.createElement('span');
    span.replaceChildren(htmlToFragment(highlightLine(lines[i])));
    ln.appendChild(span);
    code.appendChild(ln);
  }

  return { pre: code as HTMLPreElement, origToFmt, pathToFmt };
}
