use crate::domain::skill::*;
use crate::shared::time::{option_timestamp_to_millis, timestamp_to_millis};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrySkillListItemVo {
    pub skill_name: String,
    pub provider_pack_id: Option<String>,
    pub available: bool,
    pub assignable: bool,
    pub source_ref: String,
    pub description: Option<String>,
    pub source_type: SkillSourceType,
    pub registry_path: String,
    pub declared_version: Option<String>,
    pub update_status: SkillUpdateStatus,
    pub managed_by: SkillManagedBy,
    pub last_checked_at: Option<i64>,
    pub last_updated_at: Option<i64>,
    pub last_error: Option<String>,
    pub assigned_agent_count: usize,
    pub assigned: bool,
    pub disabled_reason: Option<String>,
}
impl From<RegistrySkillListItem> for RegistrySkillListItemVo {
    fn from(value: RegistrySkillListItem) -> Self {
        Self {
            skill_name: value.skill_name,
            provider_pack_id: value.provider_pack_id,
            available: value.available,
            assignable: value.available,
            source_ref: value.source_ref,
            description: value.description,
            source_type: value.source_type,
            registry_path: value.registry_path,
            declared_version: value.declared_version,
            update_status: value.update_status,
            managed_by: value.managed_by,
            last_checked_at: option_timestamp_to_millis(value.last_checked_at.as_deref()),
            last_updated_at: option_timestamp_to_millis(value.last_updated_at.as_deref()),
            last_error: value.last_error,
            assigned_agent_count: value.assigned_agent_count,
            assigned: value.assigned,
            disabled_reason: value.disabled_reason,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrySkillDetailVo {
    pub skill_name: String,
    pub provider_pack_id: Option<String>,
    pub available: bool,
    pub assignable: bool,
    pub source_ref: String,
    pub source_type: SkillSourceType,
    pub registry_path: String,
    pub declared_version: Option<String>,
    pub update_status: SkillUpdateStatus,
    pub managed_by: SkillManagedBy,
    pub last_checked_at: Option<i64>,
    pub last_updated_at: Option<i64>,
    pub last_error: Option<String>,
    pub skill_markdown: String,
    pub files: Vec<RegistrySkillFileVo>,
    pub assigned_agent_count: usize,
    pub disabled_reason: Option<String>,
}
impl From<RegistrySkillDetail> for RegistrySkillDetailVo {
    fn from(value: RegistrySkillDetail) -> Self {
        Self {
            skill_name: value.skill_name,
            provider_pack_id: value.provider_pack_id,
            available: value.available,
            assignable: value.available,
            source_ref: value.source_ref,
            source_type: value.source_type,
            registry_path: value.registry_path,
            declared_version: value.declared_version,
            update_status: value.update_status,
            managed_by: value.managed_by,
            last_checked_at: option_timestamp_to_millis(value.last_checked_at.as_deref()),
            last_updated_at: option_timestamp_to_millis(value.last_updated_at.as_deref()),
            last_error: value.last_error,
            skill_markdown: value.skill_markdown,
            files: value.files.into_iter().map(Into::into).collect(),
            assigned_agent_count: value.assigned_agent_count,
            disabled_reason: value.disabled_reason,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrySkillFileVo {
    pub path: String,
    pub is_markdown: bool,
}
impl From<RegistrySkillFile> for RegistrySkillFileVo {
    fn from(value: RegistrySkillFile) -> Self {
        Self {
            path: value.path,
            is_markdown: value.is_markdown,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSearchCandidateVo {
    pub package_ref: String,
    pub source_ref: String,
    pub skill_name: String,
    pub description: Option<String>,
    pub installs_label: Option<String>,
    pub detail_url: Option<String>,
}
impl From<SkillSearchCandidate> for SkillSearchCandidateVo {
    fn from(value: SkillSearchCandidate) -> Self {
        Self {
            package_ref: value.package_ref,
            source_ref: value.source_ref,
            skill_name: value.skill_name,
            description: value.description,
            installs_label: value.installs_label,
            detail_url: value.detail_url,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillRegistryMutationResultVo {
    pub skill_name: String,
    pub rag_index: Option<SkillRagIndexSummaryVo>,
}
impl From<SkillRegistryMutationResult> for SkillRegistryMutationResultVo {
    fn from(value: SkillRegistryMutationResult) -> Self {
        Self {
            skill_name: value.skill_name,
            rag_index: value.rag_index.map(Into::into),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillRegistryUninstallResultVo {
    pub skill_name: String,
    pub removed_assignments_count: usize,
    pub rag_index: Option<SkillRagIndexBatchSummaryVo>,
}
impl From<SkillRegistryUninstallResult> for SkillRegistryUninstallResultVo {
    fn from(value: SkillRegistryUninstallResult) -> Self {
        Self {
            skill_name: value.skill_name,
            removed_assignments_count: value.removed_assignments_count,
            rag_index: value.rag_index.map(Into::into),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillRegistrySyncResultVo {
    pub registry_root: String,
    pub discovered_count: usize,
    pub removed_count: usize,
    pub checked: bool,
    pub updated: bool,
    pub rag_index: Option<SkillRagIndexBatchSummaryVo>,
}
impl From<SkillRegistrySyncResult> for SkillRegistrySyncResultVo {
    fn from(value: SkillRegistrySyncResult) -> Self {
        Self {
            registry_root: value.registry_root,
            discovered_count: value.discovered_count,
            removed_count: value.removed_count,
            checked: value.checked,
            updated: value.updated,
            rag_index: value.rag_index.map(Into::into),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillRagIndexSummaryVo {
    pub desired_enabled: bool,
    pub source_version: String,
    pub vector_status: SkillRagIndexStatus,
    pub graph_status: SkillRagIndexStatus,
    pub content_hash: Option<String>,
    pub structure_hash: Option<String>,
    pub last_indexed_at: Option<i64>,
    pub updated_at: i64,
    pub last_error: Option<String>,
    pub warnings: Vec<String>,
}
impl From<SkillRagIndexSummary> for SkillRagIndexSummaryVo {
    fn from(value: SkillRagIndexSummary) -> Self {
        Self {
            desired_enabled: value.desired_enabled,
            source_version: value.source_version,
            vector_status: value.vector_status,
            graph_status: value.graph_status,
            content_hash: value.content_hash,
            structure_hash: value.structure_hash,
            last_indexed_at: option_timestamp_to_millis(value.last_indexed_at.as_deref()),
            updated_at: timestamp_to_millis(&value.updated_at),
            last_error: value.last_error,
            warnings: value.warnings,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillRagIndexBatchSummaryVo {
    pub indexed_count: usize,
    pub disabled_count: usize,
    pub failed_count: usize,
    pub warning_count: usize,
    pub warnings: Vec<String>,
}
impl From<SkillRagIndexBatchSummary> for SkillRagIndexBatchSummaryVo {
    fn from(value: SkillRagIndexBatchSummary) -> Self {
        Self {
            indexed_count: value.indexed_count,
            disabled_count: value.disabled_count,
            failed_count: value.failed_count,
            warning_count: value.warning_count,
            warnings: value.warnings,
        }
    }
}

pub fn skill_list(items: Vec<RegistrySkillListItem>, locale: &str) -> Vec<RegistrySkillListItemVo> {
    items
        .into_iter()
        .map(|s| {
            let mut vo = RegistrySkillListItemVo::from(s);
            if !vo.available {
                vo.disabled_reason = Some(tron_i18n::t!(
                    "skills.pack.disabled_reason",
                    locale = locale,
                    pack = vo.provider_pack_id.as_deref().unwrap_or("")
                ));
            }
            vo
        })
        .collect()
}
