//! Tron's single translation entry point.
//!
//! Every locale resource is stored once under `tron-backend/locales` as a
//! rust-i18n `_version: 2` YAML file. This crate owns the only `rust-i18n`
//! registry, so the application crate, the capability pack and any future
//! crate share one compiled translation table instead of registering their
//! own copy.
//!
//! Business code must reference stable, semantic keys through the shared
//! [`t!`](crate::t) macro or [`translate`] function and pass the locale
//! explicitly. Session-facing code must never rely on the process-global
//! `rust_i18n::set_locale` state: several Agent sessions with different
//! interface languages can run concurrently in the same process.

// Load the same single resource directory for every crate that links this
// package. The path is relative to this crate's `Cargo.toml`, never the
// process working directory.
rust_i18n::i18n!("../../locales", fallback = "en");

use std::borrow::Cow;

/// Locales supported by the Tron interface.
///
/// The Rust variant name keeps the historical `EnUs`/`ZhCn` spelling so the
/// existing `SupportedLocale` alias in `tron-backend` stays source compatible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Locale {
    EnUs,
    #[default]
    ZhCn,
}

impl Locale {
    /// BCP 47 tag stored in locale resources.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EnUs => "en-US",
            Self::ZhCn => "zh-CN",
        }
    }

    /// Locale tag used by rust-i18n resources.
    ///
    /// rust-i18n matches locale chains, so `en-US` falls back to `en` and
    /// `zh-CN` stays exact. Resources use `en` and `zh-CN` as leaf tags.
    pub fn resource_tag(self) -> &'static str {
        match self {
            Self::EnUs => "en",
            Self::ZhCn => "zh-CN",
        }
    }

    /// Resolve the request locale from an `Accept-Language` header value.
    ///
    /// Missing or unsupported languages follow Tron's documented default of
    /// Simplified Chinese.
    pub fn from_request(value: Option<&str>) -> Self {
        value.map(Self::from_raw).unwrap_or(Self::ZhCn)
    }

    /// Resolve a stored locale code, `Accept-Language` value or locale tag.
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
        Self::ZhCn
    }
}

impl AsRef<str> for Locale {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for Locale {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Process-wide default for contexts without a request or session locale.
///
/// Session-facing code must pass an explicit locale instead.
pub const DEFAULT_LOCALE: Locale = Locale::ZhCn;

/// Locale tags compiled into this binary.
pub fn supported_locales() -> [&'static str; 2] {
    ["zh-CN", "en"]
}

/// Force the compiled-in translation table to load once at program start.
pub fn init() {
    let locales = available_locales();
    assert!(
        !locales.is_empty(),
        "tron-i18n: no locale resources were compiled into the binary"
    );
}

/// All locales present in the resources.
pub fn available_locales() -> Vec<String> {
    crate::_rust_i18n_available_locales()
        .into_iter()
        .map(Cow::into_owned)
        .collect()
}

/// Translate `key` for `locale`.
///
/// Lookup follows rust-i18n's chain: exact tag, then `en-US` -> `en`, then the
/// compile-time `en` fallback. A key missing from every locale returns the key
/// itself so the gap is visible instead of silently rendering an empty string.
pub fn translate<'a>(locale: impl AsRef<str>, key: &'a str) -> Cow<'a, str> {
    let locale = locale.as_ref();
    if locale.is_empty() {
        return Cow::Borrowed(key);
    }
    crate::_rust_i18n_try_translate(locale, key).unwrap_or(Cow::Borrowed(key))
}

/// Replace `%{name}` placeholders in an already translated text.
pub fn replace(text: &str, names: &[&str], values: &[String]) -> String {
    rust_i18n::replace_patterns(text, names, values)
}

/// Translate a key and substitute `%{name}` placeholders.
pub fn translate_with<'a>(
    locale: impl AsRef<str>,
    key: &'a str,
    names: &[&str],
    values: &[String],
) -> String {
    let text = translate(locale, key);
    if names.is_empty() {
        return text.into_owned();
    }
    replace(&text, names, values)
}

/// Translate a user-facing string stored as a stable key.
///
/// The `locale` argument is mandatory: the macro intentionally has no arm that
/// silently falls back to the process-global locale.
#[macro_export]
macro_rules! t {
    ($key:expr, locale = $locale:expr $(,)?) => {
        $crate::translate($locale, $key)
    };
    ($key:expr, locale = $locale:expr, $($name:ident = $value:expr),+ $(,)?) => {{
        let __tron_i18n_names: &[&str] = &[$(stringify!($name)),+];
        let __tron_i18n_values: ::std::vec::Vec<::std::string::String> =
            ::std::vec![$(::std::string::ToString::to_string(&$value)),+];
        $crate::translate_with($locale, $key, __tron_i18n_names, &__tron_i18n_values)
    }};
    ($($other:tt)*) => {
        ::std::compile_error!(
            "tron_i18n::t! requires an explicit `locale = ...` argument; \
             pass the request, session or run locale instead of the process-global one"
        )
    };
}