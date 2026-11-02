use crate::domain::agent::{
    AgentRuntimeType, RuntimeAdapterCapabilityRecord, SystemAgentRegistryEntry,
};

use super::{RuntimeAdapterKey, runtime_adapter_descriptors};

pub(super) fn build_runtime_capabilities(
    entries: &[SystemAgentRegistryEntry],
) -> Vec<RuntimeAdapterCapabilityRecord> {
    runtime_adapter_descriptors()
        .into_iter()
        .map(|descriptor| {
            let (available, enabled) = availability_for_descriptor(entries, descriptor.key);
            RuntimeAdapterCapabilityRecord {
                adapter_key: descriptor.key.code().into(),
                name: descriptor.name.into(),
                stage: descriptor.stage.code().into(),
                available,
                enabled,
                supports_resume: descriptor.capabilities.supports_resume,
                supports_cancel: descriptor.capabilities.supports_cancel,
                supports_steer: descriptor.capabilities.supports_steer,
                supports_streaming: descriptor.capabilities.supports_streaming,
                supports_ephemeral: descriptor.capabilities.supports_ephemeral,
                supports_workdir: descriptor.capabilities.supports_workdir,
            }
        })
        .collect()
}

pub(super) fn find_runtime_capability(
    entries: &[SystemAgentRegistryEntry],
    adapter_key: &str,
) -> Option<RuntimeAdapterCapabilityRecord> {
    let normalized = adapter_key.trim().to_ascii_uppercase();
    if normalized.is_empty() {
        return None;
    }
    build_runtime_capabilities(entries)
        .into_iter()
        .find(|item| item.adapter_key.eq_ignore_ascii_case(normalized.as_str()))
}

fn availability_for_descriptor(
    entries: &[SystemAgentRegistryEntry],
    key: RuntimeAdapterKey,
) -> (bool, bool) {
    match key {
        RuntimeAdapterKey::React => (true, true),
        RuntimeAdapterKey::CodexExec => {
            system_runtime_availability(entries, AgentRuntimeType::CodexExec)
        }
        RuntimeAdapterKey::DshHeadless => {
            system_runtime_availability(entries, AgentRuntimeType::DshHeadless)
        }
        RuntimeAdapterKey::OpenCodeRun => {
            system_runtime_availability(entries, AgentRuntimeType::OpenCodeRun)
        }
        RuntimeAdapterKey::CodexAppServer
        | RuntimeAdapterKey::OpenCodeServe
        | RuntimeAdapterKey::OpenCodeAcp => (true, true),
    }
}

fn system_runtime_availability(
    entries: &[SystemAgentRegistryEntry],
    runtime_type: AgentRuntimeType,
) -> (bool, bool) {
    let Some(entry) = entries
        .iter()
        .find(|item| item.runtime_type == runtime_type)
    else {
        return (false, false);
    };
    (entry.available, entry.available && entry.enabled)
}
