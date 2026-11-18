//! Host-owned context descriptions. Plugin text comes from ContextContributor metadata.
use std::collections::BTreeMap;

use super::{ContextProvidersConfig, ContextReductionStrategy};
use eliteforge_agent_core::{
    ContextBlockMetadata, ContextBlockUpdateMode, ContextCompressionMetadata,
};

use crate::application::agent::framework_runtime::ContextLayoutContributorInfo;

/// Stable translation keys for one host-owned context block.
///
/// The catalog never carries display text: `block_metadata` answers with keys so
/// the HTTP layout/presentation layer translates them with the request locale.
/// Plugin-contributed blocks keep their own localized metadata instead.
#[derive(Clone, Copy)]
pub(crate) struct ContextBlockTextKeys {
    pub display_name: &'static str,
    pub description: &'static str,
}

pub(crate) struct ContextBlockCatalogEntry {
    pub metadata: ContextBlockMetadata,
    pub provider_id: Option<String>,
    pub i18n: Option<ContextBlockTextKeys>,
}

/// The catalog is shared by layout controls and live/historical assembly presentation.
pub(crate) fn context_block_catalog(
    contributors: Vec<ContextLayoutContributorInfo>,
    providers: &ContextProvidersConfig,
) -> BTreeMap<String, ContextBlockCatalogEntry> {
    let mut catalog = HOST_BLOCKS
        .iter()
        .map(|block| {
            (
                block.id.to_owned(),
                ContextBlockCatalogEntry {
                    metadata: configured_context_block_metadata(block.id, providers)
                        .expect("declared host context block"),
                    provider_id: None,
                    i18n: Some(block.text),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    // Plugin metadata wins, just as plugin reducers win during assembly.
    for contributor in contributors {
        let i18n = host_block_text_keys(&contributor.block_id);
        catalog.insert(
            contributor.block_id,
            ContextBlockCatalogEntry {
                metadata: contributor.metadata,
                provider_id: Some(contributor.provider_id),
                i18n,
            },
        );
    }
    for (alias, id) in [
        ("framework.instructions", "seed_prompt_prefix"),
        ("framework.history", "current_session_history"),
        ("current_session_checkpoint", "conversation_history_summary"),
    ] {
        if let Some(entry) = catalog.get(id) {
            catalog.insert(
                alias.into(),
                ContextBlockCatalogEntry {
                    metadata: entry.metadata.clone(),
                    provider_id: entry.provider_id.clone(),
                    i18n: entry.i18n,
                },
            );
        }
    }
    catalog
}

/// Metadata used both by native contributors and the presentation catalog.
pub(crate) fn native_context_block_metadata(id: &str) -> Option<ContextBlockMetadata> {
    configured_context_block_metadata(id, &ContextProvidersConfig::default())
}

/// Translation keys for a host-owned block, or `None` for plugin blocks.
pub(crate) fn host_block_text_keys(id: &str) -> Option<ContextBlockTextKeys> {
    HOST_BLOCKS
        .iter()
        .find(|block| block.id == id)
        .map(|block| block.text)
}

fn configured_context_block_metadata(
    id: &str,
    providers: &ContextProvidersConfig,
) -> Option<ContextBlockMetadata> {
    let block = HOST_BLOCKS.iter().find(|block| block.id == id)?;
    let text = block.text;
    let compression = &block.compression;
    let mode = match id {
        "current_session_history" => ContextBlockUpdateMode::Append,
        "seed_prompt_prefix" | "agent.directory-guidance" | "assistant.guidance" => {
            ContextBlockUpdateMode::Fixed
        }
        _ => ContextBlockUpdateMode::Replace,
    };
    // The base text is the stable resource key, never translated here: the
    // display layer resolves it with the request locale, and persisted layouts
    // only ever store block ids.
    Some(
        ContextBlockMetadata::new(text.display_name, text.description)
            .with_update_mode(mode)
            .with_compression(compression.resolve(id, providers)),
    )
}

/// One host-owned context block: stable id, translation keys and reduction policy.
struct HostBlock {
    id: &'static str,
    text: ContextBlockTextKeys,
    compression: CompressionSource,
}

// Each native source declares its semantics with its other presentation metadata.
// Data sources resolve the policy already used by assembly; plugins supply their own metadata.
enum CompressionSource {
    Keep,
    DataPolicy,
    History,
    HistorySummary,
    Working,
    Unspecified,
}

impl CompressionSource {
    fn resolve(&self, id: &str, providers: &ContextProvidersConfig) -> ContextCompressionMetadata {
        match self {
            Self::Keep => ContextCompressionMetadata::keep(),
            Self::DataPolicy => providers
                .data_blocks
                .get(id)
                .map(|block| block.policy.compression_metadata())
                .unwrap_or_default(),
            Self::History => providers.history.compression_metadata(),
            Self::HistorySummary
                if providers.history.strategy == ContextReductionStrategy::Checkpoint =>
            {
                ContextCompressionMetadata::coordinated_summary()
            }
            Self::HistorySummary => ContextCompressionMetadata::keep(),
            Self::Working => providers.working.compression_metadata(),
            Self::Unspecified => ContextCompressionMetadata::default(),
        }
    }
}

const HOST_BLOCKS: &[HostBlock] = &[
    HostBlock {
        id: "seed_prompt_prefix",
        text: ContextBlockTextKeys {
            display_name: "agent_runtime.context.block.seed_prompt_prefix.name",
            description: "agent_runtime.context.block.seed_prompt_prefix.description",
        },
        compression: CompressionSource::Keep,
    },
    HostBlock {
        id: "conversation_history_summary",
        text: ContextBlockTextKeys {
            display_name: "agent_runtime.context.block.conversation_history_summary.name",
            description: "agent_runtime.context.block.conversation_history_summary.description",
        },
        compression: CompressionSource::HistorySummary,
    },
    HostBlock {
        id: "current_session_history",
        text: ContextBlockTextKeys {
            display_name: "agent_runtime.context.block.current_session_history.name",
            description: "agent_runtime.context.block.current_session_history.description",
        },
        compression: CompressionSource::History,
    },
    HostBlock {
        id: "current_session_working_state",
        text: ContextBlockTextKeys {
            display_name: "agent_runtime.context.block.current_session_working_state.name",
            description: "agent_runtime.context.block.current_session_working_state.description",
        },
        compression: CompressionSource::Working,
    },
    HostBlock {
        id: "runtime_identity",
        text: ContextBlockTextKeys {
            display_name: "agent_runtime.context.block.runtime_identity.name",
            description: "agent_runtime.context.block.runtime_identity.description",
        },
        compression: CompressionSource::DataPolicy,
    },
    HostBlock {
        id: "assistant_onboarding",
        text: ContextBlockTextKeys {
            display_name: "agent_runtime.context.block.assistant_onboarding.name",
            description: "agent_runtime.context.block.assistant_onboarding.description",
        },
        compression: CompressionSource::DataPolicy,
    },
    HostBlock {
        id: "tool_domains",
        text: ContextBlockTextKeys {
            display_name: "agent_runtime.context.block.tool_domains.name",
            description: "agent_runtime.context.block.tool_domains.description",
        },
        compression: CompressionSource::DataPolicy,
    },
    HostBlock {
        id: "agent_directory",
        text: ContextBlockTextKeys {
            display_name: "agent_runtime.context.block.agent_directory.name",
            description: "agent_runtime.context.block.agent_directory.description",
        },
        compression: CompressionSource::DataPolicy,
    },
    HostBlock {
        id: "page_context",
        text: ContextBlockTextKeys {
            display_name: "agent_runtime.context.block.page_context.name",
            description: "agent_runtime.context.block.page_context.description",
        },
        compression: CompressionSource::DataPolicy,
    },
    HostBlock {
        id: "application_context",
        text: ContextBlockTextKeys {
            display_name: "agent_runtime.context.block.application_context.name",
            description: "agent_runtime.context.block.application_context.description",
        },
        compression: CompressionSource::DataPolicy,
    },
    HostBlock {
        id: "topic_memo",
        text: ContextBlockTextKeys {
            display_name: "agent_runtime.context.block.topic_memo.name",
            description: "agent_runtime.context.block.topic_memo.description",
        },
        compression: CompressionSource::Keep,
    },
    HostBlock {
        id: "long_term_memory",
        text: ContextBlockTextKeys {
            display_name: "agent_runtime.context.block.long_term_memory.name",
            description: "agent_runtime.context.block.long_term_memory.description",
        },
        compression: CompressionSource::DataPolicy,
    },
    HostBlock {
        id: "knowledge_base",
        text: ContextBlockTextKeys {
            display_name: "agent_runtime.context.block.knowledge_base.name",
            description: "agent_runtime.context.block.knowledge_base.description",
        },
        compression: CompressionSource::DataPolicy,
    },
    HostBlock {
        id: "assistant_environment",
        text: ContextBlockTextKeys {
            display_name: "agent_runtime.context.block.assistant_environment.name",
            description: "agent_runtime.context.block.assistant_environment.description",
        },
        compression: CompressionSource::DataPolicy,
    },
    HostBlock {
        id: "assistant_wakeup",
        text: ContextBlockTextKeys {
            display_name: "agent_runtime.context.block.assistant_wakeup.name",
            description: "agent_runtime.context.block.assistant_wakeup.description",
        },
        compression: CompressionSource::DataPolicy,
    },
    HostBlock {
        id: "agent.directory-guidance",
        text: ContextBlockTextKeys {
            display_name: "agent_runtime.context.block.agent_directory_guidance.name",
            description: "agent_runtime.context.block.agent_directory_guidance.description",
        },
        compression: CompressionSource::Keep,
    },
    HostBlock {
        id: "assistant.guidance",
        text: ContextBlockTextKeys {
            display_name: "agent_runtime.context.block.assistant_guidance.name",
            description: "agent_runtime.context.block.assistant_guidance.description",
        },
        compression: CompressionSource::Keep,
    },
    HostBlock {
        id: "protocol",
        text: ContextBlockTextKeys {
            display_name: "agent_runtime.context.block.protocol.name",
            description: "agent_runtime.context.block.protocol.description",
        },
        compression: CompressionSource::Unspecified,
    },
];
