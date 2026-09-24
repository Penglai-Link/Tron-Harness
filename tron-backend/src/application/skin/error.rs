#[derive(Debug)]
pub enum SkinError {
    Invalid,
    NotFound,
    Conflict,
    Storage,
    TooLarge,
}

pub struct ApplicationErrorDescriptor {
    pub code: &'static str,
    pub message_key: &'static str,
}
impl SkinError {
    pub fn descriptor(&self) -> ApplicationErrorDescriptor {
        let (code, message_key) = match self {
            Self::Invalid => ("101025001", "tron.skin.invalid"),
            Self::NotFound => ("101025002", "tron.skin.not_found"),
            Self::Conflict => ("101025003", "tron.skin.conflict"),
            Self::Storage => ("101025004", "tron.skin.storage_failed"),
            Self::TooLarge => ("101025005", "tron.skin.too_large"),
        };
        ApplicationErrorDescriptor { code, message_key }
    }
}
