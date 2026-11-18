//! Locale-independent user-facing messages that cross plain `String` boundaries.
//!
//! Application/domain code must never store translated text: the request,
//! session or job display locale is only known at the display edge. Some
//! payloads, however, are plain strings by contract:
//!
//! - persisted background-job diagnostics (`document.error`, `task.error`,
//!   quality-run warnings) keep their JSON shape, so the stable message key and
//!   its `%{name}` arguments are encoded into the string itself;
//! - legacy error payloads that other modules still construct with a `String`
//!   remain source compatible until every producer is migrated.
//!
//! [`MessageText`] is the structured value (`tron_i18n::Message` equivalent with
//! introspectable arguments), [`StoredMessage`] decodes either an encoded
//! structured message or legacy plain text and renders it for an explicit
//! locale.

use std::borrow::Cow;
use std::fmt;

use serde::{Deserialize, Serialize};
use tron_i18n::translate_with;

/// Marker that distinguishes an encoded structured message from legacy text.
pub const ENCODED_PREFIX: &str = "__i18n__";

#[derive(Serialize, Deserialize)]
struct WireMessage {
    key: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    args: Vec<(String, String)>,
}

/// Stable translation key plus named `%{name}` argument values.
///
/// Unlike [`tron_i18n::Message`], the key may be owned after decoding and the
/// argument names are introspectable, so a persisted message can be encoded and
/// decoded again. Rendering always requires an explicit locale and goes through
/// `tron_i18n`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageText {
    key: Cow<'static, str>,
    args: Vec<(String, String)>,
}

impl MessageText {
    pub fn new(key: &'static str) -> Self {
        Self {
            key: Cow::Borrowed(key),
            args: Vec::new(),
        }
    }

    /// Add a `%{name}` placeholder value.
    pub fn arg(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.args.push((name.into(), value.into()));
        self
    }

    /// Stable translation key for logs and persistence.
    pub fn key(&self) -> &str {
        self.key.as_ref()
    }

    /// Render for the display locale. This is the only place text is produced.
    pub fn render(&self, locale: impl AsRef<str>) -> String {
        let names = self
            .args
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>();
        let values = self
            .args
            .iter()
            .map(|(_, value)| value.clone())
            .collect::<Vec<_>>();
        translate_with(locale, self.key(), &names, &values)
    }

    /// Encode as a bounded plain string for persisted diagnostics.
    pub fn encode(&self) -> String {
        let wire = WireMessage {
            key: self.key.to_string(),
            args: self.args.clone(),
        };
        let json = serde_json::to_string(&wire)
            .unwrap_or_else(|_| serde_json::json!({ "key": self.key }).to_string());
        format!("{ENCODED_PREFIX}{json}")
    }

    /// Decode a previously [`MessageText::encode`]d value.
    pub fn decode(value: &str) -> Option<Self> {
        let json = value.strip_prefix(ENCODED_PREFIX)?;
        let wire: WireMessage = serde_json::from_str(json).ok()?;
        if wire.key.is_empty() {
            return None;
        }
        Some(Self {
            key: Cow::Owned(wire.key),
            args: wire.args,
        })
    }
}

impl fmt::Display for MessageText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Diagnostics only; user-facing output must call `render(locale)`.
        formatter.write_str(self.key())
    }
}

/// A decoded persisted or legacy string payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoredMessage {
    /// Structured message written by migrated code.
    Message(MessageText),
    /// Legacy/technical plain text that was persisted before the migration.
    Plain(String),
}

impl StoredMessage {
    pub fn message(message: MessageText) -> Self {
        Self::Message(message)
    }

    pub fn plain(text: impl Into<String>) -> Self {
        Self::Plain(text.into())
    }

    /// Decode a stored string: structured messages carry [`ENCODED_PREFIX`].
    pub fn decode(value: impl Into<String>) -> Self {
        let value = value.into();
        match MessageText::decode(&value) {
            Some(message) => Self::Message(message),
            None => Self::Plain(value),
        }
    }

    /// Encode for persistence. Structured messages stay locale independent;
    /// plain legacy text is stored unchanged.
    pub fn encode(&self) -> String {
        match self {
            Self::Message(message) => message.encode(),
            Self::Plain(text) => text.clone(),
        }
    }

    /// Render for the display locale.
    ///
    /// Plain values are legacy/technical text; when `legacy_key` is provided
    /// they are wrapped into that message with a `%{detail}` argument so the
    /// display stays localized instead of forwarding a foreign-language string.
    pub fn render(&self, locale: impl AsRef<str>, legacy_key: Option<&'static str>) -> String {
        match self {
            Self::Message(message) => message.render(locale),
            Self::Plain(text) => match legacy_key {
                Some(key) => translate_with(
                    locale.as_ref(),
                    key,
                    &["detail"],
                    std::slice::from_ref(text),
                ),
                None => text.clone(),
            },
        }
    }

    /// Stable translation key when this is a structured message.
    pub fn message_key(&self) -> Option<&str> {
        match self {
            Self::Message(message) => Some(message.key()),
            Self::Plain(_) => None,
        }
    }

    /// Stable diagnostic rendering: the key for structured messages, the raw
    /// text for legacy/technical values.
    pub fn render_diagnostic(&self) -> String {
        match self {
            Self::Message(message) => message.key().to_owned(),
            Self::Plain(text) => text.clone(),
        }
    }

    pub fn is_plain(&self) -> bool {
        matches!(self, Self::Plain(_))
    }
}

impl fmt::Display for StoredMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.render_diagnostic())
    }
}

impl From<&str> for StoredMessage {
    /// Compatibility for legacy payloads that still construct a plain string.
    /// Migrated producers must use [`StoredMessage::message`].
    fn from(value: &str) -> Self {
        Self::Plain(value.to_owned())
    }
}

impl From<String> for StoredMessage {
    /// Compatibility for legacy payloads that still construct a plain string.
    /// Migrated producers must use [`StoredMessage::message`].
    fn from(value: String) -> Self {
        Self::Plain(value)
    }
}

/// Render a persisted string that may hold an encoded structured message.
///
/// `legacy_key` receives the `%{detail}` argument when the value predates the
/// structured format; pass `None` to display legacy values unchanged.
pub fn render_stored(value: &str, locale: &str, legacy_key: Option<&'static str>) -> String {
    StoredMessage::decode(value).render(locale, legacy_key)
}

/// Render an optional persisted string.
pub fn render_optional_stored(
    value: Option<&str>,
    locale: &str,
    legacy_key: Option<&'static str>,
) -> Option<String> {
    value.map(|value| render_stored(value, locale, legacy_key))
}
