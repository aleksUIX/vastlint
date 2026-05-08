// ── Template ignore ───────────────────────────────────────────────────────────

/**
 * Replace every match of `regexStr` with an equal-length run of zeros.
 * Same-length replacement preserves all line/col offsets so that
 * validator-reported positions remain accurate in the original file.
 */
export function applyTemplateIgnore(text: string, regexStr: string): string {
  if (!regexStr) return text;
  let regex: RegExp;
  try {
    regex = new RegExp(regexStr, 'gs');
  } catch {
    // Invalid regex — skip silently; user will see a setting validation warning.
    return text;
  }
  return text.replace(regex, (match) => '0'.repeat(match.length));
}

// ── VAST block extractor ──────────────────────────────────────────────────────

export interface VastBlock {
  xml: string;
  /** 0-based line index in the parent document where this block starts. */
  startLine: number;
  /** 0-based column index on startLine where '<VAST' begins. */
  startCol: number;
}

/**
 * Find every top-level `<VAST … </VAST>` region in `text` and return each
 * as a VastBlock with its document-absolute start position.
 *
 * Works for files that contain multiple VAST documents, or VAST embedded
 * inside template files (ERB, Go templates, Mustache, etc.).
 */
export function extractVastBlocks(text: string): VastBlock[] {
  const OPEN  = '<VAST';
  const CLOSE = '</VAST>';
  const blocks: VastBlock[] = [];
  let pos = 0;

  while (pos < text.length) {
    const start = text.indexOf(OPEN, pos);
    if (start === -1) break;

    const closeIdx = text.indexOf(CLOSE, start);
    if (closeIdx === -1) break;

    const blockEnd = closeIdx + CLOSE.length;
    const blockXml = text.slice(start, blockEnd);

    // Count newlines before `start` to get the 0-based line number.
    const before = text.slice(0, start);
    let startLine = 0;
    for (let i = 0; i < before.length; i++) {
      if (before[i] === '\n') startLine++;
    }
    const lastNl   = before.lastIndexOf('\n');
    const startCol = lastNl === -1 ? start : start - lastNl - 1;

    blocks.push({ xml: blockXml, startLine, startCol });
    pos = blockEnd;
  }

  return blocks;
}

// ── Block-relative → document-absolute position mapping ──────────────────────

export interface BlockIssuePosition {
  /** 0-based document line */
  docLine: number;
  /** 0-based document column */
  docCol: number;
}

/**
 * Map a 1-based (line, col) position reported by the validator (relative to
 * the start of a VAST block) to 0-based document-absolute coordinates.
 *
 * Column offset is only applied on the first line of the block, because that
 * is the only line that doesn't start at column 0 in the document.
 */
export function mapBlockIssuePosition(
  blockLine: number,
  blockCol: number,
  block: Pick<VastBlock, 'startLine' | 'startCol'>,
): BlockIssuePosition {
  const blockLineIdx = blockLine - 1;
  const blockColIdx  = blockCol  - 1;
  const docLine = block.startLine + blockLineIdx;
  const docCol  = blockLineIdx === 0
    ? block.startCol + blockColIdx
    : blockColIdx;
  return { docLine, docCol };
}
