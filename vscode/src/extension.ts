import * as vscode from 'vscode';
import { spawn } from 'child_process';
// Import types only from the package (compile-time, no runtime cost).
import type { Issue, RuleMeta } from 'vastlint';
import { applyTemplateIgnore, extractVastBlocks, mapBlockIssuePosition, type VastBlock } from './utils';
// Use the CJS entry directly at runtime — the package has "type":"module" which
// causes Node v22 to pick the ESM loader, breaking __dirname and WASM init.
// eslint-disable-next-line @typescript-eslint/no-require-imports
const { validateWithOptions, rules, fix } = require('vastlint/index.cjs') as Pick<typeof import('vastlint'), 'validateWithOptions' | 'rules' | 'fix'>;

// ── Rule catalog ─────────────────────────────────────────────────────────────

/** Keyed by rule ID for O(1) lookup at diagnostic creation time. */
let ruleCatalog: Map<string, RuleMeta> = new Map();

function loadCatalog(): void {
  ruleCatalog = new Map(rules().map((r) => [r.id, r]));
}

// ── Fix hints ─────────────────────────────────────────────────────────────────
// Short, actionable "how to fix" text shown in the hover tooltip.
// These complement the rule description with concrete guidance.

const FIX_HINTS: Record<string, string> = {
  'VAST-2.0-root-element':              'Rename the root element to `<VAST>`.',
  'VAST-2.0-root-version':              'Add a `version` attribute to `<VAST>`, e.g. `version="4.2"`.',
  'VAST-2.0-root-version-value':        'Set version to a known string: 2.0, 3.0, 4.0, 4.1, 4.2, or 4.3.',
  'VAST-2.0-root-has-ad-or-error':      'Add at least one `<Ad>` or `<Error>` element inside `<VAST>`.',
  'VAST-4.0-wrapper-root-error':        'A VAST response should have either `<Ad>` elements or `<Error>`, not both.',
  'VAST-2.0-ad-has-inline-or-wrapper':  'Each `<Ad>` must contain exactly one `<InLine>` or `<Wrapper>` child.',
  'VAST-2.0-inline-adsystem':           'Add `<AdSystem>` inside `<InLine>`, e.g. `<AdSystem>My Ad Server</AdSystem>`.',
  'VAST-2.0-inline-adtitle':            'Add `<AdTitle>` inside `<InLine>`, e.g. `<AdTitle>My Ad</AdTitle>`.',
  'VAST-2.0-inline-impression':         'Add at least one `<Impression>` element with a tracking URL inside `<InLine>`.',
  'VAST-2.0-inline-creatives':          'Add a `<Creatives>` element containing at least one `<Creative>` inside `<InLine>`.',
  'VAST-4.1-adservingid-present':       'Add `<AdServingId>` with a unique ID value inside `<InLine>` (required VAST 4.1+).',
  'VAST-4.0-universaladid-present':     'Add `<UniversalAdId idRegistry="...">value</UniversalAdId>` inside each `<Creative>`.',
  'VAST-4.0-universaladid-idregistry':  'Add the `idRegistry` attribute to `<UniversalAdId>`, e.g. `idRegistry="ad-id.org"`.',
  'VAST-4.0-universaladid-idvalue':     'Add the `idValue` attribute to `<UniversalAdId>` (VAST 4.0), e.g. `idValue="abc123"`.',
  'VAST-4.1-universaladid-content':     'In VAST 4.1+, put the ID value as element text: `<UniversalAdId>abc123</UniversalAdId>`.',
  'VAST-4.1-universaladid-idvalue-removed': 'Remove the `idValue` attribute; use element text content instead (VAST 4.1+).',
  'VAST-2.0-linear-duration':           'Add `<Duration>` inside `<Linear>`, e.g. `<Duration>00:00:30</Duration>`.',
  'VAST-2.0-linear-mediafiles':         'Add `<MediaFiles>` with at least one `<MediaFile>` inside `<Linear>`.',
  'VAST-2.0-mediafile-delivery':        'Add `delivery="progressive"` or `delivery="streaming"` to `<MediaFile>`.',
  'VAST-2.0-mediafile-type':            'Add a MIME type, e.g. `type="video/mp4"` to `<MediaFile>`.',
  'VAST-2.0-mediafile-dimensions':      'Add `width` and `height` attributes to `<MediaFile>`.',
  'VAST-2.0-mediafile-delivery-enum':   'Set `delivery` to `"progressive"` or `"streaming"` — no other values are valid.',
  'VAST-3.0-minmaxbitrate-pair':         'Add both `minBitrate` and `maxBitrate`, or remove both — they must appear as a pair.',
  'VAST-3.0-bitrate-conflict':          'Remove either `bitrate` or the `minBitrate`/`maxBitrate` pair — they cannot be combined.',
  'VAST-2.0-mediafile-https':           'Change the MediaFile URL scheme from `http://` to `https://`.',
  'VAST-2.0-wrapper-adsystem':          'Add `<AdSystem>` inside `<Wrapper>`.',
  'VAST-2.0-wrapper-impression':        'Add at least one `<Impression>` tracking URL inside `<Wrapper>`.',
  'VAST-2.0-wrapper-vastadtaguri':      'Add `<VASTAdTagURI>` with the downstream tag URL inside `<Wrapper>`.',
  'VAST-2.0-duration-format':           'Use the format `HH:MM:SS` or `HH:MM:SS.mmm`, e.g. `00:00:30` or `00:00:30.500`.',
  'VAST-3.0-skipoffset-format':         'Use `HH:MM:SS` (e.g. `00:00:05`) or a percentage (e.g. `25%`) for `skipoffset`.',
  'VAST-3.0-progress-offset':           'Add an `offset` attribute to `<Tracking event="progress">`, e.g. `offset="25%"`.',
  'VAST-3.0-progress-offset-format':    'Use `HH:MM:SS` or a percentage like `25%` for the progress `offset` attribute.',
  'VAST-3.0-skip-event-no-skipoffset':  'Add `skipoffset` to `<Linear>` if you fire a `skip` tracking event.',
  'VAST-3.0-pricing-model':             'Add `model="cpm"` (or `cpc`, `cpe`, `cpv`) to `<Pricing>`.',
  'VAST-3.0-pricing-currency':          'Add `currency="USD"` (ISO 4217 3-letter code) to `<Pricing>`.',
  'VAST-3.0-pricing-currency-format':   'Use a 3-letter ISO 4217 code, e.g. `USD`, `EUR`, `GBP`.',
  'VAST-3.0-icon-program':              'Add `program="..."` to `<Icon>` to identify the icon owner.',
  'VAST-3.0-icon-width':                'Add `width="..."` in pixels to `<Icon>`.',
  'VAST-3.0-icon-height':               'Add `height="..."` in pixels to `<Icon>`.',
  'VAST-3.0-icon-xposition':            'Add `xPosition="left"` or `"right"` (or pixel value) to `<Icon>`.',
  'VAST-3.0-icon-yposition':            'Add `yPosition="top"` or `"bottom"` (or pixel value) to `<Icon>`.',
  'VAST-3.0-icon-resource':             'Add `<StaticResource>`, `<IFrameResource>`, or `<HTMLResource>` inside `<Icon>`.',
  'VAST-3.0-icon-attrs':                'Ensure `<Icon>` has `program`, `width`, `height`, `xPosition`, and `yPosition`.',
  'VAST-4.0-category-authority':        'Add `authority="..."` to `<Category>`, e.g. `authority="iabtechlab.com"`.',
  'VAST-4.0-companion-clicktracking-id':'Add an `id` attribute to `<CompanionClickTracking>`.',
  'VAST-4.0-wrapper-clickthrough':      'Remove `<ClickThrough>` from Wrapper `<VideoClicks>` — not allowed in VAST 4.0–4.1.',
  'VAST-4.1-blockedadcategories-no-authority': 'Add `authority="..."` to `<BlockedAdCategories>` to identify the taxonomy.',
  'VAST-4.1-verification-no-resource':  'Add `<JavaScriptResource>` or `<ExecutableResource>` inside `<Verification>`.',
  'VAST-4.1-verification-vendor':       'Add `vendor="company.com-omid"` (domain-useCase format) to `<Verification>`.',
  'VAST-4.1-js-resource-apiframework':  'Add `apiFramework="..."` (e.g. `"omid"`) to `<JavaScriptResource>`.',
  'VAST-4.3-js-resource-browser-optional': 'Add `browserOptional="true"` or `"false"` to `<JavaScriptResource>`.',
  'VAST-4.1-exec-resource-apiframework':'Add `apiFramework="..."` to `<ExecutableResource>`.',
  'VAST-4.1-exec-resource-type':        'Add `type="..."` (MIME type) to `<ExecutableResource>`.',
  'VAST-4.1-mezzanine-delivery':        'Add `delivery="progressive"` to `<Mezzanine>`.',
  'VAST-4.1-mezzanine-type':            'Add `type="video/mp4"` (or appropriate MIME type) to `<Mezzanine>`.',
  'VAST-4.1-mezzanine-width':           'Add `width="..."` to `<Mezzanine>`.',
  'VAST-4.1-mezzanine-height':          'Add `height="..."` to `<Mezzanine>`.',
  'VAST-4.1-mezzanine-recommended':     'Add a `<Mezzanine>` element with the high-quality source file for CTV/SSAI use.',
  'VAST-4.1-ad-serving-id-empty':       'Set `<AdServingId>` to a non-empty unique value — SSAI servers use it for deduplication.',
  'VAST-4.1-vpaid-apiframework':        'Replace VPAID with SIMID (`<InteractiveCreativeFile>`) or OMID for measurement.',
  'VAST-4.1-vpaid-in-interactive-context': 'Remove the VPAID `<MediaFile>` — CTV players cannot run VPAID alongside SIMID.',
  'VAST-4.0-mediafile-apiframework':    'Use `<InteractiveCreativeFile>` for interactive content instead of `apiFramework` on `<MediaFile>`.',
  'VAST-4.0-interactive-creative-no-api': 'Add `apiFramework="SIMID"` (or the appropriate value) to `<InteractiveCreativeFile>`.',
  'VAST-4.1-interactive-creative-type':   'Add `type="text/html"` (for SIMID) or the correct MIME type to `<InteractiveCreativeFile>`.',
  'VAST-3.0-pricing-model-case':          'Use lowercase for the `model` attribute: `cpm`, `cpc`, `cpe`, or `cpv`.',
  'VAST-4.1-survey-deprecated':         'Remove `<Survey>` — deprecated in VAST 4.1. Use `<Extensions>` if needed.',
  'VAST-4.0-conditionalad':             'Remove the `conditionalAd` attribute — deprecated in VAST 4.1.',
  'VAST-2.0-flash-mediafile':           'Remove Flash `<MediaFile>` entries — Flash is no longer supported in any browser.',
  'VAST-4.1-adtype-value':              'Set `adType` to `"video"`, `"audio"`, or `"hybrid"`.',
  'VAST-4.1-companion-renderingmode-value': 'Set `renderingMode` to `"default"`, `"end-card"`, or `"concurrent"`.',
  'VAST-3.0-companion-required-attr':   'Set `required` to `"all"`, `"any"`, or `"none"` on `<CompanionAds>`.',
  'VAST-4.0-tracking-event-removed':    'Replace `fullscreen`/`exitFullscreen` with `playerExpand`/`playerCollapse` (VAST 4.0+).',
  'VAST-4.1-tracking-event-value':      'Use a valid tracking event name for this VAST version. See IAB VAST spec §2.3.6.',
  'VAST-2.0-tracking-https':            'Change the tracking URL scheme from `http://` to `https://`.',
  'VAST-2.0-url-empty':                 'Provide a valid URL value for this element.',
  'VAST-2.0-url-invalid':               'Ensure the value is a well-formed URL (e.g. starts with `https://`).',
  'VAST-2.0-duplicate-impression':      'Remove duplicate `<Impression>` entries — each pixel URL should appear only once.',
  'VAST-2.0-version-mismatch':          'Align the `version` attribute on `<VAST>` with the actual structure of the document.',
  'VAST-2.0-wrapper-depth':             'Reduce the wrapper chain — the limit is 5 hops.',
  'VAST-2.0-ad-sequence':               'Either add `sequence` to all `<Ad>` elements or remove it from all of them.',
  'VAST-2.0-parse-error':               'Fix the XML syntax error — check for unclosed tags, invalid characters, or bad encoding.',
  'VAST-2.0-companion-resource':        'Add `<StaticResource>`, `<IFrameResource>`, or `<HTMLResource>` inside `<Companion>`.',
  'VAST-2.0-nonlinear-resource':         'Add `<StaticResource>`, `<IFrameResource>`, or `<HTMLResource>` inside `<NonLinear>`.',
  'VAST-2.0-text-only-element':         'Remove child elements from this text-only element — it should contain only text content.',
  'VAST-2.0-unknown-attribute':         'Remove the unrecognised attribute — it has no meaning in the VAST spec.',
  'VAST-2.0-inline-unknown-child':      'Remove or relocate this unrecognised element — `<InLine>` does not allow it.',
  'VAST-2.0-wrapper-unknown-child':     'Remove or relocate this unrecognised element — `<Wrapper>` does not allow it.',
  'VAST-2.0-creatives-unknown-child':   '`<Creatives>` may only contain `<Creative>` elements.',
  'VAST-2.0-creative-unknown-child':    '`<Creative>` may only contain `<Linear>`, `<NonLinearAds>`, `<CompanionAds>`, or `<UniversalAdId>`.',
  'VAST-2.0-linear-unknown-child':      'Remove the unrecognised child — see the VAST spec for valid `<Linear>` children.',
  'VAST-2.0-mediafiles-unknown-child':  '`<MediaFiles>` may only contain `<MediaFile>`, `<Mezzanine>`, `<InteractiveCreativeFile>`, or `<ClosedCaptionFiles>`.',
  'VAST-2.0-extensions-unknown-child':  '`<Extensions>` may only contain `<Extension>` elements.',
  'VAST-2.0-extension-misplaced-element':   'This VAST element belongs elsewhere in the document, not inside `<Extension>`.',
  'VAST-2.0-creative-extension-misplaced-element': 'This VAST element belongs elsewhere, not inside `<CreativeExtension>`.',
  'VAST-2.0-nonlinearads-unknown-child':'`<NonLinearAds>` may only contain `<TrackingEvents>` or `<NonLinear>` elements.',
  'VAST-2.0-nonlinear-unknown-child':   'Remove the unrecognised child — see VAST spec for valid `<NonLinear>` children.',
  'VAST-2.0-nonlinear-dimensions':      'Add `width` and `height` attributes to `<NonLinear>`.',
  'VAST-2.0-companionads-unknown-child':'`<CompanionAds>` may only contain `<Companion>` elements.',
  'VAST-2.0-companion-unknown-child':   'Remove the unrecognised child — see VAST spec for valid `<Companion>` children.',
  'VAST-2.0-companion-dimensions':      'Add `width` and `height` attributes to `<Companion>`.',
  'VAST-2.0-videoclicks-unknown-child': '`<VideoClicks>` may only contain `<ClickThrough>`, `<ClickTracking>`, or `<CustomClick>`.',
  'VAST-2.0-trackingevents-unknown-child': '`<TrackingEvents>` may only contain `<Tracking>` elements.',
  'VAST-3.0-icons-unknown-child':       '`<Icons>` may only contain `<Icon>` elements.',
  'VAST-3.0-icon-unknown-child':        'Remove the unrecognised child from `<Icon>` — see spec for valid children.',
  'VAST-3.0-iconclicks-unknown-child':  '`<IconClicks>` may only contain `<IconClickThrough>`, `<IconClickTracking>`, or `<IconClickFallbackImages>`.',
  'VAST-4.2-icon-fallback-image-width-height': 'Add `width` and `height` to `<IconClickFallbackImage>` so the player can size the overlay.',
  'VAST-4.2-closedcaptionfiles-unknown-child': '`<ClosedCaptionFiles>` may only contain `<ClosedCaptionFile>` elements.',
  'VAST-2.0-creativeextensions-unknown-child': '`<CreativeExtensions>` may only contain `<CreativeExtension>` elements.',
  'VAST-2.0-linear-tracking-quartiles':        'Add `<TrackingEvents>` inside `<Linear>` with pixels for `start`, `firstQuartile`, `midpoint`, `thirdQuartile`, and `complete` — without these events the measurement system receives no signal.',
  // SIMID rules
  'SIMID-1.0-simid-type-required':           'Set `type="text/html"` on `<InteractiveCreativeFile apiFramework="SIMID">` — SIMID creatives are HTML documents, not JavaScript files.',
  'SIMID-1.0-simid-url-empty':               'Add the HTTPS URL of your SIMID creative HTML document inside `<InteractiveCreativeFile>`, wrapped in a CDATA section.',
  'SIMID-1.0-simid-url-https':               'Change the `<InteractiveCreativeFile>` URL from `http://` to `https://` — HTTP is blocked by mixed-content policies in all modern browsers.',
  'SIMID-1.0-simid-variable-duration-value': 'Set `variableDuration="true"` or remove the attribute — per SIMID §5, `"true"` is the only valid value.',
  'SIMID-1.0-simid-mediafile-required':      'Add a `<MediaFile>` video element inside `<MediaFiles>` — SIMID §3.4 requires a media asset alongside the interactive creative.',
  'SIMID-1.1-nonlinear-simid-no-iframe':     'Add `<IFrameResource type="text/html">` with the SIMID creative URL inside `<NonLinear apiFramework="SIMID">`.',
  'SIMID-1.1-iframe-simid-type-required':    'Add `type="text/html"` to `<IFrameResource>` inside `<NonLinear apiFramework="SIMID">`.',
  'SIMID-1.1-iframe-simid-url-empty':        'Add the HTTPS URL of your SIMID nonlinear creative as text content inside `<IFrameResource>`, wrapped in a CDATA section.',
  'SIMID-1.1-iframe-simid-url-https':        'Change the `<IFrameResource>` URL from `http://` to `https://`.',
};

