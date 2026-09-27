pub(super) fn translate_en_us(key: &str) -> Option<&'static str> {
    match key {
        "agent.toolCatalog.tools.fs.apply_patch.params.patch" => Some(
            "Codex-style syntax: *** Begin Patch, then *** Add File: path with +lines; *** Delete File: path; or *** Update File: path, optional *** Move to: path, then @@ hunks. Hunk lines start with a space for context, - for removal, + for addition. @@ anchor skips past a unique exact line; *** End of File anchors the last hunk at EOF. Finish with *** End Patch. Use unique exact context; paths cannot contain ..; up to 100 files.",
        ),
        "agent.toolCatalog.tools.fs.apply_patch.params.cwd" => {
            Some("Patch base directory; defaults to the current selected application/worktree.")
        }
        "agent.toolCatalog.tools.fs.apply_patch.params.dryRun" => {
            Some("Validate and return a diff without modifying files.")
        }
        "agent.toolCatalog.tools.fs.apply_patch.params.expectedCount" => Some(
            "Legacy edit: expected total occurrence count. Without replaceAll, the target must be unique.",
        ),
        "agent.toolCatalog.domains.bash.name" => Some("Bash Tools"),
        "agent.toolCatalog.domains.bash.description" => Some(
            "Run non-interactive commands; long commands become background tasks. Use the Agent terminal only for interactive and long-running services.",
        ),
        "agent.toolCatalog.domains.chat_attachment.name" => Some("Chat Attachment Tools"),
        "agent.toolCatalog.domains.chat_attachment.description" => Some(
            "Read session attachments in bounded text chunks and parse complete JSON documents.",
        ),
        "agent.toolCatalog.domains.fs.name" => Some("Filesystem Tools"),
        "agent.toolCatalog.domains.fs.description" => {
            Some("Read, write, inspect, and patch local workspace files.")
        }
        "agent.toolCatalog.domains.search.name" => Some("Search Tools"),
        "agent.toolCatalog.domains.search.description" => {
            Some("Search files and source text inside the local workspace.")
        }
        "agent.toolCatalog.tools.bash.exec.name" => Some("Run shell command"),
        "agent.toolCatalog.tools.bash.exec.description" => Some(
            "Run a non-interactive command with captured stdout and stderr. Put the complete command line in command; args is optional for direct argv. Wait up to 10 seconds, then use bash_task_read for a still-running task. Use terminal_start for interactive processes and long-running services; do not detach them with nohup or shell backgrounding.",
        ),
        "agent.toolCatalog.tools.bash.exec.sampleResultSummary" => {
            Some("Returns stdout, stderr, exit status, and taskId when still running.")
        }
        "agent.toolCatalog.tools.bash.exec.params.command" => Some(
            "Complete command line, e.g. npm run build or bash -c 'echo ok'. When args is supplied, use only the executable name or path here.",
        ),
        "agent.toolCatalog.tools.bash.exec.params.args" => Some(
            "Optional literal argv. Omit this for a complete command line; when present, command must name only the executable.",
        ),
        "agent.toolCatalog.tools.bash.exec.params.cwd" => Some(
            "Optional working directory. Absolute, relative, or ~ home-directory paths are allowed.",
        ),
        "agent.toolCatalog.tools.bash.exec.params.stdin" => {
            Some("Optional text piped to the process standard input.")
        }
        "agent.toolCatalog.tools.bash.exec.params.env" => {
            Some("Optional environment variables added to the child process.")
        }
        "agent.toolCatalog.tools.bash.task_read.name" => Some("Read background command task"),
        "agent.toolCatalog.tools.bash.task_read.description" => Some(
            "Check a command that bash_exec moved to the background after 10 seconds. Returns process state, exit code, and output chunks. Continue with nextStdoutCursor and nextStderrCursor until running is false and outputClosed is true; avoid rapid polling.",
        ),
        "agent.toolCatalog.tools.bash.task_read.sampleResultSummary" => {
            Some("Returns command task state, exit code, and incremental stdout/stderr.")
        }
        "agent.toolCatalog.tools.bash.task_read.params.taskId" => {
            Some("taskId returned by bash_exec when the command was still running.")
        }
        "agent.toolCatalog.tools.bash.task_read.params.stdoutCursor" => {
            Some("Next stdout byte offset; pass nextStdoutCursor from the previous result.")
        }
        "agent.toolCatalog.tools.bash.task_read.params.stderrCursor" => {
            Some("Next stderr byte offset; pass nextStderrCursor from the previous result.")
        }
        "agent.toolCatalog.tools.bash.task_stop.name" => Some("Stop background command task"),
        "agent.toolCatalog.tools.bash.task_stop.description" => Some(
            "Terminate a command still running in the background. Use bash_task_read afterward if you need to collect its final output.",
        ),
        "agent.toolCatalog.tools.bash.task_stop.sampleResultSummary" => {
            Some("Reports whether the background command was stopped.")
        }
        "agent.toolCatalog.tools.bash.task_stop.params.taskId" => {
            Some("taskId returned by bash_exec.")
        }
        "agent.toolCatalog.tools.fs.read_text.name" => Some("Read text file"),
        "agent.toolCatalog.tools.fs.read_text.description" => Some(
            "Read UTF-8 text from a linked project file or this session's attachment, optionally by line range.",
        ),
        "agent.toolCatalog.tools.fs.read_text.sampleResultSummary" => {
            Some("Returns file content and the materialized line range.")
        }
        "agent.toolCatalog.tools.fs.read_text.params.path" => Some(
            "Legacy project file path. Prefer fileRef, which also supports session attachments.",
        ),
        "agent.toolCatalog.tools.fs.read_text.params.startLine" => {
            Some("Optional one-based start line.")
        }
        "agent.toolCatalog.tools.fs.read_text.params.endLine" => {
            Some("Optional one-based inclusive end line.")
        }
        "agent.toolCatalog.tools.fs.read_text.params.maxBytes" => {
            Some("Optional maximum number of bytes to return after slicing.")
        }
        "agent.toolCatalog.tools.chat_attachment.read_text.name" => {
            Some("Read chat attachment text")
        }
        "agent.toolCatalog.tools.chat_attachment.read_text.description" => Some(
            "Read a session attachment as bounded UTF-8 chunks. The attachment is not a workspace file.",
        ),
        "agent.toolCatalog.tools.chat_attachment.read_text.sampleResultSummary" => {
            Some("Returns one text chunk with byte and line cursors for the next chunk.")
        }
        "agent.toolCatalog.tools.chat_attachment.read_text.params.attachmentCode" => {
            Some("Session attachment code from the chat attachment context.")
        }
        "agent.toolCatalog.tools.chat_attachment.read_text.params.startByte" => {
            Some("Byte offset for the chunk; use nextStartByte from the previous result.")
        }
        "agent.toolCatalog.tools.chat_attachment.read_text.params.maxBytes" => {
            Some("Maximum chunk size; the server caps this at 262144 bytes.")
        }
        "agent.toolCatalog.tools.chat_attachment.parse_json.name" => {
            Some("Parse chat attachment JSON")
        }
        "agent.toolCatalog.tools.chat_attachment.parse_json.description" => Some(
            "Parse a project file or session attachment as JSON, return document statistics, and inspect an optional JSON Pointer.",
        ),
        "agent.toolCatalog.tools.chat_attachment.parse_json.sampleResultSummary" => {
            Some("Returns full-document JSON statistics and a bounded selected value.")
        }
        "agent.toolCatalog.tools.chat_attachment.parse_json.params.attachmentCode" => {
            Some("Legacy session attachment code. Prefer fileRef to specify the file source.")
        }
        "agent.toolCatalog.tools.chat_attachment.parse_json.params.jsonPath" => Some(
            "Optional RFC 6901 JSON Pointer, for example /topics/0; empty selects the document root.",
        ),
        "agent.toolCatalog.tools.chat_attachment.parse_json.params.maxValueBytes" => Some(
            "Maximum serialized bytes returned for the selected JSON value; parsing still covers the full document.",
        ),
        "agent.toolCatalog.tools.fs.write_text.name" => Some("Write text file"),
        "agent.toolCatalog.tools.fs.write_text.description" => {
            Some("Create, overwrite, or append a local text file.")
        }
        "agent.toolCatalog.tools.fs.write_text.sampleResultSummary" => {
            Some("Returns the written file path, mode, and byte count.")
        }
        "agent.toolCatalog.tools.fs.write_text.params.path" => Some(
            "Target file path, absolute, relative to the workspace root, or a ~ home-directory path.",
        ),
        "agent.toolCatalog.tools.fs.write_text.params.content" => Some("Text content to write."),
        "agent.toolCatalog.tools.fs.write_text.params.mode" => {
            Some("Write mode. Allowed values: create, overwrite, append.")
        }
        "agent.toolCatalog.tools.fs.write_text.params.ensureParentDirs" => {
            Some("Whether missing parent directories should be created before writing.")
        }
        "agent.toolCatalog.tools.fs.apply_patch.name" => Some("Apply code patch"),
        "agent.toolCatalog.tools.fs.apply_patch.description" => Some(
            "Edit code with a context-based multi-file patch: add, update, delete, or move files. All operations are validated before writing; dryRun previews changes. Legacy path/action/target text edits remain supported; ambiguous targets are rejected.",
        ),
        "agent.toolCatalog.tools.fs.apply_patch.sampleResultSummary" => {
            Some("Returns changed paths and a bounded diff; dryRun does not write files.")
        }
        "agent.toolCatalog.tools.fs.apply_patch.params.path" => Some(
            "Target file path, absolute, relative to the workspace root, or a ~ home-directory path.",
        ),
        "agent.toolCatalog.tools.fs.apply_patch.params.action" => {
            Some("Patch action. Allowed values: replace, delete, insert_before, insert_after.")
        }
        "agent.toolCatalog.tools.fs.apply_patch.params.target" => {
            Some("Target text that anchors the patch operation.")
        }
        "agent.toolCatalog.tools.fs.apply_patch.params.content" => {
            Some("Replacement or inserted text, depending on the patch action.")
        }
        "agent.toolCatalog.tools.fs.apply_patch.params.replaceAll" => Some(
            "Legacy edit: replace every occurrence explicitly; use expectedCount to guard the match count.",
        ),
        "agent.toolCatalog.tools.fs.list.name" => Some("List files"),
        "agent.toolCatalog.tools.fs.list.description" => {
            Some("List files and directories under a local path.")
        }
        "agent.toolCatalog.tools.fs.list.sampleResultSummary" => {
            Some("Returns directory entries with file type metadata.")
        }
        "agent.toolCatalog.tools.fs.list.params.path" => Some(
            "Directory path to inspect, absolute, relative to the workspace root, or a ~ home-directory path.",
        ),
        "agent.toolCatalog.tools.fs.list.params.recursive" => {
            Some("Whether nested directories should be traversed.")
        }
        "agent.toolCatalog.tools.fs.list.params.maxDepth" => {
            Some("Maximum recursive depth when traversal is enabled.")
        }
        "agent.toolCatalog.tools.fs.list.params.includeHidden" => {
            Some("Whether hidden files and directories should be included.")
        }
        "agent.toolCatalog.tools.fs.list.params.filesOnly" => {
            Some("Whether only file entries should be returned.")
        }
        "agent.toolCatalog.tools.fs.list.params.directoriesOnly" => {
            Some("Whether only directory entries should be returned.")
        }
        "agent.toolCatalog.tools.fs.stat.name" => Some("Stat path"),
        "agent.toolCatalog.tools.fs.stat.description" => {
            Some("Read metadata for one local file system path.")
        }
        "agent.toolCatalog.tools.fs.stat.sampleResultSummary" => {
            Some("Returns existence, type, size, and readonly metadata.")
        }
        "agent.toolCatalog.tools.fs.stat.params.path" => Some(
            "Path to inspect, absolute, relative to the workspace root, or a ~ home-directory path.",
        ),
        "agent.toolCatalog.tools.fs.stat.params.followSymlink" => {
            Some("Whether symlink targets should be followed before reading metadata.")
        }
        "agent.toolCatalog.tools.search.grep.name" => Some("Search text"),
        "agent.toolCatalog.tools.search.grep.description" => {
            Some("Search file contents with ripgrep inside a local root.")
        }
        "agent.toolCatalog.tools.search.grep.sampleResultSummary" => {
            Some("Returns matched lines with file and line prefixes.")
        }
        "agent.toolCatalog.tools.search.grep.params.pattern" => {
            Some("Pattern text passed to ripgrep.")
        }
        "agent.toolCatalog.tools.search.grep.params.root" => Some(
            "Optional search root directory. Absolute, relative, or ~ home-directory paths are allowed. Uses the current workspace root when omitted.",
        ),
        "agent.toolCatalog.tools.search.grep.params.glob" => {
            Some("Optional glob filters forwarded to ripgrep.")
        }
        "agent.toolCatalog.tools.search.grep.params.caseSensitive" => {
            Some("Whether the search should be case-sensitive.")
        }
        "agent.toolCatalog.tools.search.grep.params.word" => {
            Some("Whether matches must align to word boundaries.")
        }
        "agent.toolCatalog.tools.search.grep.params.contextLines" => {
            Some("Optional number of context lines around each match.")
        }
        "agent.toolCatalog.tools.search.grep.params.maxResults" => {
            Some("Maximum number of matches ripgrep should return.")
        }
        "agent.toolCatalog.tools.search.glob.name" => Some("Find files"),
        "agent.toolCatalog.tools.search.glob.description" => {
            Some("Find file paths with ripgrep glob filters inside a local root.")
        }
        "agent.toolCatalog.tools.search.glob.sampleResultSummary" => {
            Some("Returns matching relative file paths.")
        }
        "agent.toolCatalog.tools.search.glob.params.patterns" => {
            Some("One or more glob patterns used to filter file paths.")
        }
        "agent.toolCatalog.tools.search.glob.params.root" => Some(
            "Optional search root directory. Absolute, relative, or ~ home-directory paths are allowed. Uses the current workspace root when omitted.",
        ),
        "agent.toolCatalog.tools.search.glob.params.includeHidden" => {
            Some("Whether hidden files should be included.")
        }
        "agent.toolCatalog.tools.search.glob.params.maxResults" => {
            Some("Maximum number of file paths to return.")
        }
        "agent.toolCatalog.tools.search.path.name" => Some("Find paths"),
        "agent.toolCatalog.tools.search.path.description" => {
            Some("Find file and directory paths with wildcard patterns inside a local root.")
        }
        "agent.toolCatalog.tools.search.path.sampleResultSummary" => {
            Some("Returns matching relative paths with file type metadata.")
        }
        "agent.toolCatalog.tools.search.path.params.patterns" => {
            Some("One or more wildcard patterns used to match path names or relative paths.")
        }
        "agent.toolCatalog.tools.search.path.params.root" => Some(
            "Optional search root directory. Absolute, relative, or ~ home-directory paths are allowed. Uses the current workspace root when omitted.",
        ),
        "agent.toolCatalog.tools.search.path.params.entryType" => {
            Some("Which path types to return. Allowed values: any, file, directory.")
        }
        "agent.toolCatalog.tools.search.path.params.includeHidden" => {
            Some("Whether hidden files and directories should be included.")
        }
        "agent.toolCatalog.tools.search.path.params.maxDepth" => {
            Some("Maximum recursive depth to traverse from the root directory.")
        }
        "agent.toolCatalog.tools.search.path.params.maxResults" => {
            Some("Maximum number of matching paths to return.")
        }
        _ => None,
    }
}

