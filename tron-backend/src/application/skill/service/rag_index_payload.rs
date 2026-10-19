use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use serde_json::{Value, json};

use crate::{
    application::rag::{RagIngestAccessContext, RagIngestRequest, RagIngestScope, RagIngestSource},
    domain::skill::RegistrySkillFile,
};

pub(super) const SKILL_SOURCE_VERSION: &str = "v1";

const INDEX_USER_CODE: &str = "SYSTEM";
const INDEX_AGENT_CODE: &str = "AGENT_CODEX";

pub(super) fn skill_ingest_request(
    workspace_code: &str,
    skill_name: &str,
    corpus_id: &str,
    payload: Value,
) -> RagIngestRequest {
    RagIngestRequest {
        corpus_id: corpus_id.into(),
        workspace_code: workspace_code.into(),
        access_context: RagIngestAccessContext {
            user_code: INDEX_USER_CODE.into(),
            agent_code: INDEX_AGENT_CODE.into(),
            workspace_code: workspace_code.into(),
        },
        target_scope: RagIngestScope {
            scope_type: "SPACE".into(),
            scope_code: workspace_code.into(),
        },
        source: RagIngestSource {
            code: source_code(workspace_code, skill_name),
            version: SKILL_SOURCE_VERSION.into(),
        },
        payload,
    }
}

pub(super) fn skill_payload(
    workspace_code: &str,
    skill_name: &str,
    description: &str,
    text: &str,
    related_graph: Option<Value>,
) -> Value {
    let mut payload = json!({
        "source": {
            "code": source_code(workspace_code, skill_name),
            "version": SKILL_SOURCE_VERSION
        },
        "title": skill_name,
        "text": text,
        "metadata": {
            "source": "skill-registry"
        },
        "domainFields": {
            "skillName": skill_name,
            "skillDescription": description
        }
    });
    if let Some(related_graph) = related_graph {
        payload["relatedGraph"] = related_graph;
    }
    payload
}

pub(super) fn skill_index_text(skill_name: &str, description: &str) -> String {
    format!("Skill: {skill_name}\nDescription: {description}")
}

pub(super) fn related_graph_payload(
    workspace_code: &str,
    skill_name: &str,
    files: &[RegistrySkillFile],
) -> Value {
    let mut nodes = files
        .iter()
        .map(|file| {
            let kind = file_kind(&file.path);
            let reference = file_ref(&file.path);
            json!({
                "ref": reference,
                "name":file.path,
                "type":"SkillFile",
                "kind": kind,
                "label": "RagSkillFile",
                "code": file_code(workspace_code, skill_name, &file.path),
                "title": file.path,
                "text": file.path,
                "evidence":file.path,
                "properties": {
                    "path": file.path,
                    "kind": kind,
                    "isMarkdown": file.is_markdown
                }
            })
        })
        .collect::<Vec<_>>();
    nodes.push(json!({"ref":"$skill","name":skill_name,"type":"Skill","evidence":skill_name}));
    let edges = files
        .iter()
        .map(|file| {
            let kind = file_kind(&file.path);
            json!({
                "from": "$skill",
                "to": file_ref(&file.path),
                "relationship": "CONTAINS",
                "evidence":file.path,
                "properties": {
                    "path": file.path,
                    "kind": kind
                }
            })
        })
        .collect::<Vec<_>>();
    json!({ "nodes": nodes, "edges": edges })
}

pub(super) fn stable_structure_hash(files: &[RegistrySkillFile]) -> String {
    let mut paths = files
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();
    paths.sort_unstable();
    stable_hash(&paths.join("\n"))
}

pub(super) fn stable_hash(value: &str) -> String {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

pub(super) fn source_code(workspace_code: &str, skill_name: &str) -> String {
    format!("skill:{workspace_code}:{skill_name}")
}

fn file_kind(path: &str) -> &'static str {
    let normalized = path.replace('\\', "/");
    if normalized.eq_ignore_ascii_case("SKILL.md") {
        "SKILL_MD"
    } else if normalized.starts_with("scripts/") {
        "SCRIPT"
    } else if normalized.starts_with("references/") {
        "REFERENCE"
    } else {
        "OTHER"
    }
}

fn file_ref(path: &str) -> String {
    format!("file:{path}")
}

fn file_code(workspace_code: &str, skill_name: &str, path: &str) -> String {
    format!(
        "SKILL_FILE_{}",
        stable_hash(&format!("{workspace_code}:{skill_name}:{path}")).to_ascii_uppercase()
    )
}
