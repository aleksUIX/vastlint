//! Location and version mapping for the final July 2024 CTV Addendum.
//!
//! Payload validation is separate from native VAST validation: the extension
//! encoding can have a newer content model than the enclosing document.

use crate::parse::Node;
use crate::VastVersion;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CtvExtension {
    UniversalAdId,
    InteractiveCreativeFile,
    Mezzanine,
    Icon,
    AdVerifications,
}

impl CtvExtension {
    pub(crate) fn element(self) -> &'static str {
        match self {
            Self::UniversalAdId => "UniversalAdId",
            Self::InteractiveCreativeFile => "InteractiveCreativeFile",
            Self::Mezzanine => "Mezzanine",
            Self::Icon => "Icon",
            Self::AdVerifications => "AdVerifications",
        }
    }
}

pub(crate) fn ctv_extension_type(node: &Node) -> Option<CtvExtension> {
    if !matches!(node.name.as_str(), "Extension" | "CreativeExtension") {
        return None;
    }
    let extension_type = node
        .attrs
        .iter()
        .find(|a| a.name == "type" && !a.namespaced)?;
    Some(match extension_type.value.as_str() {
        "UniversalAdId" => CtvExtension::UniversalAdId,
        "InteractiveCreativeFile" => CtvExtension::InteractiveCreativeFile,
        "Mezzanine" => CtvExtension::Mezzanine,
        "Icon" => CtvExtension::Icon,
        "AdVerifications" => CtvExtension::AdVerifications,
        _ => return None,
    })
}

/// The ancestors include VAST and exclude the extension itself. Unknown
/// versions use the location to recognise an encoding, without inferring 4.x
/// from the payload. Known versions must match the addendum's version table.
pub(crate) fn ctv_extension_at<'a>(
    node: &Node,
    version: Option<VastVersion>,
    ancestors: impl Iterator<Item = &'a str>,
) -> Option<CtvExtension> {
    let kind = ctv_extension_type(node)?;
    let ancestors: Vec<_> = ancestors.collect();
    let inline = ancestors == ["VAST", "Ad", "InLine", "Extensions"];
    let wrapper = ancestors == ["VAST", "Ad", "Wrapper", "Extensions"];
    let creative = ancestors
        == [
            "VAST",
            "Ad",
            "InLine",
            "Creatives",
            "Creative",
            "CreativeExtensions",
        ];
    let allowed = match (node.name.as_str(), kind) {
        ("Extension", CtvExtension::AdVerifications) => {
            // OM SDK Web Video onboarding, July 2024: the VAST 4.1 content
            // model is carried in Extensions on VAST 2.0, 3.0 and 4.0.
            (inline || wrapper) && version.is_none_or(|v| !v.at_least(&VastVersion::V4_1))
        }
        ("Extension", _) => inline && version.is_none_or(|v| v == VastVersion::V2_0),
        ("CreativeExtension", CtvExtension::InteractiveCreativeFile) => {
            creative && version.is_none_or(|v| v.at_least(&VastVersion::V3_0))
        }
        ("CreativeExtension", CtvExtension::UniversalAdId | CtvExtension::Mezzanine) => {
            creative && version.is_none_or(|v| v == VastVersion::V3_0)
        }
        _ => false,
    };
    allowed.then_some(kind)
}

pub(crate) fn ctv_extension_at_path(
    node: &Node,
    version: Option<VastVersion>,
    path: &str,
) -> Option<CtvExtension> {
    let (ancestors, _) = path.rsplit_once('/')?;
    ctv_extension_at(
        node,
        version,
        ancestors
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.split('[').next().unwrap_or(s)),
    )
}
