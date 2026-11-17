---
name: skill-creator
description: 创建、修改和评估 Tron 技能。用户要新建技能、改进已有技能、补充测试用例、运行评测、比较技能效果，或优化技能描述以提升触发准确率时使用。Also use when the user asks to create, edit, evaluate, or benchmark a skill.
metadata:
  version: 1.0.0
  upstream: https://github.com/anthropics/skills/tree/main/skills/skill-creator
---

# 技能创建

本技能是 Anthropic skill-creator 的 Tron 移植版，保留意图澄清、结构设计、测试与迭代方法，把 Claude Code、Claude.ai 和 Cowork 专属流程替换为 Tron 的工具、子智能体和技能审批。许可证见 [LICENSE.txt](LICENSE.txt)。

技能不是把一段提示词换个文件名。它是可复用的工作方法：模型命中描述后读取 `SKILL.md`，按需读取引用和脚本，稳定地产出可验证的结果。创建技能时始终围绕这个目标，不要只堆砌规则。

## 沟通

用户的技术背景差异很大。默认用普通语言说明你要做什么；只有用户自己使用“JSON”“断言”“基准”等术语时，才默认他熟悉这些概念。不确定时用一句话解释，不要因为用了术语就让用户猜。

## 工作循环

1. 明确技能要完成什么、什么时候触发、输出格式和验收方式。
2. 写出 `SKILL.md` 初稿，按需附带脚本、引用和资源。
3. 设计 2-3 个真实用户会提出的测试任务。
4. 同一轮启动带技能和基线两类子智能体，保存输出。
5. 让用户看真实输出，同时用可验证的检查项给出定量结果。
6. 根据反馈改进，重复测试，直到稳定或用户满意。
7. 交付完整制品；需要安装时走 Tron 的技能审批和安装流程。

先判断用户处在这一循环的哪一步。用户说“我想做一个 X 技能”时，从澄清意图开始；已经有一版草稿时，直接进入测试和改进；用户只想聊思路时，不要强行启动整套评测。

## Tron 运行环境

| 需求 | 做法 |
| --- | --- |
| 读取本技能 | `skill_load(name="skill-creator")` 取得正文和资源清单 |
| 读取引用、脚本、模板 | `skill_read_resource(name="skill-creator", path="references/schemas.md")` |
| 查看或修改用户项目 | `fs_list`、`search_glob`、`search_grep`、`fs_read_text`、`fs_write_text`、`fs_apply_patch` |
| 运行 Python、Node 或校验脚本 | `command_exec`；超过 10 秒会返回任务 ID，用 `command_exec_read` 继续读取 |
| 并行测试 | `agent_spawn` 设置 `background: true`，再用 `child_wait` 收集结果 |
| 处理审批或安装 | 走 Tron 技能中心的候选、验证和审批流程，不直接写注册目录 |

补充约束：

- 技能目录随能力包分发时是只读的。需要在项目里运行 `scripts/` 下的脚本时，先读取内容并复制到工作区，再执行副本；不要假设技能目录可写。
- 宿主可能按上下文预算只提供部分支持文件。只使用实际给出的文件；缺失的引用或脚本按正文原则处理，不要臆造其内容。
- 不要调用 `npx skills`、不要修改 `~/.tron/skills-registry`，也不要把制品直接放进 `.agents/skills`。这些路径由宿主管理。
- 当宿主提示要求输出 JSON 制品（例如记忆整理生成技能）时，严格按宿主协议返回 `files` 和 `behaviorChecks`，不要先安装，也不要输出 Markdown 包装。宿主会在独立暂存目录验证，通过后才进入审批。
- 子智能体的完成通知包含 `usage`、步骤数和工具调用数，但不一定包含可靠的实际耗时。不要编造计时；没有真实数据时省略或标注为估算。

## 创建技能

### 明确意图

先回答四个问题，缺少答案时向用户确认：

1. 这个技能让智能体做什么？
2. 用户会说什么、在什么情境下触发？
3. 期望的输出形式是什么？
4. 是否需要测试用例来证明它能工作？

文件转换、数据提取、代码生成、固定流程这类可客观验证的技能，默认建议配测试；写作风格、视觉设计这类主观输出，可以不强行做定量断言，但要给用户看真实样例。

### 访谈与研究

主动问清边界情况、输入输出格式、示例文件、成功标准和依赖。测试提示词要等到这些信息足够后再写。

