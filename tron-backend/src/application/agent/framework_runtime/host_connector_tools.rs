//! Host authorization for portable Framework connector tools.
use std::sync::{Arc, Weak};

use async_trait::async_trait;
use eliteforge_agent_connector::{
    ConnectorCapability, ConnectorError, ConnectorToolAuthority, ConnectorTools, Principal,
    connector_tool_catalog,
};
use eliteforge_agent_core as fw;

use super::super::model::HostExecution;
use super::Host;

pub(super) struct HostConnectorTools(pub Weak<Host>);

#[async_trait]
impl fw::ToolProvider for HostConnectorTools {
    fn catalog_metadata(&self) -> Option<Vec<fw::ToolCatalogEntry>> {
        let mut entries = connector_tool_catalog();
        for entry in &mut entries {
            entry.category = Some(crate::application::agent::tool_catalog::builtin_category("tron.connector.tools.platform"));
        }
        entries.push(fw::ToolCatalogEntry {
            descriptor: crate::application::topic::topic_read_tool_descriptor(),
            domain_code: Some("topic".into()),
            category: Some(crate::application::agent::tool_catalog::builtin_category(
                "tron.connector.tools.topic",
            )),
        });
        Some(entries)
    }

    async fn lease(&self, run: &fw::RunContext) -> Result<fw::ToolLease, fw::AgentError> {
        if run
            .extensions
            .get::<super::super::external::ExternalExecution>()
            .is_some()
        {
            return Ok(fw::ToolLease::new(vec![]));
        }
        let host = self
            .0
            .upgrade()
            .ok_or_else(|| fw::AgentError::ToolSetup("host closed".into()))?;
        let mut lease = if let Some(integrations) = host.integrations.get() {
            ConnectorTools::new(
                integrations.connector_service(),
                Arc::new(Authority(self.0.clone())),
            )
            .lease(run)
            .await?
        } else {
            fw::ToolLease::new(vec![])
        };
        for binding in &mut lease.tools {
            binding.category = Some(crate::application::agent::tool_catalog::builtin_category(
                "tron.connector.tools.platform",
            ));
            binding.source = fw::ToolSource {
                provider_id: Some("ConnectorTools".into()),
                plugin_id: Some("tron.connector.tools".into()),
                pack_id: Some("framework.connector".into()),
                adapter_id: Some("tron.connector.tools".into()),
            };
        }
        let topic_available = if let Ok(execution) = HostExecution::from_extensions(&run.extensions)
        {
            let state = execution.state.lock().await;
            state.context.execution_scope.project_code.is_some()
                && state.context.user_code.is_some()
        } else {
            false
        };
        if host.topics.get().is_some() && topic_available {
            let mut binding = fw::ToolBinding::new(
                crate::application::topic::topic_read_tool_descriptor(),
                TopicRead(self.0.clone()),
            )
            .with_domain_code("topic");
            binding.category = Some(crate::application::agent::tool_catalog::builtin_category(
                "tron.connector.tools.topic",
            ));
            binding.source = fw::ToolSource {
                provider_id: Some("TopicTools".into()),
                plugin_id: Some("tron.connector.tools".into()),
                pack_id: None,
                adapter_id: Some("tron.connector.tools".into()),
            };
            lease.tools.push(binding);
        }
        Ok(lease)
    }
}

struct Authority(Weak<Host>);

#[async_trait]
impl ConnectorToolAuthority for Authority {
    async fn expose(
        &self,
        run: &fw::RunContext,
        _capability: ConnectorCapability,
    ) -> Result<bool, fw::AgentError> {
        let Ok(execution) = HostExecution::from_extensions(&run.extensions) else {
            return Ok(false);
        };
        let state = execution.state.lock().await;
        Ok(!state.context.execution_workspace_code().is_empty()
            && state
                .context
                .user_code
                .as_deref()
                .is_some_and(|user| !user.is_empty()))
    }

