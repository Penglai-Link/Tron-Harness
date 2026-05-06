# 产品名称：Agent Workspace Platform

这是一个面向企业内部研发团队的 AI Agent 工作空间管理平台，用于统一管理 Agent 执行任务时所需的 Workspace、App、Task、Chat、Agent、Model、Skill、MCP、Tool、Memory、Knowledge Graph、Variable、Observability、Output Resource 等资源。

产品整体风格应偏向「专业开发者工具 + 企业级 SaaS 控制台」。

这不是一个营销型 Landing Page，而是一个完整的企业级 Web App 管理后台。

---

# 1. 产品定位

## 1.1 产品目标

构建一个面向研发团队、AI Agent 平台团队、企业内部工具团队使用的 Agent 管理控制台。

平台用于管理：

- Workspace
- App
- Task
- Agent
- Chat
- Model
- Skill
- MCP Server
- Tool
- Variable
- Memory
- Knowledge Graph
- Output Resource
- Observability
- User
- Role
- Permission

核心目标是让用户能够在不同 Workspace 中管理 Agent 的执行上下文、任务流程、工具能力、模型配置、记忆数据、知识图谱和运行观测数据。

## 1.2 产品气质

整体气质应体现：

- 专业
- 克制
- 工程化
- 高信息密度
- 开发者友好
- 企业级 SaaS 控制台
- AI Agent 运行治理平台

视觉上应接近以下产品的组合气质：


- Codex App
- Claude Code App
- Vercel Dashboard
- VS Code
- GitHub
- GitHub Dark
- Apple System UI

## 1.3 核心产品逻辑
产品核心逻辑：所有数据（model/memory/obs/chats等等）都根据workspace隔离，workspace有父子继承关系，最多3层继承关系。
每个管理页面，都能看到自己的数据，是继承自父workspace还是当前workspace自己新创建的，还是当前workspace覆盖的。
```
进入 Workspace → 导入app -> 手动创建任务/从gitlab issue导入任务，绑定app → 分配给 Agent → 产生会话 → 调用 Model/Skill/Tool → 读写 Memory → 输出结果
```

```
进入 Workspace → 导入app -> 与主agent描述需求 -> 主agent输出plan -> 用户确认后，主agent创建任务，绑定app → 分配给 Agent → 产生会话 → 调用 Model/Skill/Tool → 读写 Memory → 输出结果
```

## 1.4 核心页面说明

登陆页面：背景是简单的粒子效果，点击登录后，正中间弹出登录框，需要用户输入用户名、密码和校验码

任务管理：一个敏捷任务看板界面，用于从创建到完成跟踪任务的整个流程。
任务卡片元数据：ID、标题、负责人、状态、进度、简单描述、优先级。点击任务卡片后，可以进入任务详情页面。

应用管理：应用就是一个git仓库或全新文件夹，能手动创建或从github、gitlab等平台同步应用，可以给应用打tag，比如前端还是后端，和主要开发语言。

会话管理：管理每个任务下的agent会话，或每个应用的agent会话

变量管理：用于录入和管理agent执行时会用到的各种环境变量、敏感凭据等。

技能管理： skills治理页面，支持手动创建skills，控制skill的启用停用，给agent分配可用的skill。

MCP管理： 可录入、启用、停用mcp的连接地址信息。

模型管理： 可录入、启用、停用各个vendor的大模型连接信息。

记忆管理：用于管理Agent或workspace或app的记忆，有一个数据密集型仪表板，然后还有记忆查询界面。

图谱管理：用于管理Agent或workspace或app的知识图谱，支持检索。

可观测：一个大屏，展示每个agent token的使用情况、调用链路等等

工作空间管理：用于管理员管理workspace

用户管理：用于管理员管理登录的用户，可导入或创建新的用户

权限管理：用于管理员针对用户、角色，管理数据权限、操作权限、菜单权限、按钮权限等

---

# 2. 视觉风格与主题

## 2.1 主题系统

平台支持多主题切换。

至少内置两个主题：

- Light Theme
- Dark Theme

用户可以在设置中切换主题。

主题切换应影响：

- 页面背景
- 卡片背景
- 表格背景
- 字体颜色
- 边框颜色
- 状态标签
- 图表颜色
- 代码块
- 输入框
- 弹窗
- 抽屉
- 侧边栏
- 顶部导航栏

---

## 2.2 Light Theme

浅色主题采用极简、克制、现代的 Apple 风格。

### 视觉要求

- 主色基于白色、浅灰、深灰和黑色
- 页面背景使用接近白色或极浅灰
- 卡片使用白色背景
- 边框使用浅灰色
- 使用大量留白
- 使用圆角卡片
- 使用柔和阴影
- 强调色低饱和
- 信息层级清晰
- 不要使用过多鲜艳颜色
- 不要使用强烈渐变
- 不要使用夸张装饰

### 适合的风格关键词

- Minimal
- Apple-like
- Clean
- Light
- Soft shadow
- Rounded cards
- Neutral palette
- Enterprise SaaS

---

## 2.3 Dark Theme

暗色主题采用偏开发者工具风格的工程化暗色体系。

### 视觉要求

- 背景基色为深灰、蓝黑色
- 使用低对比度灰阶进行信息分层
- 强调色使用冷色系
- 可以使用蓝色、青色、紫色作为强调色
- 不要使用纯黑作为大面积背景
- 不要使用高饱和霓虹色
- 不要使用刺眼高亮
- 适合长时间工作
- 表格、卡片、抽屉、弹窗需要有明确但克制的层级关系

### 参考风格

- VS Code
- GitHub Dark
- Linear Dark
- Vercel Dark
- Grafana Dark

---

# 3. 全局布局

## 3.1 基础布局

采用典型企业级 SaaS 控制台布局。

整体由以下区域组成：

- Top Bar
- Left Sidebar
- Main Content
- Right Context Toolbar
- Floating Main Agent Button

布局结构如下：

```text
┌─────────────────────────────────────────────────────────────┐
│ Top Bar：当前 Workspace / 全局搜索 / 设置 / 用户头像           │
├──────────────┬──────────────────────────────┬───────────────┤
│ Sidebar      │ Main Content                 │ Right Toolbar │
│              │                              │               │
│ Dashboard    │ 当前模块主视图                 │ Context Panel │
│ Tasks        │                              │ File Tree     │
│ Apps         │                              │ Page Preview  │
│ Chats        │                              │ Tool Calls    │
│ Variables    │                              │ Outputs       │
│ Models       │                              │ Resources     │
│ Skills       │                              │ Trace         │
│ MCP          │                              │ Node Detail   │
│ Memory       │                              │ Health        │
│ Graph        │                              │               │
│ Observability│                              │               │
│ Workspace    │                              │               │
│ Users        │                              │               │
│ Permissions  │                              │               │
└──────────────┴──────────────────────────────┴───────────────┘