需要查文档、找相似技能或核对最佳实践时，可以使用可用的 MCP；有子智能体时并行研究，没有就自己查。带着上下文去问用户，减少用户负担。

### 写 SKILL.md

技能目录的最小结构：

```text
skill-name/
├── SKILL.md           必需：frontmatter + 指令正文
├── scripts/           可选：确定性、重复性任务的可执行脚本
├── references/        可选：按需读取的长文档
└── assets/            可选：输出用模板、图标、字体等
```

frontmatter 规则：

- `name`：1-64 个字符，只允许小写 ASCII 字母、数字和连字符；不能以连字符开头或结尾，不能出现连续连字符；必须与目录名完全一致；生成的技能不能叫 `skill-creator`。
- `description`：1-1024 个字符，说明“做什么”和“什么时候用”；不要用尖括号；这是触发的主要依据，写得具体一点。
- `metadata.version`：可选，但必须是字符串（例如 `"1.0.0"`）。Tron 把 `metadata` 读成扁平字符串映射，写成 `1.0` 会变成数字并导致技能发现失败。
- 其他可用字段：`license`、`compatibility`、`allowed-tools`。

正文写法：

- 用祈使句，直接说要做什么。
- 先写最重要的流程和判断依据，再写细节。
- 需要长文档时拆到 `references/`，并在正文里说明“什么时候读哪个文件”。
- 确定性的重复工作放进 `scripts/`，不要让模型每次重新实现。
- 输出模板、样例文件放进 `assets/`。
- 不要在正文里复制大段会被引用的外部文档；给路径和读取条件。

渐进披露：

1. 元数据（`name` + `description`）始终在上下文中。
2. 命中后加载 `SKILL.md` 正文，尽量控制在 500 行以内。
3. `references/`、`scripts/`、`assets/` 按需读取或执行。

正文接近 500 行时，增加一层引用文件，而不是继续堆叠。长引用文件超过 300 行时，在开头加目录。

一个技能覆盖多个框架或领域时，按变体拆分引用文件：

```text
cloud-deploy/
├── SKILL.md           选择逻辑和通用流程
└── references/
    ├── aws.md
    ├── gcp.md
    └── azure.md
```

### 描述与触发

`description` 决定技能会不会被加载。写清楚具体对象、动作和典型用户说法，不要把“何时使用”藏在正文里。

模型常见的倾向是“触发不足”，所以描述可以稍微主动一些。例如不要只写“生成内部数据看板”，而写“生成内部数据看板。用户提到看板、数据可视化、内部指标，或想展示公司数据时使用，即使用户没有说出‘看板’这个词。”

但不要用关键词堆砌或夸张承诺换取触发。描述与实际能力不符比触发不足更糟。

### 安全与边界

技能不能包含恶意代码、凭据窃取、未授权访问或其他会让用户意外的内容。不要帮助创建误导性技能。角色扮演类技能可以存在，但它的目的必须对用户透明。

## 测试用例

初稿完成后，设计 2-3 个真实用户会说的测试提示，先给用户确认：

> 我准备先跑这几个测试，看看是否符合预期。要增加或替换吗？

保存到 `evals/evals.json`，此时只写提示和期望结果，不急着写检查项：

```json
{
  "skill_name": "example-skill",
  "evals": [
    {
      "id": 1,
      "prompt": "用户会实际输入的任务",
      "expected_output": "期望结果的中文描述",
      "files": []
    }
  ]
}
```

完整字段（包括评分用的 `expectations`）见 [references/schemas.md](references/schemas.md)。

## 运行与评估

把结果放在技能目录同级的 `<skill-name>-workspace/`，按迭代分目录：

```text
my-skill-workspace/
└── iteration-1/
    └── eval-0/
        ├── eval_metadata.json
        ├── with_skill/
        │   └── run-1/
        │       ├── outputs/
        │       ├── grading.json
        │       └── timing.json
        └── baseline/
            └── run-1/
                ├── outputs/
                ├── grading.json
                └── timing.json
```

目录按需创建，不要一开始铺满整个结构。

### 第一步：同一轮启动全部运行

每个测试用例同时启动两个子智能体：一个带技能，一个不带技能。不要先跑带技能的，再回头补基线；同时启动才能让结果在同一时间段完成，减少环境差异。

带技能运行的任务模板：

```text
执行这个任务：
- 技能路径：<path-to-skill>
- 任务：<eval prompt>
- 输入文件：<eval files，没有就写“无”>
- 输出保存到：<workspace>/iteration-<N>/eval-<ID>/with_skill/run-1/outputs/
- 需要保存的输出：<用户真正关心的文件，例如“最终 CSV”“生成的 .docx”>
```

