use quick_xml::{events::Event, Reader, XmlVersion};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectAdType {
    Wrapper,
    InLine,
    Unknown,
}

impl InspectAdType {
    pub fn as_str(&self) -> &'static str {
        match self {
            InspectAdType::Wrapper => "Wrapper",
            InspectAdType::InLine => "InLine",
            InspectAdType::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectMediaFile {
    pub url: String,
    pub mime_type: String,
    pub delivery: String,
    pub width: String,
    pub height: String,
    pub bitrate: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectDocumentMeta {
    pub ad_type: InspectAdType,
    pub ad_system: String,
    pub ad_title: String,
    pub duration: String,
    pub impression_count: usize,
    pub tracking_event_count: usize,
    pub media_files: Vec<InspectMediaFile>,
    pub companion_count: usize,
    pub wrapper_uri: Option<String>,
}

enum TextTarget {
    None,
    AdSystem,
    AdTitle,
    Duration,
    WrapperUri,
    MediaFileUrl,
}

pub fn inspect_document(xml: &str) -> InspectDocumentMeta {
    let mut meta = InspectDocumentMeta {
        ad_type: InspectAdType::Unknown,
        ad_system: String::new(),
        ad_title: String::new(),
        duration: String::new(),
        impression_count: 0,
        tracking_event_count: 0,
        media_files: Vec::new(),
        companion_count: 0,
        wrapper_uri: None,
    };
    let mut reader = Reader::from_str(xml);
    let mut target = TextTarget::None;
    let mut pending_media_file: Option<(String, String, String, String, String)> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Eof) | Err(_) => break,
            Ok(Event::Start(element)) => {
                let name = std::str::from_utf8(element.name().as_ref())
                    .unwrap_or("")
                    .to_owned();
                match name.as_str() {
                    "InLine" => meta.ad_type = InspectAdType::InLine,
                    "Wrapper" => meta.ad_type = InspectAdType::Wrapper,
                    "Impression" => meta.impression_count += 1,
                    "Tracking" => meta.tracking_event_count += 1,
                    "Companion" => meta.companion_count += 1,
                    "AdSystem" => target = TextTarget::AdSystem,
                    "AdTitle" => target = TextTarget::AdTitle,
                    "Duration" => target = TextTarget::Duration,
                    "VASTAdTagURI" => target = TextTarget::WrapperUri,
                    "MediaFile" => {
                        let mut mime_type = String::new();
                        let mut delivery = String::new();
                        let mut width = String::new();
                        let mut height = String::new();
                        let mut bitrate = String::new();
                        for attr in element.attributes().flatten() {
                            let key = std::str::from_utf8(attr.key.as_ref())
                                .unwrap_or("")
                                .to_owned();
                            let value = attr
                                .decoded_and_normalized_value(
                                    XmlVersion::Implicit1_0,
                                    reader.decoder(),
                                )
                                .map(|value| value.into_owned())
                                .unwrap_or_default();
                            match key.as_str() {
                                "type" => mime_type = value,
                                "delivery" => delivery = value,
                                "width" => width = value,
                                "height" => height = value,
                                "bitrate" => bitrate = value,
                                _ => {}
                            }
                        }
                        pending_media_file = Some((mime_type, delivery, width, height, bitrate));
                        target = TextTarget::MediaFileUrl;
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(text)) => {
                if let Ok(value) = text.xml10_content() {
                    apply_text(value.trim(), &mut meta, &mut target, &mut pending_media_file);
                }
            }
            Ok(Event::CData(text)) => {
                let bytes = text.into_inner();
                if let Ok(value) = std::str::from_utf8(&bytes) {
                    apply_text(value.trim(), &mut meta, &mut target, &mut pending_media_file);
                }
            }
            _ => {}
        }
    }

    meta
}

fn apply_text(
    value: &str,
    meta: &mut InspectDocumentMeta,
    target: &mut TextTarget,
    pending_media_file: &mut Option<(String, String, String, String, String)>,
) {
    if value.is_empty() {
        return;
    }

    match target {
        TextTarget::AdSystem => {
            meta.ad_system = value.to_string();
            *target = TextTarget::None;
        }
        TextTarget::AdTitle => {
            meta.ad_title = value.to_string();
            *target = TextTarget::None;
        }
        TextTarget::Duration => {
            meta.duration = value.to_string();
            *target = TextTarget::None;
        }
        TextTarget::WrapperUri => {
            meta.wrapper_uri = Some(value.to_string());
            *target = TextTarget::None;
        }
        TextTarget::MediaFileUrl => {
            if let Some((mime_type, delivery, width, height, bitrate)) = pending_media_file.take() {
                meta.media_files.push(InspectMediaFile {
                    url: value.to_string(),
                    mime_type,
                    delivery,
                    width,
                    height,
                    bitrate,
                });
            }
            *target = TextTarget::None;
        }
        TextTarget::None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::{inspect_document, InspectAdType};

    #[test]
    fn extracts_wrapper_metadata() {
                let xml = r#"<VAST version="4.2">
  <Ad>
    <Wrapper>
      <AdSystem>Wrapper Co</AdSystem>
      <AdTitle>Wrapper title</AdTitle>
      <Impression>https://example.com/imp</Impression>
      <VASTAdTagURI><![CDATA[https://ads.example.com/downstream.xml]]></VASTAdTagURI>
      <Creatives>
        <Creative>
          <Linear>
            <TrackingEvents>
                            <Tracking event="start">https://example.com/start</Tracking>
            </TrackingEvents>
            <MediaFiles>
                            <MediaFile delivery="progressive" type="video/mp4" width="640" height="360" bitrate="800">
                https://cdn.example.com/ad.mp4
              </MediaFile>
            </MediaFiles>
          </Linear>
        </Creative>
      </Creatives>
      <CompanionAds>
                <Companion width="300" height="250">
                    <StaticResource creativeType="image/png">https://cdn.example.com/companion.png</StaticResource>
        </Companion>
      </CompanionAds>
    </Wrapper>
  </Ad>
</VAST>"#;

        let meta = inspect_document(xml);
        assert_eq!(meta.ad_type, InspectAdType::Wrapper);
        assert_eq!(meta.ad_system, "Wrapper Co");
        assert_eq!(meta.ad_title, "Wrapper title");
        assert_eq!(meta.impression_count, 1);
        assert_eq!(meta.tracking_event_count, 1);
        assert_eq!(meta.companion_count, 1);
        assert_eq!(meta.wrapper_uri.as_deref(), Some("https://ads.example.com/downstream.xml"));
        assert_eq!(meta.media_files.len(), 1);
        assert_eq!(meta.media_files[0].url, "https://cdn.example.com/ad.mp4");
        assert_eq!(meta.media_files[0].mime_type, "video/mp4");
    }
}