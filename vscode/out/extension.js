"use strict";
var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, { get: all[name], enumerable: true });
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));
var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);

// src/extension.ts
var extension_exports = {};
__export(extension_exports, {
  activate: () => activate,
  deactivate: () => deactivate
});
module.exports = __toCommonJS(extension_exports);
var vscode = __toESM(require("vscode"));
var { validateWithOptions, rules } = require("./deps/vastlint/index.cjs");
var ruleCatalog = /* @__PURE__ */ new Map();
function loadCatalog() {
  ruleCatalog = new Map(rules().map((r) => [r.id, r]));
}
var FIX_HINTS = {
  "VAST-2.0-root-element": "Rename the root element to `<VAST>`.",
  "VAST-2.0-root-version": 'Add a `version` attribute to `<VAST>`, e.g. `version="4.2"`.',
  "VAST-2.0-root-version-value": "Set version to a known string: 2.0, 3.0, 4.0, 4.1, 4.2, or 4.3.",
  "VAST-2.0-root-has-ad-or-error": "Add at least one `<Ad>` or `<Error>` element inside `<VAST>`.",
  "VAST-4.0-wrapper-root-error": "A VAST response should have either `<Ad>` elements or `<Error>`, not both.",
  "VAST-2.0-ad-has-inline-or-wrapper": "Each `<Ad>` must contain exactly one `<InLine>` or `<Wrapper>` child.",
  "VAST-2.0-inline-adsystem": "Add `<AdSystem>` inside `<InLine>`, e.g. `<AdSystem>My Ad Server</AdSystem>`.",
  "VAST-2.0-inline-adtitle": "Add `<AdTitle>` inside `<InLine>`, e.g. `<AdTitle>My Ad</AdTitle>`.",
  "VAST-2.0-inline-impression": "Add at least one `<Impression>` element with a tracking URL inside `<InLine>`.",
  "VAST-2.0-inline-creatives": "Add a `<Creatives>` element containing at least one `<Creative>` inside `<InLine>`.",
  "VAST-4.1-adservingid-present": "Add `<AdServingId>` with a unique ID value inside `<InLine>` (required VAST 4.1+).",
  "VAST-4.0-universaladid-present": 'Add `<UniversalAdId idRegistry="...">value</UniversalAdId>` inside each `<Creative>`.',
  "VAST-4.0-universaladid-idregistry": 'Add the `idRegistry` attribute to `<UniversalAdId>`, e.g. `idRegistry="ad-id.org"`.',
  "VAST-4.0-universaladid-idvalue": 'Add the `idValue` attribute to `<UniversalAdId>` (VAST 4.0), e.g. `idValue="abc123"`.',
  "VAST-4.1-universaladid-content": "In VAST 4.1+, put the ID value as element text: `<UniversalAdId>abc123</UniversalAdId>`.",
  "VAST-4.1-universaladid-idvalue-removed": "Remove the `idValue` attribute; use element text content instead (VAST 4.1+).",
  "VAST-2.0-linear-duration": "Add `<Duration>` inside `<Linear>`, e.g. `<Duration>00:00:30</Duration>`.",
  "VAST-2.0-linear-mediafiles": "Add `<MediaFiles>` with at least one `<MediaFile>` inside `<Linear>`.",
  "VAST-2.0-mediafile-delivery": 'Add `delivery="progressive"` or `delivery="streaming"` to `<MediaFile>`.',
  "VAST-2.0-mediafile-type": 'Add a MIME type, e.g. `type="video/mp4"` to `<MediaFile>`.',
  "VAST-2.0-mediafile-dimensions": "Add `width` and `height` attributes to `<MediaFile>`.",
  "VAST-2.0-mediafile-delivery-enum": 'Set `delivery` to `"progressive"` or `"streaming"` \u2014 no other values are valid.',
  "VAST-2.0-mediafile-bitrate-conflict": "Remove either `bitrate` or the `minBitrate`/`maxBitrate` pair \u2014 they conflict.",
  "VAST-2.0-mediafile-https": "Change the MediaFile URL scheme from `http://` to `https://`.",
  "VAST-2.0-wrapper-adsystem": "Add `<AdSystem>` inside `<Wrapper>`.",
  "VAST-2.0-wrapper-impression": "Add at least one `<Impression>` tracking URL inside `<Wrapper>`.",
  "VAST-2.0-wrapper-vastadtaguri": "Add `<VASTAdTagURI>` with the downstream tag URL inside `<Wrapper>`.",
  "VAST-2.0-duration-format": "Use the format `HH:MM:SS` or `HH:MM:SS.mmm`, e.g. `00:00:30` or `00:00:30.500`.",
  "VAST-3.0-skipoffset-format": "Use `HH:MM:SS` (e.g. `00:00:05`) or a percentage (e.g. `25%`) for `skipoffset`.",
  "VAST-3.0-progress-offset": 'Add an `offset` attribute to `<Tracking event="progress">`, e.g. `offset="25%"`.',
  "VAST-3.0-progress-offset-format": "Use `HH:MM:SS` or a percentage like `25%` for the progress `offset` attribute.",
  "VAST-3.0-skip-event-no-skipoffset": "Add `skipoffset` to `<Linear>` if you fire a `skip` tracking event.",
  "VAST-3.0-pricing-model": 'Add `model="cpm"` (or `cpc`, `cpe`, `cpv`) to `<Pricing>`.',
  "VAST-3.0-pricing-currency": 'Add `currency="USD"` (ISO 4217 3-letter code) to `<Pricing>`.',
  "VAST-3.0-pricing-currency-format": "Use a 3-letter ISO 4217 code, e.g. `USD`, `EUR`, `GBP`.",
  "VAST-3.0-icon-program": 'Add `program="..."` to `<Icon>` to identify the icon owner.',
  "VAST-3.0-icon-width": 'Add `width="..."` in pixels to `<Icon>`.',
  "VAST-3.0-icon-height": 'Add `height="..."` in pixels to `<Icon>`.',
  "VAST-3.0-icon-xposition": 'Add `xPosition="left"` or `"right"` (or pixel value) to `<Icon>`.',
  "VAST-3.0-icon-yposition": 'Add `yPosition="top"` or `"bottom"` (or pixel value) to `<Icon>`.',
  "VAST-3.0-icon-resource": "Add `<StaticResource>`, `<IFrameResource>`, or `<HTMLResource>` inside `<Icon>`.",
  "VAST-3.0-icon-attrs": "Ensure `<Icon>` has `program`, `width`, `height`, `xPosition`, and `yPosition`.",
  "VAST-4.0-category-authority": 'Add `authority="..."` to `<Category>`, e.g. `authority="iabtechlab.com"`.',
  "VAST-4.0-companion-clicktracking-id": "Add an `id` attribute to `<CompanionClickTracking>`.",
  "VAST-4.0-wrapper-clickthrough": "Remove `<ClickThrough>` from Wrapper `<VideoClicks>` \u2014 not allowed in VAST 4.0\u20134.1.",
  "VAST-4.1-blockedadcategories-no-authority": 'Add `authority="..."` to `<BlockedAdCategories>` to identify the taxonomy.',
  "VAST-4.1-verification-no-resource": "Add `<JavaScriptResource>` or `<ExecutableResource>` inside `<Verification>`.",
  "VAST-4.1-verification-vendor": 'Add `vendor="company.com-omid"` (domain-useCase format) to `<Verification>`.',
  "VAST-4.1-js-resource-apiframework": 'Add `apiFramework="..."` (e.g. `"omid"`) to `<JavaScriptResource>`.',
  "VAST-4.3-js-resource-browser-optional": 'Add `browserOptional="true"` or `"false"` to `<JavaScriptResource>`.',
  "VAST-4.1-exec-resource-apiframework": 'Add `apiFramework="..."` to `<ExecutableResource>`.',
  "VAST-4.1-exec-resource-type": 'Add `type="..."` (MIME type) to `<ExecutableResource>`.',
  "VAST-4.1-mezzanine-delivery": 'Add `delivery="progressive"` to `<Mezzanine>`.',
  "VAST-4.1-mezzanine-type": 'Add `type="video/mp4"` (or appropriate MIME type) to `<Mezzanine>`.',
  "VAST-4.1-mezzanine-width": 'Add `width="..."` to `<Mezzanine>`.',
  "VAST-4.1-mezzanine-height": 'Add `height="..."` to `<Mezzanine>`.',
  "VAST-4.1-mezzanine-recommended": "Add a `<Mezzanine>` element with the high-quality source file for CTV/SSAI use.",
  "VAST-4.1-ad-serving-id-empty": "Set `<AdServingId>` to a non-empty unique value \u2014 SSAI servers use it for deduplication.",
  "VAST-4.1-vpaid-apiframework": "Replace VPAID with SIMID (`<InteractiveCreativeFile>`) or OMID for measurement.",
  "VAST-4.1-vpaid-in-interactive-context": "Remove the VPAID `<MediaFile>` \u2014 CTV players cannot run VPAID alongside SIMID.",
  "VAST-4.0-mediafile-apiframework": "Use `<InteractiveCreativeFile>` for interactive content instead of `apiFramework` on `<MediaFile>`.",
  "VAST-4.1-survey-deprecated": "Remove `<Survey>` \u2014 deprecated in VAST 4.1. Use `<Extensions>` if needed.",
  "VAST-4.0-conditionalad": "Remove the `conditionalAd` attribute \u2014 deprecated in VAST 4.1.",
  "VAST-2.0-flash-mediafile": "Remove Flash `<MediaFile>` entries \u2014 Flash is no longer supported in any browser.",
  "VAST-4.1-adtype-value": 'Set `adType` to `"video"`, `"audio"`, or `"hybrid"`.',
  "VAST-4.1-companion-renderingmode-value": 'Set `renderingMode` to `"default"`, `"end-card"`, or `"concurrent"`.',
  "VAST-3.0-companion-required-attr": 'Set `required` to `"all"`, `"any"`, or `"none"` on `<CompanionAds>`.',
  "VAST-4.0-tracking-event-removed": "Replace `fullscreen`/`exitFullscreen` with `playerExpand`/`playerCollapse` (VAST 4.0+).",
  "VAST-4.1-tracking-event-value": "Use a valid tracking event name for this VAST version. See IAB VAST spec \xA72.3.6.",
  "VAST-2.0-tracking-https": "Change the tracking URL scheme from `http://` to `https://`.",
  "VAST-2.0-url-empty": "Provide a valid URL value for this element.",
  "VAST-2.0-url-invalid": "Ensure the value is a well-formed URL (e.g. starts with `https://`).",
  "VAST-2.0-duplicate-impression": "Remove duplicate `<Impression>` entries \u2014 each pixel URL should appear only once.",
  "VAST-2.0-version-mismatch": "Align the `version` attribute on `<VAST>` with the actual structure of the document.",
  "VAST-2.0-wrapper-depth": "Reduce the wrapper chain \u2014 the limit is 5 hops.",
  "VAST-2.0-ad-sequence": "Either add `sequence` to all `<Ad>` elements or remove it from all of them.",
  "VAST-2.0-parse-error": "Fix the XML syntax error \u2014 check for unclosed tags, invalid characters, or bad encoding.",
  "VAST-2.0-companion-resource": "Add `<StaticResource>`, `<IFrameResource>`, or `<HTMLResource>` inside `<Companion>`.",
  "VAST-2.0-text-only-element": "Remove child elements from this text-only element \u2014 it should contain only text content.",
  "VAST-2.0-unknown-attribute": "Remove the unrecognised attribute \u2014 it has no meaning in the VAST spec.",
  "VAST-2.0-inline-unknown-child": "Remove or relocate this unrecognised element \u2014 `<InLine>` does not allow it.",
  "VAST-2.0-wrapper-unknown-child": "Remove or relocate this unrecognised element \u2014 `<Wrapper>` does not allow it.",
  "VAST-2.0-creatives-unknown-child": "`<Creatives>` may only contain `<Creative>` elements.",
  "VAST-2.0-creative-unknown-child": "`<Creative>` may only contain `<Linear>`, `<NonLinearAds>`, `<CompanionAds>`, or `<UniversalAdId>`.",
  "VAST-2.0-linear-unknown-child": "Remove the unrecognised child \u2014 see the VAST spec for valid `<Linear>` children.",
  "VAST-2.0-mediafiles-unknown-child": "`<MediaFiles>` may only contain `<MediaFile>`, `<Mezzanine>`, `<InteractiveCreativeFile>`, or `<ClosedCaptionFiles>`.",
  "VAST-2.0-extensions-unknown-child": "`<Extensions>` may only contain `<Extension>` elements.",
  "VAST-2.0-extension-misplaced-element": "This VAST element belongs elsewhere in the document, not inside `<Extension>`.",
  "VAST-2.0-creative-extension-misplaced-element": "This VAST element belongs elsewhere, not inside `<CreativeExtension>`.",
  "VAST-2.0-nonlinearads-unknown-child": "`<NonLinearAds>` may only contain `<TrackingEvents>` or `<NonLinear>` elements.",
  "VAST-2.0-nonlinear-unknown-child": "Remove the unrecognised child \u2014 see VAST spec for valid `<NonLinear>` children.",
  "VAST-2.0-nonlinear-dimensions": "Add `width` and `height` attributes to `<NonLinear>`.",
  "VAST-2.0-companionads-unknown-child": "`<CompanionAds>` may only contain `<Companion>` elements.",
  "VAST-2.0-companion-unknown-child": "Remove the unrecognised child \u2014 see VAST spec for valid `<Companion>` children.",
  "VAST-2.0-companion-dimensions": "Add `width` and `height` attributes to `<Companion>`.",
  "VAST-2.0-videoclicks-unknown-child": "`<VideoClicks>` may only contain `<ClickThrough>`, `<ClickTracking>`, or `<CustomClick>`.",
  "VAST-2.0-trackingevents-unknown-child": "`<TrackingEvents>` may only contain `<Tracking>` elements.",
  "VAST-3.0-icons-unknown-child": "`<Icons>` may only contain `<Icon>` elements.",
  "VAST-3.0-icon-unknown-child": "Remove the unrecognised child from `<Icon>` \u2014 see spec for valid children.",
  "VAST-3.0-iconclicks-unknown-child": "`<IconClicks>` may only contain `<IconClickThrough>`, `<IconClickTracking>`, or `<IconClickFallbackImages>`.",
  "VAST-4.2-icon-fallback-image-width-height": "Add `width` and `height` to `<IconClickFallbackImage>` so the player can size the overlay.",
  "VAST-4.2-closedcaptionfiles-unknown-child": "`<ClosedCaptionFiles>` may only contain `<ClosedCaptionFile>` elements.",
  "VAST-2.0-creativeextensions-unknown-child": "`<CreativeExtensions>` may only contain `<CreativeExtension>` elements."
};
function toVscodeSeverity(severity) {
  switch (severity) {
    case "error":
      return vscode.DiagnosticSeverity.Error;
    case "warning":
      return vscode.DiagnosticSeverity.Warning;
    case "info":
      return vscode.DiagnosticSeverity.Information;
  }
}
function buildDiagnostics(xml, config) {
  const ruleOverrides = config.get("ruleOverrides") ?? {};
  let result;
  try {
    result = validateWithOptions(xml, { rule_overrides: ruleOverrides });
  } catch (e) {
    const range = new vscode.Range(0, 0, 0, 0);
    const d = new vscode.Diagnostic(range, `vastlint internal error: ${e}`, vscode.DiagnosticSeverity.Error);
    d.source = "vastlint";
    return [d];
  }
  const minSeverity = config.get("minSeverity") ?? "info";
  const minLevel = minSeverity === "error" ? 2 : minSeverity === "warning" ? 1 : 0;
  const lines = xml.split("\n");
  const diagnostics = [];
  for (const issue of result.issues) {
    const severityLevel = issue.severity === "error" ? 2 : issue.severity === "warning" ? 1 : 0;
    if (severityLevel < minLevel) continue;
    let range;
    if (issue.line != null && issue.col != null) {
      const lineIdx = issue.line - 1;
      const colIdx = issue.col - 1;
      const lineText = lines[lineIdx] ?? "";
      const tagEnd = (() => {
        const rest = lineText.slice(colIdx + 1);
        const tagNameEnd = rest.search(/[\s>/]/);
        return colIdx + 1 + (tagNameEnd >= 0 ? tagNameEnd : rest.length);
      })();
      range = new vscode.Range(lineIdx, colIdx, lineIdx, tagEnd);
    } else {
      range = new vscode.Range(0, 0, 0, lines[0]?.length ?? 0);
    }
    const meta = ruleCatalog.get(issue.id);
    const fixHint = FIX_HINTS[issue.id];
    const md = new vscode.MarkdownString("", true);
    md.isTrusted = true;
    md.appendMarkdown(`**vastlint** \`${issue.id}\`

`);
    md.appendMarkdown(`${issue.message}

`);
    if (fixHint) {
      md.appendMarkdown(`**Fix:** ${fixHint}

`);
    }
    if (meta?.description && meta.description !== issue.message) {
      md.appendMarkdown(`*${meta.description}*

`);
    }
    md.appendMarkdown(`\u{1F4D6} ${issue.spec_ref}`);
    if (issue.path) {
      md.appendMarkdown(`  \`${issue.path}\``);
    }
    const diagnostic = new vscode.Diagnostic(range, issue.message, toVscodeSeverity(issue.severity));
    diagnostic.source = "vastlint";
    diagnostic.code = issue.id;
    diagnostic.message = issue.message;
    diagnostic._meta = {
      id: issue.id,
      specRef: issue.spec_ref,
      path: issue.path ?? void 0,
      fixHint,
      fullMd: md
    };
    diagnostics.push(diagnostic);
  }
  return diagnostics;
}
var VastlintHoverProvider = class {
  constructor(collection) {
    this.collection = collection;
  }
  provideHover(document, position) {
    const diags = this.collection.get(document.uri);
    if (!diags || diags.length === 0) return void 0;
    const hits = diags.filter((d) => d.range.contains(position) && d._meta);
    if (hits.length === 0) return void 0;
    const md = new vscode.MarkdownString("", true);
    md.isTrusted = true;
    for (const d of hits) {
      const m = d._meta;
      const severityIcon = d.severity === vscode.DiagnosticSeverity.Error ? "\u{1F534}" : d.severity === vscode.DiagnosticSeverity.Warning ? "\u{1F7E1}" : "\u2139\uFE0F";
      md.appendMarkdown(`${severityIcon} **vastlint** \`${m.id}\`

`);
      md.appendMarkdown(`${d.message}

`);
      if (m.fixHint) {
        md.appendMarkdown(`**\u2705 Fix:** ${m.fixHint}

`);
      }
      md.appendMarkdown(`\u{1F4D6} *${m.specRef}*`);
      if (m.path) md.appendMarkdown(`  \u2014  \`${m.path}\``);
      md.appendMarkdown("\n\n---\n\n");
    }
    return new vscode.Hover(md, hits[0].range);
  }
};
function isVastDocument(doc) {
  const lang = doc.languageId;
  if (lang === "log" || lang === "output") return false;
  const head = doc.getText(new vscode.Range(0, 0, Math.min(50, doc.lineCount), 0));
  return head.includes("<VAST") || head.includes("</VAST>");
}
function lintDocument(doc, collection) {
  const config = vscode.workspace.getConfiguration("vastlint", doc.uri);
  if (!config.get("enable", true)) {
    collection.delete(doc.uri);
    return;
  }
  if (!isVastDocument(doc)) {
    collection.delete(doc.uri);
    return;
  }
  const xml = doc.getText();
  const diagnostics = buildDiagnostics(xml, config);
  collection.set(doc.uri, diagnostics);
}
function activate(context) {
  loadCatalog();
  const collection = vscode.languages.createDiagnosticCollection("vastlint");
  context.subscriptions.push(collection);
  const hoverProvider = vscode.languages.registerHoverProvider(
    { language: "xml", scheme: "file" },
    new VastlintHoverProvider(collection)
  );
  context.subscriptions.push(hoverProvider);
  if (vscode.window.activeTextEditor) {
    lintDocument(vscode.window.activeTextEditor.document, collection);
  }
  context.subscriptions.push(
    vscode.window.onDidChangeActiveTextEditor((editor) => {
      if (editor) lintDocument(editor.document, collection);
    })
  );
  context.subscriptions.push(
    vscode.workspace.onDidSaveTextDocument((doc) => lintDocument(doc, collection))
  );
  let debounceTimer;
  context.subscriptions.push(
    vscode.workspace.onDidChangeTextDocument((event) => {
      clearTimeout(debounceTimer);
      debounceTimer = setTimeout(() => {
        lintDocument(event.document, collection);
      }, 500);
    })
  );
  context.subscriptions.push(
    vscode.workspace.onDidCloseTextDocument((doc) => collection.delete(doc.uri))
  );
}
function deactivate() {
}
// Annotate the CommonJS export names for ESM import in node:
0 && (module.exports = {
  activate,
  deactivate
});
//# sourceMappingURL=extension.js.map
