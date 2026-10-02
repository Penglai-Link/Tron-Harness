//! Prevent product code from recreating memory algorithms or dispatching compaction.
#[test]
fn all_compaction_and_stm_entrypoints_use_public_framework_memory() {
    let assembler = include_str!("../src/application/short_term_memory/assembler.rs");
    assert!(!assembler.contains("ShortTermMemoryEngine::assemble"));
    assert!(assembler.contains("PreparedContextSources"));
    let providers = include_str!("../src/application/agent/framework_runtime/context_providers.rs");
    for required in [
        "DefaultContextAssembler::new",
        "MemoryContextContributor::new",
        "MemorySemanticDataReducer",
        "ContextLayout(layout)",
    ] {
        assert!(
            providers.contains(required),
            "missing public SDK provider contract {required}"
        );
    }
    // The product allocator lives in its private module and delegates source
    // costs to the public SDK policy after the sole assembler reads each source.
    let budget = include_str!("../src/application/agent/framework_runtime/context_budget.rs");
    assert!(providers.contains("SharedSeedBudget"));
    assert!(providers.contains(".with_policy("));
    assert!(budget.contains("impl fw::ContextBudgetPolicy for SharedSeedBudget"));
    assert!(budget.contains("fw::ContextBudgetPolicy::allocate_sources("));
    assert!(budget.contains("BorrowUnusedMemoryBudget"));
    let policy = include_str!("../src/application/agent/framework_runtime/context.rs");
    assert!(!policy.contains("tron.harness.context"));
    assert!(!policy.contains("ContextAssemblyReport {"));
    let retention = include_str!("../src/application/short_term_memory/history_retention.rs");
    assert!(retention.contains("eliteforge_agent_memory::select_history"));
    assert!(!retention.contains("fn replay_boundary_at_or_before"));
    let config = include_str!("../src/application/short_term_memory/config.rs");
    assert!(config.contains("self.context.blocks.plan("));
    assert!(!config.contains("fn block_plan"));
    let preparation =
        include_str!("../src/application/short_term_memory/service_compaction_model.rs");
    for forbidden in [
        "fn compaction_model_request",
        "fn model_compaction_output",
        "render_compaction_prompt",
        ".generate(",
    ] {
        assert!(!preparation.contains(forbidden), "product owns {forbidden}");
    }
    assert!(preparation.contains("ShortTermMemoryEngine::write_projection"));
    let manual = include_str!("../src/adapters/inbound/http/chats/short_term_memory.rs");
    assert!(manual.contains(".compact_memory("));
    assert!(!manual.contains("agent_model_client.generate("));
    let automatic = include_str!("../src/application/agent/runtime_compaction.rs");
    assert!(automatic.contains("MemoryCompactor"));
    assert!(!automatic.contains("compaction_model_request"));
}
