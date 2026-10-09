# PersonAI、网页与媒体工具接入

Framework 提供工具实现；Tron 提供登录凭据、Agent 授权、领域发现和能力包开关；共享 UI 展示原生结果。

| 工具 | 能力 | 领域 | 能力包 |
| --- | --- | --- | --- |
| personai_search | PersonAI Web 搜索 | search | framework.personai |
| personai_generate_image | 异步提交图片生成 | media | framework.personai |
| personai_query_image | 查询任务状态、进度与图片 | media | framework.personai |
| personai_detect_text | AIGC 文本检测 | media | framework.personai |
| personai_detect_image | AIGC 图片检测，URL 或 Base64 输入 | media | framework.personai |
| web_fetch | 读取网页正文 | search | framework.web |
| media_read_image | 读取本地图片或会话附件 | media | framework.media |
| media_read_pdf | 读取 PDF 指定页文本 | media | framework.media |

这些工具进入统一工具目录，使用既有领域发现流程。自定义 Agent 仍需配置工具权限。关闭能力包后，后续运行不再注册该包的工具。

PersonAI 在首次实际调用时连接，复用已有模型和服务适配。运行捕获账号身份，旧账号调用不能改用新账号凭据。连接随 FrameworkHost 关闭。普通聊天准备不依赖远端发现成功。

媒体路径沿用运行目录授权；fileRef 交给宿主附件读取服务检查会话和工作区权限。两种输入互斥，读取和解码共享超时、取消与大小限制。PDF 工具提取文字，不承担 OCR 或页面渲染。

React、Vue 使用同一结果解析层展示图片、搜索来源、任务状态、检测置信度和原生文本。历史记录保留 content、structured 与 resultReference，不再只取文本而丢失媒体。原始 JSON 可展开和复制。

平台工具页展开领域后，可为每个工具选择「直接执行」「询问」「无权限」。询问支持 Safe、Low、Medium、High、Critical 五级确认阈值；本次调用的风险达到阈值才暂停审批。未配置的工具默认直接执行，保留原有行为。修改后需保存；保存失败保留草稿，可放弃改动或刷新已保存配置。

全局配置通过 GET / PUT `/api/v1/tools/permissions` 读取和整批保存，响应使用 `ApiResponse`。`data.permissions` 每项包含 `toolCode`、`permission`（`Run` / `Ask` / `No`）、`confirmLevel`（上述五级 PascalCase 值）。工具编码来自平台目录或宿主实际发现的原生工具描述；任意未知编码不能注册权限。配置持久化到 Tron SQLite。

Tron 能力包注册 Framework `ToolPolicy`，按可信工具身份、操作类型和实际参数识别风险，不接受模型自报风险。全局权限与 Agent 工具授权取交集；「无权限」既移除发现结果，也在执行时再次拒绝。关闭 Tron harness 工具包不会关闭权限检查。需要询问时走 Framework 原生审批与恢复流程，审批卡片展示风险和确认阈值；等待审批期间撤销权限，确认后也不能继续执行。

本地开发通过 make agent-deps 指向对应 Framework 和 UI 工作树，再构建共享 UI、执行 make fe-install、make be-check、make fe-check。真实远端验收需使用已登录账号调用，不应把本地模拟服务测试视为线上验收。

2026-10-10 权限验收：后端检查、前端构建、7 项页面与接口映射测试、4 项权限集成测试通过。集成测试覆盖 SQLite 重连与事务回滚、非法配置不影响已保存数据、动态工具目录和交互终端风险边界。Harness 原生循环测试覆盖批准后执行一次、拒绝不执行、阈值判断、审批期间撤权，以及 shell 包装危险命令的确认。

独立测试档案的浏览器验收使用登录态替身；权限读写走真实 Tron API 和 SQLite。已核对三档选择、五级风险下拉、保存后重新加载、失败保留草稿、放弃改动、桌面列宽与窄屏内部滚动。未调用真实 PersonAI 搜索、生成或检测服务。
