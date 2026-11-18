# 后端开发说明

修改后端源码前，阅读 [后端技术栈](docs/后端技术栈.md) 和 [后端编码规范](docs/spec/后端编码规范.md)。运行 `make be-check` 验证改动。

- 不编写任何后端测试代码，包括单元测试、集成测试和测试专用代码。
- 文件访问和命令执行使用主代理明确解析的工作目录：已选择的应用/worktree 优先，无项目会话和 PersonAI 使用各自持久的托管目录。子代理通过 workingDirectory 指定任务目录，省略时继承父目录。显式配置的工作目录不存在时应报错，不能退回后端进程目录。`default_branch` 只是配置，不能据此判断实际检出的 Git 分支。
- Agent 命令工具省略 `args` 时按命令字符串解释；显式传入 `args` 时按字面 argv 执行。普通命令先收集输出，超过 10 秒后返回任务 ID；PTY 用于交互式或持续运行的进程，由后端持有，不随前端 socket 或标签页关闭而丢失。
- 工作区文件和聊天附件通过共享的 `fileRef` 解析；文件导出必须限制在项目路径范围内。
- `chat_ask_user` 的答案在上一轮运行释放会话前保持待处理状态；续跑时把答案摘要持久化到对话记录。
- 创建 PersonAI Rust SDK 客户端时设置 `X-Product-Code`，让普通请求和 SSE 请求继承同一产品标识。
- `cargo fmt` 可能格式化无关文件；检查差异，只保留本次任务范围内的改动。

## 国际化（rust-i18n）

- 统一入口是 `crates/i18n`（包名 `tron-i18n`）：`tron_i18n::t!(key, locale = locale)`；业务 crate 不要直接依赖或调用 `rust_i18n::`。
- 所有面向用户的文本必须使用稳定、语义化的 Key，禁止在业务代码内硬编码文案，包括 `locale.text("中文", "English")`、`if english { "..." } else { "..." }` 以及按 `SupportedLocale` 分支写死的字符串。
- 翻译资源统一在 `locales/*.yml`；新增文案必须同时补齐 `zh-CN` 与 `en`，并在提交前运行 `make be-i18n-check`（CI 会执行同一检查）。
- 会话/请求相关文案显式传入 locale：请求用 `Accept-Language` 解析出的值，Agent 运行用 `AgentRunContext.locale` 或运行扩展里的 `tron_i18n::Locale`。不要依赖进程级全局语言设置。
- 风险识别、错误码、工具状态等基础能力返回结构化类型（如 `RiskReason`），展示层再翻译；语言判断不得进入领域逻辑。
