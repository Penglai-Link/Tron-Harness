use eliteforge_agent_core as fw;
use serde::Serialize;

/// Only reductions confirmed by the assembler belong in the completion notice.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompactionBlock {
    pub id: String,
    pub before_tokens: usize,
    pub after_tokens: usize,
    pub kind: &'static str,
}

pub(crate) fn compaction_blocks(report: &fw::ContextAssemblyReport) -> Vec<CompactionBlock> {
    report.blocks.iter().filter_map(|block| {
        if !matches!(block.outcome, fw::ContextBlockOutcome::Reduced) {
            return None;
        }
        let before = block.original_tokens?;
        let kind = if block.id == "conversation_history_summary" && block.final_tokens > 0 {
            "summary"
        } else if block.final_tokens < before {
            "compressed"
        } else {
            return None;
        };
        Some(CompactionBlock {
            id: block.id.clone(), before_tokens: before,
            after_tokens: block.final_tokens, kind,
        })
    }).collect()
}