    async fn authorize(
        &self,
        run: &fw::RunContext,
        capability: ConnectorCapability,
        _operation: &str,
        connection_id: &str,
        _repository_id: Option<&str>,
    ) -> Result<Principal, ConnectorError> {
        let host = self.0.upgrade().ok_or(ConnectorError::Unavailable)?;
        let execution = HostExecution::from_extensions(&run.extensions)
            .map_err(|_| ConnectorError::Forbidden)?;
        let (user, workspace) = {
            let state = execution.state.lock().await;
            let context = &state.context;
            let user = context
                .user_code
                .as_deref()
                .filter(|value| !value.is_empty())
                .ok_or(ConnectorError::Forbidden)?;
            if !context.is_tool_authorized(capability.call_name()) {
                return Err(ConnectorError::Forbidden);
            }
            (user.to_owned(), context.execution_workspace_code().to_owned())
        };
        if !host
            .integrations
            .get()
            .is_some_and(|service| service.contains(&workspace, &user, connection_id))
        {
            return Err(ConnectorError::Forbidden);
        }
        Ok(Principal {
            id: user,
            scope: workspace,
        })
    }
}

struct TopicRead(Weak<Host>);

#[async_trait]
impl fw::Tool for TopicRead {
    async fn call(
        &self,
        arguments: serde_json::Value,
        run: &fw::RunContext,
    ) -> Result<fw::ToolOutput, fw::ToolError> {
        let fail = |message: &str| fw::ToolError::Failed(message.into());
        run.check_cancelled()
            .map_err(|error| fw::ToolError::Failed(error.to_string()))?;
        let host = self.0.upgrade().ok_or_else(|| fail("host closed"))?;
        let service = host
            .topics
            .get()
            .ok_or_else(|| fail("topic service unavailable"))?
            .clone();
        let execution =
            HostExecution::from_extensions(&run.extensions).map_err(|_| fail("missing caller"))?;
        let (user, project) = {
            let state = execution.state.lock().await;
            if !state.context.is_tool_authorized("topic_read") {
                return Err(fail("access denied"));
            }
            (
                state
                    .context
                    .user_code
                    .clone()
                    .ok_or_else(|| fail("missing caller"))?,
                state
                    .context
                    .execution_scope
                    .project_code
                    .clone()
                    .ok_or_else(|| fail("no current topic"))?,
            )
        };
        let memo = service
            .get(&user, &project)
            .map_err(|_| fail("topic unavailable"))?;
        if arguments
            .get("revision")
            .and_then(serde_json::Value::as_u64)
            .is_some_and(|revision| revision != memo.revision)
        {
            return Err(fail("topic changed; restart from offset 0"));
        }
        let offset = arguments
            .get("offset")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            .min(usize::MAX as u64) as usize;
        let limit = arguments
            .get("limit")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(20)
            .clamp(1, 50) as usize;
        // Count the encoded response, including JSON escaping and envelope,
        // rather than the raw notes. A page must always advance its cursor.
        const MAX_OUTPUT_BYTES: usize = 512 * 1024;
        let notes = (offset == 0).then_some(memo.notes.as_str());
        let mut references = Vec::new();
        for reference in memo.references.iter().skip(offset).take(limit) {
            let value =
                serde_json::to_value(reference).map_err(|_| fail("invalid topic reference"))?;
            references.push(value);
            let next = offset.saturating_add(references.len());
            let candidate = serde_json::json!({
                "projectCode": &project, "revision": memo.revision,
                "notes": notes, "references": &references,
                "nextOffset": (next < memo.references.len()).then_some(next),
            }).to_string();
            if candidate.len() > MAX_OUTPUT_BYTES {
                references.pop();
                if references.is_empty() {
                    return Err(fail("topic reference exceeds the output limit"));
                }
                break;
            }
        }
        let next = offset.saturating_add(references.len());
        let output = serde_json::json!({
            "projectCode": project, "revision": memo.revision,
            "notes": notes,
            "references": references, "nextOffset": (next < memo.references.len()).then_some(next),
        }).to_string();
        if output.len() > MAX_OUTPUT_BYTES {
            return Err(fail("topic notes exceed the output limit"));
        }
        Ok(fw::ToolOutput::text(output))
    }
}