// ── CLI integration ───────────────────────────────────────────────────────────

// (applyTemplateIgnore, extractVastBlocks, mapBlockIssuePosition and VastBlock
//  are imported from ./utils)

/** JSON shape produced by `vastlint check - --format json`. Issues are identical to the WASM Issue type. */
interface CliCheckResult {
  issues: Issue[];
}

/**
 * Common locations where `cargo install vastlint` or Homebrew place the binary.
 * Tried in order when the configured path is the default `"vastlint"` and
 * is not found on the VS Code process PATH (common on macOS GUI launches).
 */
const FALLBACK_CLI_PATHS = [
  `${process.env['HOME'] ?? ''}/.cargo/bin/vastlint`,
  '/opt/homebrew/bin/vastlint',
  '/usr/local/bin/vastlint',
  '/usr/bin/vastlint',
];

/** Cached resolved CLI path. `undefined` = not yet probed; `null` = not found (use WASM). */
let resolvedCliPath: string | null | undefined = undefined;
/** The configured path that `resolvedCliPath` corresponds to. */
let resolvedCliConfigValue: string | undefined = undefined;

/**
 * Resolve the effective CLI binary path.
 * Probes `configuredPath` first; if it fails and the path is the default
 * `"vastlint"`, also tries common install locations.
 * Returns `null` when no binary is found — callers fall back to WASM.
 */
