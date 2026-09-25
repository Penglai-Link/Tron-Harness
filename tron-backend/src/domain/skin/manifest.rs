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
    pub css: Option<String>,
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
            && matches!(self.version, 1 | 2)
            && (self.resources.css.is_none() || self.version == 2)
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
            && self
                .resources
                .css
                .as_ref()
                .is_none_or(|path| valid_resource_path(path))
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

pub fn validate_theme_css(source: &str, package_root: &std::path::Path) -> Result<(), String> {
    if source.len() > 1024 * 1024 {
        return Err("CSS exceeds 1 MiB".into());
    }
    if source.contains('\\') {
        return Err("CSS escapes are not supported".into());
    }
    let bytes = source.as_bytes();
    let mut quote = 0u8;
    let mut comment = false;
    let mut escaped = false;
    let mut parens = 0i32;
    let mut brackets = 0i32;
    let mut stack: Vec<bool> = Vec::new(); // true means a declaration block
    let mut overlay_stack: Vec<bool> = Vec::new();
    let mut segment = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if comment {
            if b == b'*' && bytes.get(i + 1) == Some(&b'/') {
                comment = false;
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        if quote != 0 {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == quote {
                quote = 0;
            }
            i += 1;
            continue;
        }
        if b == b'/' && bytes.get(i + 1) == Some(&b'*') {
            comment = true;
            i += 2;
            continue;
        }
        if b == b'\'' || b == b'"' {
            quote = b;
            i += 1;
            continue;
        }
        match b {
            b'(' => parens += 1,
            b')' => {
                parens -= 1;
                if parens < 0 {
                    return Err("unmatched ')'".into());
                }
            }
            b'[' => brackets += 1,
            b']' => {
                brackets -= 1;
                if brackets < 0 {
                    return Err("unmatched ']'".into());
                }
            }
            b'{' if parens == 0 && brackets == 0 => {
                let prelude = source[segment..i].trim();
                let frame = if prelude.to_ascii_lowercase().starts_with("@keyframes ")
                    || prelude
                        .to_ascii_lowercase()
                        .starts_with("@-webkit-keyframes ")
                {
                    if !stack.is_empty()
                        || !prelude.split_whitespace().nth(1).is_some_and(|n| {
                            n.bytes()
                                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                        })
                    {
                        return Err("only top-level named @keyframes is allowed".into());
                    }
                    false
                } else if stack.last() == Some(&false) && stack.len() == 1 {
                    if !matches!(prelude, "from" | "to")
                        && !prelude
                            .trim_end_matches('%')
                            .parse::<f32>()
                            .is_ok_and(|n| (0.0..=100.0).contains(&n))
                    {
                        return Err("invalid keyframe selector".into());
                    }
                    true
                } else if stack.is_empty() {
                    let lower = prelude.to_ascii_lowercase();
                    if lower.is_empty() || lower.contains('@') {
                        return Err("invalid selector prelude or nested at-rule".into());
                    }
                    true
                } else {
                    return Err("nested CSS rules are not allowed".into());
                };
                let is_overlay = is_theme_effect_selector(prelude)
                    || overlay_stack.last().copied().unwrap_or(false);
                stack.push(frame);
                segment = i + 1;
                overlay_stack.push(is_overlay);
            }
            b';' if parens == 0 && brackets == 0 => {
                if stack.last() == Some(&true) {
                    validate_css_declaration(
                        source[segment..i].trim(),
                        package_root,
                        overlay_stack.last().copied().unwrap_or(false),
                    )?;
                } else if !source[segment..i].trim().is_empty() {
                    return Err("unexpected statement or @import".into());
                }
                segment = i + 1;
            }
            b'}' if parens == 0 && brackets == 0 => {
                if let Some(true) = stack.last() {
                    validate_css_declaration(
                        source[segment..i].trim(),
                        package_root,
                        overlay_stack.last().copied().unwrap_or(false),
                    )?;
                }
                if stack.pop().is_none() {
                    return Err("unmatched '}'".into());
                }
                overlay_stack.pop();
                segment = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    if quote != 0
        || comment
        || parens != 0
        || brackets != 0
        || !stack.is_empty()
        || !source[segment..].trim().is_empty()
    {
        return Err("incomplete CSS rule or unbalanced delimiter".into());
    }
    Ok(())
}

fn is_theme_effect_selector(selector: &str) -> bool {
    let selector = selector.trim();
    if !selector.starts_with("[data-theme-effect-layer") || selector.contains(',') {
        return false;
    }
    let Some(end) = selector.find(']') else {
        return false;
    };
    matches!(&selector[end + 1..], "" | "::before" | "::after")
}

fn validate_css_declaration(
    declaration: &str,
    package_root: &std::path::Path,
    overlay: bool,
) -> Result<(), String> {
    if declaration.is_empty() {
        return Ok(());
    }
    let (property, value) = declaration
        .split_once(':')
        .ok_or_else(|| "expected a CSS property and value".to_string())?;
    let property = property.trim().to_ascii_lowercase();
    const VISUAL: &[&str] = &[
        "color",
        "background",
        "background-color",
        "background-image",
        "background-position",
        "background-size",
        "background-repeat",
        "background-blend-mode",
        "border",
        "border-color",
        "border-style",
        "border-width",
        "border-radius",
        "outline",
        "outline-color",
        "outline-style",
        "outline-width",
        "box-shadow",
        "text-shadow",
        "opacity",
        "filter",
        "backdrop-filter",
        "mix-blend-mode",
        "animation",
        "animation-name",
        "animation-duration",
        "animation-delay",
        "animation-direction",
        "animation-fill-mode",
        "animation-iteration-count",
        "animation-play-state",
        "animation-timing-function",
        "transition",
        "transition-property",
        "transition-duration",
        "transition-delay",
        "transition-timing-function",
        "transform",
        "content",
        "font-family",
        "font-style",
        "font-weight",
        "text-decoration",
        "text-decoration-color",
        "text-decoration-style",
        "text-decoration-line",
        "-webkit-text-stroke",
        "-webkit-text-fill-color",
    ];
    let overlay_geometry = overlay
        && matches!(
            property.as_str(),
            "position"
                | "inset"
                | "inset-block"
                | "inset-inline"
                | "top"
                | "right"
                | "bottom"
                | "left"
                | "width"
                | "height"
                | "min-width"
                | "min-height"
                | "max-width"
                | "max-height"
        );
    if !property.starts_with("--") && !VISUAL.contains(&property.as_str()) && !overlay_geometry {
        return Err(format!(
            "property '{property}' can change component geometry"
        ));
    }
    let lower = value.to_ascii_lowercase();
    if lower.contains("!important")
        || lower.contains("expression(")
        || lower.contains("javascript:")
        || lower.contains("behavior:")
    {
        return Err(format!("unsafe value for '{property}'"));
    }
    const FUNCTIONS: &[&str] = &[
        "url",
        "var",
        "rgb",
        "rgba",
        "hsl",
        "hsla",
        "hwb",
        "lab",
        "lch",
        "oklab",
        "oklch",
        "color",
        "color-mix",
        "linear-gradient",
        "repeating-linear-gradient",
        "radial-gradient",
        "repeating-radial-gradient",
        "conic-gradient",
        "repeating-conic-gradient",
        "blur",
        "brightness",
        "contrast",
        "drop-shadow",
        "grayscale",
        "hue-rotate",
        "invert",
        "opacity",
        "saturate",
        "sepia",
        "translate",
        "translatex",
        "translatey",
        "translatez",
        "translate3d",
        "scale",
        "scalex",
        "scaley",
        "scale3d",
        "rotate",
        "rotatex",
        "rotatey",
        "rotatez",
        "rotate3d",
        "skew",
        "skewx",
        "skewy",
        "matrix",
        "matrix3d",
        "cubic-bezier",
        "steps",
        "calc",
        "min",
        "max",
        "clamp",
    ];
    let mut cursor = 0;
    while let Some(open) = lower[cursor..].find('(') {
        let open = cursor + open;
        let name_start = lower[..open]
            .trim_end()
            .rfind(|character: char| !character.is_ascii_alphanumeric() && character != '-')
            .map_or(0, |position| position + 1);
        let function = lower[name_start..open].trim();
        if !function.is_empty() && !FUNCTIONS.contains(&function) {
            return Err(format!("unsupported CSS function '{function}'"));
        }
        cursor = open + 1;
    }
    let mut rest = lower.as_str();
    while let Some(index) = rest.find("url(") {
        let after = &value[(value.len() - rest.len()) + index + 4..];
        let end = after
            .find(')')
            .ok_or_else(|| "unterminated url()".to_string())?;
        let path = after[..end].trim().trim_matches(['\'', '"']);
        if !valid_resource_path(path) || !package_root.join(path).is_file() {
            return Err(format!(
                "url() must reference a file in the theme package: {path}"
            ));
        }
        let consumed = (value.len() - rest.len()) + index + 4 + end + 1;
        rest = &lower[consumed..];
    }
    Ok(())
}

pub fn resource_mime(path: &str) -> Option<&'static str> {
    match path.rsplit('.').next()? {
        "css" => Some("text/css; charset=utf-8"),
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
