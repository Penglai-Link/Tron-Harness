impl ShortTermMemoryService {
    pub fn record_model_response(
        &self,
        context: &AgentRunContext,
        response: &ModelResponse,
    ) -> Result<(), ShortTermMemoryError> {
        self.record_model_response_with_facts(
            context,
            response,
            ModelResponseTranscriptFacts { usage_audit: None },
        )
    }

    pub fn record_model_response_with_facts(
        &self,
        context: &AgentRunContext,
        response: &ModelResponse,
        facts: ModelResponseTranscriptFacts,
    ) -> Result<(), ShortTermMemoryError> {
        let Some(memory) = context.short_term_memory.as_ref() else {
            return Ok(());
        };
        let parts = response
            .assistant
            .parts
            .iter()
            .enumerate()
            .filter_map(|(index, part)| {
                transcript_part_from_agent_part(
                    &memory.session_code,
                    &context.workspace_code,
                    (index + 1) as i64,
                    part,
                    false,
                )
            })
            .collect::<Vec<_>>();
        if parts.is_empty() {
            return self.append_event(
                &memory.session_code,
                &context.workspace_code,
                Some(&context.run_code),
                Some(&memory.input_code),
                None,
                None,
                "agent.model.responded",
                "MODEL",
                json!({
                    "responseKind": match response.finish {
                        crate::domain::agent::ModelFinish::Final => "final_output",
                        crate::domain::agent::ModelFinish::ToolCalls => "tool_calls",
                    },
                    "tokenUsage": response.token_usage(),
                    "usageAudit": facts.usage_audit,
                    "messageCode": null,
                }),
                None,
            );
        }
        let (message_code, mut message, parts) = self.build_transcript_message(
            &memory.session_code,
            &context.workspace_code,
            Some(&context.run_code),
            "assistant",
            "normal",
            Some(context),
            parts,
            response.token_usage(),
        )?;
        if let Some(usage_audit) = facts.usage_audit.clone() {
            message.metadata_json = Some(json!({
                "usageAudit": usage_audit,
            }));
        }
        let responded_event = self.new_event(
            &memory.session_code,
            &context.workspace_code,
            Some(&context.run_code),
            Some(&memory.input_code),
            Some(&message_code),
            None,
            "agent.model.responded",
            "MODEL",
            json!({
                "responseKind": match response.finish {
                    crate::domain::agent::ModelFinish::Final => "final_output",
                    crate::domain::agent::ModelFinish::ToolCalls => "tool_calls",
                },
                "tokenUsage": response.token_usage(),
                "usageAudit": facts.usage_audit,
                "messageCode": message_code.clone(),
            }),
            None,
        )?;
        let event = match response.finish {
            crate::domain::agent::ModelFinish::Final => self.new_event(
                &memory.session_code,
                &context.workspace_code,
                Some(&context.run_code),
                Some(&memory.input_code),
                Some(&message_code),
                None,
                "agent.message.assistant_completed",
                "MODEL",
                json!({"runCode": context.run_code, "messageCode": message_code.clone()}),
                None,
            )?,
            crate::domain::agent::ModelFinish::ToolCalls => self.new_event(
                &memory.session_code,
                &context.workspace_code,
                Some(&context.run_code),
                Some(&memory.input_code),
                Some(&message_code),
                None,
                "agent.message.assistant_tool_calls_created",
                "MODEL",
                json!({"count": response.tool_calls().len(), "messageCode": message_code.clone()}),
                None,
            )?,
        };
        let events = [responded_event, event]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        self.repository()
            .create_message_with_events(&message, &parts, &events)?;
        Ok(())
    }

    pub(super) fn build_transcript_message(
        &self,
        session_code: &str,
        workspace_code: &str,
        run_code: Option<&str>,
        role: &str,
        message_type: &str,
        context: Option<&AgentRunContext>,
        mut parts: Vec<ShortTermMemoryPart>,
        token_usage: Option<ModelTokenUsage>,
    ) -> Result<(String, ShortTermMemoryMessage, Vec<ShortTermMemoryPart>), ShortTermMemoryError>
    {
        let message_code = self.next_code("ATM")?;
        let now = current_timestamp();
        for part in &mut parts {
            part.id = 0;
            if part.part_code.is_empty() {
                part.part_code = self.next_code("ATP")?;
            }
            part.message_code = message_code.clone();
        }
        let message = ShortTermMemoryMessage {
            id: 0,
            message_code: message_code.clone(),
            session_code: session_code.to_string(),
            workspace_code: workspace_code.to_string(),
            run_code: run_code.map(str::to_string),
            seq: 0,
            role: role.into(),
            message_type: message_type.into(),
            status: "completed".into(),
            agent_code: context
                .and_then(|ctx| ctx.short_term_memory.as_ref())
                .and_then(|memory| memory.agent_code.clone())
                .or_else(|| context.and_then(|ctx| ctx.execution_scope.app_code.clone())),
            model_code: context.map(|ctx| ctx.model.model_code.clone()),
            model_provider: None,
            model_name: context.map(|ctx| ctx.model.model_name.clone()),
            model_protocol: context.map(|ctx| format!("{:?}", ctx.model.protocol)),
            finish_reason: None,
            error_json: None,
            input_tokens: token_usage.map(|usage| usage.input_tokens),
            output_tokens: token_usage.map(|usage| usage.output_tokens),
            reasoning_tokens: None,
            total_tokens: token_usage.map(|usage| usage.total_tokens),
            metadata_json: None,
            created_at: now.clone(),
            completed_at: Some(now),
        };
        Ok((message_code, message, parts))
    }
}
