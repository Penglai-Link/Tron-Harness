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

本地开发通过 make agent-deps 指向对应 Framework 和 UI 工作树，再构建共享 UI、执行 make fe-install、make be-check、make fe-check。真实远端验收需使用已登录账号调用，不应把本地模拟服务测试视为线上验收。
