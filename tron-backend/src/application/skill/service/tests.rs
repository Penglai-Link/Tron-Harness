use std::time::{SystemTime, UNIX_EPOCH};
use std::{collections::BTreeMap, fs};

use crate::{
    application::{
        role::{RoleRepository, RoleService},
        user::{PasswordHasher, UserRepository, UserService},
        workspace::{WorkspaceRepository, WorkspaceService},
    },
    config::AppConfig,
    domain::{
        auth::{AuthError, OAuth2IdentityLink, RefreshSession},
        role::{Role, RoleError},
        skill::{
            RegistrySkill, SkillManagedBy, SkillSourceType, SkillUpdateStatus,
            WorkspaceSkillBinding,
        },
        user::{ListUsersQuery, PagedUsers, User, UserError, UserStatistics},
        workspace::{Workspace, WorkspaceError, WorkspaceRoleAccess, WorkspaceUserAccess},
    },
};

use super::{
    DEFAULT_TIMESTAMP, SkillRegistryRepository, SkillRegistryService, build_package_ref,
    parse_repo_listing_output, parse_search_output, parse_skill_frontmatter, read_skills_lock,
    strip_ansi,
};

#[derive(Default)]
struct TestSkillRegistryRepository {
    registry_skills: BTreeMap<String, RegistrySkill>,
    workspace_bindings: Vec<WorkspaceSkillBinding>,
}

impl SkillRegistryRepository for TestSkillRegistryRepository {
    fn list_registry_skills(
        &mut self,
    ) -> Result<Vec<RegistrySkill>, crate::domain::skill::SkillError> {
        Ok(self.registry_skills.values().cloned().collect())
    }

    fn upsert_registry_skill(
        &mut self,
        skill: &RegistrySkill,
    ) -> Result<(), crate::domain::skill::SkillError> {
        self.registry_skills
            .insert(skill.skill_name.clone(), skill.clone());
        Ok(())
    }

    fn delete_registry_skill(
        &mut self,
        skill_name: &str,
    ) -> Result<bool, crate::domain::skill::SkillError> {
        Ok(self.registry_skills.remove(skill_name).is_some())
    }

    fn list_workspace_bindings(
        &mut self,
        workspace_code: Option<&str>,
    ) -> Result<Vec<WorkspaceSkillBinding>, crate::domain::skill::SkillError> {
        Ok(self
            .workspace_bindings
            .iter()
            .filter(|binding| workspace_code.is_none_or(|code| binding.workspace_code == code))
            .cloned()
            .collect())
    }

    fn upsert_workspace_binding(
        &mut self,
        binding: &WorkspaceSkillBinding,
    ) -> Result<(), crate::domain::skill::SkillError> {
        if let Some(existing) = self.workspace_bindings.iter_mut().find(|existing| {
            existing.workspace_code == binding.workspace_code
                && existing.skill_name == binding.skill_name
        }) {
            *existing = binding.clone();
        } else {
            self.workspace_bindings.push(binding.clone());
        }
        Ok(())
    }

    fn delete_workspace_binding(
        &mut self,
        workspace_code: &str,
        skill_name: &str,
    ) -> Result<bool, crate::domain::skill::SkillError> {
        let original_len = self.workspace_bindings.len();
        self.workspace_bindings.retain(|binding| {
            !(binding.workspace_code == workspace_code && binding.skill_name == skill_name)
        });
        Ok(self.workspace_bindings.len() != original_len)
    }

    fn delete_bindings_for_skill(
        &mut self,
        skill_name: &str,
    ) -> Result<(), crate::domain::skill::SkillError> {
        self.workspace_bindings
            .retain(|binding| binding.skill_name != skill_name);
        Ok(())
    }
}

struct DummyWorkspaceRepository;

impl WorkspaceRepository for DummyWorkspaceRepository {
    fn list_workspaces(&mut self) -> Result<Vec<Workspace>, WorkspaceError> {
        unreachable!("workspace repository should not be used in sync_registry tests")
    }

    fn next_id(&mut self) -> Result<i64, WorkspaceError> {
        unreachable!("workspace repository should not be used in sync_registry tests")
    }

    fn create_workspace(&mut self, _workspace: &Workspace) -> Result<(), WorkspaceError> {
        unreachable!("workspace repository should not be used in sync_registry tests")
    }

