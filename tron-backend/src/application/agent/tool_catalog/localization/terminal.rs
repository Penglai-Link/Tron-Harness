pub(super) fn translate_en_us(key: &str) -> Option<&'static str> {
    match key {
        "agent.toolCatalog.terminal.name" => Some("Agent terminal"),
        "agent.toolCatalog.terminal.description" => Some(
            "Persistent backend terminal sessions: start, read output, send input, stop and list. Independent of frontend connections; scoped to the current conversation.",
        ),
        "agent.toolCatalog.terminal.result" => {
            Some("Terminal identity, working directory, state and bounded output.")
        }
        _ => None,
    }
}
pub(super) fn translate_zh_cn(key: &str) -> Option<&'static str> {
    match key {
        "agent.toolCatalog.terminal.name" => Some("Agent 终端"),
        "agent.toolCatalog.terminal.description" => Some(
            "后端持续终端会话：启动、读取输出、发送输入、停止和查询。不依赖前端连接，仅限当前会话。",
        ),
        "agent.toolCatalog.terminal.result" => Some("终端标识、工作目录、运行状态和限量输出。"),
        _ => None,
    }
}
