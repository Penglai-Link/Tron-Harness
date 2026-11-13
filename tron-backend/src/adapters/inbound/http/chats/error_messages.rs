use crate::shared::{auth_holder::AuthHolder, locale::SupportedLocale};

// HTTP and SSE share stable error codes; diagnostic details stay in server logs.
pub(super) fn message(code: &str, detail: &str) -> String {
    let en = AuthHolder::get()
        .is_some_and(|auth| SupportedLocale::from_raw(&auth.locale) == SupportedLocale::EnUs);
    let (zh, english) = match code {
        "CHAT_ATTACHMENT_TOO_LARGE" => (
            "附件超过大小限制，请缩小文件后重新上传。",
            "The attachment exceeds the size limit. Upload a smaller file.",
        ),
        "CHAT_ATTACHMENT_INVALID" => (
            "附件无效，请检查文件类型和大小。",
            "The attachment is invalid. Check its type and size.",
        ),
        "CHAT_ATTACHMENT_NOT_FOUND" => (
            "附件不存在或已被删除，请重新上传。",
            "The attachment was not found. Upload it again.",
        ),
        "CHAT_MESSAGE_REQUIRED" | "AGENT_PROMPT_REQUIRED" => {
            ("请输入消息内容。", "Enter a message.")
        }
        "CHAT_SESSION_NOT_FOUND" | "CHAT_RUN_CONTROL_SESSION_NOT_FOUND" => (
            "会话不存在或已被删除，请重新打开会话。",
            "The conversation was not found. Reopen the conversation.",
        ),
        "CHAT_SESSION_MUST_BE_ARCHIVED" => (
            "请先归档会话，再永久删除。",
            "Archive the conversation before permanently deleting it.",
        ),
        "CHAT_SESSION_ARCHIVED" => (
            "会话已归档，请恢复后再操作。",
            "Restore the archived conversation before continuing.",
        ),
        "CHAT_SESSION_RUN_ACTIVE" => (
            "会话正在运行，请停止或等待结束后再操作。",
            "Stop the active run or wait for it to finish before continuing.",
        ),
        "CHAT_CONTAINER_DELETE_REJECTED" => (
            "磁盘目录仍被其他主题或会话使用，或路径不适合整目录删除。请解除共享引用或检查目录后重试。",
            "Disk directories are shared with another topic or chat, or are unsafe to delete. Remove shared references or check the paths and retry.",
        ),
        "CHAT_SESSION_LOCATION_CHANGED" => (
            "会话位置刚刚发生更改，请重试。",
            "The chat location just changed. Please retry.",
        ),
        "CHAT_SESSION_TITLE_REQUIRED" => ("请输入会话名称。", "Enter a conversation name."),
        "CHAT_WORKSPACE_REQUIRED" | "AGENT_WORKSPACE_REQUIRED" => {
            ("请先选择工作空间。", "Select a workspace first.")
        }
        "CHAT_WORKSPACE_FORBIDDEN" => (
            "你没有访问此工作空间的权限。",
            "You do not have access to this workspace.",
        ),
        "AGENT_WORKSPACE_INVALID" => (
            "工作空间不可用，请选择有效的工作空间。",
            "The workspace is unavailable. Select an active workspace.",
        ),
        "CHAT_PROJECT_REQUIRED" => ("请先选择主题。", "Select a topic first."),
        "101045005" => (
            "当前工作空间已有同名的未归档主题，请先重命名该主题，再恢复归档会话。",
            "An active topic with this name already exists. Rename it before restoring this archived conversation.",
        ),
        "CHAT_PROJECT_INVALID" => (
            "主题不属于当前工作空间或已不可用，请重新选择。",
            "The topic is unavailable in this workspace. Select another topic.",
        ),
        "CHAT_DIRECTORY_FIXED" => (
            "目录会话的工作目录不可更改。",
            "Directory chats must use their own directory.",
        ),
        "CHAT_DIRECTORY_INVALID" => (
            "目录不属于当前空间或已不可用，请重新选择。",
            "The directory is unavailable in this space. Select another directory.",
        ),
        "CHAT_USER_INPUT_ALREADY_ANSWERED" => (
            "此请求已回答，无需重复提交。请刷新会话查看结果。",
            "This request has already been answered. Refresh the conversation to view the result.",
        ),
        "CHAT_USER_INPUT_ANSWER_INVALID" => (
            "审批或回答无效，请刷新会话并检查填写内容。",
            "The approval or answer is invalid. Refresh the conversation and check your input.",
        ),
        "CHAT_USER_INPUT_PENDING" => (
            "请先回答会话中等待处理的问题或审批。",
            "Answer the pending question or approval first.",
        ),
        "CHAT_USER_INPUT_REQUEST_NOT_FOUND" => (
            "待回答的请求不存在或已过期，请刷新会话。",
            "The pending request was not found or has expired. Refresh the conversation.",
        ),
        "CHAT_STORAGE_FAILED" | "AGENT_HISTORY_STORAGE_FAILED" => (
            "会话数据保存失败，请稍后刷新会话，确认状态后再操作。",
            "Conversation data could not be saved. Refresh later and check its status before continuing.",
        ),
        "CHAT_RUN_ALREADY_ACTIVE" | "RUN_ALREADY_ACTIVE" => (
            "会话已有正在执行的任务，请等待结束后重试。",
            "This conversation already has an active run. Wait for it to finish.",
        ),
        "CHAT_SESSION_CODE_REQUIRED" | "CHAT_RUN_CONTROL_SESSION_CODE_REQUIRED" => (
            "缺少会话信息，请重新打开会话。",
            "Conversation information is missing. Reopen the conversation.",
        ),
        "CHAT_EXPECTED_RUN_CODE_REQUIRED" | "CHAT_RUN_CONTROL_EXPECTED_RUN_CODE_REQUIRED" => (
            "缺少运行信息，请刷新会话后重试。",
            "Run information is missing. Refresh the conversation and try again.",
        ),
        "CHAT_STEER_NO_ACTIVE_RUN" | "CHAT_RUN_CONTROL_NO_ACTIVE_RUN" => (
            "当前没有正在执行的任务，请刷新会话查看最新状态。",
            "There is no active run. Refresh the conversation to view its current status.",
        ),
        "CHAT_STEER_RUN_MISMATCH" | "CHAT_RUN_CONTROL_RUN_MISMATCH" => (
            "当前运行已发生变化，请刷新会话后重试。",
            "The active run has changed. Refresh the conversation and try again.",
        ),
        "CHAT_STEER_RUNTIME_UNSUPPORTED" => (
            "当前智能体不支持在运行中追加指令，请等待本次运行结束。",
            "This agent cannot accept instructions during a run. Wait for it to finish.",
        ),
        "CHAT_STEER_RUN_STARTING" => (
            "任务正在启动，请稍后重试。",
            "The run is starting. Try again shortly.",
        ),
        "CHAT_STEER_RUN_FINISHING" | "CHAT_RUN_CONTROL_RUN_FINISHING" => (
            "任务正在结束，请稍后刷新会话。",
            "The run is finishing. Refresh the conversation shortly.",
        ),
        "CHAT_STEER_REGISTRY_UNAVAILABLE" | "CHAT_RUN_CONTROL_REGISTRY_UNAVAILABLE" => (
            "暂时无法获取运行状态，请稍后重试。",
            "The run status is temporarily unavailable. Try again later.",
        ),
        "CHAT_STOP_RUN_REQUEST_INVALID"
        | "CHAT_ACTIVE_RUN_QUERY_INVALID"
        | "CHAT_STEER_REQUEST_INVALID" => (
            "请求格式无效，请刷新页面后重试。",
            "The request is invalid. Refresh the page and try again.",
        ),
        "AGENT_MODEL_REQUIRED" => (
            "请先选择模型或配置自动路由。",
            "Select a model or configure automatic routing first.",
        ),
        "AGENT_RUN_STOPPED" | "CANCELLED" => ("本次运行已停止。", "This run has stopped."),
        "AGENT_RUNTIME_FAILED" => (
            "智能体执行失败，请检查运行日志后重试。",
            "The agent could not complete this run. Check the run logs before retrying.",
        ),
        "VARIABLE_RUNTIME_SECRET_DELIVERY_UNTRUSTED" => (
            "当前运行环境未获准接收敏感变量，请检查变量授权。",
            "This runtime is not trusted to receive secret variables. Check variable permissions.",
        ),
        "VARIABLE_RUNTIME_DECRYPT_FAILED" => (
            "运行变量解密失败，请检查加密配置。",
            "Runtime variables could not be decrypted. Check encryption settings.",
        ),
        "VARIABLE_RUNTIME_ENCRYPTION_NOT_CONFIGURED" => (
            "尚未配置变量加密，请先完成加密配置。",
            "Variable encryption is not configured. Configure it first.",
        ),
        "VARIABLE_RUNTIME_STORAGE_FAILED" | "VARIABLE_RUNTIME_RESOLUTION_FAILED" => (
            "无法读取运行变量，请检查变量配置后重试。",
            "Runtime variables could not be loaded. Check their configuration and try again.",
        ),
        "AGENT_MODEL_FAILED" => (
            "模型服务调用失败，请检查模型配置和服务状态后重试。",
            "The model service failed. Check its configuration and availability before retrying.",
        ),
        "AGENT_TOOL_FAILED" => (
            "工具执行失败，请检查运行详情后重试。",
            "A tool failed. Check the run details before retrying.",
        ),
        "AGENT_EMPTY_FINAL_OUTPUT" => (
            "模型未返回有效回复，请重试或更换模型。",
            "The model returned no usable reply. Retry or select another model.",
        ),
        "AGENT_MAX_STEPS_EXCEEDED" | "STEP_LIMIT" => (
            "本次运行已达到执行步数上限。请发送新消息继续任务；审批和回答不会重置本次运行的额度。",
            "This run reached its step limit. Send a new message to continue; approvals and answers do not reset the current run budget.",
        ),
        "AGENT_MAX_TOOL_CALLS_EXCEEDED" | "TOOL_CALL_LIMIT" => (
            "本次运行已达到工具调用次数上限。请发送新消息继续任务；审批和回答不会重置本次运行的额度。",
            "This run reached its tool-call limit. Send a new message to continue; approvals and answers do not reset the current run budget.",
        ),
        "CHAT_STREAM_COMPLETION_UNAVAILABLE" | "CHAT_STREAM_COMPLETION_INVALID" | "101031004" => (
            "尚未确认运行结果，请刷新会话查看最新状态，避免重复提交。",
            "The run result could not be confirmed. Refresh the conversation before submitting again.",
        ),
        "CHAT_STREAM_SERIALIZATION_FAILED" => (
            "运行结果传输失败，请刷新会话查看已保存的结果。",
            "The run result could not be delivered. Refresh the conversation to view saved results.",
        ),
        _ => {
            tracing::warn!(code, detail, "unmapped chat error");
            return if en {
                "The operation failed. Refresh the conversation and check its status before retrying.".into()
            } else {
                "操作失败，请刷新会话并确认状态后重试。".into()
            };
        }
    };
    if !detail.is_empty() {
        tracing::debug!(code, detail, "chat error diagnostic");
    }
    if en { english.into() } else { zh.into() }
}
