//! Rule engine.
//!
//! Rules are statically defined functions. There is no dynamic rule loading.
//! Each rule is a function that inspects VastDocument and appends to the
//! issues Vec. Rules are dispatched from the run() function below.
//!
//! Rule ID format: VAST-{earliest-version}-{short-descriptor}
//! The version segment is the earliest VAST version where the rule applies.
//! Rules that apply to all versions use "2.0" as the floor.

pub mod ambiguous;
pub mod consistency;
pub mod ctv;
pub mod deprecated;
pub mod required;
pub mod schema;
pub mod security;
pub mod simid;
pub mod structure;
pub mod values;

use crate::parse::Node;
use crate::parse::VastDocument;
use crate::{DetectedVersion, Issue, RuleMeta, RuleSource, Severity, ValidationContext};
use RuleSource::{
    IanaMediaTypes, IndustryBestPractice, Inferred, Iso4217, Rfc3986, SimidSpec, VastSpec, VastXsd,
    Xml,
};

/// Run all applicable rules against the document and collect issues.
pub fn run(
    doc: &VastDocument,
    version: &DetectedVersion,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    required::check(doc, version, ctx, issues);
    schema::check(doc, version, ctx, issues);
    structure::check(doc, version, ctx, issues);
    security::check(doc, version, ctx, issues);
    consistency::check(doc, version, ctx, issues);
    deprecated::check(doc, version, ctx, issues);
    ambiguous::check(doc, version, ctx, issues);
    values::check(doc, version, ctx, issues);
    ctv::check(doc, version, ctx, issues);
    simid::check(doc, version, ctx, issues);
}

// ── Helper: emit an issue respecting rule overrides ───────────────────────────

/// Emit an issue if the rule is not suppressed by context.
///
/// `default_severity` is the recommended severity as defined in the spec
/// reference docs. The caller's rule_overrides may change or silence it.
///
/// Pass `node` to attach the element's source position to the issue. Pass
/// `None` for document-level issues (e.g. missing root element, parse errors).
#[inline]
#[allow(clippy::too_many_arguments)]
pub(crate) fn emit(
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
    id: &'static str,
    default_severity: Severity,
    message: &'static str,
    path: Option<String>,
    spec_ref: &'static str,
    node: Option<&Node>,
) {
    if let Some(severity) = ctx.resolve(id, default_severity) {
        let (line, col) = match node {
            Some(n) => (Some(n.line), Some(n.col)),
            None => (None, None),
        };
        issues.push(Issue {
            id,
            severity,
            message,
            path,
            spec_ref,
            line,
            col,
        });
    }
}

// ── Rule catalog ──────────────────────────────────────────────────────────────

