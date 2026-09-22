# 内部 Agent 的项目目录与指令

内部 React Agent 的文件、搜索、命令和新终端工具默认使用 `execution_scope.local_directory`，它来自主应用当前选中的目录/worktree。不会更改整个 Rust 服务的进程工作目录，因此不同会话不共享 `chdir` 状态。目录不可用时沿用现有报错机制，不回退到后端启动目录。

## 加载规则

每次准备内部 Agent run 时动态读取项目指令，进入 system 消息及短期记忆必带的 `prompt_prefix`，在现有“系统提示词”预览中可查看文件路径和正文。压缩历史不删除这个前缀。

同一 run 每次模型请求前重新检查当前目录和指令。文件修改、删除或切换 worktree 后替换旧指令，不追加重复内容；使用有状态 Responses 时重置续接状态，以完整新前缀重新发起请求。

从当前 Git/worktree 根目录到当前工作目录逐级加载；非 Git 目录只检查当前目录。每一级取第一个非空文件，优先级为：

1. `AGENTS.override.md`
2. `AGENTS.md`
3. `CLAUDE.md`
4. `.claude/CLAUDE.md`

文件正文合计限制为 32 KiB，超出时标明截断。项目指令应服从平台规则和用户明确要求，只在对应项目/目录生效，不能修改运行时授权和目录边界。不会自动加载其他关联应用、兄弟 worktree、所有子目录、README、用户全局设置或凭据文件；引用/import 由 Agent 按需读取。指向当前仓库外的符号链接被拒绝，已有文件读取失败会报错，不能悄悄当作没有规则。

## 参考与差异

- [Codex 项目指令](https://learn.chatgpt.com/docs/agent-configuration/agents-md)：参考逐级查找、override 优先、每级一个文件及大小限制。官方说明通常启动时构建指令链；Tron 在模型请求边界继续刷新，以适应会话内工作区切换。
- [Claude Code 项目记忆](https://code.claude.com/docs/en/memory)：参考项目指令和子目录按需读取的区分。这里兼容 CLAUDE 文件作为回退，不声称完整实现 Claude 的 imports、全局配置和 rules 机制。

## 验证

- `make be-check` 通过。
- `make be-test TEST="--test session_application_context --test short_term_memory_transcript"`：21 项会话/工具回归和 61 项短期记忆回归通过，共 82 项。
- 使用真实临时目录、Git worktree 和 `pwd` 进程验证默认目录、优先级、UTF-8 大文件上限、外部链接拒绝、文件更新/删除和必带前缀同步。
- 使用脚本模型捕获四次模型请求，验证切换到其他 worktree 后不带旧目录的指令，切回来重新加载；尚未用真实外部模型执行端到端对话。
