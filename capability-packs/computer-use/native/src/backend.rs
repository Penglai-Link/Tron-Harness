use crate::desktop::DesktopOwner;
use async_trait::async_trait;
use cua_driver_sdk::{CuaDriver, DriverHostOptions};
use eliteforge_agent_capability_runtime::*;
use eliteforge_agent_core as fw;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, OnceLock},
};

const TOOLS: &str = "tron.computer-use.tools";
const SKILLS: &str = "tron.computer-use.skills";
const PREFIX: &str = "tron.computer-use.";
const GUIDANCE: &str = "Use this native computer_* tool directly; no global cua-driver or MCP server is needed. Load the cua-driver skill for guidance. After an uncertain action, observe before retrying.";

pub fn inventory() -> Value {
    CuaDriver::inspect_host_tools(DriverHostOptions {
        cursor: Default::default(),
        host_owns_permission_ux: true,
        host_bundle_id: None,
        claude_code_compatibility: false,
        prepare_desktop_environment: false,
        register_host_tools: None,
        authorization_host: None,
        activity_observer: None,
    })
}

pub fn tool_catalog() -> Result<Vec<fw::ToolCatalogEntry>, RpcFault> {
    inventory()["tools"]
        .as_array()
        .ok_or_else(|| RpcFault::new("invalid SDK tool inventory"))?
        .iter()
        .map(|tool| {
            let name = tool["name"]
                .as_str()
                .ok_or_else(|| RpcFault::new("missing SDK tool name"))?;
            Ok(fw::ToolCatalogEntry {
                descriptor: fw::ToolDescriptor::new(
                    format!("{PREFIX}{name}"),
                    format!("computer_{name}"),
                    format!(
                        "{GUIDANCE}\n{}",
                        tool["description"].as_str().unwrap_or_default()
                    ),
                    tool["inputSchema"].clone(),
                )
                .with_operation_kind(match tool["annotations"]["readOnlyHint"].as_bool() {
                    Some(true) => fw::ToolOperationKind::Query,
                    Some(false) => fw::ToolOperationKind::Command,
                    None => fw::ToolOperationKind::Unknown,
                })
                .with_display_resource(include_str!("../tool-display.json")),
                domain_code: Some("tron.computer-use".into()),
                category: Some(fw::ToolCategory::localized(
                    "tron.computer-use",
                    "电脑操作",
                    "Computer Use",
                )),
            })
        })
        .collect()
}

type Leases = Arc<Mutex<BTreeMap<String, Value>>>;
type Runtime = Arc<tokio::sync::Mutex<Option<DesktopRuntime>>>;
struct DesktopRuntime {
    run: Value,
    _owner: DesktopOwner,
    driver: Arc<CuaDriver>,
}
fn run_identity(params: &Value) -> Value {
    json!({"run_id":params["run"]["run_id"], "execution_id":params["run"]["execution_id"], "session_id":params["run"]["session_id"]})
}
struct RunLease {
    id: OnceLock<String>,
    leases: Leases,
    run: Value,
    runtime: Runtime,
}
#[async_trait]
impl BackendResource for RunLease {
    async fn call(&self, _: &str, _: Value, _: BackendCallContext) -> Result<Value, RpcFault> {
        Err(RpcFault::new("this resource only owns a tool lease"))
    }
    async fn close(&self) -> Result<(), RpcFault> {
        if let Some(id) = self.id.get() {
            self.leases.lock().unwrap().remove(id);
        }
        let mut runtime = self.runtime.lock().await;
        if runtime
            .as_ref()
            .is_some_and(|runtime| runtime.run == self.run)
        {
            let desktop = runtime.take().unwrap();
            desktop
                .driver
                .shutdown()
                .await
                .map_err(|error| RpcFault::new(error.to_string()))?;
        }
        Ok(())
    }
}

#[derive(Default)]
struct ComputerUseBackend {
    runtime: Runtime,
    leases: Leases,
}

