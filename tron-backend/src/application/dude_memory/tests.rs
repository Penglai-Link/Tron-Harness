use super::*;
use crate::{
    adapters::outbound::persistence::sqlite::{
        SqliteDudeMemoryRepository, SqliteSkillRegistryRepository,
    },
    application::skill::SkillRegistryService,
    domain::dude_memory::*,
};
use std::{fs, path::Path};

fn fixture() -> (tempfile::TempDir, DudeMemoryService) {
    let root = tempfile::tempdir().unwrap();
    let db = root.path().join("state.sqlite");
    let skills = SkillRegistryService::new(
        SqliteSkillRegistryRepository::connect(db.to_str().unwrap()).unwrap(),
        root.path().join("registry"),
    );
    let service = DudeMemoryService::new(
        SqliteDudeMemoryRepository::connect(db.to_str().unwrap()).unwrap(),
        skills,
        root.path().join("staging"),
    );
    (root, service)
}
fn source(workspace: &str, code: &str, version: &str) -> MemorySource {
    MemorySource {
        source_code: code.into(),
        workspace_code: workspace.into(),
        version: version.into(),
        excerpt: "User confirmed whitespace normalization; regression fixture passes".into(),
        root_owner_dude_code: None,
        lineage_dude_codes: vec![],
        kind: MemorySourceKind::Conversation,
    }
}
fn install_creator(root: &Path, service: &DudeMemoryService) {
    let path = root.join("registry/.agents/skills/skill-creator");
    fs::create_dir_all(&path).unwrap();
    fs::write(path.join("SKILL.md"),"---\nname: skill-creator\ndescription: Create focused reusable instructions from validated experience.\nmetadata:\n  version: '1.2.3'\n---\nAuthor SKILL.md with a clear trigger, preserve intent, and verify real behavior.\n").unwrap();
    service.skills.rescan_registry().unwrap();
}
fn artifact(name: &str, pass: bool) -> Vec<SkillArtifactFile> {
    vec![
        SkillArtifactFile {
            path: "SKILL.md".into(),
            content: format!(
                "---\nname: {name}\ndescription: Normalize repeated whitespace in imported labels.\n---\nUse scripts/normalize.py and preserve non-whitespace text.\n"
            ),
            encoding: ArtifactEncoding::Utf8,
        },
        SkillArtifactFile {
            path: "scripts/normalize.py".into(),
            content: "def normalize(text):\n    return ' '.join(text.split())\n".into(),
            encoding: ArtifactEncoding::Utf8,
        },
        SkillArtifactFile {
            path: "scripts/check.py".into(),
            content: format!(
                "from normalize import normalize\nassert normalize(' a  b ') == '{}'\nprint('normalization behavior verified')\n",
                if pass { "a b" } else { "wrong" }
            ),
            encoding: ArtifactEncoding::Utf8,
        },
    ]
}
fn behavior() -> Vec<BehaviorCheck> {
    vec![BehaviorCheck {
        name: "normalizes-real-input".into(),
        program: "python3".into(),
        args: vec!["scripts/check.py".into()],
    }]
}
fn prepared(service: &DudeMemoryService) -> MemoryBatch {
    service
        .prepare_batch("memory-dude", "user", vec![source("w-a", "chat", "v1")])
        .unwrap()
        .unwrap()
}
fn candidate(
    service: &DudeMemoryService,
    batch: &MemoryBatch,
    pass: bool,
    auto: bool,
) -> SkillCandidate {
    let context = service
        .begin_skill_authoring(
            "user",
            &batch.batch_code,
            "normalize-labels",
            "author-session",
        )
        .unwrap();
    assert!(
        context
            .creator_markdown
            .unwrap()
            .contains("preserve intent")
    );
    service
        .submit_candidate(
            "user",
            &context.candidate.candidate_code,
            "author-session",
            artifact("normalize-labels", pass),
            behavior(),
            auto,
        )
        .unwrap()
}
#[test]
fn dude_memory_batches_snapshot_across_workspaces_resume_and_commit_cursors() {
    let (root, service) = fixture();
    let mut excluded = source("w-a", "memory-child", "1");
    excluded.lineage_dude_codes.push("memory-dude".into());
    let mut log = source("w-a", "log", "1");
    log.kind = MemorySourceKind::RuntimeLog;
    let mut self_owned = source("w-b", "self", "1");
    self_owned.root_owner_dude_code = Some("memory-dude".into());
    let sources = vec![
        source("w-a", "same-id", "1"),
        source("w-b", "same-id", "1"),
        excluded,
        log,
        self_owned,
    ];
    let batch = service
        .prepare_batch("memory-dude", "user", sources.clone())
        .unwrap()
        .unwrap();
    assert_eq!(batch.sources.len(), 2);
    service
        .attach_batch_session("user", &batch.batch_code, "native-session")
        .unwrap();
    let reopened = DudeMemoryService::new(
        SqliteDudeMemoryRepository::connect(root.path().join("state.sqlite").to_str().unwrap())
            .unwrap(),
        service.skills.clone(),
        root.path().join("staging"),
    );
    let resumed = reopened
        .prepare_batch(
            "memory-dude",
            "user",
            vec![source("new-workspace", "new-chat", "1")],
        )
        .unwrap()
        .unwrap();
    assert_eq!(resumed.batch_code, batch.batch_code);
    assert_eq!(resumed.session_code.as_deref(), Some("native-session"));
    reopened
        .complete_batch(
            "user",
            &batch.batch_code,
            "native-session",
            vec![],
            "No stable fact changes".into(),
        )
        .unwrap();
    assert!(
        reopened
            .prepare_batch("memory-dude", "user", sources)
            .unwrap()
            .is_none()
    );
    let next = reopened
        .prepare_batch("memory-dude", "user", vec![source("w-a", "same-id", "2")])
        .unwrap()
        .unwrap();
    assert_ne!(next.batch_code, batch.batch_code);
    assert_eq!(next.sources.len(), 1);
}
#[test]
fn dude_memory_creator_is_required_and_cannot_be_bypassed() {
    let (_root, service) = fixture();
    let batch = prepared(&service);
    let context = service
        .begin_skill_authoring(
            "user",
            &batch.batch_code,
            "normalize-labels",
            "author-session",
        )
        .unwrap();
    assert_eq!(
        context.candidate.status,
        SkillCandidateStatus::WaitingDependency
    );
    assert!(context.creator_markdown.is_none());
    assert_eq!(
        service
            .submit_candidate(
                "user",
                &context.candidate.candidate_code,
                "author-session",
                artifact("normalize-labels", true),
                behavior(),
                true
            )
            .unwrap_err(),
        DudeMemoryError::Conflict
    );
}
#[test]
fn dude_memory_approval_is_bound_to_validated_artifact_and_installs_once() {
    let (root, service) = fixture();
    install_creator(root.path(), &service);
    let batch = prepared(&service);
    let candidate = candidate(&service, &batch, true, false);
    assert_eq!(candidate.status, SkillCandidateStatus::PendingApproval);
    assert!(
        candidate
            .validation
            .as_ref()
            .unwrap()
            .checks
            .iter()
            .any(|check| check.evidence.contains("normalization behavior verified"))
    );
    assert_eq!(
        service
            .approve(
                "user",
                &candidate.candidate_code,
                candidate.revision,
                "changed-hash"
            )
            .unwrap_err(),
        DudeMemoryError::Conflict
    );
    assert_eq!(
        service
            .approve(
                "other-user",
                &candidate.candidate_code,
                candidate.revision,
                &candidate.artifact_hash
            )
            .unwrap_err(),
        DudeMemoryError::NotFound
    );
    let installed = service
        .approve(
            "user",
            &candidate.candidate_code,
            candidate.revision,
            &candidate.artifact_hash,
        )
        .unwrap();
    assert_eq!(installed.status, SkillCandidateStatus::Installed);
    let retried = service
        .retry_install(
            "user",
            &candidate.candidate_code,
            candidate.revision,
            &candidate.artifact_hash,
        )
        .unwrap();
    assert_eq!(retried, installed);
    assert_eq!(
        fs::read_to_string(
            root.path()
                .join("registry/.agents/skills/normalize-labels/scripts/normalize.py")
        )
        .unwrap(),
        "def normalize(text):\n    return ' '.join(text.split())\n"
    );
    assert!(
        service
            .skills
            .skill_agents("normalize-labels")
            .unwrap()
            .is_empty()
    );
}
#[test]
fn dude_memory_auto_approval_still_requires_real_behavior_validation() {
    let (root, service) = fixture();
    install_creator(root.path(), &service);
    let batch = prepared(&service);
    let failed = candidate(&service, &batch, false, true);
    assert_eq!(failed.status, SkillCandidateStatus::ValidationFailed);
    assert!(failed.approved_hash.is_none());
    assert!(
        !root
            .path()
            .join("registry/.agents/skills/normalize-labels")
            .exists()
    );
    service
        .request_revision(
            "user",
            &failed.candidate_code,
            failed.revision,
            &failed.artifact_hash,
            "Fix the expected output".into(),
        )
        .unwrap();
    let context = service
        .begin_skill_authoring(
            "user",
            &batch.batch_code,
            "normalize-labels",
            "second-session",
        )
        .unwrap();
    let success = service
        .submit_candidate(
            "user",
            &context.candidate.candidate_code,
            "second-session",
            artifact("normalize-labels", true),
            behavior(),
            true,
        )
        .unwrap();
    assert_eq!(success.status, SkillCandidateStatus::Installed);
    assert_eq!(success.revision, 2);
    assert_eq!(
        service
            .approve(
                "user",
                &success.candidate_code,
                failed.revision,
                &failed.artifact_hash
            )
            .unwrap_err(),
        DudeMemoryError::Conflict
    );
}
#[test]
fn dude_memory_update_does_not_overwrite_user_changes_and_retry_reconciles() {
    let (root, service) = fixture();
    install_creator(root.path(), &service);
    let initial = artifact("normalize-labels", true);
    let hash = artifact_hash(&initial).unwrap();
    service
        .skills
        .install_approved_artifact("normalize-labels", &initial, None, &hash)
        .unwrap();
    let batch = prepared(&service);
    let mut changed = artifact("normalize-labels", true);
    changed[0].content.push_str("New reusable observation.\n");
    let context = service
        .begin_skill_authoring(
            "user",
            &batch.batch_code,
            "normalize-labels",
            "author-session",
        )
        .unwrap();
    let pending = service
        .submit_candidate(
            "user",
            &context.candidate.candidate_code,
            "author-session",
            changed,
            behavior(),
            false,
        )
        .unwrap();
    let target = root
        .path()
        .join("registry/.agents/skills/normalize-labels/SKILL.md");
    fs::write(&target, "user edited content").unwrap();
    let failed = service
        .approve(
            "user",
            &pending.candidate_code,
            pending.revision,
            &pending.artifact_hash,
        )
        .unwrap();
    assert_eq!(failed.status, SkillCandidateStatus::InstallFailed);
    assert_eq!(fs::read_to_string(&target).unwrap(), "user edited content");
    fs::write(&target, &initial[0].content).unwrap();
    let installed = service
        .retry_install(
            "user",
            &pending.candidate_code,
            pending.revision,
            &pending.artifact_hash,
        )
        .unwrap();
    assert_eq!(installed.status, SkillCandidateStatus::Installed);
}
#[test]
fn dude_memory_artifacts_reject_traversal_and_binary_hash_is_encoding_independent() {
    let files = vec![SkillArtifactFile {
        path: "../escape".into(),
        content: "x".into(),
        encoding: ArtifactEncoding::Utf8,
    }];
    assert!(artifact_hash(&files).is_err());
    let a = vec![SkillArtifactFile {
        path: "assets/a".into(),
        content: "hello".into(),
        encoding: ArtifactEncoding::Utf8,
    }];
    let b = vec![SkillArtifactFile {
        path: "assets/a".into(),
        content: "aGVsbG8=".into(),
        encoding: ArtifactEncoding::Base64,
    }];
    assert_eq!(artifact_hash(&a).unwrap(), artifact_hash(&b).unwrap());
}