async function getCliPath(configuredPath: string): Promise<string | null> {
  if (resolvedCliConfigValue === configuredPath && resolvedCliPath !== undefined) {
    return resolvedCliPath;
  }
  resolvedCliConfigValue = configuredPath;

  const candidates = configuredPath === 'vastlint'
    ? [configuredPath, ...FALLBACK_CLI_PATHS]
    : [configuredPath];

  for (const candidate of candidates) {
    if (!candidate) continue;
    if (await probeCliPath(candidate)) {
      resolvedCliPath = candidate;
      return candidate;
    }
  }

  resolvedCliPath = null;
  return null;
}

/** Returns true when `path` resolves to an executable vastlint binary. */
function probeCliPath(path: string): Promise<boolean> {
  return new Promise((resolve) => {
    const proc = spawn(path, ['--version'], { stdio: 'ignore' });
    proc.on('error', () => resolve(false));
    proc.on('close', () => resolve(true));
  });
}

/**
 * Spawn `vastlint check - --format json --no-color --no-fail`, write `blockXml`
 * to stdin, and resolve with the parsed JSON result.
 * Rejects on spawn failure (ENOENT) or CLI exit code 2 (usage error).
 */
interface CliSpawnOptions {
  /** Passed as --vast-version when set. */
  vastVersion?: string;
}

