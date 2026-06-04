pub(super) fn translate_en_us(key: &str) -> Option<&'static str> {
    match key {
        "agent.toolCatalog.domains.bash.name" => Some("Bash Tools"),
        "agent.toolCatalog.domains.bash.description" => {
            Some("Run shell fallback commands with structured arguments.")
        }
        "agent.toolCatalog.domains.fs.name" => Some("Filesystem Tools"),
        "agent.toolCatalog.domains.fs.description" => {
            Some("Read, write, inspect, and patch local workspace files.")
        }
        "agent.toolCatalog.domains.search.name" => Some("Search Tools"),
        "agent.toolCatalog.domains.search.description" => {
            Some("Search files and source text inside the local workspace.")
        }
        "agent.toolCatalog.tools.bash.exec.name" => Some("Run command"),
        "agent.toolCatalog.tools.bash.exec.description" => {
            Some("Run one shell command directly when structured tools are not enough.")
        }
        "agent.toolCatalog.tools.bash.exec.sampleResultSummary" => {
            Some("Returns command stdout, stderr, and exit status.")
        }
        "agent.toolCatalog.tools.bash.exec.params.command" => {
            Some("Executable name or absolute path to run. Use bash with -c when shell parsing is required.")
        }
        "agent.toolCatalog.tools.bash.exec.params.args" => {
            Some("Structured command arguments passed as-is. Use bash -c or -lc when you need shell parsing, expansion, pipes, or redirects.")
        }
        "agent.toolCatalog.tools.bash.exec.params.cwd" => {
            Some("Optional working directory. Absolute, relative, or ~ home-directory paths are allowed.")
        }
        "agent.toolCatalog.tools.bash.exec.params.stdin" => {
            Some("Optional text piped to the process standard input.")
        }
        "agent.toolCatalog.tools.bash.exec.params.env" => {
            Some("Optional environment variables added to the child process.")
        }
        "agent.toolCatalog.tools.fs.read_text.name" => Some("Read text file"),
        "agent.toolCatalog.tools.fs.read_text.description" => {
            Some("Read text from a local file, optionally by line range.")
        }
        "agent.toolCatalog.tools.fs.read_text.sampleResultSummary" => {
            Some("Returns file content and the materialized line range.")
        }
        "agent.toolCatalog.tools.fs.read_text.params.path" => {
            Some("File path to read, absolute, relative to the workspace root, or a ~ home-directory path.")
        }
        "agent.toolCatalog.tools.fs.read_text.params.startLine" => {
            Some("Optional one-based start line.")
        }
        "agent.toolCatalog.tools.fs.read_text.params.endLine" => {
            Some("Optional one-based inclusive end line.")
        }
        "agent.toolCatalog.tools.fs.read_text.params.maxBytes" => {
            Some("Optional maximum number of bytes to return after slicing.")
        }
        "agent.toolCatalog.tools.fs.write_text.name" => Some("Write text file"),
        "agent.toolCatalog.tools.fs.write_text.description" => {
            Some("Create, overwrite, or append a local text file.")
        }
        "agent.toolCatalog.tools.fs.write_text.sampleResultSummary" => {
            Some("Returns the written file path, mode, and byte count.")
        }
        "agent.toolCatalog.tools.fs.write_text.params.path" => {
            Some("Target file path, absolute, relative to the workspace root, or a ~ home-directory path.")
        }
        "agent.toolCatalog.tools.fs.write_text.params.content" => {
            Some("Text content to write.")
        }
        "agent.toolCatalog.tools.fs.write_text.params.mode" => {
            Some("Write mode. Allowed values: create, overwrite, append.")
        }
        "agent.toolCatalog.tools.fs.write_text.params.ensureParentDirs" => {
            Some("Whether missing parent directories should be created before writing.")
        }
        "agent.toolCatalog.tools.fs.apply_patch.name" => Some("Patch text file"),
        "agent.toolCatalog.tools.fs.apply_patch.description" => {
            Some("Apply a bounded text patch operation to one local file.")
        }
        "agent.toolCatalog.tools.fs.apply_patch.sampleResultSummary" => {
            Some("Returns the target file, action, and replacement count.")
        }
        "agent.toolCatalog.tools.fs.apply_patch.params.path" => {
            Some("Target file path, absolute, relative to the workspace root, or a ~ home-directory path.")
        }
        "agent.toolCatalog.tools.fs.apply_patch.params.action" => {
            Some("Patch action. Allowed values: replace, delete, insert_before, insert_after.")
        }
        "agent.toolCatalog.tools.fs.apply_patch.params.target" => {
            Some("Target text that anchors the patch operation.")
        }
        "agent.toolCatalog.tools.fs.apply_patch.params.content" => {
            Some("Replacement or inserted text, depending on the patch action.")
        }
        "agent.toolCatalog.tools.fs.apply_patch.params.replaceAll" => {
            Some("Whether the patch should affect every matching target.")
        }
        "agent.toolCatalog.tools.fs.list.name" => Some("List files"),
        "agent.toolCatalog.tools.fs.list.description" => {
            Some("List files and directories under a local path.")
        }
        "agent.toolCatalog.tools.fs.list.sampleResultSummary" => {
            Some("Returns directory entries with file type metadata.")
        }
        "agent.toolCatalog.tools.fs.list.params.path" => {
            Some("Directory path to inspect, absolute, relative to the workspace root, or a ~ home-directory path.")
        }
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
        "agent.toolCatalog.tools.fs.stat.params.path" => {
            Some("Path to inspect, absolute, relative to the workspace root, or a ~ home-directory path.")
        }
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
        "agent.toolCatalog.tools.search.grep.params.root" => {
            Some("Optional search root directory. Absolute, relative, or ~ home-directory paths are allowed. Uses the current workspace root when omitted.")
        }
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
        "agent.toolCatalog.tools.search.glob.params.root" => {
            Some("Optional search root directory. Absolute, relative, or ~ home-directory paths are allowed. Uses the current workspace root when omitted.")
        }
        "agent.toolCatalog.tools.search.glob.params.includeHidden" => {
            Some("Whether hidden files should be included.")
        }
        "agent.toolCatalog.tools.search.glob.params.maxResults" => {
            Some("Maximum number of file paths to return.")
        }
        _ => None,
    }
}

