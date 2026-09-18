use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{
    domain::{
        agent::{AgentMessage, AgentRunContext},
        short_term_memory::{ShortTermMemoryContextEpoch, ShortTermMemoryError},
    },
    shared::time::current_timestamp,
};

use super::{
    ModelContextBudgetPlan, ModelRequestTranscriptFacts, ShortTermMemoryService,
    service_parts::estimate_text_tokens,
};

impl ShortTermMemoryService {
    pub(super) fn record_context_epoch(
        &self,
        context: &AgentRunContext,
        prepared: &super::PreparedModelContext,
        budget_plan: &ModelContextBudgetPlan,
        assembled_message_count: usize,
        estimated_tokens: u32,
        rebased: bool,
        over_budget: bool,
    ) -> Result<(), ShortTermMemoryError> {
        let Some(memory) = context.short_term_memory.as_ref() else {
            return Ok(());
        };
        let now = current_timestamp();
        let prompt_prefix_hash = hash_json(&prompt_prefix_snapshot(&memory.prompt_prefix));
        let exposed_tools_snapshot = tools_snapshot(&context.tools);
        let effective_tools_snapshot = tools_snapshot(context.authorized_tools());
        let tool_hash = hash_json(&exposed_tools_snapshot);
        let skill_hash = hash_json(&skills_snapshot(context));
        let variable_hash = hash_json(&variables_snapshot(context));
        let execution_scope_hash = hash_json(&json!(context.execution_scope));
        let memory_policy = json!({
            "todoEnabled": memory.todo_enabled,
            "inputKind": memory.input_kind,
            "context": self.config.context,
            "familyLoad": self.config.family_load,
            "compaction": self.config.compaction,
            "goalLoop": self.config.goal_loop,
        });
        let family_context = prepared.messages.iter().flat_map(|message| message.parts.iter()).filter_map(|part| {
            match part {
                crate::domain::agent::AgentMessagePart::Context { kind, text, .. } if kind == "session_family_context" => Some(text.clone()),
                _ => None,
            }
        }).collect::<Vec<_>>().join("\n\n");
        let memory_hash = hash_json(&memory_policy);
        let epoch = ShortTermMemoryContextEpoch {
            id: 0,
            epoch_code: self.next_code("ATCE")?,
            session_code: memory.session_code.clone(),
            workspace_code: context.workspace_code.clone(),
            baseline_seq: 0,
            replacement_seq: 0,
            agent_code: memory.agent_code.clone(),
            prompt_prefix_message_count: memory.prompt_prefix.len() as u32,
            prompt_prefix_hash: Some(prompt_prefix_hash.clone()),
            prompt_prefix_preview: prompt_prefix_preview(&memory.prompt_prefix),
            snapshot_json: json!({
                "promptPrefix": {
                    "messageCount": memory.prompt_prefix.len(),
                    "hash": prompt_prefix_hash,
                    "preview": prompt_prefix_preview(&memory.prompt_prefix),
                    "messages": prompt_prefix_messages(&memory.prompt_prefix),
                },
                "familyContext": family_context,
                // Keep `tools` as the backward-compatible model-exposure snapshot.
                "tools": exposed_tools_snapshot.clone(),
                "exposedTools": exposed_tools_snapshot,
                "effectiveTools": effective_tools_snapshot,
                "skills": skills_snapshot(context),
                "variables": variables_snapshot(context),
                "executionScope": context.execution_scope,
                "memory": {
                    "policyHash": memory_hash,
                    "todoEnabled": memory.todo_enabled,
                    "inputKind": memory.input_kind,
                    "budgetPlan": budget_plan,
                    "assembledMessageCount": assembled_message_count,
                    "estimatedTokens": estimated_tokens,
                    "rebased": rebased,
                    "overBudget": over_budget,
                }
            }),
            system_hash: role_hash(&memory.prompt_prefix, "system"),
            developer_hash: role_hash(&memory.prompt_prefix, "developer"),
            tool_hash: Some(tool_hash),
            skill_hash: Some(skill_hash),
            variable_hash: Some(variable_hash),
            execution_scope_hash: Some(execution_scope_hash),
            memory_hash: Some(memory_hash),
            revision: 0,
            created_at: now.clone(),
            updated_at: now,
        };
        let event = self.new_event(
            &memory.session_code,
            &context.workspace_code,
            Some(&context.run_code),
            Some(&memory.input_code),
            None,
            None,
            "agent.context.baseline_created",
            "RUNTIME",
            json!({
                "epochCode": epoch.epoch_code,
                "baselineSeq": epoch.baseline_seq,
                "replacementSeq": epoch.replacement_seq,
                "revision": epoch.revision,
                "promptPrefixHash": epoch.prompt_prefix_hash,
                "toolHash": epoch.tool_hash,
                "skillHash": epoch.skill_hash,
                "variableHash": epoch.variable_hash,
                "executionScopeHash": epoch.execution_scope_hash,
                "memoryHash": epoch.memory_hash,
            }),
            None,
        )?;
        let mut repository = self.repository();
        if let Some(event) = event {
            repository
                .create_context_epoch_with_event(&epoch, &event)
                .map(|_| ())
        } else {
            repository.create_context_epoch(&epoch)
        }
    }

