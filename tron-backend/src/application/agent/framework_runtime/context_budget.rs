//! Product budget mapping into the SDK source-aware allocation hook.
use async_trait::async_trait;
use eliteforge_agent_core as fw;
use eliteforge_agent_memory as memory;

/// Instructions and application data share the product seed pool. The SDK reads
/// each source once under the pool cap, then calls this existing source-aware
/// allocation hook before reducing data and lending unused space to history.
pub(super) struct SharedSeedBudget {
    pub(super) max_tokens: usize,
    pub(super) working_pool: Option<usize>,
    pub(super) knowledge_cap: usize,
}
#[async_trait]
impl fw::ContextBudgetPolicy for SharedSeedBudget {
    async fn allocate(
        &self,
        _blocks: &[fw::ContextBlockConfig],
        _available: fw::ContextBudget,
    ) -> Result<Vec<fw::ContextBudget>, fw::AgentError> {
        Err(fw::AgentError::InvalidConfiguration(
            "shared seed allocation requires source costs".into(),
        ))
    }
    async fn allocate_sources(
        &self,
        blocks: &[fw::ContextBlockConfig],
        available: fw::ContextBudget,
        source_tokens: &[usize],
    ) -> Result<Vec<fw::ContextBudget>, fw::AgentError> {
        if blocks.len() != source_tokens.len() {
            return Err(fw::AgentError::InvalidConfiguration(
                "shared seed source cost count mismatch".into(),
            ));
        }
        let instructions = blocks
            .iter()
            .position(|block| block.id == fw::INSTRUCTIONS_BLOCK)
            .ok_or_else(|| {
                fw::AgentError::InvalidConfiguration("missing protected instructions".into())
            })?;
        let application = blocks
            .iter()
            .position(|block| block.id == "application_context")
            .ok_or_else(|| {
                fw::AgentError::InvalidConfiguration("missing application context".into())
            })?;
        let instruction_tokens = source_tokens[instructions];
        let application_tokens =
            self.max_tokens
                .checked_sub(instruction_tokens)
                .ok_or_else(|| {
                    fw::ContextError::BudgetExceeded(
                        "protected instructions exceed their shared seed budget".into(),
                    )
                })?;
        let mut adjusted = blocks.to_vec();
        adjusted[instructions].budget = fw::BlockBudget::tokens(instruction_tokens);
        adjusted[application].budget = fw::BlockBudget::tokens(application_tokens);
        if let Some(pool) = self.working_pool {
            let working = blocks
                .iter()
                .position(|block| block.id == "current_session_working_state")
                .ok_or_else(|| {
                    fw::AgentError::InvalidConfiguration("missing working state".into())
                })?;
            let knowledge = blocks
                .iter()
                .position(|block| block.id == "knowledge_base")
                .ok_or_else(|| {
                    fw::AgentError::InvalidConfiguration("missing knowledge base".into())
                })?;
            let knowledge_tokens = source_tokens[knowledge]
                .min(pool / 2)
                .min(self.knowledge_cap);
            adjusted[knowledge].budget = fw::BlockBudget::tokens(knowledge_tokens);
            adjusted[working].budget =
                fw::BlockBudget::tokens(pool.saturating_sub(knowledge_tokens));
        }
        fw::ContextBudgetPolicy::allocate_sources(
            &memory::BorrowUnusedMemoryBudget,
            &adjusted,
            available,
            source_tokens,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fw::ContextBudgetPolicy;
    #[tokio::test]
    async fn seed_pool_is_shared_with_application_and_unused_space_goes_to_history() {
        for available in [20_000, 50_000] {
            let capacity = fw::ContextBudget {
                max_tokens: available,
                max_messages: 100,
            };
            let configured = fw::ContextBlockConfig::new(
                fw::INSTRUCTIONS_BLOCK,
                fw::BlockBudget {
                    max_tokens: Some(16_000),
                    max_ratio_bps: Some(5_000),
                    max_messages: None,
                },
            );
            let cap = fw::configured_block_budget(&configured, capacity)
                .unwrap()
                .max_tokens;
            assert_eq!(cap, 16_000.min(available / 2));
            let blocks = vec![
                configured,
                fw::ContextBlockConfig::new("application_context", fw::BlockBudget::tokens(cap)),
                fw::ContextBlockConfig::new(fw::HISTORY_BLOCK, fw::BlockBudget::tokens(available)),
            ];
            let policy = SharedSeedBudget {
                max_tokens: cap,
                working_pool: None,
                knowledge_cap: 1200,
            };
            let allocated = policy
                .allocate_sources(&blocks, capacity, &[cap - 100, 1_000, 50])
                .await
                .unwrap();
            assert_eq!(allocated[0].max_tokens, cap - 100);
            assert_eq!(allocated[1].max_tokens, 100);
            assert_eq!(allocated[2].max_tokens, available - cap);
            assert!(
                policy
                    .allocate_sources(&blocks, capacity, &[cap + 1, 0, 0])
                    .await
                    .is_err()
            );
        }
    }
    #[tokio::test]
    async fn retrieval_borrows_existing_working_pool_without_growing_total() {
        let capacity = fw::ContextBudget {
            max_tokens: 5000,
            max_messages: 100,
        };
        let blocks = vec![
            fw::ContextBlockConfig::new(fw::INSTRUCTIONS_BLOCK, fw::BlockBudget::tokens(1000)),
            fw::ContextBlockConfig::new("application_context", fw::BlockBudget::tokens(1000)),
            fw::ContextBlockConfig::new(
                "current_session_working_state",
                fw::BlockBudget::tokens(1000),
            ),
            fw::ContextBlockConfig::new("knowledge_base", fw::BlockBudget::tokens(500)),
            fw::ContextBlockConfig::new(fw::HISTORY_BLOCK, fw::BlockBudget::tokens(5000)),
        ];
        let policy = SharedSeedBudget {
            max_tokens: 1000,
            working_pool: Some(1000),
            knowledge_cap: 1200,
        };
        for retrieved in [0, 300, 900] {
            let result = policy
                .allocate_sources(&blocks, capacity, &[1000, 0, 1000, retrieved, 5000])
                .await
                .unwrap();
            assert_eq!(result[2].max_tokens + result[3].max_tokens, 1000);
            assert_eq!(result[3].max_tokens, retrieved.min(500));
            assert_eq!(result[4].max_tokens, 3000);
            assert_eq!(result.iter().map(|b| b.max_tokens).sum::<usize>(), 5000);
        }
    }
}