function spawnCli(cliPath: string, blockXml: string, opts: CliSpawnOptions = {}): Promise<CliCheckResult> {
  const args = ['check', '-', '--format', 'json', '--no-color', '--no-fail'];
  if (opts.vastVersion) {
    args.push('--vast-version', opts.vastVersion);
  }
  return new Promise((resolve, reject) => {
    const proc = spawn(
      cliPath,
      args,
      { stdio: ['pipe', 'pipe', 'pipe'] },
    );

    let stdout = '';
    proc.stdout.on('data', (chunk: Buffer) => { stdout += chunk.toString('utf8'); });
    proc.on('error', reject);
    proc.on('close', (code) => {
      if (code === 2) {
        reject(new Error('vastlint CLI usage error'));
        return;
      }
      try {
        resolve(JSON.parse(stdout.trim()) as CliCheckResult);
      } catch {
        reject(new Error(`vastlint CLI: unexpected output: ${stdout.slice(0, 200)}`));
      }
    });

    proc.stdin.write(blockXml, 'utf8');
    proc.stdin.end();
  });
}

// ── Severity mapping ──────────────────────────────────────────────────────────

function toVscodeSeverity(severity: Issue['severity']): vscode.DiagnosticSeverity {
  switch (severity) {
    case 'error':   return vscode.DiagnosticSeverity.Error;
    case 'warning': return vscode.DiagnosticSeverity.Warning;
    case 'info':    return vscode.DiagnosticSeverity.Information;
  }
}

