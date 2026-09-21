use std::time::{SystemTime, UNIX_EPOCH};
use std::{collections::BTreeMap, fs};

use crate::{
    application::workspace::{WorkspaceRepository, WorkspaceService},
    domain::{
        skill::{
            RegistrySkill, SkillManagedBy, SkillRagIndexRecord, SkillSourceType,
            SkillUpdateStatus, WorkspaceSkillBinding,
        },
        workspace::{Workspace, WorkspaceError},
    },
};

use super::{
    DEFAULT_TIMESTAMP, SkillRegistryRepository, SkillRegistryService, build_package_ref,
    parse_repo_listing_output, parse_search_output, parse_skill_frontmatter, strip_ansi,
};

#[derive(Default)]
struct TestSkillRegistryRepository {
    registry_skills: BTreeMap<String, RegistrySkill>,
    workspace_bindings: Vec<WorkspaceSkillBinding>,
    rag_index_states: BTreeMap<(String, String), SkillRagIndexRecord>,
}

impl SkillRegistryRepository for TestSkillRegistryRepository {
    fn list_registry_skills(&mut self) -> Result<Vec<RegistrySkill>, crate::domain::skill::SkillError> {
        Ok(self.registry_skills.values().cloned().collect())
    }

    fn upsert_registry_skill(&mut self, skill: &RegistrySkill) -> Result<(), crate::domain::skill::SkillError> {
        self.registry_skills.insert(skill.skill_name.clone(), skill.clone());
        Ok(())
    }

    fn delete_registry_skill(&mut self, skill_name: &str) -> Result<bool, crate::domain::skill::SkillError> {
        Ok(self.registry_skills.remove(skill_name).is_some())
    }

    fn list_workspace_bindings(&mut self, workspace_code: Option<&str>) -> Result<Vec<WorkspaceSkillBinding>, crate::domain::skill::SkillError> {
        Ok(self.workspace_bindings.iter().filter(|binding| workspace_code.is_none_or(|code| binding.workspace_code == code)).cloned().collect())
    }

    fn upsert_workspace_binding(&mut self, binding: &WorkspaceSkillBinding) -> Result<(), crate::domain::skill::SkillError> {
        if let Some(existing) = self.workspace_bindings.iter_mut().find(|existing| existing.workspace_code == binding.workspace_code && existing.skill_name == binding.skill_name) {
            *existing = binding.clone();
        } else {
            self.workspace_bindings.push(binding.clone());
        }
        Ok(())
    }

    fn delete_workspace_binding(&mut self, workspace_code: &str, skill_name: &str) -> Result<bool, crate::domain::skill::SkillError> {
        let original_len = self.workspace_bindings.len();
        self.workspace_bindings.retain(|binding| !(binding.workspace_code == workspace_code && binding.skill_name == skill_name));
        Ok(self.workspace_bindings.len() != original_len)
    }

    fn delete_bindings_for_skill(&mut self, skill_name: &str) -> Result<(), crate::domain::skill::SkillError> {
        self.workspace_bindings.retain(|binding| binding.skill_name != skill_name);
        Ok(())
    }

    fn get_rag_index_state(&mut self, workspace_code: &str, skill_name: &str) -> Result<Option<SkillRagIndexRecord>, crate::domain::skill::SkillError> {
        Ok(self.rag_index_states.get(&(workspace_code.to_owned(), skill_name.to_owned())).cloned())
    }

    fn list_rag_index_states(&mut self, workspace_code: Option<&str>) -> Result<Vec<SkillRagIndexRecord>, crate::domain::skill::SkillError> {
        Ok(self.rag_index_states.values().filter(|state| workspace_code.is_none_or(|code| state.workspace_code == code)).cloned().collect())
    }

    fn upsert_rag_index_state(&mut self, state: &SkillRagIndexRecord) -> Result<(), crate::domain::skill::SkillError> {
        self.rag_index_states.insert((state.workspace_code.clone(), state.skill_name.clone()), state.clone());
        Ok(())
    }
}

struct DummyWorkspaceRepository;

impl WorkspaceRepository for DummyWorkspaceRepository {
    fn list_workspaces(&mut self) -> Result<Vec<Workspace>, WorkspaceError> { unreachable!() }
    fn next_id(&mut self) -> Result<i64, WorkspaceError> { unreachable!() }
    fn create_workspace(&mut self, _: &Workspace) -> Result<(), WorkspaceError> { unreachable!() }
    fn update_workspace(&mut self, _: &Workspace) -> Result<(), WorkspaceError> { unreachable!() }
    fn archive_subtree(&mut self, _: &[String], _: &str) -> Result<usize, WorkspaceError> { unreachable!() }
    fn restore_subtree(&mut self, _: &[String], _: &str) -> Result<usize, WorkspaceError> { unreachable!() }
}

