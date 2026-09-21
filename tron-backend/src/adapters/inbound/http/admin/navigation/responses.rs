#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DomainVo {
    domain_code: String,
    name: LocalizedTextVo,
    description: LocalizedTextVo,
    built_in: bool,
    enabled: bool,
    is_default: bool,
    position: i64,
    created_at: i64,
    updated_at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalizedTextVo {
    #[serde(rename = "zh-CN")]
    zh_cn: String,
    #[serde(rename = "en-US")]
    en_us: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MenuVo {
    menu_code: String,
    source: String,
    menu_type: String,
    translations: LocalizedTextVo,
    page_code: Option<String>,
    url: Option<String>,
    icon: Option<String>,
    enabled: bool,
    created_at: i64,
    updated_at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MenuTreeItemVo {
    menu_code: String,
    source: String,
    menu_type: String,
    translations: LocalizedTextVo,
    page_code: Option<String>,
    url: Option<String>,
    icon: Option<String>,
    enabled: bool,
    position: i64,
    children: Vec<MenuTreeItemVo>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RestoreDefaultMenuTreesVo {
    restored_domain_codes: Vec<String>,
    restored_item_count: i64,
}
