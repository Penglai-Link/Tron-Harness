use crate::domain::agent::{RuntimeAdapterCapabilityRecord, SystemAgentRegistryEntry};

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
        RuntimeAdapterKey::MainReact => (true, true),
        RuntimeAdapterKey::CodexExec => system_agent_availability(entries, "codex"),
        RuntimeAdapterKey::OpenCodeRun => system_agent_availability(entries, "opencode"),
        RuntimeAdapterKey::CodexAppServer
        | RuntimeAdapterKey::OpenCodeServe
        | RuntimeAdapterKey::OpenCodeAcp => (true, true),
    }
}

fn system_agent_availability(
    entries: &[SystemAgentRegistryEntry],
    agent_code: &str,
) -> (bool, bool) {
    let Some(entry) = entries
        .iter()
        .find(|item| item.agent_code.eq_ignore_ascii_case(agent_code))
    else {
        return (false, false);
    };
    (entry.available, entry.available && entry.enabled)
}