/// Static catalog of every rule in definition order.
/// The CLI's `vastlint rules` command and config-file validation both read this.
pub static CATALOG: &[RuleMeta] = &[
    // required.rs
    RuleMeta { id: "VAST-2.0-root-element",            default_severity: Severity::Error,   description: "Root element must be <VAST>",                                                                    source: VastSpec },
    RuleMeta { id: "VAST-2.0-root-version",            default_severity: Severity::Error,   description: "<VAST> must have a version attribute",                                                           source: VastSpec },
    RuleMeta { id: "VAST-2.0-root-version-value",      default_severity: Severity::Warning, description: "VAST version attribute must be a recognised version string",                                      source: VastXsd  },
    RuleMeta { id: "VAST-2.0-root-has-ad-or-error",    default_severity: Severity::Error,   description: "<VAST> must contain at least one <Ad> or <Error>",                                               source: VastSpec },
    RuleMeta { id: "VAST-4.0-wrapper-root-error",      default_severity: Severity::Warning, description: "<VAST> root contains both <Ad> and <Error> elements (invalid per VAST 4.0)",                     source: VastSpec },
    RuleMeta { id: "VAST-2.0-ad-has-inline-or-wrapper",default_severity: Severity::Error,   description: "Each <Ad> must contain exactly one <InLine> or <Wrapper>",                                       source: VastSpec },
    RuleMeta { id: "VAST-2.0-inline-adsystem",         default_severity: Severity::Error,   description: "<InLine> must contain <AdSystem>",                                                               source: VastSpec },
    RuleMeta { id: "VAST-2.0-inline-adtitle",          default_severity: Severity::Error,   description: "<InLine> must contain <AdTitle>",                                                                source: VastSpec },
    RuleMeta { id: "VAST-2.0-inline-impression",       default_severity: Severity::Error,   description: "<InLine> must contain at least one <Impression>",                                                source: VastSpec },
    RuleMeta { id: "VAST-2.0-inline-creatives",        default_severity: Severity::Error,   description: "<InLine> must contain <Creatives> with at least one <Creative>",                                 source: VastSpec },
    RuleMeta { id: "VAST-4.1-adservingid-present",     default_severity: Severity::Error,   description: "<InLine> must contain <AdServingId> (VAST 4.1+)",                                                source: VastSpec },
    RuleMeta { id: "VAST-4.0-universaladid-present",   default_severity: Severity::Error,   description: "<Creative> must contain <UniversalAdId> (VAST 4.0+)",                                           source: VastSpec },
    RuleMeta { id: "VAST-4.0-universaladid-idregistry",default_severity: Severity::Error,   description: "<UniversalAdId> must have an idRegistry attribute",                                              source: VastSpec },
    RuleMeta { id: "VAST-2.0-linear-duration",         default_severity: Severity::Error,   description: "<Linear> must contain <Duration>",                                                               source: VastSpec },
    RuleMeta { id: "VAST-2.0-linear-mediafiles",       default_severity: Severity::Error,   description: "<Linear> must contain <MediaFiles> with at least one <MediaFile>",                               source: VastSpec },
    RuleMeta { id: "VAST-2.0-mediafile-delivery",      default_severity: Severity::Error,   description: "<MediaFile> must have a delivery attribute",                                                     source: VastSpec },
    RuleMeta { id: "VAST-2.0-mediafile-type",          default_severity: Severity::Error,   description: "<MediaFile> must have a type attribute",                                                         source: VastSpec },
    RuleMeta { id: "VAST-2.0-mediafile-dimensions",    default_severity: Severity::Error,   description: "<MediaFile> must have width and height attributes",                                              source: VastSpec },
    RuleMeta { id: "VAST-2.0-wrapper-adsystem",        default_severity: Severity::Error,   description: "<Wrapper> must contain <AdSystem>",                                                              source: VastSpec },
    RuleMeta { id: "VAST-2.0-wrapper-impression",      default_severity: Severity::Error,   description: "<Wrapper> must contain at least one <Impression>",                                               source: VastSpec },
    RuleMeta { id: "VAST-2.0-wrapper-vastadtaguri",    default_severity: Severity::Error,   description: "<Wrapper> must contain <VASTAdTagURI>",                                                          source: VastSpec },
    RuleMeta { id: "VAST-2.0-companion-resource",      default_severity: Severity::Error,   description: "<Companion> must contain at least one StaticResource, IFrameResource, or HTMLResource",         source: VastSpec },
    RuleMeta { id: "VAST-2.0-nonlinear-resource",      default_severity: Severity::Error,   description: "InLine <NonLinear> must contain at least one StaticResource, IFrameResource, or HTMLResource",  source: VastSpec },
    // structure.rs
    RuleMeta { id: "VAST-2.0-wrapper-depth",           default_severity: Severity::Error,   description: "Wrapper chain depth exceeds the configured maximum",                                             source: VastSpec },
    RuleMeta { id: "VAST-2.0-ad-sequence",             default_severity: Severity::Warning, description: "Mixed use of sequence attribute across <Ad> elements in a pod",                                 source: VastSpec },
    // schema.rs
    RuleMeta { id: "VAST-2.0-text-only-element",       default_severity: Severity::Error,   description: "Text-only element contains a child element",                                                    source: VastXsd  },
    RuleMeta { id: "VAST-2.0-unknown-attribute",       default_severity: Severity::Warning, description: "Element has an attribute not defined in the VAST spec",                                         source: VastXsd  },
    RuleMeta { id: "VAST-2.0-inline-unknown-child",    default_severity: Severity::Error,   description: "<InLine> contains an unrecognised child element",                                               source: VastXsd  },
    RuleMeta { id: "VAST-2.0-wrapper-unknown-child",   default_severity: Severity::Error,   description: "<Wrapper> contains an unrecognised child element",                                              source: VastXsd  },
    RuleMeta { id: "VAST-2.0-creatives-unknown-child", default_severity: Severity::Error,   description: "<Creatives> may only contain <Creative> elements",                                              source: VastXsd  },
    RuleMeta { id: "VAST-2.0-creative-unknown-child",  default_severity: Severity::Error,   description: "<Creative> contains an unrecognised child element",                                             source: VastXsd  },
    RuleMeta { id: "VAST-2.0-linear-unknown-child",    default_severity: Severity::Error,   description: "<Linear> contains an unrecognised child element",                                               source: VastXsd  },
    RuleMeta { id: "VAST-2.0-trackingevents-unknown-child", default_severity: Severity::Error, description: "<TrackingEvents> may only contain <Tracking> elements",                                      source: VastXsd  },
    RuleMeta { id: "VAST-2.0-mediafiles-unknown-child",default_severity: Severity::Error,   description: "<MediaFiles> contains an unrecognised child element",                                           source: VastXsd  },
    RuleMeta { id: "VAST-2.0-extensions-unknown-child",default_severity: Severity::Error,   description: "<Extensions> may only contain <Extension> elements",                                            source: VastXsd  },
    RuleMeta { id: "VAST-2.0-videoclicks-unknown-child",   default_severity: Severity::Error, description: "<VideoClicks> contains an unrecognised child element",                                        source: VastXsd  },
    RuleMeta { id: "VAST-2.0-nonlinearads-unknown-child",  default_severity: Severity::Error, description: "<NonLinearAds> contains an unrecognised child element",                                       source: VastXsd  },
    RuleMeta { id: "VAST-2.0-nonlinear-unknown-child",     default_severity: Severity::Error, description: "<NonLinear> contains an unrecognised child element",                                          source: VastXsd  },
    RuleMeta { id: "VAST-2.0-companionads-unknown-child",  default_severity: Severity::Error, description: "<CompanionAds> may only contain <Companion> elements",                                        source: VastXsd  },
    RuleMeta { id: "VAST-2.0-companion-unknown-child",     default_severity: Severity::Error, description: "<Companion> contains an unrecognised child element",                                          source: VastXsd  },
    RuleMeta { id: "VAST-3.0-icons-unknown-child",         default_severity: Severity::Error, description: "<Icons> may only contain <Icon> elements",                                                    source: VastXsd  },
    RuleMeta { id: "VAST-3.0-icon-unknown-child",          default_severity: Severity::Error, description: "<Icon> contains an unrecognised child element",                                               source: VastXsd  },
    RuleMeta { id: "VAST-3.0-iconclicks-unknown-child",    default_severity: Severity::Error, description: "<IconClicks> contains an unrecognised child element",                                         source: VastXsd  },
    RuleMeta { id: "VAST-2.0-creativeextensions-unknown-child", default_severity: Severity::Error, description: "<CreativeExtensions> may only contain <CreativeExtension> elements",                     source: VastXsd  },
    RuleMeta { id: "VAST-4.2-closedcaptionfiles-unknown-child", default_severity: Severity::Error, description: "<ClosedCaptionFiles> may only contain <ClosedCaptionFile> elements",                     source: VastXsd  },
    RuleMeta { id: "VAST-2.0-extension-misplaced-element",          default_severity: Severity::Warning, description: "<Extension> contains an element that has a dedicated location in the VAST spec",   source: VastSpec },
    RuleMeta { id: "VAST-2.0-creative-extension-misplaced-element", default_severity: Severity::Warning, description: "<CreativeExtension> contains an element that has a dedicated location in the VAST spec", source: VastSpec },
    // security.rs
    RuleMeta { id: "VAST-2.0-mediafile-https",         default_severity: Severity::Warning, description: "<MediaFile> URL uses HTTP instead of HTTPS — blocked by mixed-content policy on secure inventory",                source: IndustryBestPractice },
    RuleMeta { id: "VAST-2.0-tracking-https",          default_severity: Severity::Warning, description: "Tracking or click URL uses HTTP instead of HTTPS — blocked by mixed-content policy; measurement signal lost",   source: IndustryBestPractice },
    RuleMeta { id: "VAST-2.0-url-empty",               default_severity: Severity::Error,   description: "URL field is empty",                                                                            source: VastSpec },
    RuleMeta { id: "VAST-2.0-url-invalid",             default_severity: Severity::Warning, description: "URL field does not appear to be a valid URI",                                                   source: Rfc3986  },
    // consistency.rs
    RuleMeta { id: "VAST-2.0-parse-error",             default_severity: Severity::Error,   description: "XML parse error — document may be malformed",                                                   source: Xml      },
    RuleMeta { id: "VAST-2.0-version-mismatch",        default_severity: Severity::Warning, description: "Declared version does not match structural signals",                                            source: Inferred },
    RuleMeta { id: "VAST-2.0-duplicate-impression",    default_severity: Severity::Warning, description: "Duplicate <Impression> URL within the same <Ad> — causes double-counted billing and disputes", source: IndustryBestPractice },
    // deprecated.rs
    RuleMeta { id: "VAST-4.0-conditionalad",           default_severity: Severity::Warning, description: "conditionalAd attribute is deprecated as of VAST 4.1",                                         source: VastSpec },
    RuleMeta { id: "VAST-4.1-survey-deprecated",       default_severity: Severity::Warning, description: "<Survey> is deprecated as of VAST 4.1",                                                        source: VastSpec },
    RuleMeta { id: "VAST-4.1-vpaid-apiframework",      default_severity: Severity::Warning, description: "apiFramework=\"VPAID\" is deprecated as of VAST 4.1",                                          source: VastSpec },
    RuleMeta { id: "VAST-4.0-mediafile-apiframework",  default_severity: Severity::Info,    description: "<MediaFile apiFramework> is deprecated in VAST 4.0+ — use <InteractiveCreativeFile>",          source: VastSpec },
    RuleMeta { id: "VAST-2.0-flash-mediafile",         default_severity: Severity::Warning, description: "Flash-based MediaFile type is no longer supported",                                             source: Inferred },
    // ambiguous.rs
    RuleMeta { id: "VAST-3.0-progress-offset",         default_severity: Severity::Error,   description: "<Tracking event=\"progress\"> requires an offset attribute",                                    source: VastSpec },
    RuleMeta { id: "VAST-3.0-icon-attrs",              default_severity: Severity::Warning, description: "Icon missing recommended attributes (program/width/height/position)",                           source: VastSpec },
    RuleMeta { id: "VAST-2.0-nonlinear-dimensions",    default_severity: Severity::Warning, description: "<NonLinear> missing width or height",                                                           source: VastSpec },
    RuleMeta { id: "VAST-2.0-companion-dimensions",    default_severity: Severity::Warning, description: "<Companion> missing width or height",                                                           source: VastSpec },
    RuleMeta { id: "VAST-4.0-wrapper-clickthrough",    default_severity: Severity::Warning, description: "<ClickThrough> inside Wrapper <VideoClicks> was removed in VAST 4.0 (re-allowed in 4.2)",      source: VastSpec },
    RuleMeta { id: "VAST-4.2-icon-fallback-image-width-height", default_severity: Severity::Warning, description: "<IconClickFallbackImage> should have width and height attributes",                     source: VastSpec },
    RuleMeta { id: "VAST-2.0-linear-tracking-quartiles", default_severity: Severity::Warning, description: "<Linear> has no standard quartile tracking events — impression serves but measurement system receives no signal", source: IndustryBestPractice },
    // required.rs — VAST 3.0+ additions
    RuleMeta { id: "VAST-3.0-pricing-model",                  default_severity: Severity::Error,   description: "<Pricing> missing required model attribute",                                             source: VastSpec },
    RuleMeta { id: "VAST-3.0-pricing-currency",               default_severity: Severity::Error,   description: "<Pricing> missing required currency attribute",                                          source: VastSpec },
    RuleMeta { id: "VAST-3.0-pricing-model-case",             default_severity: Severity::Warning, description: "<Pricing> model value should be lowercase in VAST 3.0 (cpm/cpc/cpe/cpv)",               source: VastXsd  },
    RuleMeta { id: "VAST-3.0-pricing-currency-format",        default_severity: Severity::Warning, description: "<Pricing> currency attribute must be a 3-letter ISO-4217 code",                          source: Iso4217  },
    RuleMeta { id: "VAST-3.0-icon-program",                   default_severity: Severity::Error,   description: "<Icon> missing required program attribute",                                              source: VastSpec },
    RuleMeta { id: "VAST-3.0-icon-width",                     default_severity: Severity::Error,   description: "<Icon> missing required width attribute",                                                source: VastSpec },
    RuleMeta { id: "VAST-3.0-icon-height",                    default_severity: Severity::Error,   description: "<Icon> missing required height attribute",                                               source: VastSpec },
    RuleMeta { id: "VAST-3.0-icon-xposition",                 default_severity: Severity::Error,   description: "<Icon> missing required xPosition attribute",                                            source: VastSpec },
    RuleMeta { id: "VAST-3.0-icon-yposition",                 default_severity: Severity::Error,   description: "<Icon> missing required yPosition attribute",                                            source: VastSpec },
    RuleMeta { id: "VAST-3.0-icon-resource",                  default_severity: Severity::Error,   description: "<Icon> must have at least one resource element",                                         source: VastSpec },
    RuleMeta { id: "VAST-4.0-category-authority",             default_severity: Severity::Error,   description: "<Category> missing required authority attribute",                                        source: VastSpec },
    RuleMeta { id: "VAST-4.0-companion-clicktracking-id",     default_severity: Severity::Error,   description: "<CompanionClickTracking> missing required id attribute",                                 source: VastSpec },
    RuleMeta { id: "VAST-4.0-universaladid-idvalue",          default_severity: Severity::Error,   description: "<UniversalAdId> missing required idValue attribute (VAST 4.0)",                         source: VastSpec },
    RuleMeta { id: "VAST-4.1-universaladid-idvalue-removed",  default_severity: Severity::Warning, description: "<UniversalAdId> idValue attribute was removed in VAST 4.1",                             source: VastSpec },
    RuleMeta { id: "VAST-4.1-universaladid-content",          default_severity: Severity::Error,   description: "<UniversalAdId> must have text content in VAST 4.1+",                                   source: VastSpec },
    RuleMeta { id: "VAST-4.1-mezzanine-delivery",             default_severity: Severity::Error,   description: "<Mezzanine> missing required delivery attribute",                                        source: VastSpec },
    RuleMeta { id: "VAST-4.1-mezzanine-type",                 default_severity: Severity::Error,   description: "<Mezzanine> missing required type attribute",                                            source: VastSpec },
    RuleMeta { id: "VAST-4.1-mezzanine-width",                default_severity: Severity::Error,   description: "<Mezzanine> missing required width attribute",                                           source: VastSpec },
    RuleMeta { id: "VAST-4.1-mezzanine-height",               default_severity: Severity::Error,   description: "<Mezzanine> missing required height attribute",                                          source: VastSpec },
    RuleMeta { id: "VAST-4.1-verification-no-resource",       default_severity: Severity::Warning, description: "<Verification> should have JavaScriptResource or ExecutableResource",                    source: VastSpec },
    RuleMeta { id: "VAST-4.1-blockedadcategories-no-authority", default_severity: Severity::Warning, description: "<BlockedAdCategories> should have authority attribute",                                source: VastSpec },
    RuleMeta { id: "VAST-4.0-interactive-creative-no-api",    default_severity: Severity::Warning, description: "<InteractiveCreativeFile> should have an apiFramework attribute",                        source: VastSpec },
    RuleMeta { id: "VAST-4.1-interactive-creative-type",        default_severity: Severity::Warning, description: "<InteractiveCreativeFile> should have a type attribute identifying the MIME type",     source: IanaMediaTypes },
    RuleMeta { id: "VAST-4.1-verification-vendor",              default_severity: Severity::Error,   description: "<Verification> is missing required vendor attribute",                                  source: VastSpec },
    RuleMeta { id: "VAST-4.1-js-resource-apiframework",         default_severity: Severity::Error,   description: "<JavaScriptResource> is missing required apiFramework attribute",                      source: VastSpec },
    RuleMeta { id: "VAST-4.3-js-resource-browser-optional",     default_severity: Severity::Warning, description: "<JavaScriptResource> should have a browserOptional attribute",                        source: VastSpec },
    RuleMeta { id: "VAST-4.1-exec-resource-apiframework",       default_severity: Severity::Error,   description: "<ExecutableResource> is missing required apiFramework attribute",                      source: VastSpec },
    RuleMeta { id: "VAST-4.1-exec-resource-type",               default_severity: Severity::Error,   description: "<ExecutableResource> is missing required type attribute",                              source: VastSpec },
    // values.rs
    RuleMeta { id: "VAST-2.0-duration-format",                default_severity: Severity::Error,   description: "<Duration> value does not match HH:MM:SS[.mmm] format",                                 source: VastSpec },
    RuleMeta { id: "VAST-2.0-mediafile-delivery-enum",        default_severity: Severity::Error,   description: "<MediaFile> delivery must be \"progressive\" or \"streaming\"",                          source: VastXsd  },
    RuleMeta { id: "VAST-3.0-skipoffset-format",              default_severity: Severity::Warning, description: "Linear skipoffset does not match HH:MM:SS[.mmm] or n% format",                          source: VastSpec },
    RuleMeta { id: "VAST-3.0-progress-offset-format",         default_severity: Severity::Warning, description: "Tracking progress offset does not match required format",                                source: VastSpec },
    RuleMeta { id: "VAST-3.0-skip-event-no-skipoffset",       default_severity: Severity::Warning, description: "skip tracking event present but Linear has no skipoffset attribute",                     source: VastSpec },
    RuleMeta { id: "VAST-3.0-minmaxbitrate-pair",             default_severity: Severity::Error,   description: "<MediaFile> must have both minBitrate and maxBitrate or neither",                        source: VastSpec },
    RuleMeta { id: "VAST-3.0-bitrate-conflict",               default_severity: Severity::Warning, description: "<MediaFile> has both bitrate and minBitrate/maxBitrate",                                 source: VastSpec },
    RuleMeta { id: "VAST-4.0-tracking-event-removed",         default_severity: Severity::Warning, description: "fullscreen/exitFullscreen tracking events were removed in VAST 4.0",                    source: VastSpec },
    RuleMeta { id: "VAST-4.1-tracking-event-value",           default_severity: Severity::Error,   description: "Tracking event attribute not in the valid set for this VAST version",                   source: VastXsd  },
    RuleMeta { id: "VAST-4.1-adtype-value",                   default_severity: Severity::Warning, description: "Ad adType must be video, audio, or hybrid",                                              source: VastXsd  },
    RuleMeta { id: "VAST-4.1-companion-renderingmode-value",  default_severity: Severity::Warning, description: "Companion renderingMode must be default, end-card, or concurrent",                       source: VastXsd  },
    RuleMeta { id: "VAST-3.0-companion-required-attr",        default_severity: Severity::Warning, description: "<CompanionAds> required attribute must be all, any, or none",                            source: VastXsd  },
    // ctv.rs
    RuleMeta { id: "VAST-4.1-mezzanine-recommended",          default_severity: Severity::Info,    description: "<MediaFiles> has no <Mezzanine> — ad-stitching servers may reject in CTV/SSAI contexts",  source: IndustryBestPractice },
    RuleMeta { id: "VAST-4.1-vpaid-in-interactive-context",   default_severity: Severity::Warning, description: "VPAID MediaFile alongside InteractiveCreativeFile — VPAID unsupported in CTV, zero fill",  source: IndustryBestPractice },
    RuleMeta { id: "VAST-4.1-ad-serving-id-empty",            default_severity: Severity::Warning, description: "<AdServingId> is present but empty",                                                     source: Inferred },
    // simid.rs
    RuleMeta { id: "SIMID-1.0-simid-type-required",           default_severity: Severity::Error,   description: "<InteractiveCreativeFile apiFramework=\"SIMID\"> must have type=\"text/html\"",           source: SimidSpec },
    RuleMeta { id: "SIMID-1.0-simid-url-empty",               default_severity: Severity::Error,   description: "<InteractiveCreativeFile apiFramework=\"SIMID\"> must contain a non-empty URL",           source: SimidSpec },
    RuleMeta { id: "SIMID-1.0-simid-url-https",               default_severity: Severity::Error,   description: "<InteractiveCreativeFile apiFramework=\"SIMID\"> URL must use HTTPS",                    source: SimidSpec },
    RuleMeta { id: "SIMID-1.0-simid-variable-duration-value", default_severity: Severity::Warning, description: "<InteractiveCreativeFile> variableDuration attribute must be \"true\" when present",     source: SimidSpec },
    RuleMeta { id: "SIMID-1.0-simid-mediafile-required",      default_severity: Severity::Error,   description: "Linear SIMID ad must include a video/audio <MediaFile> alongside the interactive creative", source: SimidSpec },
    RuleMeta { id: "SIMID-1.1-nonlinear-simid-no-iframe",     default_severity: Severity::Error,   description: "<NonLinear apiFramework=\"SIMID\"> must contain an <IFrameResource>",                    source: SimidSpec },
    RuleMeta { id: "SIMID-1.1-iframe-simid-type-required",    default_severity: Severity::Warning, description: "<IFrameResource> in SIMID <NonLinear> should have type=\"text/html\"",                   source: SimidSpec },
    RuleMeta { id: "SIMID-1.1-iframe-simid-url-empty",        default_severity: Severity::Error,   description: "<IFrameResource> in SIMID <NonLinear> must contain a non-empty URL",                     source: SimidSpec },
    RuleMeta { id: "SIMID-1.1-iframe-simid-url-https",        default_severity: Severity::Error,   description: "<IFrameResource> in SIMID <NonLinear> URL must use HTTPS",                               source: SimidSpec },
];
