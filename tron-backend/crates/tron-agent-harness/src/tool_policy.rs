//! Product permissions are evaluated by the native Framework tool pipeline.
use super::{ToolRisk, identify_tool_risk};
use async_trait::async_trait;
use eliteforge_agent_core as fw;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolPermission {
    #[default]
    Run,
    Ask,
    No,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ToolPermissionSetting {
    pub tool_code: String,
    pub permission: ToolPermission,
    pub confirm_level: ToolRisk,
}
impl ToolPermissionSetting {
    pub fn default_for(tool_code: impl Into<String>) -> Self {
        Self {
            tool_code: tool_code.into(),
            permission: ToolPermission::Run,
            confirm_level: ToolRisk::Low,
        }
    }
}

/// Implemented by the trusted host. Neither metadata nor settings come from model arguments.
pub trait ToolPermissionSource: Send + Sync {
    fn setting(&self, descriptor: &fw::ToolDescriptor) -> Result<ToolPermissionSetting, String>;
    /// Some(false) is a known query; Some(true) is a known mutation; unknown stays None.
    fn operation(&self, descriptor: &fw::ToolDescriptor) -> Option<bool>;
}

pub struct TronToolPolicy(pub Arc<dyn ToolPermissionSource>);
#[async_trait]
impl fw::ToolPolicy for TronToolPolicy {
    async fn admit_binding(
        &self,
        binding: &fw::ToolBinding,
        _: &fw::RunContext,
    ) -> Result<fw::ToolBindingAdmission, fw::AgentError> {
        let setting = self
            .0
            .setting(&binding.descriptor)
            .map_err(fw::AgentError::ToolSetup)?;
        Ok(if setting.permission == ToolPermission::No {
            fw::ToolBindingAdmission::Exclude
        } else {
            fw::ToolBindingAdmission::Allow
        })
    }
    async fn evaluate(
        &self,
        invocation: &fw::ToolInvocationContext<'_>,
    ) -> Result<fw::ToolPolicyDecision, fw::AgentError> {
        let setting = self
            .0
            .setting(invocation.descriptor)
            .map_err(fw::AgentError::Run)?;
        Ok(match setting.permission {
            ToolPermission::No => {
                fw::ToolPolicyDecision::Deny("This tool has no platform permission".into())
            }
            ToolPermission::Run => fw::ToolPolicyDecision::Allow,
            ToolPermission::Ask => {
                let operation = self.0.operation(invocation.descriptor);
                let risk = identify_tool_risk(
                    invocation.descriptor,
                    &invocation.call.arguments,
                    operation,
                );
                let reason = if invocation.descriptor.id.starts_with("mcp/") {
                    "远端 MCP 工具的副作用无法由宿主验证"
                } else if invocation.descriptor.id == "personai.generate_image" {
                    "将提交远端图片生成任务，可能产生费用"
                } else if invocation.descriptor.id.starts_with("filesystem.") {
                    "本地文件读取或修改，风险按真实路径和操作判断"
                } else if invocation.call.arguments.get("command").is_some()
                    || invocation.descriptor.call_name.starts_with("terminal_")
                {
                    "系统命令风险按真实可执行程序、参数及 shell 行为判断"
                } else if operation == Some(true) {
                    "该工具会修改业务数据或运行状态"
                } else if operation == Some(false) {
                    "该工具用于查询，参数中的可执行或批量操作会提高风险"
                } else {
                    "未知原生工具的副作用无法由宿主验证"
                };
                if risk >= setting.confirm_level {
                    // Framework persists the original call and resumes it exactly once.
                    fw::ToolPolicyDecision::RequireApproval(
                        format!(
                            "工具 {} 的风险为 {:?}，达到确认阈值 {:?}。{}。是否执行本次调用？",
                            invocation.descriptor.call_name, risk, setting.confirm_level, reason
                        )
                        .into(),
                    )
                } else {
                    fw::ToolPolicyDecision::Allow
                }
            }
        })
    }
}
