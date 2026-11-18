//! Request locale resolution.
//!
//! The locale type itself lives in the shared `tron-i18n` crate so every
//! crate, every translation lookup and every persisted locale tag uses the
//! same two supported values. `SupportedLocale` remains the historical name
//! used across the backend; new code may use [`TronLocale`].

pub use tron_i18n::Locale as SupportedLocale;
pub use tron_i18n::Locale as TronLocale;

/// Resolve an `Accept-Language` header value to the locale tag sent to clients
/// and stored with sessions. Unsupported or missing languages default to
/// Simplified Chinese.
pub fn resolve_request_locale(value: Option<&str>) -> String {
    SupportedLocale::from_request(value).as_str().to_string()
}