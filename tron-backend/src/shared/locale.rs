#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportedLocale {
    EnUs,
    ZhCn,
}

impl SupportedLocale {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EnUs => "en-US",
            Self::ZhCn => "zh-CN",
        }
    }

    pub fn from_request(value: Option<&str>) -> Self {
        value.map(Self::from_raw).unwrap_or(Self::EnUs)
    }

    pub fn from_raw(value: &str) -> Self {
        for segment in value.split(',') {
            let tag = segment
                .split_once(';')
                .map(|(head, _)| head)
                .unwrap_or(segment)
                .trim()
                .to_ascii_lowercase();
            if tag.is_empty() {
                continue;
            }
            if tag == "zh-cn" || tag == "zh" || tag.starts_with("zh-") {
                return Self::ZhCn;
            }
            if tag == "en-us" || tag == "en" || tag.starts_with("en-") {
                return Self::EnUs;
            }
        }
        Self::EnUs
    }
}

pub fn resolve_request_locale(value: Option<&str>) -> String {
    SupportedLocale::from_request(value).as_str().to_string()
}