#[async_trait]
impl CapabilityBackend for ComputerUseBackend {
    async fn initialize(
        &self,
        _: InstanceIdentity,
        _: Value,
        _: Vec<String>,
    ) -> Result<(), RpcFault> {
        // Registration/discovery acquires neither desktop ownership nor TCC.
        Ok(())
    }
    async fn shutdown(&self) -> Result<(), RpcFault> {
        if let Some(desktop) = self.runtime.lock().await.take() {
            desktop
                .driver
                .shutdown()
                .await
                .map_err(|error| RpcFault::new(error.to_string()))?;
        }
        self.leases.lock().unwrap().clear();
        Ok(())
    }
    async fn call(
        &self,
        implementation: &str,
        operation: &str,
        params: Value,
        context: BackendCallContext,
    ) -> Result<Value, RpcFault> {
        if implementation == SKILLS {
            return crate::skills::call(operation, &params);
        }
        if implementation != TOOLS {
            return Err(RpcFault::new("unknown native implementation"));
        }
        match operation {
            "lease" => {
                if params["run"].is_null() {
                    return Err(RpcFault::new("missing tool run identity"));
                }
                let resource = Arc::new(RunLease {
                    id: OnceLock::new(),
                    leases: self.leases.clone(),
                    run: run_identity(&params),
                    runtime: self.runtime.clone(),
                });
                let handle = context.register_resource(resource.clone())?;
                resource
                    .id
                    .set(handle.id.clone())
                    .map_err(|_| RpcFault::new("duplicate tool lease"))?;
                self.leases
                    .lock()
                    .unwrap()
                    .insert(handle.id.clone(), run_identity(&params));
                let tools = tool_catalog()?
                    .into_iter()
                    .map(|entry| {
                        json!({
                            "descriptor":entry.descriptor,"domain_code":entry.domain_code,
                            "category":entry.category,"exposed":true,"parallel_safe":false
                        })
                    })
                    .collect::<Vec<_>>();
                Ok(json!({"handle":handle,"tools":tools}))
            }
            "call" => {
                let lease = params["lease"]["id"]
                    .as_str()
                    .ok_or_else(|| RpcFault::new("missing tool lease"))?;
                if params["lease"]["implementation"] != TOOLS
                    || self.leases.lock().unwrap().get(lease) != Some(&run_identity(&params))
                {
                    return Err(RpcFault::new(
                        "tool lease is closed or belongs to another run",
                    ));
                }
                let id = params["tool"]
                    .as_str()
                    .ok_or_else(|| RpcFault::new("missing tool identity"))?;
                if !tool_catalog()?
                    .iter()
                    .any(|entry| entry.descriptor.id == id)
                {
                    return Err(RpcFault::new("unknown native desktop tool"));
                }
                let mut runtime = self.runtime.lock().await;
                if runtime
                    .as_ref()
                    .is_some_and(|runtime| runtime.run != run_identity(&params))
                {
                    return Err(RpcFault::new(
                        "desktop_busy: another Agent run owns this desktop",
                    ));
                }
                if runtime.is_none() {
                    let owner =
                        DesktopOwner::claim().map_err(|error| RpcFault::new(error.to_string()))?;
                    let driver = CuaDriver::create(None)
                        .map_err(|error| RpcFault::new(error.to_string()))?;
                    *runtime = Some(DesktopRuntime {
                        run: run_identity(&params),
                        _owner: owner,
                        driver,
                    });
                }
                let driver = &runtime.as_ref().unwrap().driver;
                let result = driver
                    .call_tool(
                        id.strip_prefix(PREFIX).unwrap().into(),
                        params["arguments"].to_string(),
                    )
                    .await
                    .map_err(|error| {
                        let failure = if matches!(
                            error,
                            cua_driver_sdk::DriverError::ActionInterrupted { .. }
                        ) {
                            fw::ToolError::Indeterminate(format!(
                                "{error}; observe fresh state before retrying"
                            ))
                        } else {
                            fw::ToolError::Failed(error.to_string())
                        };
                        RpcFault {
                            code: -32000,
                            message: failure.to_string(),
                            data: serde_json::to_value(failure).ok(),
                        }
                    })?;
                let result: Value = serde_json::from_str(&result.raw_json)
                    .map_err(|error| RpcFault::new(error.to_string()))?;
                let content = result["content"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|block| match block["type"].as_str() {
                        Some("text") => fw::ContentBlock::Text {
                            text: block["text"].as_str().unwrap_or_default().into(),
                        },
                        Some("image" | "audio") => fw::ContentBlock::Media {
                            media: fw::MediaContent {
                                mime_type: block["mimeType"]
                                    .as_str()
                                    .unwrap_or("application/octet-stream")
                                    .into(),
                                source: fw::MediaSource::Base64(
                                    block["data"].as_str().unwrap_or_default().into(),
                                ),
                                name: None,
                                estimated_tokens: None,
                            },
                        },
                        _ => fw::ContentBlock::Json {
                            value: block.clone(),
                        },
                    })
                    .collect();
                serde_json::to_value(fw::ToolOutput {
                    content: fw::Content(content),
                    structured: result.get("structuredContent").cloned(),
                    is_error: result["isError"].as_bool().unwrap_or(false),
                    ..Default::default()
                })
                .map_err(|error| RpcFault::new(error.to_string()))
            }
            _ => Err(RpcFault::new("unknown native tool operation")),
        }
    }
}

pub async fn serve() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let manifest = serde_json::from_str(include_str!("../../manifest.json"))?;
    let backend = Arc::new(ComputerUseBackend::default());
    let result = BackendServer::new(manifest, backend.clone())?.serve().await;
    let shutdown = backend.shutdown().await;
    result?;
    shutdown.map_err(|error| error.message.into())
}
