# CTV Addendum 2024 support

vastlint validates supplied standardized payloads from the final July 2024
[IAB CTV Addendum](https://iabtechlab.com/wp-content/uploads/2024/07/VAST-CTV-Addendum-2024-FINAL.pdf).
The enclosing document keeps its declared VAST version. These extensions do not
imply a version upgrade and do not make optional features mandatory.

## Supported locations

- VAST 2.0: `InLine/Extensions/Extension` with type `UniversalAdId`,
  `InteractiveCreativeFile`, `Mezzanine` or `Icon`. Each extension contains its
  matching element directly.
- VAST 3.0: `InLine/Creatives/Creative/CreativeExtensions/CreativeExtension`
  with type `UniversalAdId`, `InteractiveCreativeFile` or `Mezzanine`.
- VAST 4.x: the same creative-extension location with type
  `InteractiveCreativeFile` for the richer addendum payload. Native VAST 4.x
  nodes retain their existing content model.
- VAST 2.0, 3.0 and 4.0: `InLine/Extensions/Extension` or
  `Wrapper/Extensions/Extension` with type `AdVerifications`, containing an
  `AdVerifications` element using the VAST 4.1 content model. This follows the
  [OM SDK Web Video onboarding guide](https://iabtechlab.com/wp-content/uploads/2024/08/Open-Measurement-SDK-for-Web-Video-Integrator-Onboarding.pdf)
  referenced by the addendum.
- Native Icons in VAST 3.0 and later, and the VAST 2.0 Icon extension, may carry
  `regs="0"` (not applicable) or `regs="1"` (DSA).

The standardized type values are case-sensitive. A recognized type at a wrong
location/version receives `VAST-2.0-ctv-extension-location`. Expected elements
hidden in a vendor wrapper are not treated as standardized payloads. Existing
vendor payload handling and misplaced-element warnings still apply.

## Payload validation

UniversalAdId extensions require non-empty `idRegistry` and element text. The
text may be `unknown` for an unregistered ad. A registry value that is not a
root domain or `unknown` receives a format warning. Validation does not contact
registries or verify that the ID was registered.

Mezzanine extensions use the typed delivery model: `delivery`, `type`, `width`
and `height` are required, and their values and asset URI are checked. Native
VAST 4.0 Mezzanine elements remain simple URI values.

InteractiveCreativeFile extensions require `type="text/html"` and
`apiFramework="SIMID"`. They accept an HTTPS URL, an HTML data URI, or a
non-empty CDATA `HTMLResource`, plus optional `AdParameters`. The addendum's
`variableDuration` is an XML boolean, so `true`, `false`, `1` and `0` are
accepted. Native SIMID rules retain their existing behavior. A SIMID extension
also needs a playable media asset in the creative it describes.

Icon extension content follows the VAST 3.0 model. OMID extension content is
checked for schema shape, required resources, vendor and framework attributes,
parameters, duplicate vendors and verification tracking events. The existing
native `browserOptional` recommendation does not apply to this extension model,
where the attribute is optional in the VAST 4.1 schema.

## Creative and media bindings

In the VAST 2.0 ad-level encoding, `UniversalAdId@creativeId` selects
`Creative@id`. `InteractiveCreativeFile@mediaFileId` and
`Mezzanine@mediaFileId` select `MediaFile@id` within the same ad's Linear
creatives. An explicit binding must match exactly one target. Missing or
duplicate targets receive `VAST-2.0-ctv-binding-unmatched`.

Without an explicit binding, the addendum defines a first-creative and
first-media association. A tag with multiple creatives receives the info-level
`VAST-2.0-ctv-binding-implicit` advisory. Creative-level extensions already
identify their creative by containment.

## Rule configuration

The twelve new rules have source `IAB CTV Addendum 2024` in the catalog.
Existing media, Icon, OMID and SIMID rule IDs are reused where their requirements
apply to the extension content. Individual findings can be suppressed or have
their severity changed through the existing rule overrides. No additional
profile or flag is needed to validate supplied extensions.

`regs="1"` identifies the intended regulation. It does not certify that the
advertiser disclosure content or player behavior satisfies that regulation.
