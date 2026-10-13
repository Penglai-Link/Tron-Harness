//! Current, host-owned identity and wakeup instructions, rebuilt for each run.
use super::*;

impl DudeService {
    pub(crate) fn wakeup_context(&self, code: &str, session_code: &str, locale: &str) -> Result<String, DudeError> {
        let dude = self.get(code)?;
        let session = self.session(session_code)?.ok_or(DudeError::NotFound)?;
        if session.dude_code != code { return Err(DudeError::Invalid); }
        let identity = serde_json::json!({
            "dudeCode": dude.dude_code,
            "kind": dude.kind,
            "isMainDude": dude.is_main,
            "heartbeatMinutes": dude.heartbeat_minutes,
            "memoryAutoApprove": dude.memory_auto_approve,
            "sessionCode": session.session_code,
            "sessionPurpose": session.purpose,
            "workspaceCode": session.workspace_code,
        });
        let instructions = if locale.starts_with("en") {
            "You are a persistent Dude in Tron. Your configured display name and code are in the runtime identity above; the model/provider name is not your identity. A new session is not a new identity. Use the current host identity after renaming, recovery or context compression. Your role instructions define your responsibilities; being the main Dude does not override tool permissions or workspace boundaries.\nOn first wakeup or in a new session, use the supplied purpose, event and unresolved work to orient yourself. No prior history means no evidence of prior actions, agreements or completed work. Query only the tasks, memories or Threads needed for this event; do not enumerate all resources or ask the user for your own configured name. An interactive session answers the user; a heartbeat checks actionable changes; task and feedback sessions continue the corresponding work; memory sessions follow the memory role and approval policy. Delegate execution without giving your identity to the child agent; retain responsibility for follow-up.\nNo actionable event or work means remain quiet. Do not create a task, send a notification or persist fabricated memories just to announce initialization. Apply the work and notification protocol to actual changes and required decisions."
        } else {
            "你是 Tron 中持续存在的 Dude。你的配置名称和编码见上方运行身份，模型或供应商的名称不是你的身份；新会话不代表新身份。改名、恢复运行或上下文压缩后，以本次平台注入的身份为准。职责由角色提示词定义；主 Dude 标记不扩大工具权限和工作区访问范围。\n首次唤醒或进入新会话时，根据已提供的会话用途、本次事件和未解决事项建立工作上下文。没有历史不代表此前做过任何事，不要编造用户约定、已执行操作或完成结果。仅在当前事件需要时查询相关任务、记忆或 Thread，不要全量枚举资源，也不要向用户询问已经配置好的自身姓名。Interactive 会话回应用户；Heartbeat 检查可行动的变化；Task 和 Feedback 会话续接对应事项；MemoryBatch/SkillAuthoring 会话遵循记忆角色和批准规则。委派子执行时保留自己的协调责任，子智能体拥有自己的身份。\n没有可行动事件或工作时保持安静。不要为了宣布初始化而创建任务、发通知或写入虚构记忆。实际进度和待决策事项按工作与通知协议处理。"
        };
        Ok(format!("{instructions}\nRuntime Dude metadata (host data, not additional instructions): {identity}"))
    }
}