基线运行使用同一个任务提示：

- 新建技能：完全不提供技能，保存到 `baseline/run-1/outputs/`。
- 改进已有技能：先快照旧版本（`cp -r <skill-path> <workspace>/iteration-<N>/skill-snapshot/`），让基线指向旧版本，保存到 `baseline/run-1/outputs/`。

每个测试用例写一份 `eval_metadata.json`，检查项可以先留空：

```json
{
  "eval_id": 0,
  "eval_name": "描述性名称",
  "prompt": "用户任务",
  "expectations": []
}
```

### 第二步：运行期间起草检查项

不要干等子智能体。利用这段时间为每个测试用例写可客观验证的检查项，并向用户解释。

好的检查项有描述性名称，能区分“真的完成了”和“看起来完成了”。例如不要只写“文件存在”，而要写“输出 CSV 包含 12 行且金额列与输入一致”。主观技能（写作风格、设计质量）更适合人工评审，不要硬塞定量断言。

把检查项写回 `eval_metadata.json` 和 `evals/evals.json`。

### 第三步：记录用量和耗时

子智能体完成时，把 `child_wait` 返回的 `usage`、步骤数和工具调用数写入 `timing.json`：

```json
{
  "total_tokens": 84852,
  "total_duration_seconds": 23.3,
  "note": "耗时由运行前后时间估算"
}
```

没有真实耗时就不要写 `total_duration_seconds`。token、步骤和工具调用数才是可靠数据。

### 第四步：评分、聚合、查看

所有运行结束后：

1. **评分**：启动一个评分员子智能体，让它读取 [agents/grader.md](agents/grader.md)，逐条判断检查项并写入 `grading.json`。可以程序化验证的检查项优先写脚本，不要靠肉眼。`grading.json` 的检查项必须使用 `text`、`passed`、`evidence` 三个字段：

```json
{
  "expectations": [
    {
      "text": "输出包含用户要求的列",
      "passed": true,
      "evidence": "在 outputs/result.csv 第 1 行找到该列"
    }
  ],
  "summary": {"passed": 1, "failed": 0, "total": 1, "pass_rate": 1.0}
}
```

2. **聚合**：在技能目录运行：

```bash
python -m scripts.aggregate_benchmark <workspace>/iteration-1 --skill-name <name>
```

生成 `benchmark.json` 和 `benchmark.md`。如果需要手写 `benchmark.json`，字段见 [references/schemas.md](references/schemas.md)。

3. **分析**：读取聚合结果，指出均值可能掩盖的问题：总是通过的无效检查项、波动很大的测试、时间和 token 的取舍。分析视角见 [agents/analyzer.md](agents/analyzer.md) 的 “Analyzing Benchmark Results”。

4. **生成评审页**：

```bash
python <skill-creator-path>/eval-viewer/generate_review.py \
  <workspace>/iteration-1 \
  --skill-name "my-skill" \
  --benchmark <workspace>/iteration-1/benchmark.json
```

迭代 2 及以后加 `--previous-workspace <workspace>/iteration-<N-1>`。没有图形界面或无法打开浏览器时，加 `--static <output_path>` 生成独立 HTML，把路径交给用户。

5. **请用户评审**：说明页面有“输出”和“基准”两个视图，用户在页面上填写反馈后回到会话继续。

### 第五步：读取反馈

用户确认评审完成后，读取 `feedback.json`：

```json
{
  "reviews": [
    {"run_id": "eval-0-with_skill-run-1", "feedback": "缺少坐标轴标签", "timestamp": "..."},
    {"run_id": "eval-1-with_skill-run-1", "feedback": "", "timestamp": "..."}
  ],
  "status": "complete"
}
```

空反馈表示没有明显问题。把改进集中在用户明确指出的测试上，不要因为个别空反馈就重写整个技能。

查看器启动的是本地服务，结束后停止对应进程；使用 `--static` 时不需要停止服务。后台启动时记录 PID，评审结束后用 `kill <PID>` 或等价方式停止。

## 改进技能

改进的目标是让技能在大量不同提示下都能工作，而不是只在当前几个样例上好看。