    fn update_workspace(&mut self, _workspace: &Workspace) -> Result<(), WorkspaceError> {
        unreachable!("workspace repository should not be used in sync_registry tests")
    }

    fn list_user_access(
        &mut self,
        _workspace_code: &str,
    ) -> Result<Vec<WorkspaceUserAccess>, WorkspaceError> {
        unreachable!("workspace repository should not be used in sync_registry tests")
    }

    fn replace_user_access(
        &mut self,
        _workspace_code: &str,
        _entries: &[WorkspaceUserAccess],
    ) -> Result<Vec<WorkspaceUserAccess>, WorkspaceError> {
        unreachable!("workspace repository should not be used in sync_registry tests")
    }

    fn list_role_access(
        &mut self,
        _workspace_code: &str,
    ) -> Result<Vec<WorkspaceRoleAccess>, WorkspaceError> {
        unreachable!("workspace repository should not be used in sync_registry tests")
    }

    fn replace_role_access(
        &mut self,
        _workspace_code: &str,
        _entries: &[WorkspaceRoleAccess],
    ) -> Result<Vec<WorkspaceRoleAccess>, WorkspaceError> {
        unreachable!("workspace repository should not be used in sync_registry tests")
    }

    fn archive_subtree(
        &mut self,
        _workspace_codes: &[String],
        _archived_at: &str,
    ) -> Result<usize, WorkspaceError> {
        unreachable!("workspace repository should not be used in sync_registry tests")
    }

    fn restore_subtree(
        &mut self,
        _workspace_codes: &[String],
        _restored_at: &str,
    ) -> Result<usize, WorkspaceError> {
        unreachable!("workspace repository should not be used in sync_registry tests")
    }
}

struct DummyUserRepository;

impl UserRepository for DummyUserRepository {
    fn list_users(&mut self, _query: ListUsersQuery) -> Result<PagedUsers, UserError> {
        unreachable!("user repository should not be used in sync_registry tests")
    }

    fn get_user(&mut self, _user_code: &str) -> Result<User, UserError> {
        unreachable!("user repository should not be used in sync_registry tests")
    }

    fn statistics(&mut self) -> Result<UserStatistics, UserError> {
        unreachable!("user repository should not be used in sync_registry tests")
    }

    fn next_id(&mut self) -> Result<i64, UserError> {
        unreachable!("user repository should not be used in sync_registry tests")
    }

    fn next_user_code(&mut self, _username: &str) -> Result<String, UserError> {
        unreachable!("user repository should not be used in sync_registry tests")
    }

    fn user_code_exists(
        &mut self,
        _user_code: &str,
        _except_user_code: Option<&str>,
    ) -> Result<bool, UserError> {
        unreachable!("user repository should not be used in sync_registry tests")
    }

    fn username_exists(
        &mut self,
        _username: &str,
        _except_user_code: Option<&str>,
    ) -> Result<bool, UserError> {
        unreachable!("user repository should not be used in sync_registry tests")
    }

    fn email_exists(
        &mut self,
        _email: &str,
        _except_user_code: Option<&str>,
    ) -> Result<bool, UserError> {
        unreachable!("user repository should not be used in sync_registry tests")
    }

    fn insert_user(&mut self, _user: &User) -> Result<(), UserError> {
        unreachable!("user repository should not be used in sync_registry tests")
    }

    fn update_user(&mut self, _user: &User) -> Result<(), UserError> {
        unreachable!("user repository should not be used in sync_registry tests")
    }

    fn replace_role_codes(&mut self, _user: &User) -> Result<(), UserError> {
        unreachable!("user repository should not be used in sync_registry tests")
    }

    fn replace_space_codes(&mut self, _user: &User) -> Result<(), UserError> {
        unreachable!("user repository should not be used in sync_registry tests")
    }

    fn record_password_reset(&mut self, _user: &User, _reset_id: i64) -> Result<(), UserError> {
        unreachable!("user repository should not be used in sync_registry tests")
    }

    fn find_user_by_login(&mut self, _login: &str) -> Result<Option<User>, AuthError> {
        unreachable!("user repository should not be used in sync_registry tests")
    }

    fn find_active_user_by_email(&mut self, _email: &str) -> Result<Option<User>, AuthError> {
        unreachable!("user repository should not be used in sync_registry tests")
    }

    fn find_user_by_provider_subject(
        &mut self,
        _provider_code: &str,
        _provider_subject: &str,
    ) -> Result<Option<User>, AuthError> {
        unreachable!("user repository should not be used in sync_registry tests")
    }

