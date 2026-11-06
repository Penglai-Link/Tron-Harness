pub mod agent;
pub mod app;
pub mod automation;
pub mod chat;
pub mod chat_auto_model_router;
pub mod connector;
pub mod repository_checkout;
pub mod directory;
pub mod graph;
pub mod llm;
pub mod mcp;
pub mod memory;
pub mod model;
pub mod model_provider;
pub mod model_router;
pub mod navigation;
pub mod observability;
pub mod profile;
pub mod project;
pub mod prompt;
pub mod rag;
pub mod resource;
pub(crate) mod shell_program;
pub mod short_term_memory;
pub mod skill;
pub mod system_config;
pub mod task;
pub mod topic;
pub mod terminal;
pub mod variable;
pub mod workspace;

pub mod skin;

pub mod knowledge_base;
mod knowledge_rag_ingestion;

pub mod capability;

pub mod dude;
pub mod dude_memory;
pub mod document;
pub mod knowledge_quality;