fn build_test_workspace_service() -> WorkspaceService {
    WorkspaceService::new(DummyWorkspaceRepository)
}

#[test]
fn parses_search_output_candidates() {
    let output = "\u{1b}[38;5;102mInstall with\u{1b}[0m npx skills add <owner/repo@skill>\n\
wshobson/agents@typescript-advanced-types 42K installs\n\
└ https://skills.sh/wshobson/agents/typescript-advanced-types\n\
github/awesome-copilot@javascript-typescript-jest 10.5K installs\n\
└ https://skills.sh/github/awesome-copilot/javascript-typescript-jest\n";
    let candidates = parse_search_output(output.as_bytes());
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].source_ref, "wshobson/agents");
    assert_eq!(candidates[0].skill_name, "typescript-advanced-types");
    assert_eq!(candidates[1].detail_url.as_deref(), Some("https://skills.sh/github/awesome-copilot/javascript-typescript-jest"));
}

#[test]
fn parses_repo_listing_output_candidates() {
    let output = "◇  Available Skills\n│\n│    eliteforge-agent-doctor\n│      检查开发环境配置\n│    eliteforge-be-fe-consensus-spec\n│      约束前后端接口共识\n└  Use --skill <name> to install specific skills\n";
    let candidates = parse_repo_listing_output(output.as_bytes(), "cloudsen/eliteforge-skills");
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].package_ref, "cloudsen/eliteforge-skills@eliteforge-agent-doctor");
    assert_eq!(candidates[1].skill_name, "eliteforge-be-fe-consensus-spec");
}

#[test]
fn strips_ansi_sequences() { assert_eq!(strip_ansi("\u{1b}[38;5;102mhello\u{1b}[0m"), "hello"); }

#[test]
fn parses_skill_frontmatter_fields() {
    let frontmatter = parse_skill_frontmatter("---\nversion: 1.2.3\ndescription: Example skill\n---\n# Skill");
    assert_eq!(frontmatter.declared_version.as_deref(), Some("1.2.3"));
    assert_eq!(frontmatter.description.as_deref(), Some("Example skill"));
}

#[test]
fn builds_package_ref_from_source_and_skill_name() {
    assert_eq!(build_package_ref(Some("wshobson/agents".into()), Some("typescript-advanced-types".into())).as_deref(), Some("wshobson/agents@typescript-advanced-types"));
}

#[test]
fn sync_registry_keeps_workspace_bindings_when_registry_skill_disappears() {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH).expect("system clock").as_nanos();
    let registry_root = std::env::temp_dir().join(format!("tron-skill-registry-sync-{unique}"));
    let skills_dir = registry_root.join(".agents/skills");
    fs::create_dir_all(&skills_dir).expect("skills directory should exist");
    let skill_name = "eliteforge-agent-doctor".to_owned();
    let repository = TestSkillRegistryRepository {
        registry_skills: BTreeMap::from([(skill_name.clone(), RegistrySkill {
            skill_name: skill_name.clone(), source_ref: "cloudsen/eliteforge-skills".into(),
            source_type: SkillSourceType::GitHub, registry_path: skills_dir.join(&skill_name).display().to_string(),
            declared_version: Some("1.0.0".into()), update_status: SkillUpdateStatus::UpToDate,
            managed_by: SkillManagedBy::Cli, last_checked_at: None, last_updated_at: None,
            last_error: None, created_at: DEFAULT_TIMESTAMP.into(), updated_at: DEFAULT_TIMESTAMP.into(),
        })]),
        workspace_bindings: vec![WorkspaceSkillBinding { workspace_code: "TRON_CORE".into(), skill_name: skill_name.clone(), enabled: true, created_at: DEFAULT_TIMESTAMP.into(), updated_at: DEFAULT_TIMESTAMP.into() }],
        rag_index_states: BTreeMap::new(),
    };
    let service = SkillRegistryService::new(repository, build_test_workspace_service(), &registry_root);
    service.sync_registry(false, false).expect("registry sync should succeed");
    let remaining_registry_skills = service.repository.lock().expect("repository mutex").list_registry_skills().expect("skills");
    assert!(remaining_registry_skills.is_empty());
    let remaining_bindings = service.repository.lock().expect("repository mutex").list_workspace_bindings(None).expect("bindings");
    assert_eq!(remaining_bindings.len(), 1);
    assert_eq!(remaining_bindings[0].skill_name, skill_name);
    fs::remove_dir_all(&registry_root).expect("temp skill registry should be removed");
}
