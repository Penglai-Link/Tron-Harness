mod agent_event_replay_store;
mod agent_management_repository;
mod agent_repository;
mod app_repository;
mod automation_repository;
mod automation_repository_helpers;
mod chat_repository;
pub(crate) mod framework_storage;
mod mcp_repository;
mod memory_work_domain_repository;
mod model_repository;
mod navigation_repository;
pub(crate) mod project_repository;
mod prompt_repository;
mod rag_repository;
mod resource_repository;
pub(crate) mod schema;
mod skill_repository;
mod task_repository;
mod variable_repository;
mod workspace_repository;

pub use agent_event_replay_store::SqliteAgentEventReplayStore;
pub use agent_management_repository::SqliteAgentManagementRepository;
pub use agent_repository::SqliteAgentRunRepository;
pub use app_repository::SqliteAppRepository;
pub use automation_repository::SqliteAutomationRepository;
pub use chat_repository::SqliteChatRepository;
pub use mcp_repository::SqliteMcpRepository;
pub use memory_work_domain_repository::SqliteMemoryWorkDomainRepository;
pub use model_repository::SqliteModelRepository;
pub use navigation_repository::SqliteNavigationRepository;
pub use project_repository::SqliteProjectRepository;
pub use prompt_repository::SqlitePromptRepository;
pub use rag_repository::SqliteRagRepository;
pub use resource_repository::SqliteResourceRepository;
pub use skill_repository::SqliteSkillRegistryRepository;
pub use task_repository::SqliteTaskRepository;
pub use variable_repository::SqliteVariableRepository;
pub use workspace_repository::SqliteWorkspaceRepository;

mod knowledge_base_repository;
mod knowledge_ingestion_queue;
pub use knowledge_base_repository::SqliteKnowledgeBaseRepository;
pub(crate) use knowledge_base_repository::{StoredChunkContext,OptimizedDocumentPublication,stored_contexts};
pub use knowledge_ingestion_queue::SqliteKnowledgeIngestionQueue;

pub mod capability_repository;

mod dude_repository;
pub use dude_repository::SqliteDudeRepository;
mod dude_memory_repository;
pub use dude_memory_repository::SqliteDudeMemoryRepository;

mod rag_graph_index;
pub use rag_graph_index::SqliteRagGraphIndexRepository;

#[cfg(test)]
mod query_scaling_tests;
