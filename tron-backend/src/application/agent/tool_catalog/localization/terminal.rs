pub(super) fn translate_en_us(key: &str) -> Option<&'static str> {
    match key {
        "agent.toolCatalog.terminal.name" => Some("Agent terminal"),
        "agent.toolCatalog.terminal.description" => Some(
            "Persistent backend terminal sessions: start, read output, send input, stop and list. Independent of frontend connections; scoped to the current conversation.",
        ),
        "agent.toolCatalog.terminal.start.name" => Some("Start terminal process"),
        "agent.toolCatalog.terminal.start.description" => Some(
            "Start an executable and return immediately with terminalId. Uses a PTY without platform AI shell integration. cwd defaults to the current application/worktree and remains fixed for this process. To run shell syntax explicitly launch a shell; prefer bash --noprofile --norc or zsh -f. Read output later with terminal_read. Sessions survive agent turns, not backend restart.",
        ),
        "agent.toolCatalog.terminal.read.name" => Some("Read terminal output"),
        "agent.toolCatalog.terminal.read.description" => Some(
            "Read output from cursor without waiting for exit. Pass nextCursor on subsequent reads. Output is capped at 32 KiB per read and 256 KiB retained; truncated indicates discarded earlier output. running and exitCode describe the process; outputClosed indicates the stream ended. Avoid busy polling.",
        ),
        "agent.toolCatalog.terminal.write.name" => Some("Write terminal input"),
        "agent.toolCatalog.terminal.write.description" => Some(
            "Send raw input to a running terminal. Include a newline to submit a command or answer. Control-C interrupts the foreground command. Does not inject AI commands.",
        ),
        "agent.toolCatalog.terminal.stop.name" => Some("Stop terminal process"),
        "agent.toolCatalog.terminal.stop.description" => Some(
            "Terminate the selected terminal process; retain buffered output for inspection. Already exited processes are accepted.",
        ),
        "agent.toolCatalog.terminal.list.name" => Some("List conversation terminals"),
        "agent.toolCatalog.terminal.list.description" => Some(
            "List only terminals owned by this conversation with cwd and exit state, without replaying output.",
        ),
        "agent.toolCatalog.terminal.result" => {
            Some("Terminal identity, working directory, state and bounded output.")
        }
        "agent.toolCatalog.terminal.params.command" => {
            Some("Executable name or path; shell syntax is not interpreted.")
        }
        "agent.toolCatalog.terminal.params.args" => {
            Some("Arguments passed directly to the executable.")
        }
        "agent.toolCatalog.terminal.params.cwd" => {
            Some("Allowed working directory; defaults to the selected application/worktree.")
        }
        "agent.toolCatalog.terminal.params.env" => {
            Some("Additional process environment variables.")
        }
        "agent.toolCatalog.terminal.params.terminalId" => {
            Some("Terminal identifier returned by start or list in this conversation.")
        }
        "agent.toolCatalog.terminal.params.cursor" => {
            Some("Byte offset from nextCursor; omit for the oldest available output.")
        }
        "agent.toolCatalog.terminal.params.data" => {
            Some("Raw terminal input, including newline when required.")
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
        "agent.toolCatalog.terminal.start.name" => Some("启动终端进程"),
        "agent.toolCatalog.terminal.start.description" => Some(
            "启动可执行程序并立即返回 terminalId。使用伪终端，不注入平台 AI shell 功能。cwd 默认当前应用/worktree，已有进程不随会话切换目录。执行 shell 语法时显式启动 shell，建议 bash --noprofile --norc 或 zsh -f。后续使用 terminal_read 读取输出。支持跨轮次使用，后端重启后不保留。",
        ),
        "agent.toolCatalog.terminal.read.name" => Some("读取终端输出"),
        "agent.toolCatalog.terminal.read.description" => Some(
            "从 cursor 读取输出，无需等待进程结束。下次读取传入 nextCursor。每次最多 32 KiB，保留最近 256 KiB；truncated 表示早期输出已丢弃。running 和 exitCode 表示进程状态，outputClosed 表示输出流已结束。避免频繁轮询。",
        ),
        "agent.toolCatalog.terminal.write.name" => Some("发送终端输入"),
        "agent.toolCatalog.terminal.write.description" => Some(
            "向运行中的终端发送原始输入。提交命令或回答时带换行；Control-C 中断前台命令。不注入 AI 指令。",
        ),
        "agent.toolCatalog.terminal.stop.name" => Some("停止终端进程"),
        "agent.toolCatalog.terminal.stop.description" => {
            Some("终止指定终端进程，保留缓存输出供查询。已退出进程也可调用。")
        }
        "agent.toolCatalog.terminal.list.name" => Some("查询会话终端"),
        "agent.toolCatalog.terminal.list.description" => {
            Some("只查询当前会话的终端、工作目录和退出状态，不重复输出日志。")
        }
        "agent.toolCatalog.terminal.result" => Some("终端标识、工作目录、运行状态和限量输出。"),
        "agent.toolCatalog.terminal.params.command" => {
            Some("可执行程序名称或路径，不自动解析 shell 语法。")
        }
        "agent.toolCatalog.terminal.params.args" => Some("直接传给程序的参数。"),
        "agent.toolCatalog.terminal.params.cwd" => {
            Some("允许访问的工作目录，默认当前应用/worktree。")
        }
        "agent.toolCatalog.terminal.params.env" => Some("进程附加环境变量。"),
        "agent.toolCatalog.terminal.params.terminalId" => {
            Some("当前会话启动或查询返回的终端标识。")
        }
        "agent.toolCatalog.terminal.params.cursor" => {
            Some("上次 nextCursor 返回的字节偏移，省略时从最早保留输出开始。")
        }
        "agent.toolCatalog.terminal.params.data" => Some("终端原始输入，需要提交时包含换行。"),
        _ => None,
    }
}
