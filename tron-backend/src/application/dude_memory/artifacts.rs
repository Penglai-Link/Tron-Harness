use crate::domain::dude_memory::*;
use base64::Engine;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub fn artifact_bytes(file: &SkillArtifactFile) -> Result<Vec<u8>, DudeMemoryError> {
    match file.encoding {
        ArtifactEncoding::Utf8 => Ok(file.content.as_bytes().to_vec()),
        ArtifactEncoding::Base64 => base64::engine::general_purpose::STANDARD
            .decode(&file.content)
            .map_err(|_| DudeMemoryError::Invalid("invalid artifact Base64".into())),
    }
}
fn checked_files(files: &[SkillArtifactFile]) -> Result<BTreeMap<&str, Vec<u8>>, DudeMemoryError> {
    if files.len() > 128 {
        return Err(DudeMemoryError::Invalid(
            "a skill can contain at most 128 files".into(),
        ));
    }
    let mut ordered = BTreeMap::new();
    let mut size = 0;
    for file in files {
        let path = Path::new(&file.path);
        if file.path.is_empty()
            || file.path.contains('\\')
            || path.is_absolute()
            || path
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err(DudeMemoryError::Invalid(
                "artifact paths must be relative and cannot traverse directories".into(),
            ));
        }
        let bytes = artifact_bytes(file)?;
        size += bytes.len();
        if size > 5 * 1024 * 1024 || ordered.insert(file.path.as_str(), bytes).is_some() {
            return Err(DudeMemoryError::Invalid(
                "artifact is oversized or contains duplicate paths".into(),
            ));
        }
    }
    Ok(ordered)
}
pub fn artifact_hash(files: &[SkillArtifactFile]) -> Result<String, DudeMemoryError> {
    let mut hash = Sha256::new();
    for (path, content) in checked_files(files)? {
        hash.update((path.len() as u64).to_be_bytes());
        hash.update(path.as_bytes());
        hash.update((content.len() as u64).to_be_bytes());
        hash.update(content);
    }
    Ok(hex::encode(hash.finalize()))
}
pub fn materialize_artifact(
    root: &Path,
    files: &[SkillArtifactFile],
) -> Result<(), DudeMemoryError> {
    let files = checked_files(files)?;
    fs::create_dir(root).map_err(|e| DudeMemoryError::Storage(e.to_string()))?;
    for (path, content) in files {
        let destination = root.join(path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|e| DudeMemoryError::Storage(e.to_string()))?;
        }
        fs::write(destination, content).map_err(|e| DudeMemoryError::Storage(e.to_string()))?;
    }
    Ok(())
}
pub fn read_artifact(root: &Path) -> Result<Vec<SkillArtifactFile>, DudeMemoryError> {
    fn visit(
        root: &Path,
        current: &Path,
        files: &mut Vec<SkillArtifactFile>,
    ) -> Result<(), DudeMemoryError> {
        for entry in fs::read_dir(current).map_err(|e| DudeMemoryError::Storage(e.to_string()))? {
            let entry = entry.map_err(|e| DudeMemoryError::Storage(e.to_string()))?;
            let kind = entry
                .file_type()
                .map_err(|e| DudeMemoryError::Storage(e.to_string()))?;
            if kind.is_symlink() {
                return Err(DudeMemoryError::Invalid(
                    "skill artifacts cannot contain symlinks".into(),
                ));
            }
            if kind.is_dir() {
                visit(root, &entry.path(), files)?;
            } else if kind.is_file() {
                let bytes =
                    fs::read(entry.path()).map_err(|e| DudeMemoryError::Storage(e.to_string()))?;
                let (content, encoding) = match String::from_utf8(bytes.clone()) {
                    Ok(text) => (text, ArtifactEncoding::Utf8),
                    Err(_) => (
                        base64::engine::general_purpose::STANDARD.encode(bytes),
                        ArtifactEncoding::Base64,
                    ),
                };
                files.push(SkillArtifactFile {
                    path: entry
                        .path()
                        .strip_prefix(root)
                        .map_err(|_| DudeMemoryError::Conflict)?
                        .to_string_lossy()
                        .replace('\\', "/"),
                    content,
                    encoding,
                });
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    visit(root, root, &mut files)?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    checked_files(&files)?;
    Ok(files)
}
pub fn validate_artifact(
    skill_name: &str,
    files: &[SkillArtifactFile],
    behavior: &[BehaviorCheck],
) -> Result<SkillValidation, DudeMemoryError> {
    let hash = artifact_hash(files)?;
    let checked = checked_files(files)?;
    let markdown = checked
        .get("SKILL.md")
        .and_then(|bytes| std::str::from_utf8(bytes).ok());
    let frontmatter = markdown
        .and_then(|text| text.strip_prefix("---\n"))
        .and_then(|text| text.split_once("\n---"))
        .and_then(|(yaml, body)| {
            serde_yaml::from_str::<serde_yaml::Value>(yaml)
                .ok()
                .map(|value| (value, body))
        });
    let valid_name = !skill_name.is_empty()
        && skill_name.len() < 64
        && skill_name != "skill-creator"
        && skill_name
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-');
    let valid_document = frontmatter.is_some_and(|(yaml, body)| {
        yaml["name"].as_str() == Some(skill_name)
            && yaml["description"]
                .as_str()
                .is_some_and(|s| !s.trim().is_empty())
            && !body.trim().is_empty()
    });
    let mut checks = vec![ValidationCheck {
        name: "skill-structure".into(),
        passed: valid_name && valid_document,
        evidence:
            "Validated SKILL.md frontmatter, skill name, description and nonempty instructions"
                .into(),
    }];
    let static_passed = valid_name && valid_document;
    let directory = tempfile::tempdir().map_err(|e| DudeMemoryError::Storage(e.to_string()))?;
    let root = directory.path().join(skill_name);
    materialize_artifact(&root, files)?;
    if behavior.is_empty() || behavior.len() > 8 {
        checks.push(ValidationCheck { name: "behavior-validation".into(), passed: false, evidence: "Provide 1-8 executable behavioral checks; structural validation alone cannot approve a skill".into() });
    } else if static_passed {
        for check in behavior {
            checks.push(run_behavior_check(&root, check));
        }
    }
    let behavior_passed = static_passed
        && !behavior.is_empty()
        && behavior.len() <= 8
        && checks.iter().all(|check| check.passed);
    Ok(SkillValidation {
        static_passed,
        behavior_passed,
        checks,
        artifact_hash: hash,
    })
}
fn run_behavior_check(root: &Path, check: &BehaviorCheck) -> ValidationCheck {
    let execute = || -> Result<(bool, String), String> {
        if !matches!(check.program.as_str(), "python3" | "node" | "bash" | "sh") {
            return Err(
                "behavior checks use python3, node, bash or sh with a staged script".into(),
            );
        }
        let script = check
            .args
            .first()
            .ok_or("behavior check requires a staged script")?;
        if script.starts_with('-')
            || Path::new(script).is_absolute()
            || Path::new(script)
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
            || !root.join(script).is_file()
        {
            return Err("behavior check script must exist inside the exact staged artifact".into());
        }
        let stdout = tempfile::tempfile().map_err(|e| e.to_string())?;
        let stderr = stdout.try_clone().map_err(|e| e.to_string())?;
        let mut child = Command::new(&check.program)
            .args(&check.args)
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(stdout.try_clone().map_err(|e| e.to_string())?)
            .stderr(stderr)
            .spawn()
            .map_err(|e| e.to_string())?;
        let start = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
                break status;
            }
            if start.elapsed() > Duration::from_secs(30) {
                let _ = child.kill();
                let _ = child.wait();
                return Err("behavior validation exceeded 30 seconds".into());
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        use std::io::{Read, Seek};
        let mut output = stdout;
        output.rewind().map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        output
            .take(8192)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        Ok((
            status.success(),
            format!(
                "{} {:?}: exit {:?}\n{}",
                check.program,
                check.args,
                status.code(),
                String::from_utf8_lossy(&bytes)
            ),
        ))
    };
    let (passed, evidence) = execute().unwrap_or_else(|error| (false, error));
    ValidationCheck {
        name: check.name.clone(),
        passed,
        evidence,
    }
}
