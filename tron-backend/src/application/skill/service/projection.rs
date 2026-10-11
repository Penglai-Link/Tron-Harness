use super::*;

pub(super) fn matches_skill_search(skill: &RegistrySkill, search: Option<&str>) -> bool {
    let Some(search) = search else {
        return true;
    };
    let haystack = [
        skill.skill_name.as_str(),
        skill.source_ref.as_str(),
        skill.registry_path.as_str(),
        skill.declared_version.as_deref().unwrap_or(""),
    ]
    .join(" ")
    .to_lowercase();
    haystack.contains(search)
}

pub(super) fn parse_source_type(value: &str) -> Option<SkillSourceType> {
    match value {
        "marketplace" => Some(SkillSourceType::Marketplace),
        "github" => Some(SkillSourceType::GitHub),
        "git" => Some(SkillSourceType::Git),
        "url" => Some(SkillSourceType::Url),
        "local" => Some(SkillSourceType::Local),
        "manual" => Some(SkillSourceType::Manual),
        _ => None,
    }
}
