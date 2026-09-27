pub(super) fn translate_en_us(key: &str) -> Option<&'static str> {
    match key {
        "agent.toolCatalog.terminal.name" => Some("Agent terminal"),
        "agent.toolCatalog.terminal.description" => Some(
            "Persistent backend terminal sessions: start, read output, send input, stop and list. Independent of frontend connections; scoped to the current conversation.",
        ),
        "agent.toolCatalog.terminal.start.name" => Some("Start terminal process"),
        "agent.toolCatalog.terminal.start.description" => Some(
            "Start a long-running service with command, or open the user's preferred interactive shell with interactive:true and no command or args. A bare shell command is rejected to prevent accidental empty terminals. Complete command strings run through the user's preferred shell in non-interactive mode; explicit args run as literal argv. Uses a PTY without platform AI shell integration. cwd defaults to the current application/worktree. Read output later with terminal_read. Sessions survive agent turns, not backend restart.",
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
        "agent.toolCatalog.terminal.params.command" => Some(
            "Complete command line, e.g. npm run dev. When args is supplied, use only the executable name or path.",
        ),
        "agent.toolCatalog.terminal.params.args" => {
            Some("Optional literal argv. Omit when command already contains the full command line.")
        }
        "agent.toolCatalog.terminal.params.interactive" => {
            Some("Set true to open the user's preferred shell. Do not provide command or args.")
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
            "command 用于启动长期运行的服务；只有明确需要空交互 Shell 时才传 interactive:true，且不能传 command 或 args。避免直接启动裸 bash/zsh 等空 Shell，它会被拒绝。完整命令字符串由用户默认 Shell 以非交互方式执行；显式 args 按原样作为参数传入。使用伪终端，不注入平台 AI shell 功能。cwd 默认当前应用/worktree；后续用 terminal_read 读取输出。支持跨轮次，后端重启后不保留。",
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
            Some("完整命令行，如 npm run dev；提供 args 时这里只填写程序名或路径。")
        }
        "agent.toolCatalog.terminal.params.args" => {
            Some("可选的逐项参数；command 已含完整命令行时省略。")
        }
        "agent.toolCatalog.terminal.params.interactive" => {
            Some("设为 true 可打开用户默认 Shell；此时不能传 command 或 args。")
        }
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
