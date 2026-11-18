//! Product permissions are evaluated by the native Framework tool pipeline.
//!
//! Risk classification is structured and language independent
//! ([`super::tool_risk::RiskReason`]); this module is the approval display
//! layer that turns it into the run locale's text.
use super::tool_risk::{RiskReason, ToolRisk, assess_tool_risk, command_assessment};
use async_trait::async_trait;
use eliteforge_agent_core as fw;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tron_i18n::{Locale, t};

/// Locale supplied by the trusted host for this run, never by tool arguments.
///
/// Kept as an alias so existing host code keeps using the historical name.
pub use tron_i18n::Locale as TronToolPolicyLocale;

/// Read the run locale. Missing language follows Tron's backend fallback to
/// Simplified Chinese; session-facing code always receives it explicitly.
pub fn run_locale(run: &fw::RunContext) -> Locale {
    run.extensions
        .get::<Locale>()
        .map(|locale| *locale)
        .unwrap_or_default()
}

pub fn permission_denied_message(locale: Locale) -> String {
    t!("approval.permission_denied", locale = locale).into_owned()
}

pub fn plan_mode_denied_message(locale: Locale) -> String {
    t!("approval.plan_mode_denied", locale = locale).into_owned()
}

fn risk_label(locale: Locale, risk: ToolRisk) -> String {
    let key = match risk {
        ToolRisk::Safe => "approval.risk_label.safe",
        ToolRisk::Low => "approval.risk_label.low",
        ToolRisk::Medium => "approval.risk_label.medium",
        ToolRisk::High => "approval.risk_label.high",
        ToolRisk::Critical => "approval.risk_label.critical",
    };
    t!(key, locale = locale).into_owned()
}

fn risk_reason_message(locale: Locale, reason: RiskReason) -> String {
    t!(reason.message_key(), locale = locale).into_owned()
}

fn approval_prompt(
    locale: Locale,
    risk: Option<ToolRisk>,
    reason: &str,
    reason_code: &str,
) -> fw::Content {
    fw::Content(vec![
        fw::ContentBlock::Text {
            text: reason.into(),
        },
        fw::ContentBlock::Json {
            value: serde_json::json!({
                "kind": "toolApproval",
                "risk": risk,
                "riskLabel": risk.map(|risk| risk_label(locale, risk)),
                "riskReasonCode": reason_code,
            }),
        },
    ])
}

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

/// Persisted by the host; this never comes from tool arguments.
pub const SMART_MODE_KEY: &str = "tool-policy.smart";

#[derive(Debug, Clone, Copy)]
pub struct ToolPolicyConfig {
    pub smart: bool,
}
impl Default for ToolPolicyConfig {
    fn default() -> Self {
        Self { smart: true }
    }
}

/// None means insufficient evidence, not a fabricated risk level.
pub struct CommandRiskAssessment {
    pub risk: Option<ToolRisk>,
    pub reason: String,
}
#[async_trait]
pub trait CommandRiskReviewer: Send + Sync {
    async fn assess(
        &self,
        invocation: &fw::ToolInvocationContext<'_>,
    ) -> Result<CommandRiskAssessment, fw::AgentError>;
}
pub struct CommandRiskReviewerService(pub Arc<dyn CommandRiskReviewer>);

fn is_command_execution(id: &str) -> bool {
    matches!(
        id,
        "system.command.exec"
            | "framework.terminal.start"
            | "system.command.task.write"
            | "framework.terminal.write"
    )
}

/// Tools that only touch the current interaction, the session's task list or the
/// ephemeral interface surfaces. They are classified as commands but leave the
/// workspace and persisted data untouched, so plan mode keeps them.
fn is_interaction_tool(name: &str) -> bool {
    matches!(
        name,
        "chat_ask_user" | "ask_user" | "a2ui_request_input" | "a2ui_publish" | "todo_update"
    )
}

const REASON_CODE_MODEL_REVIEW: &str = "modelReview";

fn uncertain_assessment(locale: Locale) -> CommandRiskAssessment {
    CommandRiskAssessment {
        risk: None,
        reason: t!("approval.command_uncertain", locale = locale).into_owned(),
    }
}

pub struct TronToolPolicy(pub Arc<dyn ToolPermissionSource>);

/// Trusted run-local approval preference. Denied tools remain denied.
#[derive(Debug, Clone, Copy)]
pub struct TronToolAutoApproval;

/// Trusted run-local plan mode marker. The host sets it for runs whose session
/// is in plan mode; mutating tools are denied so the Agent can only research
/// and present a plan.
#[derive(Debug, Clone, Copy)]
pub struct TronPlanMode;

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
        let locale = run_locale(invocation.run);
        // Plan mode is a trusted run preference: read-only work stays available,
        // state-changing tools are refused instead of requesting approval.
        // Asking the user is classified as a command but changes nothing, so it
        // stays available for clarifying and confirming the plan.
        if invocation.run.extensions.get::<TronPlanMode>().is_some()
            && !is_interaction_tool(&invocation.descriptor.call_name)
            && self.0.operation(invocation.descriptor) == Some(true)
        {
            return Ok(fw::ToolPolicyDecision::Deny(
                plan_mode_denied_message(locale).into(),
            ));
        }
        Ok(match setting.permission {
            ToolPermission::No => {
                fw::ToolPolicyDecision::Deny(permission_denied_message(locale).into())
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
                if is_command_execution(&invocation.descriptor.id) {
                    let smart = invocation
                        .run
                        .extensions
                        .get::<ToolPolicyConfig>()
                        .is_none_or(|config| config.smart);
                    let assessment = if smart {
                        match invocation
                            .run
                            .extensions
                            .get::<CommandRiskReviewerService>()
                        {
                            Some(reviewer) => match reviewer.0.assess(invocation).await {
                                Ok(assessment) => assessment,
                                Err(error)
                                    if matches!(error.primary(), fw::AgentError::Cancelled) =>
                                {
                                    return Err(error);
                                }
                                Err(_) => uncertain_assessment(locale),
                            },
                            None => uncertain_assessment(locale),
                        }
                    } else {
                        // Static rules may prove simple operations, but their conservative
                        // high/critical fallback is not evidence of destructive effects.
                        let evaluated = command_assessment(&invocation.call.arguments);
                        if evaluated.risk >= ToolRisk::High {
                            uncertain_assessment(locale)
                        } else {
                            CommandRiskAssessment {
                                risk: Some(evaluated.risk),
                                reason: risk_reason_message(locale, evaluated.reason),
                            }
                        }
                    };
                    invocation.run.check_cancelled()?;
                    return Ok(
                        if assessment
                            .risk
                            .is_none_or(|risk| risk >= setting.confirm_level)
                        {
                            fw::ToolPolicyDecision::RequireApproval(approval_prompt(
                                locale,
                                assessment.risk,
                                &assessment.reason,
                                REASON_CODE_MODEL_REVIEW,
                            ))
                        } else {
                            fw::ToolPolicyDecision::Allow
                        },
                    );
                }
                let operation = self.0.operation(invocation.descriptor);
                let evaluated =
                    assess_tool_risk(invocation.descriptor, &invocation.call.arguments, operation);
                if evaluated.risk >= setting.confirm_level {
                    let reason = risk_reason_message(locale, evaluated.reason);
                    // Framework persists the original call and resumes it exactly once.
                    fw::ToolPolicyDecision::RequireApproval(approval_prompt(
                        locale,
                        Some(evaluated.risk),
                        &reason,
                        evaluated.reason.code(),
                    ))
                } else {
                    fw::ToolPolicyDecision::Allow
                }
            }
        })
    }
}
