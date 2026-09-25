use crate::domain::skin::{Skin, SkinMode};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinVo {
    market_id: Option<String>,
    id: String,
    name: String,
    version: u32,
    mode: &'static str,
    colors: BTreeMap<String, String>,
    layout: BTreeMap<String, String>,
    resources: ResourceVo,
}

#[derive(Serialize)]
pub struct ResourceVo {
    background: Option<String>,
    font: Option<String>,
    css: Option<String>,
}

impl From<Skin> for SkinVo {
    fn from(skin: Skin) -> Self {
        Self {
            market_id: skin.market_id,
            id: skin.id,
            name: skin.name,
            version: skin.version,
            mode: match skin.mode {
                SkinMode::Light => "Light",
                SkinMode::Dark => "Dark",
            },
            colors: skin.colors,
            layout: skin.layout,
            resources: ResourceVo {
                background: skin.resources.background,
                font: skin.resources.font,
                css: skin.resources.css,
            },
        }
    }
}

#[derive(Serialize)]
pub struct ActiveSkinVo {
    pub skin: SkinVo,
}
#[derive(Serialize)]
pub struct ActivateSkinVo {
    pub skin: SkinVo,
}
#[derive(Serialize)]
pub struct InstallSkinVo {
    pub skin: SkinVo,
}