    fn create_refresh_session(&mut self, _session: &RefreshSession) -> Result<(), AuthError> {
        unreachable!("user repository should not be used in sync_registry tests")
    }

    fn find_refresh_session(
        &mut self,
        _session_code: &str,
    ) -> Result<Option<RefreshSession>, AuthError> {
        unreachable!("user repository should not be used in sync_registry tests")
    }

    fn replace_refresh_session(&mut self, _session: &RefreshSession) -> Result<(), AuthError> {
        unreachable!("user repository should not be used in sync_registry tests")
    }

    fn revoke_refresh_session(
        &mut self,
        _session_code: &str,
        _revoked_at: i64,
    ) -> Result<(), AuthError> {
        unreachable!("user repository should not be used in sync_registry tests")
    }

    fn upsert_oauth_identity(&mut self, _identity: &OAuth2IdentityLink) -> Result<(), AuthError> {
        unreachable!("user repository should not be used in sync_registry tests")
    }
}

struct DummyPasswordHasher;

impl PasswordHasher for DummyPasswordHasher {
    fn hash_password(&self, _value: &str) -> Result<String, UserError> {
        unreachable!("password hasher should not be used in sync_registry tests")
    }

    fn verify_password(&self, _value: &str, _hash: &str) -> Result<bool, AuthError> {
        unreachable!("password hasher should not be used in sync_registry tests")
    }
}

struct DummyRoleRepository;

impl RoleRepository for DummyRoleRepository {
    fn list_roles(&mut self) -> Result<Vec<Role>, RoleError> {
        unreachable!("role repository should not be used in sync_registry tests")
    }

    fn next_id(&mut self) -> Result<i64, RoleError> {
        unreachable!("role repository should not be used in sync_registry tests")
    }

    fn replace_roles(&mut self, _roles: &[Role]) -> Result<Vec<Role>, RoleError> {
        unreachable!("role repository should not be used in sync_registry tests")
    }
}

fn build_test_workspace_service() -> WorkspaceService {
    WorkspaceService::new(
        DummyWorkspaceRepository,
        UserService::new(
            DummyUserRepository,
            DummyPasswordHasher,
            AppConfig::default().auth,
        ),
        RoleService::new(DummyRoleRepository),
    )
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
    assert_eq!(candidates[0].description, None);
    assert_eq!(
        candidates[0].installs_label.as_deref(),
        Some("42K installs")
    );
    assert_eq!(
        candidates[1].detail_url.as_deref(),
        Some("https://skills.sh/github/awesome-copilot/javascript-typescript-jest")
    );
}

#[test]
fn parses_repo_listing_output_candidates() {
    let output = "◇  Available Skills\n\
│\n\
│    eliteforge-agent-doctor\n\
│\n\
│      检查开发环境是否配置完成\n\
│      缺少命令时自动补齐\n\
│\n\
│    eliteforge-be-fe-consensus-spec\n\
│\n\
│      约束前后端接口共识\n\
│\n\
└  Use --skill <name> to install specific skills\n";
    let candidates = parse_repo_listing_output(output.as_bytes(), "cloudsen/eliteforge-skills");
    assert_eq!(candidates.len(), 2);
    assert_eq!(
        candidates[0].package_ref,
        "cloudsen/eliteforge-skills@eliteforge-agent-doctor"
    );
    assert_eq!(
        candidates[0].description.as_deref(),
        Some("检查开发环境是否配置完成 缺少命令时自动补齐")
    );
    assert_eq!(candidates[1].skill_name, "eliteforge-be-fe-consensus-spec");
    assert_eq!(candidates[1].installs_label, None);
    assert_eq!(candidates[1].detail_url, None);
}

#[test]
fn strips_ansi_sequences() {
    let output = strip_ansi("\u{1b}[38;5;102mhello\u{1b}[0m");
    assert_eq!(output, "hello");
}

#[test]
fn parses_skill_frontmatter_fields() {
    let markdown = "---\nversion: 1.2.3\ndescription: Example skill\n---\n# Skill";
    let frontmatter = parse_skill_frontmatter(markdown);
    assert_eq!(frontmatter.declared_version.as_deref(), Some("1.2.3"));
    assert_eq!(frontmatter.description.as_deref(), Some("Example skill"));
}