pub(super) fn translate_zh_cn(key: &str) -> Option<&'static str> {
    match key {
        "agent.toolCatalog.tools.fs.apply_patch.params.patch" => Some(
            "Codex 风格语法：以 *** Begin Patch 开始，使用 *** Add File: 路径 和 +内容新增；*** Delete File: 路径 删除；*** Update File: 路径 修改，可跟 *** Move to: 新路径 移动，再写 @@ 补丁块。块内空格开头表示上下文，- 删除，+ 新增。@@ 锚点跳过唯一匹配行，*** End of File 将末块限定在文件尾。以 *** End Patch 结束。上下文须精确唯一，路径不含 ..，最多 100 个文件。",
        ),
        "agent.toolCatalog.tools.fs.apply_patch.params.cwd" => {
            Some("补丁基准目录，默认当前选中的应用或 worktree。")
        }
        "agent.toolCatalog.tools.fs.apply_patch.params.dryRun" => {
            Some("校验并返回差异，不修改文件。")
        }
        "agent.toolCatalog.tools.fs.apply_patch.params.expectedCount" => {
            Some("单文件编辑：预期总匹配次数；未指定 replaceAll 时目标必须唯一。")
        }
        "agent.toolCatalog.domains.bash.name" => Some("命令行工具"),
        "agent.toolCatalog.domains.bash.description" => Some(
            "执行非交互命令；耗时命令会转为后台任务。交互程序和长期运行的服务才使用 Agent 终端。",
        ),
        "agent.toolCatalog.domains.chat_attachment.name" => Some("会话附件工具"),
        "agent.toolCatalog.domains.chat_attachment.description" => {
            Some("对会话附件进行分块文本读取和完整 JSON 解析。")
        }
        "agent.toolCatalog.domains.fs.name" => Some("文件工具"),
        "agent.toolCatalog.domains.fs.description" => {
            Some("读取、写入、检查并修补本地工作区文件。")
        }
        "agent.toolCatalog.domains.search.name" => Some("搜索工具"),
        "agent.toolCatalog.domains.search.description" => {
            Some("在本地工作区内搜索文件和源码文本。")
        }
        "agent.toolCatalog.tools.bash.exec.name" => Some("执行 Shell 命令"),
        "agent.toolCatalog.tools.bash.exec.description" => Some(
            "执行非交互命令并分别捕获 stdout 和 stderr。完整命令行直接放进 command；仅需逐项传参时才使用 args。最多等待 10 秒，仍在运行时用 bash_task_read 查询。交互程序和长期运行的服务使用 terminal_start；不要用 nohup 或 shell 后台符号脱离终端。",
        ),
        "agent.toolCatalog.tools.bash.exec.sampleResultSummary" => {
            Some("返回 stdout、stderr、退出状态，以及运行中的 taskId。")
        }
        "agent.toolCatalog.tools.bash.exec.params.command" => Some(
            "完整命令行，如 npm run build 或 bash -c 'echo ok'；提供 args 时这里只填写程序名或路径。",
        ),
        "agent.toolCatalog.tools.bash.exec.params.args" => {
            Some("可选的逐项参数。传完整命令行时省略；填写后 command 只能是程序名或路径。")
        }
        "agent.toolCatalog.tools.bash.exec.params.cwd" => {
            Some("可选工作目录；可填写绝对路径、相对路径或 `~` 家目录路径。")
        }
        "agent.toolCatalog.tools.bash.exec.params.stdin" => {
            Some("可选标准输入文本，会写入子进程 stdin。")
        }
        "agent.toolCatalog.tools.bash.exec.params.env" => {
            Some("可选环境变量，会附加到子进程环境。")
        }
        "agent.toolCatalog.tools.bash.task_read.name" => Some("读取后台命令任务"),
        "agent.toolCatalog.tools.bash.task_read.description" => Some(
            "查询 bash_exec 等待 10 秒后转到后台的命令，返回进程状态、退出码和增量输出。继续传入 nextStdoutCursor 和 nextStderrCursor，直到 running 为 false 且 outputClosed 为 true；避免频繁轮询。",
        ),
        "agent.toolCatalog.tools.bash.task_read.sampleResultSummary" => {
            Some("返回命令任务状态、退出码及增量 stdout/stderr。")
        }
        "agent.toolCatalog.tools.bash.task_read.params.taskId" => {
            Some("bash_exec 在命令仍运行时返回的 taskId。")
        }
        "agent.toolCatalog.tools.bash.task_read.params.stdoutCursor" => {
            Some("stdout 的下一字节偏移；传入上次结果中的 nextStdoutCursor。")
        }
        "agent.toolCatalog.tools.bash.task_read.params.stderrCursor" => {
            Some("stderr 的下一字节偏移；传入上次结果中的 nextStderrCursor。")
        }
        "agent.toolCatalog.tools.bash.task_stop.name" => Some("停止后台命令任务"),
        "agent.toolCatalog.tools.bash.task_stop.description" => {
            Some("终止仍在后台运行的命令。需要收集最终输出时，之后调用 bash_task_read。")
        }
        "agent.toolCatalog.tools.bash.task_stop.sampleResultSummary" => {
            Some("返回后台命令是否已停止。")
        }
        "agent.toolCatalog.tools.bash.task_stop.params.taskId" => Some("bash_exec 返回的 taskId。"),
        "agent.toolCatalog.tools.fs.read_text.name" => Some("读取文本文件"),
        "agent.toolCatalog.tools.fs.read_text.description" => {
            Some("读取已关联项目中的 UTF-8 文件或当前会话附件，可按行范围截取。")
        }
        "agent.toolCatalog.tools.fs.read_text.sampleResultSummary" => {
            Some("返回文件内容以及实际读取的行范围。")
        }
        "agent.toolCatalog.tools.fs.read_text.params.path" => {
            Some("兼容旧版的项目文件路径。优先使用 fileRef，它也支持读取会话附件。")
        }
        "agent.toolCatalog.tools.fs.read_text.params.startLine" => Some("可选起始行，1 开始。"),
        "agent.toolCatalog.tools.fs.read_text.params.endLine" => {
            Some("可选结束行，按 1 开始且包含该行。")
        }
        "agent.toolCatalog.tools.fs.read_text.params.maxBytes" => {
            Some("按行截取后最多返回的字节数。")
        }
        "agent.toolCatalog.tools.chat_attachment.read_text.name" => Some("读取会话附件文本"),
        "agent.toolCatalog.tools.chat_attachment.read_text.description" => {
            Some("将会话附件按受限大小读取为 UTF-8 文本块；附件不是工作区文件。")
        }
        "agent.toolCatalog.tools.chat_attachment.read_text.sampleResultSummary" => {
            Some("返回一个文本块以及下一块所需的字节和行游标。")
        }
        "agent.toolCatalog.tools.chat_attachment.read_text.params.attachmentCode" => {
            Some("聊天附件上下文中的会话附件编码。")
        }
        "agent.toolCatalog.tools.chat_attachment.read_text.params.startByte" => {
            Some("本次读取的字节偏移；后续使用上一次结果中的 nextStartByte。")
        }
        "agent.toolCatalog.tools.chat_attachment.read_text.params.maxBytes" => {
            Some("本次最多读取的字节数；服务端上限为 262144 字节。")
        }
        "agent.toolCatalog.tools.chat_attachment.parse_json.name" => Some("解析会话附件 JSON"),
        "agent.toolCatalog.tools.chat_attachment.parse_json.description" => Some(
            "解析项目文件或会话附件中的 JSON，返回文档统计，并可通过 JSON Pointer 查看指定节点。",
        ),
        "agent.toolCatalog.tools.chat_attachment.parse_json.sampleResultSummary" => {
            Some("返回完整 JSON 文档统计和受限大小的指定节点值。")
        }
        "agent.toolCatalog.tools.chat_attachment.parse_json.params.attachmentCode" => {
            Some("兼容旧版的会话附件编号。优先用 fileRef 明确指定文件来源。")
        }
        "agent.toolCatalog.tools.chat_attachment.parse_json.params.jsonPath" => {
            Some("可选 RFC 6901 JSON Pointer，例如 /topics/0；为空时选择整个文档根节点。")
        }
        "agent.toolCatalog.tools.chat_attachment.parse_json.params.maxValueBytes" => {
            Some("指定 JSON 节点最多返回的序列化字节数；完整文档仍会被解析。")
        }
        "agent.toolCatalog.tools.fs.write_text.name" => Some("写入文本文件"),
        "agent.toolCatalog.tools.fs.write_text.description" => {
            Some("创建、覆盖或追加写入本地文本文件。")
        }
        "agent.toolCatalog.tools.fs.write_text.sampleResultSummary" => {
            Some("返回写入后的文件路径、模式和字节数。")
        }
        "agent.toolCatalog.tools.fs.write_text.params.path" => {
            Some("目标文件路径，可为绝对路径、相对工作区根目录路径，或 `~` 家目录路径。")
        }
        "agent.toolCatalog.tools.fs.write_text.params.content" => Some("要写入的文本内容。"),
        "agent.toolCatalog.tools.fs.write_text.params.mode" => {
            Some("写入模式，可选值：create、overwrite、append。")
        }
        "agent.toolCatalog.tools.fs.write_text.params.ensureParentDirs" => {
            Some("写入前是否自动创建缺失的父目录。")
        }
        "agent.toolCatalog.tools.fs.apply_patch.name" => Some("应用代码补丁"),
        "agent.toolCatalog.tools.fs.apply_patch.description" => Some(
            "使用上下文补丁一次新增、修改、删除或移动多个代码文件；全部校验后再写入，dryRun 可预览。兼容 path/action/target 单文件文本编辑，拒绝歧义匹配。",
        ),
        "agent.toolCatalog.tools.fs.apply_patch.sampleResultSummary" => {
            Some("返回变更路径和限定长度的差异；dryRun 不写文件。")
        }
        "agent.toolCatalog.tools.fs.apply_patch.params.path" => {
            Some("目标文件路径，可为绝对路径、相对工作区根目录路径，或 `~` 家目录路径。")
        }
        "agent.toolCatalog.tools.fs.apply_patch.params.action" => {
            Some("修补动作，可选值：replace、delete、insert_before、insert_after。")
        }
        "agent.toolCatalog.tools.fs.apply_patch.params.target" => {
            Some("修补锚点文本，用来定位替换或插入位置。")
        }
        "agent.toolCatalog.tools.fs.apply_patch.params.content" => {
            Some("根据修补动作作为替换文本或插入文本。")
        }
        "agent.toolCatalog.tools.fs.apply_patch.params.replaceAll" => {
            Some("单文件编辑：显式处理所有匹配项；建议用 expectedCount 校验次数。")
        }
        "agent.toolCatalog.tools.fs.list.name" => Some("列出文件"),
        "agent.toolCatalog.tools.fs.list.description" => Some("列出本地路径下的文件和目录。"),
        "agent.toolCatalog.tools.fs.list.sampleResultSummary" => {
            Some("返回目录项以及文件类型元数据。")
        }
        "agent.toolCatalog.tools.fs.list.params.path" => {
            Some("要查看的目录路径，可为绝对路径、相对工作区根目录路径，或 `~` 家目录路径。")
        }
        "agent.toolCatalog.tools.fs.list.params.recursive" => Some("是否递归遍历子目录。"),
        "agent.toolCatalog.tools.fs.list.params.maxDepth" => Some("启用递归时允许的最大深度。"),
        "agent.toolCatalog.tools.fs.list.params.includeHidden" => Some("是否包含隐藏文件和目录。"),
        "agent.toolCatalog.tools.fs.list.params.filesOnly" => Some("是否只返回文件项。"),
        "agent.toolCatalog.tools.fs.list.params.directoriesOnly" => Some("是否只返回目录项。"),
        "agent.toolCatalog.tools.fs.stat.name" => Some("查看路径元数据"),
        "agent.toolCatalog.tools.fs.stat.description" => Some("读取单个本地文件系统路径的元数据。"),
        "agent.toolCatalog.tools.fs.stat.sampleResultSummary" => {
            Some("返回是否存在、类型、大小和只读状态等元数据。")
        }
        "agent.toolCatalog.tools.fs.stat.params.path" => {
            Some("要检查的路径，可为绝对路径、相对工作区根目录路径，或 `~` 家目录路径。")
        }
        "agent.toolCatalog.tools.fs.stat.params.followSymlink" => {
            Some("读取元数据前是否跟随符号链接目标。")
        }
        "agent.toolCatalog.tools.search.grep.name" => Some("搜索文本"),
        "agent.toolCatalog.tools.search.grep.description" => {
            Some("在本地根目录内使用 ripgrep 搜索文件内容。")
        }
        "agent.toolCatalog.tools.search.grep.sampleResultSummary" => {
            Some("返回带文件和行号前缀的命中文本。")
        }
        "agent.toolCatalog.tools.search.grep.params.pattern" => {
            Some("传给 ripgrep 的搜索模式文本。")
        }
        "agent.toolCatalog.tools.search.grep.params.root" => Some(
            "可选搜索根目录；可填写绝对路径、相对路径或 `~` 家目录路径；缺省时使用当前工作区根目录。",
        ),
        "agent.toolCatalog.tools.search.grep.params.glob" => {
            Some("可选 glob 过滤条件，会透传给 ripgrep。")
        }
        "agent.toolCatalog.tools.search.grep.params.caseSensitive" => Some("搜索是否区分大小写。"),
        "agent.toolCatalog.tools.search.grep.params.word" => Some("命中是否必须满足单词边界。"),
        "agent.toolCatalog.tools.search.grep.params.contextLines" => {
            Some("每个命中周围额外返回的上下文行数。")
        }
        "agent.toolCatalog.tools.search.grep.params.maxResults" => {
            Some("ripgrep 最多返回的命中数量。")
        }
        "agent.toolCatalog.tools.search.glob.name" => Some("查找文件"),
        "agent.toolCatalog.tools.search.glob.description" => {
            Some("在本地根目录内使用 ripgrep 的 glob 过滤查找文件路径。")
        }
        "agent.toolCatalog.tools.search.glob.sampleResultSummary" => {
            Some("返回命中的相对文件路径列表。")
        }
        "agent.toolCatalog.tools.search.glob.params.patterns" => {
            Some("一个或多个用于过滤文件路径的 glob 模式。")
        }
        "agent.toolCatalog.tools.search.glob.params.root" => Some(
            "可选搜索根目录；可填写绝对路径、相对路径或 `~` 家目录路径；缺省时使用当前工作区根目录。",
        ),
        "agent.toolCatalog.tools.search.glob.params.includeHidden" => Some("是否包含隐藏文件。"),
        "agent.toolCatalog.tools.search.glob.params.maxResults" => Some("最多返回的文件路径数量。"),
        "agent.toolCatalog.tools.search.path.name" => Some("查找路径"),
        "agent.toolCatalog.tools.search.path.description" => {
            Some("在本地根目录内按通配模式查找文件和目录路径。")
        }
        "agent.toolCatalog.tools.search.path.sampleResultSummary" => {
            Some("返回命中的相对路径以及文件类型元数据。")
        }
        "agent.toolCatalog.tools.search.path.params.patterns" => {
            Some("一个或多个用于匹配路径名或相对路径的通配模式。")
        }
        "agent.toolCatalog.tools.search.path.params.root" => Some(
            "可选搜索根目录；可填写绝对路径、相对路径或 `~` 家目录路径。省略时使用当前工作区根目录。",
        ),
        "agent.toolCatalog.tools.search.path.params.entryType" => {
            Some("要返回的路径类型；可选值：any、file、directory。")
        }
        "agent.toolCatalog.tools.search.path.params.includeHidden" => {
            Some("是否包含隐藏文件和目录。")
        }
        "agent.toolCatalog.tools.search.path.params.maxDepth" => {
            Some("从根目录开始递归遍历的最大深度。")
        }
        "agent.toolCatalog.tools.search.path.params.maxResults" => Some("最多返回的命中路径数量。"),
        _ => None,
    }
}