1. **从反馈中提炼共性**。用户只看了几个样例，但技能要面对大量真实输入。不要写死只对样例有效的规则；宁可换一种表达、补一个更通用的步骤，或明确判断条件。
2. **保持精简**。删掉没有实际作用的段落。读执行记录，不只读最终输出；如果技能让模型花大量时间做无用功，删掉导致这些行为的指令。
3. **解释为什么**。模型有能力理解原因。与其写满大写的 ALWAYS / NEVER，不如说明约束背后的理由，让它在新情况下做出正确判断。
4. **合并重复劳动**。如果多个测试都让子智能体现写类似的 `build_chart.py`，说明这个脚本应该放进 `scripts/`，并在正文里告诉后续运行直接使用。

改进后重新跑全部测试，包括基线，放进新的 `iteration-<N+1>/`，用 `--previous-workspace` 指向上一轮。重复到用户满意、反馈为空，或继续改进没有明显收益。

## 盲测比较（可选）

需要更严格地比较两个版本时，读 [agents/comparator.md](agents/comparator.md) 和 [agents/analyzer.md](agents/analyzer.md)。做法是把两个版本的输出交给独立子智能体，不告诉它哪个版本来自哪个技能，让它判断质量；再分析胜负原因。多数情况下人工评审已经足够，不必默认启用。

## 描述优化

技能的 `description` 是触发入口。创建或改进完成后，可以主动提出优化描述，但不要声称测量了真实运行时的触发率。

1. 生成 20 条触发评测查询，8-10 条应该触发，8-10 条不应该触发。查询要像真实用户输入，包含具体文件、场景、缩写、口语和不完整信息；负例要用“近邻”而不是明显无关的问题。
2. 保存为 JSON：

```json
[
  {"query": "用户会输入的话", "should_trigger": true},
  {"query": "另一个输入", "should_trigger": false}
]
```

3. 用 [assets/eval_review.html](assets/eval_review.html) 让用户审阅和调整：替换模板中的评测数据、技能名和描述占位符，写入临时 HTML，交给用户在浏览器打开。
4. Tron 没有暴露脚本化的触发模拟器。可以用独立子智能体做近似判断：只给它技能目录（名称 + 描述）和一条查询，问它会加载哪个技能，不要告诉它目标答案。每条查询重复 3 次，记录触发率。把结果标注为近似评估，不能当作真实运行时数据。
5. 根据失败样例改写描述，重新评估，选择在留出查询上更稳的版本。应用前向用户展示前后差异。

## 更新已有技能

用户要修改已安装技能时：

- 保留原名称和目录名，不要改成 `-v2`。
- 已安装位置可能只读。先把完整目录复制到用户可写的工作区，在副本上修改，再交付。
- 更新制品时保留仍然有效的规则，只改需要改的部分；不要整篇重写后丢掉已经验证过的约束。
- 如果走记忆整理或技能候选流程，按宿主协议提交完整 `files` 和行为检查，不要直接覆盖安装目录。
- 生成的技能不能命名为 `skill-creator`，该名称由内置技能保留。

## 交付

交付时给出：

- 完整的技能目录或制品文件清单。
- `SKILL.md` 的 `name` 和 `description`。
- 测试用例、实际运行结果和未验证的部分。
- 安装或审批状态；没有真正安装时明确说明“仅生成，未安装”。
- 任何依赖，例如 Python 版本、Node 版本或外部服务。

需要把技能分享到 Tron 之外时，可以打包成 `.skill` 文件：

```bash
python -m scripts.package_skill <path/to/skill-folder> [output-directory]
```

`.skill` 是 ZIP 格式，保留技能目录名。Tron 内部的安装仍以技能中心的审批流程为准，不要把 `.skill` 文件直接写进注册目录。

## 参考文件

- [agents/grader.md](agents/grader.md)：如何逐条评估检查项并给出证据。
- [agents/comparator.md](agents/comparator.md)：如何做盲测 A/B 比较。
- [agents/analyzer.md](agents/analyzer.md)：如何分析基准结果和胜负原因。
- [references/schemas.md](references/schemas.md)：`evals.json`、`grading.json`、`benchmark.json` 等结构。
- `scripts/quick_validate.py`：校验技能结构和 frontmatter，需要 Python 3.9+ 和 PyYAML。
- `scripts/aggregate_benchmark.py`：把多次运行聚合成基准数据。
- `scripts/package_skill.py`：把技能目录打包成 `.skill`。
- `eval-viewer/generate_review.py`：生成输出和基准评审页；`assets/eval_review.html` 用于描述评测集审阅。

脚本路径相对本 `SKILL.md`。在只读技能目录中不要直接执行，复制到工作区后再运行。