pub(super) fn translate_zh_cn(key: &str) -> Option<&'static str> {
    match key {
        "agent.toolCatalog.domains.bash.name" => Some("命令行工具"),
        "agent.toolCatalog.domains.bash.description" => {
            Some("以结构化参数执行 shell 兜底命令。")
        }
        "agent.toolCatalog.domains.fs.name" => Some("文件工具"),
        "agent.toolCatalog.domains.fs.description" => {
            Some("读取、写入、检查并修补本地工作区文件。")
        }
        "agent.toolCatalog.domains.search.name" => Some("搜索工具"),
        "agent.toolCatalog.domains.search.description" => {
            Some("在本地工作区内搜索文件和源码文本。")
        }
        "agent.toolCatalog.tools.bash.exec.name" => Some("执行命令"),
        "agent.toolCatalog.tools.bash.exec.description" => {
            Some("在结构化工具不够用时直接执行一条 shell 命令。")
        }
        "agent.toolCatalog.tools.bash.exec.sampleResultSummary" => {
            Some("返回命令的 stdout、stderr 和退出状态。")
        }
        "agent.toolCatalog.tools.bash.exec.params.command" => Some("要执行的程序名或绝对路径；需要 shell 解析时请使用 bash 并传入 -c。"),
        "agent.toolCatalog.tools.bash.exec.params.args" => {
            Some("按参数数组传入的结构化命令参数，不经过 shell 解析；需要 shell 解析、展开、管道或重定向时请使用 bash -c 或 -lc。")
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
        "agent.toolCatalog.tools.fs.read_text.name" => Some("读取文本文件"),
        "agent.toolCatalog.tools.fs.read_text.description" => {
            Some("读取本地文本文件，可按行范围截取。")
        }
        "agent.toolCatalog.tools.fs.read_text.sampleResultSummary" => {
            Some("返回文件内容以及实际读取的行范围。")
        }
        "agent.toolCatalog.tools.fs.read_text.params.path" => {
            Some("要读取的文件路径，可为绝对路径、相对工作区根目录路径，或 `~` 家目录路径。")
        }
        "agent.toolCatalog.tools.fs.read_text.params.startLine" => Some("可选起始行，1 开始。"),
        "agent.toolCatalog.tools.fs.read_text.params.endLine" => Some("可选结束行，按 1 开始且包含该行。"),
        "agent.toolCatalog.tools.fs.read_text.params.maxBytes" => {
            Some("按行截取后最多返回的字节数。")
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
        "agent.toolCatalog.tools.fs.apply_patch.name" => Some("修补文本文件"),
        "agent.toolCatalog.tools.fs.apply_patch.description" => {
            Some("对单个本地文件执行受限文本修补操作。")
        }
        "agent.toolCatalog.tools.fs.apply_patch.sampleResultSummary" => {
            Some("返回目标文件、修补动作和替换次数。")
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
            Some("是否对所有命中的锚点都执行修补。")
        }
        "agent.toolCatalog.tools.fs.list.name" => Some("列出文件"),
        "agent.toolCatalog.tools.fs.list.description" => {
            Some("列出本地路径下的文件和目录。")
        }
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
        "agent.toolCatalog.tools.search.grep.params.pattern" => Some("传给 ripgrep 的搜索模式文本。"),
        "agent.toolCatalog.tools.search.grep.params.root" => {
            Some("可选搜索根目录；可填写绝对路径、相对路径或 `~` 家目录路径；缺省时使用当前工作区根目录。")
        }
        "agent.toolCatalog.tools.search.grep.params.glob" => {
            Some("可选 glob 过滤条件，会透传给 ripgrep。")
        }
        "agent.toolCatalog.tools.search.grep.params.caseSensitive" => Some("搜索是否区分大小写。"),
        "agent.toolCatalog.tools.search.grep.params.word" => Some("命中是否必须满足单词边界。"),
        "agent.toolCatalog.tools.search.grep.params.contextLines" => Some("每个命中周围额外返回的上下文行数。"),
        "agent.toolCatalog.tools.search.grep.params.maxResults" => Some("ripgrep 最多返回的命中数量。"),
        "agent.toolCatalog.tools.search.glob.name" => Some("查找文件"),
        "agent.toolCatalog.tools.search.glob.description" => {
            Some("在本地根目录内使用 ripgrep 的 glob 过滤查找文件路径。")
        }
        "agent.toolCatalog.tools.search.glob.sampleResultSummary" => Some("返回命中的相对文件路径列表。"),
        "agent.toolCatalog.tools.search.glob.params.patterns" => {
            Some("一个或多个用于过滤文件路径的 glob 模式。")
        }
        "agent.toolCatalog.tools.search.glob.params.root" => {
            Some("可选搜索根目录；可填写绝对路径、相对路径或 `~` 家目录路径；缺省时使用当前工作区根目录。")
        }
        "agent.toolCatalog.tools.search.glob.params.includeHidden" => Some("是否包含隐藏文件。"),
        "agent.toolCatalog.tools.search.glob.params.maxResults" => Some("最多返回的文件路径数量。"),
        _ => None,
    }
}
