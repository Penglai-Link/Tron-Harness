//! Compare admitted inventories at new-run/resume boundaries, never mid-call.
use super::{ShortTermMemoryService, native_service::error};
use crate::domain::{agent::AgentRunContext, short_term_memory::ShortTermMemoryError};
use eliteforge_agent_core as fw;
use serde_json::{Value, json};
use std::collections::BTreeSet;

fn update(previous: Option<&Value>, current: &Value) -> Option<Value> {
    if previous == Some(current) {
        return None;
    }
    let names = |value: &Value| {
        value["tools"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|tool| tool.as_str().map(str::to_owned))
            .collect::<BTreeSet<_>>()
    };
    let before = previous.map(names).unwrap_or_default();
    let after = names(current);
    Some(
        json!({"type":"TOOL_CATALOG_UPDATED","previousKnown":previous.is_some(),
        "added":after.difference(&before).collect::<Vec<_>>(),"removed":before.difference(&after).collect::<Vec<_>>(),
        "revision":current["revision"],"effectiveBoundary":"new_run_or_resume",
        "message":"Tool permissions or schemas changed. Use tool_search to rediscover current schemas. Earlier turns and tool results do not grant current authority."}),
    )
}
impl ShortTermMemoryService {
    pub(crate) async fn admitted_tool_catalog_update(
        &self,
        context: &AgentRunContext,
        bindings: &[fw::ToolBinding],
    ) -> Result<Option<Value>, ShortTermMemoryError> {
        let Some(session) = context.conversation_code.as_deref() else {
            return Ok(None);
        };
        let mut descriptors = bindings
            .iter()
            .filter(|b| b.exposed && !fw::is_gateway_descriptor(&b.descriptor))
            .map(|binding| &binding.descriptor)
            .collect::<Vec<_>>();
        descriptors.sort_by(|a, b| a.call_name.cmp(&b.call_name));
        use sha2::Digest;
        let revision = format!(
            "{:x}",
            sha2::Sha256::digest(
                serde_json::to_vec(&descriptors)
                    .map_err(|_| ShortTermMemoryError::StorageFailure)?
            )
        );
        let current = json!({"agentCode":context.agent_code,"userCode":context.user_code,"tools":descriptors.iter().map(|d| &d.call_name).collect::<Vec<_>>(),"revision":revision});
        let framework = self.framework().await?;
        let events = framework
            .events()
            .map_err(|_| ShortTermMemoryError::StorageFailure)?;
        let previous = events
            .latest_by_kind(session, "tron.tools.authorized")
            .await
            .map_err(error)?;
        let previous = previous
            .as_ref()
            .map(|event| &event.payload)
            .filter(|payload| {
                payload["agentCode"] == current["agentCode"]
                    && payload["userCode"] == current["userCode"]
            });
        let Some(notice) = update(previous, &current) else {
            return Ok(None);
        };
        // The first inventory already has fresh discovery instructions. Persist
        // its baseline without duplicating the entire tool list in the prompt.
        let changed = previous.is_some();
        let mut event = fw::SessionEvent::new(session, "tron.tools.authorized", current);
        event.run_id = fw::RunId::parse_str(&context.run_code).ok();
        framework
            .sessions()
            .map_err(|_| ShortTermMemoryError::StorageFailure)?
            .store
            .append_event(event)
            .await
            .map_err(error)?;
        Ok(changed.then_some(notice))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_changes_report_additions_removals_and_schema_revisions() {
        let old = json!({"tools":["memory_search","dude_task_page"],"revision":"old"});
        let new = json!({"tools":["dude_memory_search","dude_task_page"],"revision":"new"});
        let notice = update(Some(&old), &new).unwrap();
        assert_eq!(notice["added"], json!(["dude_memory_search"]));
        assert_eq!(notice["removed"], json!(["memory_search"]));
        assert!(update(Some(&new), &new).is_none());
        let mut changed = new.clone();
        changed["revision"] = json!("schema-changed");
        let notice = update(Some(&new), &changed).unwrap();
        assert_eq!(notice["added"], json!([]));
        assert_eq!(notice["effectiveBoundary"], "new_run_or_resume");
    }
    struct NeverExecuted;
    #[async_trait::async_trait]
    impl fw::Tool for NeverExecuted {
        async fn call(
            &self,
            _: Value,
            _: &fw::RunContext,
        ) -> Result<fw::ToolOutput, fw::ToolError> {
            panic!("catalog admission must not execute tools")
        }
    }
    #[tokio::test]
    async fn catalog_revision_survives_service_recreation_and_reports_changes_once() {
        use crate::application::agent::FrameworkHost;
        use crate::application::short_term_memory::ShortTermMemoryConfig;
        let host = FrameworkHost::temporary(None).unwrap();
        let framework = host.storage_framework().await.unwrap();
        let mut session = fw::SessionRecord::new("");
        session.id = "catalog-session".into();
        framework.sessions().unwrap().create(session).await.unwrap();
        let context: AgentRunContext = serde_json::from_value(json!({
            "agentCode":"main-dude", "runCode":fw::RunId::new_v4().to_string(),
            "conversationCode":"catalog-session", "workspaceCode":"", "userCode":"user",
            "locale":"en-US", "agentKind":"REACT",
            "model":{"modelCode":"local", "protocol":"CHAT_COMPLETIONS", "baseUrl":"http://127.0.0.1:1", "apiKey":"unused", "modelName":"local"},
            "variables":[], "skills":[], "tools":[], "executionScope":{}, "limits":{"maxSteps":12,"maxToolCalls":12}, "messages":[], "observations":[], "startedAt":"unused"
        })).unwrap();
        let binding = |name: &str| {
            fw::ToolBinding::new(
                fw::ToolDescriptor::new(name, name, "authorized tool", json!({"type":"object"})),
                NeverExecuted,
            )
        };
        let old = [binding("dude_task_page")];
        let service =
            ShortTermMemoryService::with_host(host.clone(), ShortTermMemoryConfig::default());
        assert!(service.admitted_tool_catalog_update(&context, &old).await.unwrap().is_none());
        let service = ShortTermMemoryService::with_host(host, ShortTermMemoryConfig::default());
        assert!(
            service
                .admitted_tool_catalog_update(&context, &old)
                .await
                .unwrap()
                .is_none()
        );
        let next = [
            binding("dude_task_get"),
            binding("internal_hidden").hidden(),
        ];
        let notice = service
            .admitted_tool_catalog_update(&context, &next)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(notice["added"], json!(["dude_task_get"]));
        assert_eq!(notice["removed"], json!(["dude_task_page"]));
        assert!(
            service
                .admitted_tool_catalog_update(&context, &next)
                .await
                .unwrap()
                .is_none()
        );
        let event = framework
            .events()
            .unwrap()
            .latest_by_kind("catalog-session", "tron.tools.authorized")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(event.payload["tools"], json!(["dude_task_get"]));
        framework.close().await;
    }
}
