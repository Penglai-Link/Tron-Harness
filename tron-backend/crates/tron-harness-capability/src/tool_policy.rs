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
        let tool_code = tool_code.into();
        let (permission, confirm_level) = if tool_code == "bash.exec" {
            (ToolPermission::Ask, ToolRisk::High)
        } else {
            (ToolPermission::Run, ToolRisk::Low)
        };
        Self {
            tool_code,
            permission,
            confirm_level,
        }
    }
}

/// Implemented by the trusted host. Neither metadata nor settings come from model arguments.
pub trait ToolPermissionSource: Send + Sync {
    fn setting(&self, descriptor: &fw::ToolDescriptor) -> Result<ToolPermissionSetting, String>;
    /// Some(false) is a known query; Some(true) is a known mutation; unknown stays None.
    fn operation(&self, descriptor: &fw::ToolDescriptor) -> Option<bool>;
}

/// Display language supplied by the trusted host for this run, never by tool arguments.
/// Missing language follows Tron's backend fallback to Simplified Chinese.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum TronToolPolicyLocale {
    EnUs,
    #[default]
    ZhCn,
}

impl TronToolPolicyLocale {
    pub fn from_run(run: &fw::RunContext) -> Self {
        run.extensions
            .get::<Self>()
            .map(|locale| *locale)
            .unwrap_or_default()
    }

    fn text(self, zh: &'static str, en: &'static str) -> &'static str {
        match self {
            Self::ZhCn => zh,
            Self::EnUs => en,
        }
    }

    pub fn permission_denied_message(self) -> &'static str {
        self.text(
            "该工具没有平台执行权限",
            "This tool has no platform permission",
        )
    }

    fn risk_label(self, risk: ToolRisk) -> &'static str {
        match risk {
            ToolRisk::Safe => self.text("无风险", "Safe"),
            ToolRisk::Low => self.text("低", "Low"),
            ToolRisk::Medium => self.text("中", "Medium"),
            ToolRisk::High => self.text("高", "High"),
            ToolRisk::Critical => self.text("非常危险", "Critical"),
        }
    }

    fn approval_prompt(self, risk: ToolRisk, reason: &str) -> fw::Content {
        fw::Content(vec![
            fw::ContentBlock::Text { text: reason.into() },
            fw::ContentBlock::Json {
                value: serde_json::json!({
                    "kind": "toolApproval",
                    "risk": risk,
                    "riskLabel": self.risk_label(risk),
                }),
            },
        ])
    }
}

pub struct TronToolPolicy(pub Arc<dyn ToolPermissionSource>);

/// Trusted run-local approval preference. Denied tools remain denied.
#[derive(Debug, Clone, Copy)]
pub struct TronToolAutoApproval;

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
        let locale = TronToolPolicyLocale::from_run(invocation.run);
        Ok(match setting.permission {
            ToolPermission::No => {
                fw::ToolPolicyDecision::Deny(locale.permission_denied_message().into())
            }
            ToolPermission::Run => fw::ToolPolicyDecision::Allow,
            ToolPermission::Ask
                if invocation
                    .run
                    .extensions
                    .get::<TronToolAutoApproval>()
                    .is_some() =>
            {
                fw::ToolPolicyDecision::Allow
            }
            ToolPermission::Ask => {
                let operation = self.0.operation(invocation.descriptor);
                let risk = identify_tool_risk(
                    invocation.descriptor,
                    &invocation.call.arguments,
                    operation,
                );
                let reason = if invocation.descriptor.id.starts_with("mcp/") {
                    locale.text(
                        "远端 MCP 工具的副作用无法由宿主验证",
                        "The host cannot verify the side effects of this remote MCP tool",
                    )
                } else if invocation.descriptor.id == "personai.generate_image" {
                    locale.text(
                        "将提交远端图片生成任务，可能产生费用",
                        "This submits a remote image generation task and may incur charges",
                    )
                } else if invocation.descriptor.id.starts_with("filesystem.") {
                    locale.text(
                        "本地文件读取或修改，风险按真实路径和操作判断",
                        "Local file access risk is assessed from the actual paths and operations",
                    )
                } else if invocation.call.arguments.get("command").is_some()
                    || invocation.descriptor.call_name.starts_with("terminal_")
                {
                    let (zh, en) =
                        super::tool_risk::command_approval_reason(&invocation.call.arguments);
                    locale.text(zh, en)
                } else if operation == Some(true) {
                    locale.text(
                        "该工具会修改业务数据或运行状态",
                        "This tool changes business data or runtime state",
                    )
                } else if operation == Some(false) {
                    locale.text("该工具用于查询，参数中的可执行或批量操作会提高风险", "This tool performs queries; executable or batch operations in its arguments increase risk")
                } else {
                    locale.text(
                        "未知原生工具的副作用无法由宿主验证",
                        "The host cannot verify the side effects of this unknown native tool",
                    )
                };
                if risk >= setting.confirm_level {
                    // Framework persists the original call and resumes it exactly once.
                    fw::ToolPolicyDecision::RequireApproval(
                        locale.approval_prompt(risk, reason),
                    )
                } else {
                    fw::ToolPolicyDecision::Allow
                }
            }
        })
    }
}