// ── Diagnostic builder ────────────────────────────────────────────────────────

/**
 * Build VS Code diagnostics for `fullText`.
 *
 * 1. Applies `templateIgnoreRegex` (same-length replacement, preserves positions).
 * 2. Extracts every `<VAST … </VAST>` block.
 * 3. Validates each block via the CLI binary; falls back to WASM if not found.
 * 4. Maps block-relative positions to document-absolute positions.
 */
async function buildDiagnostics(
  fullText: string,
  config: vscode.WorkspaceConfiguration,
): Promise<vscode.Diagnostic[]> {
  const ruleOverrides = (config.get<Record<string, string>>('ruleOverrides') ?? {}) as Record<string, 'error' | 'warning' | 'info' | 'off'>;
  const templateRegex = config.get<string>('templateIgnoreRegex') ?? '';
  const cliPathConfig = config.get<string>('cliPath') ?? 'vastlint';
  const vastVersion   = config.get<string>('vastVersion') ?? '';

  // Strip template expressions while preserving all character positions.
  const processedText = applyTemplateIgnore(fullText, templateRegex);

  // Extract every VAST block (supports multi-VAST files and template wrappers).
  const blocks = extractVastBlocks(processedText);
  if (blocks.length === 0) return [];

  // Resolve CLI binary once per configured path (cached).
  const cliPath = await getCliPath(cliPathConfig);

  const minSeverity = config.get<string>('minSeverity') ?? 'info';
  const minLevel = minSeverity === 'error' ? 2 : minSeverity === 'warning' ? 1 : 0;

  // Use the original (non-stripped) lines for hover/squiggle text display.
  const docLines = fullText.split('\n');
  const allDiagnostics: vscode.Diagnostic[] = [];

  for (const block of blocks) {
    let issues: Issue[];
    try {
      if (cliPath) {
        const result = await spawnCli(cliPath, block.xml, { vastVersion: vastVersion || undefined });
        issues = result.issues;
      } else {
        // WASM fallback — also applies ruleOverrides (CLI reads them from vastlint.toml).
        const result = validateWithOptions(block.xml, { rule_overrides: ruleOverrides });
        issues = result.issues;
      }
    } catch (e) {
      const range = new vscode.Range(block.startLine, block.startCol, block.startLine, block.startCol);
      const d = new vscode.Diagnostic(range, `vastlint error: ${e}`, vscode.DiagnosticSeverity.Error);
      d.source = 'vastlint';
      allDiagnostics.push(d);
      continue;
    }

    for (const issue of issues) {
      const severityLevel = issue.severity === 'error' ? 2 : issue.severity === 'warning' ? 1 : 0;
      if (severityLevel < minLevel) continue;

      // Build the range. VS Code uses 0-based lines and columns.
      let range: vscode.Range;
      if (issue.line != null && issue.col != null) {
        // Map block-relative 1-based (line, col) → document-absolute 0-based.
        const { docLine: docLineIdx, docCol: docColIdx } = mapBlockIssuePosition(
          issue.line, issue.col, block,
        );

        const lineText = docLines[docLineIdx] ?? '';
        // Extend the squiggle to cover the tag name (up to the first space or >).
        const tagEnd = (() => {
          const rest = lineText.slice(docColIdx + 1); // skip <
          const tagNameEnd = rest.search(/[\s>/]/);
          return docColIdx + 1 + (tagNameEnd >= 0 ? tagNameEnd : rest.length);
        })();
        range = new vscode.Range(docLineIdx, docColIdx, docLineIdx, tagEnd);
      } else {
        // Document-level issue — point at the opening <VAST> tag.
        const lineText = docLines[block.startLine] ?? '';
        range = new vscode.Range(
          block.startLine, block.startCol,
          block.startLine, block.startCol + lineText.slice(block.startCol).search(/[\s>]|$/) + 5,
        );
      }

      const meta    = ruleCatalog.get(issue.id);
      const fixHint = FIX_HINTS[issue.id];

      // Build the hover message as Markdown.
      // isTrusted is intentionally left false (default): issue.message,
      // issue.spec_ref, and issue.path are derived from XML content and could
      // be attacker-controlled.  isTrusted=true would enable command: URIs,
      // creating an RCE-on-hover vector.  No command: URIs are used here,
      // so there is no functional cost to keeping it untrusted.
      const md = new vscode.MarkdownString('', true);
      md.appendMarkdown(`**vastlint** \`${issue.id}\`\n\n`);
      md.appendMarkdown(`${issue.message}\n\n`);
      if (fixHint) {
        md.appendMarkdown(`**Fix:** ${fixHint}\n\n`);
      }
      if (meta?.description && meta.description !== issue.message) {
        md.appendMarkdown(`*${meta.description}*\n\n`);
      }
      md.appendMarkdown(`---\n`);
      md.appendMarkdown(`**Spec:** ${issue.spec_ref}`);
      if (meta?.source) {
        md.appendMarkdown(` · **Source:** ${meta.source}`);
      }
      md.appendMarkdown(`  \n[vastlint.org/docs/rules/${issue.id}](https://vastlint.org/docs/rules/${issue.id}/)`);
      if (issue.path) {
        md.appendMarkdown(`  \n**Path:** \`${issue.path}\``);
      }

      const diagnostic = new vscode.Diagnostic(range, issue.message, toVscodeSeverity(issue.severity));
      diagnostic.source = 'vastlint';
      diagnostic.message = issue.message;

      (diagnostic as DiagnosticWithMeta)._meta = {
        id:      issue.id,
        specRef: issue.spec_ref,
        path:    issue.path ?? undefined,
        fixHint: fixHint,
        fullMd:  md,
      };

      allDiagnostics.push(diagnostic);
    }
  }

  return allDiagnostics;
}