#[test]
fn parses_skill_frontmatter_metadata_fields() {
    let markdown = "---\nmetadata:\n  version: 2.3.4\n  description: Metadata skill\n---\n# Skill";
    let frontmatter = parse_skill_frontmatter(markdown);
    assert_eq!(frontmatter.declared_version.as_deref(), Some("2.3.4"));
    assert_eq!(frontmatter.description.as_deref(), Some("Metadata skill"));
}

#[test]
fn builds_package_ref_from_source_and_skill_name() {
    let package_ref = build_package_ref(
        Some("wshobson/agents".into()),
        Some("typescript-advanced-types".into()),
    );
    assert_eq!(
        package_ref.as_deref(),
        Some("wshobson/agents@typescript-advanced-types")
    );
}

#[test]
fn reads_skills_lock_file() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("tron-skill-lock-{unique}"));
    fs::create_dir_all(&temp_dir).expect("temp skill dir should exist");
    let lock_path = temp_dir.join("skills-lock.json");
    fs::write(
        &lock_path,
        r#"{
  "version": 1,
  "skills": {
    "typescript-advanced-types": {
      "source": "wshobson/agents",
      "sourceType": "github"
    }
  }
}"#,
    )
    .expect("skills lock should be written");

    let skills = read_skills_lock(&lock_path).expect("lock file should parse");
    let entry = skills
        .get("typescript-advanced-types")
        .expect("skill should exist");
    assert_eq!(entry.source, "wshobson/agents");
    assert_eq!(entry.source_type.as_deref(), Some("github"));

    fs::remove_dir_all(&temp_dir).expect("temp skill dir should be removed");
}

#[test]
fn enums_are_copyable_for_registry_models() {
    let source_type = SkillSourceType::GitHub;
    let update_status = SkillUpdateStatus::Unknown;
    let managed_by = SkillManagedBy::Cli;
    assert!(matches!(source_type, SkillSourceType::GitHub));
    assert!(matches!(update_status, SkillUpdateStatus::Unknown));
    assert!(matches!(managed_by, SkillManagedBy::Cli));
}

#[test]
fn sync_registry_keeps_workspace_bindings_when_registry_skill_disappears() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    let registry_root = std::env::temp_dir().join(format!("tron-skill-registry-sync-{unique}"));
    let skills_dir = registry_root.join(".agents/skills");
    fs::create_dir_all(&skills_dir).expect("skills directory should exist");

    let skill_name = "eliteforge-agent-doctor".to_string();
    let repository = TestSkillRegistryRepository {
        registry_skills: BTreeMap::from([(
            skill_name.clone(),
            RegistrySkill {
                skill_name: skill_name.clone(),
                source_ref: "cloudsen/eliteforge-skills".into(),
                source_type: SkillSourceType::GitHub,
                registry_path: skills_dir.join(&skill_name).display().to_string(),
                declared_version: Some("1.0.0".into()),
                update_status: SkillUpdateStatus::UpToDate,
                managed_by: SkillManagedBy::Cli,
                last_checked_at: None,
                last_updated_at: None,
                last_error: None,
                created_at: DEFAULT_TIMESTAMP.into(),
                updated_at: DEFAULT_TIMESTAMP.into(),
            },
        )]),
        workspace_bindings: vec![WorkspaceSkillBinding {
            workspace_code: "TRON_CORE".into(),
            skill_name: skill_name.clone(),
            enabled: true,
            created_at: DEFAULT_TIMESTAMP.into(),
            updated_at: DEFAULT_TIMESTAMP.into(),
        }],
    };

    let service =
        SkillRegistryService::new(repository, build_test_workspace_service(), &registry_root);

    service
        .sync_registry(false, false)
        .expect("registry sync should succeed");

    let remaining_registry_skills = service
        .repository
        .lock()
        .expect("skill registry repository mutex poisoned")
        .list_registry_skills()
        .expect("registry skills should be listed");
    assert!(remaining_registry_skills.is_empty());

    let remaining_bindings = service
        .repository
        .lock()
        .expect("skill registry repository mutex poisoned")
        .list_workspace_bindings(None)
        .expect("workspace bindings should be listed");
    assert_eq!(remaining_bindings.len(), 1);
    assert_eq!(remaining_bindings[0].skill_name, skill_name);

    fs::remove_dir_all(&registry_root).expect("temp skill registry should be removed");
}
