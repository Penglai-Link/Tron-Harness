#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DomainVo {
    domain_code: String,
    provider_pack_id: Option<String>,
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
struct MenuTreeVo {
    tree_code: String,
    tree_kind: String,
    name: LocalizedTextVo,
    description: LocalizedTextVo,
    built_in: bool,
    enabled: bool,
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
    description: LocalizedTextVo,
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
    description: LocalizedTextVo,
    page_code: Option<String>,
    url: Option<String>,
    icon: Option<String>,
    enabled: bool,
    parent_menu_code: Option<String>,
    position: i64,
    child_display: String,
    children: Vec<MenuTreeItemVo>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RestoreDefaultMenuTreesVo {
    restored_domain_codes: Vec<String>,
    restored_item_count: i64,
}