// ── Extended diagnostic type ──────────────────────────────────────────────────

interface DiagnosticMeta {
  id:      string;
  specRef: string;
  path?:   string;
  fixHint?: string;
  fullMd:  vscode.MarkdownString;
}

interface DiagnosticWithMeta extends vscode.Diagnostic {
  _meta?: DiagnosticMeta;
}

function getDiagnosticRuleId(diagnostic: vscode.Diagnostic): string | undefined {
  const metaId = (diagnostic as DiagnosticWithMeta)._meta?.id;
  if (metaId) return metaId;
  if (typeof diagnostic.code === 'string') return diagnostic.code;
  if (diagnostic.code && typeof diagnostic.code === 'object' && 'value' in diagnostic.code) {
    return String(diagnostic.code.value);
  }
  return undefined;
}

// ── Auto-fix ──────────────────────────────────────────────────────────────────

/** Rule IDs that vastlint-core can repair automatically. */
const FIXABLE_RULE_IDS = new Set([
  'VAST-2.0-mediafile-https',
  'VAST-2.0-tracking-https',
  'VAST-4.0-conditionalad',
]);

class VastlintCodeActionProvider implements vscode.CodeActionProvider {
  constructor(private readonly collection: vscode.DiagnosticCollection) {}

  provideCodeActions(
    document: vscode.TextDocument,
    _range: vscode.Range,
    context: vscode.CodeActionContext,
  ): vscode.CodeAction[] {
    // Use all diagnostics for the document, not just those in the cursor range.
    // context.diagnostics is often empty when the cursor isn't on the squiggle.
    const allDiags = (this.collection.get(document.uri) ?? []) as vscode.Diagnostic[];
    const fixableDiags = allDiags.filter(
      (d) => d.source === 'vastlint' && FIXABLE_RULE_IDS.has(getDiagnosticRuleId(d) ?? ''),
    );
    if (fixableDiags.length === 0) return [];

    const xml = document.getText();
    let fixResult: ReturnType<typeof fix>;
    try {
      fixResult = fix(xml);
    } catch {
      return [];
    }
    if (fixResult.applied.length === 0) return [];

    const fullRange = new vscode.Range(
      document.positionAt(0),
      document.positionAt(xml.length),
    );

    const actions: vscode.CodeAction[] = [];

    // One quick-fix per fixable diagnostic whose rule was actually applied.
    const appliedIds = new Set(fixResult.applied.map((f) => f.rule_id));
    for (const diag of fixableDiags) {
      const ruleId = getDiagnosticRuleId(diag);
      if (!ruleId || !appliedIds.has(ruleId)) continue;
      const action = new vscode.CodeAction(
        `vastlint: Fix \`${ruleId}\``,
        vscode.CodeActionKind.QuickFix,
      );
      action.diagnostics = [diag];
      action.isPreferred = true;
      const edit = new vscode.WorkspaceEdit();
      edit.replace(document.uri, fullRange, fixResult.xml);
      action.edit = edit;
      actions.push(action);
    }

    // "Fix all" source action.
    const fixAll = new vscode.CodeAction(
      `vastlint: Fix all auto-fixable issues (${fixResult.applied.length} fix${fixResult.applied.length === 1 ? '' : 'es'})`,
      vscode.CodeActionKind.SourceFixAll,
    );
    fixAll.diagnostics = fixableDiags;
    const edit = new vscode.WorkspaceEdit();
    edit.replace(document.uri, fullRange, fixResult.xml);
    fixAll.edit = edit;
    actions.push(fixAll);

    return actions;
  }
}

