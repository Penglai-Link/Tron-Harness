use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkinMode {
    Light,
    Dark,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkinResources {
    pub background: Option<String>,
    pub font: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Skin {
    pub id: String,
    pub name: String,
    pub version: u32,
    pub mode: SkinMode,
    #[serde(default)]
    pub colors: BTreeMap<String, String>,
    #[serde(default)]
    pub layout: BTreeMap<String, String>,
    #[serde(default)]
    pub resources: SkinResources,
}

pub fn valid_skin_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

pub fn valid_resource_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 240
        && value.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        })
}

impl Skin {
    pub fn validate(&self) -> bool {
        valid_skin_id(&self.id)
            && !self.name.trim().is_empty()
            && self.name.len() <= 120
            && self.version == 1
            && self.colors.len() + self.layout.len() <= 512
            && self.colors.iter().chain(&self.layout).all(|(key, value)| {
                key.starts_with("--")
                    && key.len() > 2
                    && key.len() <= 100
                    && key
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
                    && key != "--tron-system-font-size"
                    && !value.is_empty()
                    && value.len() <= 512
                    && !value.contains([';', '{', '}', '<', '>', '\\', '@'])
                    && !value
                        .split_whitespace()
                        .collect::<String>()
                        .to_ascii_lowercase()
                        .contains("url(")
                    && !value.contains("/*")
                    && !value.to_ascii_lowercase().contains("!important")
            })
            && [&self.resources.background, &self.resources.font]
                .into_iter()
                .flatten()
                .all(|path| {
                    if path.starts_with("https://") {
                        path.len() <= 2048
                            && !path.contains(['"', '\'', '\\', '\n', '\r', '<', '>'])
                            && url::Url::parse(path).is_ok_and(|url| {
                                url.host_str().is_some()
                                    && url.username().is_empty()
                                    && url.password().is_none()
                            })
                    } else {
                        valid_resource_path(path)
                    }
                })
    }
}

pub fn resource_mime(path: &str) -> Option<&'static str> {
    match path.rsplit('.').next()? {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        "gif" => Some("image/gif"),
        "woff" => Some("font/woff"),
        "woff2" => Some("font/woff2"),
        "ttf" => Some("font/ttf"),
        "otf" => Some("font/otf"),
        _ => None,
    }
}
