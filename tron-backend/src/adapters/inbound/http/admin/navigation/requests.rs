mod request {
    use super::*;

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct LocalizedTextRequest {
        #[serde(rename = "zh-CN")]
        pub zh_cn: String,
        #[serde(rename = "en-US")]
        pub en_us: String,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    pub enum MenuTypeRequest {
        Group,
        Internal,
        ExternalLink,
        Widget,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    pub enum MenuSourceRequest {
        BuiltIn,
        Custom,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    pub enum MenuChildDisplayRequest {
        Inline,
        Flyout,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DomainQueryRequest {
        pub current: i64,
        pub size: i64,
        pub keyword: String,
        pub enabled: Option<bool>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateDomainRequest {
        pub domain_code: String,
        pub name: LocalizedTextRequest,
        pub description: LocalizedTextRequest,
        pub enabled: bool,
        pub is_default: bool,
        pub position: i64,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct UpdateDomainRequest {
        pub name: LocalizedTextRequest,
        pub description: LocalizedTextRequest,
        pub enabled: bool,
        pub is_default: bool,
        pub position: i64,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct MenuQueryRequest {
        pub current: i64,
        pub size: i64,
        pub keyword: String,
        pub source: Option<MenuSourceRequest>,
        pub menu_type: Option<MenuTypeRequest>,
        pub enabled: Option<bool>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateMenuRequest {
        pub menu_code: String,
        pub menu_type: MenuTypeRequest,
        pub translations: LocalizedTextRequest,
        pub page_code: Option<String>,
        pub url: Option<String>,
        pub icon: Option<String>,
        pub enabled: bool,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct UpdateMenuRequest {
        pub menu_type: MenuTypeRequest,
        pub translations: LocalizedTextRequest,
        pub page_code: Option<String>,
        pub url: Option<String>,
        pub icon: Option<String>,
        pub enabled: bool,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct AddMenuItemRequest {
        pub menu_code: String,
        pub parent_menu_code: Option<String>,
        pub position: i64,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct UpdateMenuItemRequest {
        pub parent_menu_code: Option<String>,
        pub position: i64,
        pub child_display: Option<MenuChildDisplayRequest>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CopyMenuTreeRequest {
        #[serde(alias = "sourceDomainCode")]
        pub source_tree_code: String,
        pub source_menu_code: Option<String>,
        pub target_parent_menu_code: Option<String>,
        pub position: i64,
    }

    impl From<MenuTypeRequest> for MenuType {
        fn from(value: MenuTypeRequest) -> Self {
            match value {
                MenuTypeRequest::Group => Self::Group,
                MenuTypeRequest::Internal => Self::Internal,
                MenuTypeRequest::ExternalLink => Self::ExternalLink,
                MenuTypeRequest::Widget => Self::Widget,
            }
        }
    }

    impl From<MenuSourceRequest> for MenuSource {
        fn from(value: MenuSourceRequest) -> Self {
            match value {
                MenuSourceRequest::BuiltIn => Self::BuiltIn,
                MenuSourceRequest::Custom => Self::Custom,
            }
        }
    }

    impl From<MenuChildDisplayRequest> for NavigationMenuChildDisplay {
        fn from(value: MenuChildDisplayRequest) -> Self {
            match value {
                MenuChildDisplayRequest::Inline => Self::Inline,
                MenuChildDisplayRequest::Flyout => Self::Flyout,
            }
        }
    }
}

use request::*;