// ── Hover provider ────────────────────────────────────────────────────────────

class VastlintHoverProvider implements vscode.HoverProvider {
  constructor(private readonly collection: vscode.DiagnosticCollection) {}

  provideHover(
    document: vscode.TextDocument,
    position: vscode.Position,
  ): vscode.Hover | undefined {
    const diags = this.collection.get(document.uri) as DiagnosticWithMeta[] | undefined;
    if (!diags || diags.length === 0) return undefined;

    // Find all diagnostics whose range contains the cursor position.
    const hits = diags.filter((d) => d.range.contains(position) && d._meta);
    if (hits.length === 0) return undefined;

    // isTrusted is intentionally left false (default): d.message, m.specRef,
    // and m.path are derived from XML content and could be attacker-controlled.
    // m.id is a fixed rule ID from the Rust validator (never from XML), so it
    // is safe to embed in the vastlint.org docs URL.
    const md = new vscode.MarkdownString('', true);

    for (let i = 0; i < hits.length; i++) {
      const d = hits[i];
      const m = d._meta!;
      const icon = d.severity === vscode.DiagnosticSeverity.Error   ? '\u{1F7E5}' :
                   d.severity === vscode.DiagnosticSeverity.Warning  ? '\u{1F7E8}' : '\u{1F7E6}';

      // Line 1: severity + message.
      md.appendMarkdown(`${icon} ${d.message}`);

      // Line 2: fix hint (if any).
      if (m.fixHint) {
        md.appendMarkdown(`  \n\u{1F527} *${m.fixHint}*`);
      }

      // Line 3: docs link only — spec and XML path are already visible elsewhere.
      const docsUrl = `https://vastlint.org/docs/rules/${m.id}/`;
      md.appendMarkdown(`  \n[\`${m.id}\`](${docsUrl})`);

      if (i < hits.length - 1) {
        md.appendMarkdown('\n\n---\n\n');
      }
    }

    return new vscode.Hover(md, hits[0].range);
  }
}

