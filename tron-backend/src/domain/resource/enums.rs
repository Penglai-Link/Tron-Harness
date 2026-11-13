use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ResourceSourceKind {
    Local,
    Inherited,
    Override,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ResourceKind {
    File,
    Image,
    Pdf,
}

impl ResourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::File => "FILE",
            Self::Image => "IMAGE",
            Self::Pdf => "PDF",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "FILE" => Some(Self::File),
            "IMAGE" => Some(Self::Image),
            "PDF" => Some(Self::Pdf),
            _ => None,
        }
    }

    pub fn is_file_kind(&self) -> bool {
        matches!(self, Self::File | Self::Image | Self::Pdf)
    }
}
