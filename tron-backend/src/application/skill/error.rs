use crate::domain::skill::SkillError;
pub struct SkillErrorDescriptor {
    pub code: &'static str,
    pub message_key: &'static str,
}
impl SkillError {
    pub fn descriptor(&self) -> SkillErrorDescriptor {
        let (code, message_key) = match self {
            Self::SearchQueryRequired => ("101030001", "tron.skill.searchqueryrequired"),
            Self::SourceRefRequired => ("101030002", "tron.skill.sourcerefrequired"),
            Self::SkillNameRequired => ("101030003", "tron.skill.skillnamerequired"),
            Self::InvalidAgent => ("101030004", "tron.skill.invalidagent"),
            Self::Unavailable => ("101030005", "tron.skill.unavailable"),
            Self::NotFound => ("101030006", "tron.skill.notfound"),
            Self::DuplicateSkill => ("101030007", "tron.skill.duplicateskill"),
            Self::InvalidRegistryPath => ("101030008", "tron.skill.invalidregistrypath"),
            Self::StorageFailure => ("101030009", "tron.skill.storagefailure"),
            Self::CommandFailed => ("101030010", "tron.skill.commandfailed"),
        };
        SkillErrorDescriptor { code, message_key }
    }
}