// ── Linting orchestration ─────────────────────────────────────────────────────

/** Returns true if the document looks like it might contain a VAST document. */
function isVastDocument(doc: vscode.TextDocument): boolean {
  // Accept any text-like language — detect VAST by content, not languageId,
  // so template files (.erb, .go, .html, etc.) with embedded VAST are supported.
  const lang = doc.languageId;
  if (lang === 'log' || lang === 'output') return false;
  // Scan full document text for <VAST (may appear anywhere in template files).
  return doc.getText().includes('<VAST');
}

function lintDocument(
  doc: vscode.TextDocument,
  collection: vscode.DiagnosticCollection,
): void {
  const config = vscode.workspace.getConfiguration('vastlint', doc.uri);
  if (!config.get<boolean>('enable', true)) {
    collection.delete(doc.uri);
    return;
  }

  if (!isVastDocument(doc)) {
    collection.delete(doc.uri);
    return;
  }

  void buildDiagnostics(doc.getText(), config).then((diagnostics) => {
    // Guard: doc may have been closed while we were awaiting.
    collection.set(doc.uri, diagnostics);
  });
}

// ── Extension lifecycle ───────────────────────────────────────────────────────

export function activate(context: vscode.ExtensionContext): void {
  loadCatalog();

  const collection = vscode.languages.createDiagnosticCollection('vastlint');
  context.subscriptions.push(collection);

  // Hover provider — any file, content-gated by isVastDocument.
  const hoverProvider = vscode.languages.registerHoverProvider(
    [{ scheme: 'file' }, { scheme: 'untitled' }],
    new VastlintHoverProvider(collection),
  );
  context.subscriptions.push(hoverProvider);

  // Code action provider — offers "Fix" actions for auto-repairable diagnostics.
  const codeActionProvider = vscode.languages.registerCodeActionsProvider(
    [{ scheme: 'file' }, { scheme: 'untitled' }],
    new VastlintCodeActionProvider(collection),
    {
      providedCodeActionKinds: [
        vscode.CodeActionKind.QuickFix,
        vscode.CodeActionKind.SourceFixAll,
      ],
    },
  );
  context.subscriptions.push(codeActionProvider);

  // Command palette: "vastlint: Fix All Auto-fixable Issues"
  context.subscriptions.push(
    vscode.commands.registerCommand('vastlint.fixAll', () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor) return;
      const doc = editor.document;
      const xml = doc.getText();
      let fixResult: ReturnType<typeof fix>;
      try {
        fixResult = fix(xml);
      } catch (e) {
        vscode.window.showErrorMessage(`vastlint: fix failed — ${e}`);
        return;
      }
      if (fixResult.applied.length === 0) {
        vscode.window.showInformationMessage('vastlint: nothing to fix');
        return;
      }
      const fullRange = new vscode.Range(
        doc.positionAt(0),
        doc.positionAt(xml.length),
      );
      const edit = new vscode.WorkspaceEdit();
      edit.replace(doc.uri, fullRange, fixResult.xml);
      vscode.workspace.applyEdit(edit).then(() => {
        vscode.window.showInformationMessage(
          `vastlint: applied ${fixResult.applied.length} fix${fixResult.applied.length === 1 ? '' : 'es'} ⚠️ Auto-fix is experimental — review changes before saving.`,
        );
      });
    }),
  );

  // Lint the active editor on startup.
  if (vscode.window.activeTextEditor) {
    lintDocument(vscode.window.activeTextEditor.document, collection);
  }

  // Lint when a new editor becomes active.
  context.subscriptions.push(
    vscode.window.onDidChangeActiveTextEditor((editor) => {
      if (editor) lintDocument(editor.document, collection);
    }),
  );

  // Re-lint on every save.
  context.subscriptions.push(
    vscode.workspace.onDidSaveTextDocument((doc) => lintDocument(doc, collection)),
  );

  // Re-lint while typing (debounced).
  let debounceTimer: ReturnType<typeof setTimeout> | undefined;
  context.subscriptions.push(
    vscode.workspace.onDidChangeTextDocument((event) => {
      clearTimeout(debounceTimer);
      debounceTimer = setTimeout(() => {
        lintDocument(event.document, collection);
      }, 500);
    }),
  );

  // Clean up when a document is closed.
  context.subscriptions.push(
    vscode.workspace.onDidCloseTextDocument((doc) => collection.delete(doc.uri)),
  );
}

export function deactivate(): void {
  // nothing — subscriptions cleaned up automatically
}