    pub fn record_model_requested(
        &self,
        context: &AgentRunContext,
        facts: ModelRequestTranscriptFacts,
    ) -> Result<(), ShortTermMemoryError> {
        let Some(memory) = context.short_term_memory.as_ref() else {
            return Ok(());
        };
        self.append_event(
            &memory.session_code,
            &context.workspace_code,
            Some(&context.run_code),
            Some(&memory.input_code),
            None,
            None,
            "agent.model.requested",
            "RUNTIME",
            json!({
                "messageHashes": facts.message_hashes,
                "toolSchemaTokenEstimate": facts.tool_schema_token_estimate,
                "protocol": facts.protocol,
                "reasoningEffort": context.model.reasoning_effort,
                "previousResponseIdPresent": facts.previous_response_id_present,
                "contextWindowEstimate": facts.context_window_estimate,
                "checkpointCode": facts.checkpoint_code,
                "estimatedTokens": facts.estimated_tokens,
                "usageEstimate": facts.usage_estimate,
                "budgetBreakdown": facts.budget_breakdown,
                "overBudgetReason": facts.over_budget_reason,
            }),
            None,
        )
    }
}

fn prompt_prefix_snapshot(messages: &[AgentMessage]) -> Value {
    json!(
        messages
            .iter()
            .map(|message| json!({
                "role": message.role,
                "textHash": hash_text(&message.text_content()),
                "tokenEstimate": estimate_text_tokens(&message.text_content()),
            }))
            .collect::<Vec<_>>()
    )
}

fn prompt_prefix_preview(messages: &[AgentMessage]) -> Option<String> {
    let text = messages
        .iter()
        .take(4)
        .map(|message| format!("{}: {}", message.role, message.text_content()))
        .collect::<Vec<_>>()
        .join("\n");
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        Some(truncate_chars(text, 512))
    }
}

fn prompt_prefix_messages(messages: &[AgentMessage]) -> Vec<Value> {
    messages
        .iter()
        .map(|message| {
            json!({
                "role": message.role,
                "content": message.text_content(),
            })
        })
        .collect()
}

fn role_hash(messages: &[AgentMessage], role: &str) -> Option<String> {
    let values = messages
        .iter()
        .filter(|message| message.role == role)
        .map(|message| message.text_content())
        .collect::<Vec<_>>();
    (!values.is_empty()).then(|| hash_text(&values.join("\n")))
}

fn tools_snapshot(tools: &[crate::domain::agent::EffectiveTool]) -> Value {
    json!(
        tools
            .iter()
            .map(|tool| json!({
                "name": tool.tool_name,
                "domainCode": tool.domain_code,
                "skillName": tool.skill_name,
                "schemaHash": hash_json(&tool.input_schema),
                "requiredVariableKeys": tool.required_variable_keys,
            }))
            .collect::<Vec<_>>()
    )
}

fn skills_snapshot(context: &AgentRunContext) -> Value {
    json!(
        context
            .skills
            .iter()
            .map(|skill| json!({
                "name": skill.skill_name,
                "instructionHash": skill.instruction.as_ref().map(|value| hash_text(value)),
                "toolNames": skill.tool_names,
            }))
            .collect::<Vec<_>>()
    )
}

fn variables_snapshot(context: &AgentRunContext) -> Value {
    json!(
        context
            .variables
            .iter()
            .map(|variable| json!({
                "key": variable.key,
                "source": variable.source,
                "sensitive": variable.sensitive,
                "valueHash": hash_text(&variable.value),
            }))
            .collect::<Vec<_>>()
    )
}

fn hash_json(value: &Value) -> String {
    hash_text(&serde_json::to_string(value).unwrap_or_default())
}

fn hash_text(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("sha256:{}", hex::encode(hasher.finalize()))
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut text = value
        .chars()
        .take(max_chars.saturating_sub(24))
        .collect::<String>();
    text.push_str("\n...[preview truncated]");
    text
}
