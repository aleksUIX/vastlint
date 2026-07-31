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
pub mod ctv_portfolio;
pub mod daast;
pub mod deprecated;
pub mod macros;
pub mod quality;
pub mod required;
pub mod schema;
pub mod security;
pub mod simid;
pub mod structure;
pub mod values;
pub mod vmap;

use crate::detect::detect_document_type;
use crate::parse::Node;
use crate::parse::VastDocument;
use crate::{
    DetectedVersion, DocumentType, Issue, RuleMeta, RuleSource, Severity, ValidationContext,
};
use RuleSource::{
    CtvAdPortfolio, DaastSpec, DaastXsd, IanaMediaTypes, IndustryBestPractice, Inferred, Iso4217,
    Rfc3986, SimidSpec, VastSpec, VastXsd, VmapSpec, Xml,
};

/// Run all applicable rules against the document and collect issues.
///
/// Dispatches on the document type: VMAP and DAAST documents run their own
/// rule chains; everything else runs the VAST chain. VMAP recurses back into
/// this function for inline `<vmap:VASTAdData>` VAST documents.
pub fn run(
    doc: &VastDocument,
    version: &DetectedVersion,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    if doc.parse_error.is_some() {
        consistency::check(doc, version, ctx, issues);
        return;
    }

    match detect_document_type(doc) {
        DocumentType::Vmap => {
            vmap::check(doc, ctx, issues);
            return;
        }
        DocumentType::Daast => {
            daast::check(doc, ctx, issues);
            return;
        }
        DocumentType::Vast => {}
    }

    required::check(doc, version, ctx, issues);
    schema::check(doc, version, ctx, issues);
    structure::check(doc, version, ctx, issues);
    security::check(doc, version, ctx, issues);
    consistency::check(doc, version, ctx, issues);
    deprecated::check(doc, version, ctx, issues);
    ambiguous::check(doc, version, ctx, issues);
    values::check(doc, version, ctx, issues);
    ctv::check(doc, version, ctx, issues);
    ctv_portfolio::check(doc, version, ctx, issues);
    simid::check(doc, version, ctx, issues);
    macros::check(doc, version, ctx, issues);
    quality::check(doc, version, ctx, issues);
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
    RuleMeta { id: "VAST-2.0-extension-cdata",                      default_severity: Severity::Warning, description: "<Extension> leaf text payload with XML-sensitive characters should be wrapped in CDATA so JSON blobs and URL-rich vendor data do not rely on fragile XML escaping", source: Xml },
    RuleMeta { id: "VAST-2.0-creative-extension-misplaced-element", default_severity: Severity::Warning, description: "<CreativeExtension> contains an element that has a dedicated location in the VAST spec", source: VastSpec },
    RuleMeta { id: "VAST-2.0-creative-extension-cdata",             default_severity: Severity::Warning, description: "<CreativeExtension> leaf text payload with XML-sensitive characters should be wrapped in CDATA so JSON blobs and URL-rich vendor data do not rely on fragile XML escaping", source: Xml },
    // security.rs
    RuleMeta { id: "VAST-2.0-mediafile-https",         default_severity: Severity::Warning, description: "<MediaFile> URL uses HTTP instead of HTTPS — blocked by mixed-content policy on secure inventory",                source: IndustryBestPractice },
    RuleMeta { id: "VAST-2.0-tracking-https",          default_severity: Severity::Warning, description: "Tracking or click URL uses HTTP instead of HTTPS — blocked by mixed-content policy; measurement signal lost",   source: IndustryBestPractice },
    RuleMeta { id: "VAST-2.0-url-cdata",               default_severity: Severity::Warning, description: "URI value is not wrapped in CDATA",                                                             source: VastSpec },
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
    RuleMeta { id: "VAST-4.1-verification-vendor-format",     default_severity: Severity::Warning, description: "<Verification> vendor should use a domain-qualified identifier such as company.com-omid",     source: VastSpec },
    RuleMeta { id: "VAST-4.1-verification-duplicate-vendor",  default_severity: Severity::Warning, description: "<AdVerifications> contains duplicate vendor identifiers",                                     source: VastSpec },
    RuleMeta { id: "VAST-4.1-verification-parameters",        default_severity: Severity::Warning, description: "OMID <Verification> should include non-empty <VerificationParameters>",                   source: VastSpec },
    RuleMeta { id: "VAST-4.1-verification-tracking-reason",   default_severity: Severity::Warning, description: "verificationNotExecuted tracking URI should include the [REASON] macro",                    source: VastSpec },
    RuleMeta { id: "VAST-4.1-blockedadcategories-no-authority", default_severity: Severity::Warning, description: "<BlockedAdCategories> should have authority attribute",                                source: VastSpec },
    RuleMeta { id: "VAST-4.0-category-authority-not-uri",     default_severity: Severity::Warning, description: "<Category> authority attribute is not a valid authority URL",                            source: Rfc3986  },
    RuleMeta { id: "VAST-4.0-category-authority-unknown",     default_severity: Severity::Info,    description: "<Category> authority is not a recognised IAB Content Taxonomy authority",                source: Inferred },
    RuleMeta { id: "VAST-4.1-blockedadcategories-authority-not-uri", default_severity: Severity::Warning, description: "<BlockedAdCategories> authority attribute is not a valid authority URL",           source: Rfc3986  },
    RuleMeta { id: "VAST-4.1-blockedadcategories-authority-unknown", default_severity: Severity::Info,    description: "<BlockedAdCategories> authority is not a recognised IAB Content Taxonomy authority",  source: Inferred },
    RuleMeta { id: "VAST-4.0-interactive-creative-no-api",    default_severity: Severity::Warning, description: "<InteractiveCreativeFile> should have an apiFramework attribute",                        source: VastSpec },
    RuleMeta { id: "VAST-4.1-interactive-creative-type",        default_severity: Severity::Warning, description: "<InteractiveCreativeFile> should have a type attribute identifying the MIME type",     source: IanaMediaTypes },
    RuleMeta { id: "VAST-4.1-verification-vendor",              default_severity: Severity::Error,   description: "<Verification> is missing required vendor attribute",                                  source: VastSpec },
    RuleMeta { id: "VAST-4.1-js-resource-apiframework",         default_severity: Severity::Error,   description: "<JavaScriptResource> is missing required apiFramework attribute",                      source: VastSpec },
    RuleMeta { id: "VAST-4.1-js-resource-apiframework-value",   default_severity: Severity::Warning, description: "OMID <JavaScriptResource> should declare apiFramework=\"omid\"",                          source: VastSpec },
    RuleMeta { id: "VAST-4.1-js-resource-https",                default_severity: Severity::Warning, description: "OMID <JavaScriptResource> URL should use HTTPS",                                          source: IndustryBestPractice },
    RuleMeta { id: "VAST-4.3-js-resource-browser-optional",     default_severity: Severity::Warning, description: "<JavaScriptResource> should have a browserOptional attribute",                        source: VastSpec },
    RuleMeta { id: "VAST-4.1-exec-resource-apiframework",       default_severity: Severity::Error,   description: "<ExecutableResource> is missing required apiFramework attribute",                      source: VastSpec },
    RuleMeta { id: "VAST-4.1-exec-resource-apiframework-value", default_severity: Severity::Warning, description: "OMID <ExecutableResource> should declare apiFramework=\"omid\"",                         source: VastSpec },
    RuleMeta { id: "VAST-4.1-exec-resource-type",               default_severity: Severity::Error,   description: "<ExecutableResource> is missing required type attribute",                              source: VastSpec },
    RuleMeta { id: "VAST-4.1-exec-resource-https",              default_severity: Severity::Warning, description: "OMID <ExecutableResource> reference should use HTTPS when it is a URL",                  source: IndustryBestPractice },
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
    // ctv_portfolio.rs — IAB CTV Ad Portfolio (final 2026-07-22) + VAST 4.4 draft schema
    RuleMeta { id: "VAST-4.4-version-attribute",              default_severity: Severity::Info,    description: "Document declares VAST 4.4, a working-group draft rather than a published spec",         source: VastXsd },
    RuleMeta { id: "VAST-4.4-nonlinear-no-renderable-asset",  default_severity: Severity::Warning, description: "<NonLinear> has a SIMID interactive file but no renderable fallback asset",              source: CtvAdPortfolio },
    RuleMeta { id: "VAST-4.4-nonlinear-mediafiles-empty",     default_severity: Severity::Error,   description: "<NonLinear> <MediaFiles> contains no renderable or interactive asset",                   source: CtvAdPortfolio },
    RuleMeta { id: "VAST-4.4-nonlinear-simid-iframe",         default_severity: Severity::Info,    description: "<IFrameResource apiFramework=\"SIMID\"> is superseded by <InteractiveCreativeFile> in NonLinear", source: CtvAdPortfolio },
    RuleMeta { id: "VAST-4.4-nonlinear-video-no-duration",    default_severity: Severity::Warning, description: "<NonLinear> delivers video but has no <Duration> — quartile tracking cannot fire",       source: CtvAdPortfolio },
    RuleMeta { id: "VAST-4.4-adcom-extension-unknown-signal", default_severity: Severity::Warning, description: "<Extension ext=\"adcom\"> type is not plcmt, pos, playbackmethod or attr",                source: CtvAdPortfolio },
    RuleMeta { id: "VAST-4.4-adcom-extension-type-mismatch",  default_severity: Severity::Warning, description: "<Extension> type attribute and AdCOM payload element name disagree",                     source: CtvAdPortfolio },
    RuleMeta { id: "VAST-4.4-adcom-signal-not-integer",       default_severity: Severity::Error,   description: "AdCOM signal payload in <Extension> is not an integer",                                  source: CtvAdPortfolio },
    RuleMeta { id: "VAST-4.4-adcom-plcmt-value",              default_severity: Severity::Warning, description: "AdCOM plcmt outside the Plcmt Subtypes (Video) range 1-9",                               source: CtvAdPortfolio },
    RuleMeta { id: "VAST-4.4-adcom-playbackmethod-value",     default_severity: Severity::Warning, description: "AdCOM playbackmethod outside the Playback Methods range 1-11",                            source: CtvAdPortfolio },
    RuleMeta { id: "VAST-4.4-adcom-pos-value",                default_severity: Severity::Warning, description: "AdCOM pos outside the Placement Positions range 0-17",                                   source: CtvAdPortfolio },
    RuleMeta { id: "VAST-4.4-adcom-pos-format-mismatch",      default_severity: Severity::Warning, description: "AdCOM pos is not a position the declared plcmt format supports",                         source: CtvAdPortfolio },
    RuleMeta { id: "VAST-4.4-adcom-playbackmethod-format-mismatch", default_severity: Severity::Warning, description: "AdCOM playbackmethod and plcmt name different CTV Ad Portfolio formats",            source: CtvAdPortfolio },
    RuleMeta { id: "VAST-4.4-adcom-attr-not-motion",          default_severity: Severity::Info,    description: "AdCOM attr is not a CTV Ad Portfolio creative attribute (19 QR, 20 alpha, 21-23 motion)",  source: CtvAdPortfolio },
    RuleMeta { id: "VAST-4.4-qrcode-position-attrs",          default_severity: Severity::Error,   description: "<QrCodePosition> requires both xPosition and yPosition",                                 source: VastXsd },
    RuleMeta { id: "VAST-4.4-qrcode-position-percent",        default_severity: Severity::Error,   description: "<QrCodePosition> coordinates must be percentages, not pixels",                           source: VastXsd },
    RuleMeta { id: "VAST-4.4-qrcode-size-attr",               default_severity: Severity::Error,   description: "<QrCodeSize> requires a size attribute",                                                 source: VastXsd },
    RuleMeta { id: "VAST-4.4-qrcode-size-percent",            default_severity: Severity::Error,   description: "<QrCodeSize> size must be a percentage",                                                 source: VastXsd },
    RuleMeta { id: "VAST-4.4-qrcode-missing-scan-url",        default_severity: Severity::Warning, description: "QR code geometry declared without a <QrCodeScanUrl> destination",                        source: CtvAdPortfolio },
    // ctv_portfolio.rs — the VAST 2.0 extension path (extensions/ctv_ad_portfolio.md, 2026-07-17)
    RuleMeta { id: "VAST-2.0-ctv-portfolio-creative-id-required",  default_severity: Severity::Error,   description: "<Extension type=\"ctv_ad_portfolio\"> needs <CreativeId> when the ad has multiple creatives", source: CtvAdPortfolio },
    RuleMeta { id: "VAST-2.0-ctv-portfolio-creative-id-unmatched", default_severity: Severity::Error,   description: "<Extension type=\"ctv_ad_portfolio\"> <CreativeId> matches no <Creative> id in the ad",       source: CtvAdPortfolio },
    RuleMeta { id: "VAST-2.0-ctv-portfolio-mediafiles-required",   default_severity: Severity::Error,   description: "<Extension type=\"ctv_ad_portfolio\"> carries no <MediaFiles>",                              source: CtvAdPortfolio },
    RuleMeta { id: "VAST-2.0-ctv-portfolio-mediafiles-empty",      default_severity: Severity::Error,   description: "<Extension type=\"ctv_ad_portfolio\"> <MediaFiles> contains no renderable or interactive asset", source: CtvAdPortfolio },
    RuleMeta { id: "VAST-2.0-ctv-portfolio-no-renderable-asset",   default_severity: Severity::Warning, description: "<Extension type=\"ctv_ad_portfolio\"> has a SIMID file but no <MediaFile> fallback",          source: CtvAdPortfolio },
    RuleMeta { id: "VAST-2.0-ctv-portfolio-no-duration",           default_severity: Severity::Warning, description: "<Extension type=\"ctv_ad_portfolio\"> delivers a timed asset but has no <Duration>",          source: CtvAdPortfolio },
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
    // macros.rs
    RuleMeta { id: "VAST-2.0-macro-unknown",                  default_severity: Severity::Warning, description: "URL contains a [MACRO] that is not a recognised IAB VAST macro",                          source: VastSpec },
    RuleMeta { id: "VAST-2.0-macro-lowercase",                default_severity: Severity::Warning, description: "Recognised macro is not uppercase — players match macro names case-sensitively",          source: VastSpec },
    RuleMeta { id: "VAST-4.1-macro-deprecated",               default_severity: Severity::Info,    description: "[CONTENTPLAYHEAD]/[MEDIAPLAYHEAD] are deprecated as of VAST 4.1 — use [ADPLAYHEAD]",      source: VastSpec },
    RuleMeta { id: "VAST-2.0-macro-wrong-context",            default_severity: Severity::Info,    description: "Context-restricted macro ([ERRORCODE]/[REASON]) used where it has no defined value",      source: VastSpec },
    RuleMeta { id: "VAST-2.0-macro-uri-unencoded",            default_severity: Severity::Warning, description: "Macro-bearing URL contains characters that must be percent-encoded per RFC 3986",         source: Rfc3986  },
    // quality.rs
    RuleMeta { id: "VAST-2.0-adtitle-quality",                default_severity: Severity::Warning, description: "<AdTitle> value is a known placeholder string; reporting and ops tooling cannot identify the creative",  source: IndustryBestPractice },
    RuleMeta { id: "VAST-2.0-adsystem-quality",               default_severity: Severity::Info,    description: "<AdSystem> value is a known placeholder string; the tag cannot be traced back to its serving system",    source: IndustryBestPractice },
    RuleMeta { id: "VAST-2.0-adsystem-no-version",            default_severity: Severity::Info,    description: "<AdSystem> has no version attribute; provenance is harder to trace in partner discrepancy debugging",    source: IndustryBestPractice },
    // vmap.rs
    RuleMeta { id: "VMAP-1.0-root-version",                   default_severity: Severity::Error,   description: "Root <VMAP> element must have a version attribute",                                      source: VmapSpec },
    RuleMeta { id: "VMAP-1.0-root-version-value",             default_severity: Severity::Warning, description: "<VMAP> version attribute should be \"1.0\" — the only published VMAP version",            source: VmapSpec },
    RuleMeta { id: "VMAP-1.0-root-namespace",                 default_severity: Severity::Warning, description: "<VMAP> should declare the VMAP namespace URI http://www.iab.net/videosuite/vmap",        source: VmapSpec },
    RuleMeta { id: "VMAP-1.0-root-unknown-child",             default_severity: Severity::Error,   description: "<VMAP> may only contain <AdBreak> and <Extensions> elements",                            source: VmapSpec },
    RuleMeta { id: "VMAP-1.0-adbreak-timeoffset",             default_severity: Severity::Error,   description: "<AdBreak> must have a timeOffset attribute",                                             source: VmapSpec },
    RuleMeta { id: "VMAP-1.0-adbreak-timeoffset-format",      default_severity: Severity::Error,   description: "<AdBreak> timeOffset must be hh:mm:ss[.mmm], n%, \"start\", \"end\", or #m",             source: VmapSpec },
    RuleMeta { id: "VMAP-1.0-adbreak-breaktype",              default_severity: Severity::Error,   description: "<AdBreak> must have a breakType attribute",                                              source: VmapSpec },
    RuleMeta { id: "VMAP-1.0-adbreak-breaktype-value",        default_severity: Severity::Error,   description: "<AdBreak> breakType must be a comma-separated list of linear, nonlinear, or display",    source: VmapSpec },
    RuleMeta { id: "VMAP-1.0-adbreak-repeatafter-format",     default_severity: Severity::Warning, description: "<AdBreak> repeatAfter does not match the required hh:mm:ss[.mmm] format",                source: VmapSpec },
    RuleMeta { id: "VMAP-1.0-adbreak-unknown-child",          default_severity: Severity::Error,   description: "<AdBreak> may only contain <AdSource>, <TrackingEvents>, and <Extensions> elements",     source: VmapSpec },
    RuleMeta { id: "VMAP-1.0-adbreak-multiple-adsource",      default_severity: Severity::Error,   description: "<AdBreak> may contain at most one <AdSource> element",                                   source: VmapSpec },
    RuleMeta { id: "VMAP-1.0-adsource-bool-attr",             default_severity: Severity::Warning, description: "<AdSource> allowMultipleAds and followRedirects must be \"true\" or \"false\"",          source: VmapSpec },
    RuleMeta { id: "VMAP-1.0-adsource-content",               default_severity: Severity::Error,   description: "<AdSource> must contain exactly one of <VASTAdData>, <AdTagURI>, or <CustomAdData>",     source: VmapSpec },
    RuleMeta { id: "VMAP-1.0-adtaguri-empty",                 default_severity: Severity::Error,   description: "<AdTagURI> must contain a URI referencing an ad response",                               source: VmapSpec },
    RuleMeta { id: "VMAP-1.0-adtaguri-cdata",                 default_severity: Severity::Error,   description: "<AdTagURI> URI must be contained within a CDATA block",                                  source: VmapSpec },
    RuleMeta { id: "VMAP-1.0-customaddata-cdata",             default_severity: Severity::Error,   description: "<CustomAdData> data must be contained within a CDATA block",                             source: VmapSpec },
    RuleMeta { id: "VMAP-1.0-vastaddata-vast-root",           default_severity: Severity::Error,   description: "<VASTAdData> must contain an embedded <VAST> element (as XML, not CDATA)",               source: VmapSpec },
    RuleMeta { id: "VMAP-1.0-embedded-vast-version",          default_severity: Severity::Info,    description: "Embedded VAST is not version 3.0 — VMAP players are only required to support VAST 3.0",  source: VmapSpec },
    RuleMeta { id: "VMAP-1.0-trackingevents-unknown-child",   default_severity: Severity::Error,   description: "VMAP <TrackingEvents> may only contain <Tracking> elements",                             source: VmapSpec },
    RuleMeta { id: "VMAP-1.0-tracking-event",                 default_severity: Severity::Error,   description: "VMAP <Tracking> must have an event attribute",                                           source: VmapSpec },
    RuleMeta { id: "VMAP-1.0-tracking-event-value",           default_severity: Severity::Error,   description: "VMAP <Tracking> event must be breakStart, breakEnd, or error",                           source: VmapSpec },
    RuleMeta { id: "VMAP-1.0-error-tracking-macro",           default_severity: Severity::Info,    description: "VMAP error tracking URI should include the [ERROR_CODE] macro",                          source: VmapSpec },
    RuleMeta { id: "VMAP-1.0-tracking-url-empty",             default_severity: Severity::Error,   description: "VMAP <Tracking> element does not contain a tracking URI",                                source: VmapSpec },
    RuleMeta { id: "VMAP-1.0-repeatafter-conflict",           default_severity: Severity::Warning, description: "repeatAfter has no effect when timeOffset is \"start\" or \"end\"",                        source: VmapSpec },
    RuleMeta { id: "VMAP-1.0-display-break-no-companions",    default_severity: Severity::Info,    description: "breakType includes \"display\" but the inline VAST has no <CompanionAds> to fill the break", source: Inferred },
    // daast.rs
    RuleMeta { id: "DAAST-1.0-root-version",                  default_severity: Severity::Error,   description: "Root <DAAST> element must have a version attribute",                                     source: DaastSpec },
    RuleMeta { id: "DAAST-1.0-root-version-value",            default_severity: Severity::Warning, description: "<DAAST> version attribute must be a recognised version string (1.0 or 1.1)",             source: DaastXsd },
    RuleMeta { id: "DAAST-1.0-root-has-ad-or-error",          default_severity: Severity::Error,   description: "<DAAST> must contain at least one <Ad> or <Error>",                                      source: DaastSpec },
    RuleMeta { id: "DAAST-1.0-ad-has-inline-or-wrapper",      default_severity: Severity::Error,   description: "Each DAAST <Ad> must contain exactly one <InLine> or <Wrapper>",                         source: DaastSpec },
    RuleMeta { id: "DAAST-1.0-inline-adtitle",                default_severity: Severity::Error,   description: "DAAST <InLine> must contain <AdTitle>",                                                  source: DaastSpec },
    RuleMeta { id: "DAAST-1.0-inline-impression",             default_severity: Severity::Error,   description: "DAAST <InLine> must contain at least one <Impression>",                                  source: DaastSpec },
    RuleMeta { id: "DAAST-1.0-inline-category",               default_severity: Severity::Error,   description: "DAAST <InLine> must contain <Category> (required in DAAST, unlike VAST)",                source: DaastSpec },
    RuleMeta { id: "DAAST-1.0-inline-creatives",              default_severity: Severity::Error,   description: "DAAST <InLine> must contain <Creatives> with at least one <Creative>",                   source: DaastSpec },
    RuleMeta { id: "DAAST-1.0-wrapper-daastadtaguri",         default_severity: Severity::Error,   description: "DAAST <Wrapper> must contain <DAASTAdTagURI>",                                           source: DaastSpec },
    RuleMeta { id: "DAAST-1.0-wrapper-vast-adtaguri",         default_severity: Severity::Warning, description: "<VASTAdTagURI> is a VAST element — DAAST wrappers redirect via <DAASTAdTagURI>",         source: Inferred },
    RuleMeta { id: "DAAST-1.0-wrapper-impression",            default_severity: Severity::Error,   description: "DAAST <Wrapper> must contain at least one <Impression>",                                 source: DaastSpec },
    RuleMeta { id: "DAAST-1.0-videoclicks-element",           default_severity: Severity::Warning, description: "<VideoClicks> is a VAST element — DAAST uses <AdInteractions>",                          source: Inferred },
    RuleMeta { id: "DAAST-1.0-audiointeractions-renamed",     default_severity: Severity::Warning, description: "<AudioInteractions> was renamed <AdInteractions> in the final DAAST release",            source: DaastSpec },
    RuleMeta { id: "DAAST-1.0-linear-duration",               default_severity: Severity::Error,   description: "DAAST <Linear> must contain <Duration>",                                                 source: DaastSpec },
    RuleMeta { id: "DAAST-1.0-duration-format",               default_severity: Severity::Error,   description: "DAAST <Duration> value does not match HH:MM:SS[.mmm] format",                            source: DaastSpec },
    RuleMeta { id: "DAAST-1.0-linear-mediafiles",             default_severity: Severity::Error,   description: "DAAST <Linear> must contain <MediaFiles> with at least one <MediaFile>",                 source: DaastSpec },
    RuleMeta { id: "DAAST-1.0-mediafile-delivery",            default_severity: Severity::Error,   description: "DAAST <MediaFile> must have a delivery attribute",                                       source: DaastSpec },
    RuleMeta { id: "DAAST-1.0-mediafile-delivery-enum",       default_severity: Severity::Error,   description: "DAAST <MediaFile> delivery must be \"progressive\" or \"streaming\"",                    source: DaastXsd },
    RuleMeta { id: "DAAST-1.0-mediafile-type",                default_severity: Severity::Error,   description: "DAAST <MediaFile> must have a type attribute",                                           source: DaastSpec },
    RuleMeta { id: "DAAST-1.0-mediafile-audio-type",          default_severity: Severity::Warning, description: "DAAST <MediaFile> type is a video MIME type — DAAST creative is audio",                  source: Inferred },
    RuleMeta { id: "DAAST-1.0-mediafile-id",                  default_severity: Severity::Warning, description: "DAAST <MediaFile> should have an id attribute (required by the DAAST XSD)",              source: DaastXsd },
    RuleMeta { id: "DAAST-1.0-mediafile-url-empty",           default_severity: Severity::Error,   description: "DAAST <MediaFile> does not contain a media URI",                                         source: DaastSpec },
    RuleMeta { id: "DAAST-1.0-tracking-event-value",          default_severity: Severity::Error,   description: "DAAST <Tracking> event is not in the DAAST audio event set",                             source: DaastXsd },
    RuleMeta { id: "DAAST-1.0-progress-offset",               default_severity: Severity::Error,   description: "DAAST <Tracking event=\"progress\"> requires a valid offset attribute",                  source: DaastSpec },
    RuleMeta { id: "DAAST-1.0-pricing-model",                 default_severity: Severity::Error,   description: "DAAST <Pricing> is missing the required model attribute",                                source: DaastSpec },
    RuleMeta { id: "DAAST-1.0-pricing-model-value",           default_severity: Severity::Warning, description: "DAAST <Pricing> model must be one of cpm, cpc, cpe, cpv, cpo",                           source: DaastXsd },
    RuleMeta { id: "DAAST-1.0-pricing-currency",              default_severity: Severity::Error,   description: "DAAST <Pricing> is missing the required currency attribute",                             source: DaastSpec },
    RuleMeta { id: "DAAST-1.0-error-url-empty",               default_severity: Severity::Warning, description: "DAAST <Error> element is present but contains no URI",                                    source: DaastSpec },
    RuleMeta { id: "DAAST-1.0-error-tracking-macro",          default_severity: Severity::Info,    description: "DAAST <Error> URI does not include the [ERRORCODE] macro",                                source: DaastSpec },
];
