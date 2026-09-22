# Agent 代码编辑工具

`code_check` 和独立的 `code` 领域已删除。构建、测试继续通过已有 `bash_exec` 执行。

## 对照与改动

Codex 的核心文件编辑能力是 `apply_patch`。参考 [OpenAI Apply Patch 文档](https://developers.openai.com/api/docs/guides/tools-apply-patch) 和本次 Codex 会话提供的编辑工具，Tron 已有读取、搜索、终端、整文件写入，因此保留这些入口，升级原有 `fs_apply_patch`，避免再堆叠功能重复的工具。

| 能力 | 本次处理 |
| --- | --- |
| 读取文件、搜索文本/文件名 | 保留 `fs_read_text`、`search_grep`、`search_glob`、`search_path` |
| 多文件增删改、移动并修改 | 为 `fs_apply_patch` 新增 `patch` 模式 |
| 不依赖行号的修改定位 | 支持精确上下文、`@@` 锚点、文件尾定位 |
| 修改前查看差异 | `dryRun: true` 返回预览，不写文件 |
| 读取后文件发生变化 | 修改区域上下文仍匹配时继续，保留其他区域的新内容；上下文不匹配则报错并重新读取 |
| 单文件文本替换 | 兼容原调用参数；保留 `expectedCount`；非全量替换时拒绝重复目标 |
| 写到一半才发现后续补丁错误 | 先解析和校验全部文件，再暂存写入；写入异常时尝试回滚本次已写入文件 |

## 调用例子

以下 JSON 是 Agent 的工具调用参数，不是新增用户表单。

```json
{
  "patch": "*** Begin Patch\n*** Update File: src/main.rs\n@@\n-fn old() {}\n+fn new() {}\n*** Update File: old.rs\n*** Move to: src/renamed.rs\n*** Add File: notes.txt\n+Updated entry point.\n*** End Patch",
  "dryRun": true
}
```

预览是可选步骤。Agent 可直接提交补丁；需要预览时先设置 `dryRun: true`，实际应用时设置 `dryRun: false`。

按 Codex 的无哈希调用方式，工具不再提供 `expectedHashes`、`expectedHash` 或读写结果中的文件哈希。执行时读取当前文件，按补丁上下文定位修改；上下文不匹配则返回错误，供 Agent 重新读取和调整补丁。工具仍保留单次写入期间的并发检查，避免在准备和落盘之间覆盖检测到的修改。

补丁相对路径以当前应用/worktree 为基准，也可显式给出 `cwd`。每个修改块用 `@@` 开始，空格前缀表示保留上下文，`-` 删除，`+` 新增。`@@ 唯一锚点行` 从锚点之后定位；`*** End of File` 将末块限定在文件尾。这是 Codex 风格的严格上下文格式，不接受 Git unified diff 的行号头；匹配失败时让 Agent 重新读取，而不是模糊替换。

## 边界

- 保留 CRLF、已有文件末尾是否换行、可执行权限。新文件默认 LF。
- 拒绝二进制、文件符号链接、越界路径、包含 `..` 的路径、已存在的新增/移动目标和同次补丁的重叠路径。
- 上限为 100 个文件操作、单文件 8 MiB、补丁 4 MiB、工作集 32 MiB；预览 diff 上限 64 KiB，截断会明确标记。
- 整批校验失败不修改目标文件。文件写入使用临时文件替换；发生写入异常时尝试回滚，失败会报告具体路径。这不是跨文件的崩溃安全事务；外部程序同时修改文件时仍有竞争窗口，回滚不会覆盖检测到的并发修改。
- 未修改前端组件；名称、说明、新参数继续走现有工具详情页的 en-US / zh-CN 国际化。

## 验证

通过真实临时文件调用已注册工具，覆盖多文件增删改与移动、预览不写入、后续文件不匹配不留下前序变更、无关区域改动得以保留、重复目标、路径边界、CRLF、无末尾换行、UTF-8、可执行权限、符号链接、旧接口兼容和工具目录移除。工具领域发现和模型 schema 预算也纳入回归。

2026-09-22 定向回归：`session_application_context` 15 项、`tool_schema_budget` 10 项全部通过，共 25 项。通过 Makefile 的 `be-test` 入口执行，使用隔离 Cargo 缓存及 `cargo --offline --locked`，没有修改依赖版本。此结果仅代表上述定向回归，不代表全仓测试全部通过。
