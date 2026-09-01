use std::{error::Error, fmt};

use crate::shared::id::SnowflakeId;

pub const ADVANCED_DOMAIN_CODE: &str = "ADVANCED";
pub const ZH_CN: &str = "zh-CN";
pub const EN_US: &str = "en-US";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuSource {
    BuiltIn,
    Custom,
}

impl MenuSource {
    pub fn as_storage(self) -> &'static str {
        match self {
            Self::BuiltIn => "BuiltIn",
            Self::Custom => "Custom",
        }
    }

    pub fn from_storage(value: &str) -> Result<Self, NavigationError> {
        match value {
            "BuiltIn" => Ok(Self::BuiltIn),
            "Custom" => Ok(Self::Custom),
            _ => Err(NavigationError::StorageFailure),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuType {
    Group,
    Internal,
    ExternalLink,
}

impl MenuType {
    pub fn as_storage(self) -> &'static str {
        match self {
            Self::Group => "Group",
            Self::Internal => "Internal",
            Self::ExternalLink => "ExternalLink",
        }
    }

    pub fn from_storage(value: &str) -> Result<Self, NavigationError> {
        match value {
            "Group" => Ok(Self::Group),
            "Internal" => Ok(Self::Internal),
            "ExternalLink" => Ok(Self::ExternalLink),
            _ => Err(NavigationError::StorageFailure),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizedText {
    pub zh_cn: String,
    pub en_us: String,
}

impl LocalizedText {
    pub fn value(&self, locale: &str) -> &str {
        if locale.eq_ignore_ascii_case(EN_US) || locale.to_ascii_lowercase().starts_with("en-") {
            &self.en_us
        } else {
            &self.zh_cn
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationDomain {
    pub id: SnowflakeId,
    pub domain_code: String,
    pub name: LocalizedText,
    pub description: LocalizedText,
    pub built_in: bool,
    pub enabled: bool,
    pub is_default: bool,
    pub position: i64,
    pub visible_roles: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationMenu {
    pub id: SnowflakeId,
    pub menu_code: String,
    pub source: MenuSource,
    pub menu_type: MenuType,
    pub translations: LocalizedText,
    pub page_code: Option<String>,
    pub url: Option<String>,
    pub icon: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationTreeItem {
    pub id: SnowflakeId,
    pub menu: NavigationMenu,
    pub parent_menu_code: Option<String>,
    pub position: i64,
    pub visible_roles: Vec<String>,
    pub children: Vec<Self>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuQuery {
    pub current: i64,
    pub size: i64,
    pub keyword: String,
    pub source: Option<MenuSource>,
    pub menu_type: Option<MenuType>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationPage<T> {
    pub current: i64,
    pub size: i64,
    pub total: i64,
    pub records: Vec<T>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigationError {
    DomainNotFound,
    MenuNotFound,
    ItemNotFound,
    DuplicateDomainCode,
    DuplicateMenuCode,
    DuplicateDefaultDomain,
    DuplicateDomainItem,
    DuplicateDomainItems(Vec<String>),
    InvalidDomainCode,
    InvalidMenuCode,
    InvalidName,
    InvalidTranslation,
    InvalidMenuTarget,
    InvalidExternalUrl,
    InvalidParent,
    InvalidHierarchy,
    MaxDepthExceeded,
    NonLeafItem,
    BuiltInMenuProtected,
    BuiltInDomainProtected,
    MenuInUse,
    DomainNotEmpty,
    DomainAccessDenied,
    ManagementAccessDenied,
    InvalidRoleCode,
    StorageFailure,
}

impl fmt::Display for NavigationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Self::DuplicateDomainItems(codes) = self {
            return write!(
                formatter,
                "menus already exist in this domain: {}",
                codes.join(", ")
            );
        }
        formatter.write_str(match self {
            Self::DomainNotFound => "navigation domain was not found",
            Self::MenuNotFound => "navigation menu was not found",
            Self::ItemNotFound => "navigation menu item was not found",
            Self::DuplicateDomainCode => "navigation domain code already exists",
            Self::DuplicateMenuCode => "navigation menu code already exists",
            Self::DuplicateDefaultDomain => "only one navigation domain can be default",
            Self::DuplicateDomainItem => "menu already exists in this domain",
            Self::DuplicateDomainItems(_) => "menus already exist in this domain",
            Self::InvalidDomainCode => "navigation domain code is invalid",
            Self::InvalidMenuCode => "navigation menu code is invalid",
            Self::InvalidName => "navigation name is required",
            Self::InvalidTranslation => "zh-CN and en-US translations are required",
            Self::InvalidMenuTarget => "menu target does not match its type",
            Self::InvalidExternalUrl => "external menu URL must use http or https",
            Self::InvalidParent => "navigation parent is invalid",
            Self::InvalidHierarchy => "navigation hierarchy operation is invalid",
            Self::MaxDepthExceeded => "navigation menu depth exceeds the configured maximum",
            Self::NonLeafItem => "navigation menu item must be a leaf",
            Self::BuiltInMenuProtected => "built-in navigation menu is protected",
            Self::BuiltInDomainProtected => "built-in navigation domain is protected",
            Self::MenuInUse => "navigation menu is still used by a domain",
            Self::DomainNotEmpty => "navigation domain must be empty before deletion",
            Self::DomainAccessDenied => "navigation domain is not accessible",
            Self::ManagementAccessDenied => "navigation management permission is required",
            Self::InvalidRoleCode => "navigation role code is invalid",
            Self::StorageFailure => "navigation storage operation failed",
        })
    }
}

impl Error for NavigationError {}

pub fn validate_domain_code(value: &str) -> Result<String, NavigationError> {
    validate_code(value, NavigationError::InvalidDomainCode)
}

pub fn validate_menu_code(value: &str) -> Result<String, NavigationError> {
    validate_code(value, NavigationError::InvalidMenuCode)
}

fn validate_code(value: &str, error: NavigationError) -> Result<String, NavigationError> {
    let value = value.trim().to_ascii_uppercase();
    if value.is_empty()
        || value.len() > 64
        || !value
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
    {
        return Err(error);
    }
    Ok(value)
}

pub fn validate_translations(text: &LocalizedText) -> Result<(), NavigationError> {
    if text.zh_cn.trim().is_empty() || text.en_us.trim().is_empty() {
        return Err(NavigationError::InvalidTranslation);
    }
    Ok(())
}

pub fn validate_roles(roles: &[String]) -> Result<(), NavigationError> {
    if roles.iter().any(|role| {
        role.trim().is_empty()
            || role.len() > 64
            || !role
                .chars()
                .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
    }) {
        return Err(NavigationError::InvalidRoleCode);
    }
    Ok(())
}

pub fn validate_menu_target(
    menu_type: MenuType,
    page_code: Option<&str>,
    url: Option<&str>,
) -> Result<(), NavigationError> {
    match menu_type {
        MenuType::Group if page_code.is_none() && url.is_none() => Ok(()),
        MenuType::Internal
            if page_code.is_some_and(|value| !value.trim().is_empty()) && url.is_none() =>
        {
            Ok(())
        }
        MenuType::ExternalLink if page_code.is_none() && url.is_some_and(is_http_url) => Ok(()),
        MenuType::ExternalLink => Err(NavigationError::InvalidExternalUrl),
        _ => Err(NavigationError::InvalidMenuTarget),
    }
}

fn is_http_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}
