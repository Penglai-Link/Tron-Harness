use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{
    domain::{
        agent::AgentRunContext,
        short_term_memory::{ShortTermMemoryError, ShortTermMemoryInput},
    },
    shared::time::current_timestamp,
};

use super::{ShortTermMemoryService, service_parts::part_text};

impl ShortTermMemoryService {
    pub fn record_user_input(
        &self,
        context: &AgentRunContext,
        prompt: &str,
    ) -> Result<String, ShortTermMemoryError> {
        self.record_invocation_input(context, prompt, "USER_PROMPT", None)
    }

    pub fn record_invocation_input(
        &self,
        context: &AgentRunContext,
        prompt: &str,
        input_kind: &str,
        metadata_json: Option<Value>,
    ) -> Result<String, ShortTermMemoryError> {
        let session_code = context
            .conversation_code
            .as_deref()
            .ok_or(ShortTermMemoryError::SessionNotFound)?;
        let workspace_code = self
            .repository()
            .session_workspace(session_code)?
            .ok_or(ShortTermMemoryError::SessionNotFound)?;
        let input_code = self.next_code("ATI")?;
        let now = current_timestamp();
        let input_source = metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("source"))
            .cloned();
        let idempotency_key =
            transcript_input_idempotency_key(context, prompt, input_kind, metadata_json.as_ref());
        if let Some(existing_input_code) = self
            .repository()
            .input_code_by_idempotency_key(session_code, &idempotency_key)?
        {
            return Ok(existing_input_code);
        }
        let input = ShortTermMemoryInput {
            id: 0,
            input_code: input_code.clone(),
            session_code: session_code.to_string(),
            workspace_code: workspace_code.clone(),
            run_code: Some(context.run_code.clone()),
            input_kind: input_kind.into(),
            delivery: "IMMEDIATE".into(),
            status: "CONSUMED".into(),
            priority: 0,
            idempotency_key: Some(idempotency_key.clone()),
            prompt_json: json!({"content": prompt}),
            metadata_json,
            admitted_seq: None,
            promoted_seq: None,
            created_at: now.clone(),
            promoted_at: Some(now.clone()),
            consumed_at: Some(now.clone()),
            cancelled_at: None,
        };
        let mut payload = json!({"inputCode": input_code, "inputKind": input_kind});
        if input_kind == "USER_PROMPT" {
            let (message_code, message, parts) = self.build_transcript_message(
                session_code,
                &workspace_code,
                Some(&context.run_code),
                "user",
                "normal",
                Some(context),
                vec![part_text(
                    session_code,
                    &workspace_code,
                    "",
                    1,
                    "text",
                    prompt,
                    "model",
                )],
                None,
            )?;
            payload["messageCode"] = json!(message_code.clone());
            let input_event = self.new_event(
                session_code,
                &workspace_code,
                Some(&context.run_code),
                Some(&input_code),
                Some(&message_code),
                None,
                "agent.input.consumed",
                "USER",
                payload,
                None,
            )?;
            let message_event = self.new_event(
                session_code,
                &workspace_code,
                Some(&context.run_code),
                Some(&input_code),
                Some(&message_code),
                None,
                "agent.message.user_created",
                "USER",
                json!({
                    "inputCode": input_code,
                    "messageCode": message_code,
                    "role": "user",
                    "messageType": "normal"
                }),
                None,
            )?;
            let events = [input_event, message_event]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            let result = self.repository().create_input_with_message_and_events(
                &input,
                Some(&message),
                &parts,
                &events,
            );
            if let Some(existing_input_code) =
                self.resolve_idempotent_input_write_result(session_code, &idempotency_key, result)?
            {
                return Ok(existing_input_code);
            }
        } else {
            payload["source"] = input_source.unwrap_or_else(|| json!("GOAL_LOOP"));
            payload["modelVisibility"] = json!("runtime_context_block");
            let event = self.new_event(
                session_code,
                &workspace_code,
                Some(&context.run_code),
                Some(&input_code),
                None,
                None,
                "agent.input.consumed",
                "RUNTIME",
                payload,
                None,
            )?;
            let events = event.into_iter().collect::<Vec<_>>();
            let result =
                self.repository()
                    .create_input_with_message_and_events(&input, None, &[], &events);
            if let Some(existing_input_code) =
                self.resolve_idempotent_input_write_result(session_code, &idempotency_key, result)?
            {
                return Ok(existing_input_code);
            }
        };
        Ok(input_code)
    }

    fn resolve_idempotent_input_write_result(
        &self,
        session_code: &str,
        idempotency_key: &str,
        result: Result<(), ShortTermMemoryError>,
    ) -> Result<Option<String>, ShortTermMemoryError> {
        match result {
            Ok(()) => Ok(None),
            Err(error) => {
                if let Some(existing_input_code) = self
                    .repository()
                    .input_code_by_idempotency_key(session_code, idempotency_key)?
                {
                    return Ok(Some(existing_input_code));
                }
                Err(error)
            }
        }
    }
}

fn transcript_input_idempotency_key(
    context: &AgentRunContext,
    prompt: &str,
    input_kind: &str,
    metadata_json: Option<&Value>,
) -> String {
    let normalized_kind = input_kind.trim().to_ascii_uppercase();
    if normalized_kind == "CONTINUATION" {
        let source = metadata_string(metadata_json, "source").unwrap_or("RUNTIME");
        let goal_code = metadata_string(metadata_json, "goalCode")
            .or_else(|| metadata_string(metadata_json, "goal_code"))
            .unwrap_or("NO_GOAL");
        let request_or_run = input_request_or_run_key(context);
        return format!(
            "stm:v1:continuation:{source}:{goal_code}:{request_or_run}:{}",
            hash_text(prompt)
        );
    }
    let request_or_run = input_request_or_run_key(context);
    format!(
        "stm:v1:{}:{request_or_run}",
        normalized_kind.to_ascii_lowercase()
    )
}

fn input_request_or_run_key(context: &AgentRunContext) -> String {
    context
        .request_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("request:{value}:run:{}", context.run_code))
        .unwrap_or_else(|| format!("run:{}", context.run_code))
}

fn metadata_string<'a>(metadata_json: Option<&'a Value>, key: &str) -> Option<&'a str> {
    metadata_json
        .and_then(|metadata| metadata.get(key))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn hash_text(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("sha256:{}", hex::encode(hasher.finalize()))
}
