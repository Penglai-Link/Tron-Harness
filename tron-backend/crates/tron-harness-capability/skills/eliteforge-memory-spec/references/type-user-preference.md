# 用户偏好（USER_PREFERENCE）

用户明确表达、适用于后续协作的长期偏好。一次性任务指令、推测的喜好、项目规则和第三方画像不属于此类。

## 正文结构

`text` 使用普通 Markdown，按下面顺序表达。标签和可读内容跟随 `metadata.language`；以下为中文模板，不能原样保存占位文字。

```md
偏好：<谁希望在什么输出或协作中采用什么行为>

适用条件：<未来哪些场景需要采用该偏好>

例外：<本次指令、详细说明等覆盖条件>

用法：<如何调整输出、计划或工具选择>

证据：<指向 sourceRefs 的简短摘要>
```

## 类型字段

- `preferenceOwner`：偏好归属的稳定用户或身份。
- `preferenceTarget`：受影响的输出或协作领域。
- `behavior`：期望的具体行为。
- `strength`：强度，例如 explicit、strong、soft、situational。
- `exceptions`：例外、临时覆盖条件；没有已知例外时明确说明。

## 语义键与证据

`semanticKey` 的结构：

```text
user-preference:<范围编码>:<归属者>:<目标>:<行为>
```

尖括号内容替换为稳定的小写 ASCII 段，保持同主题、同用途的键稳定；复核或补强时更新原记录，不靠改写措辞新建近似副本。

`sourceRefs` 要求：用户原话、经用户确认的资料或明确授权的治理事件；推测的偏好通常保留候选，不能仅凭一次模糊表达定案。

## 合格示例

用户希望代码交付说明先说结果和验证，详细审查按要求展开。

对应 `metadata.typeSpecific` 示例：

```json
{
  "preferenceOwner": "当前授权用户",
  "preferenceTarget": "代码交付说明",
  "behavior": "简洁说明结果和验证",
  "strength": "explicit",
  "exceptions": "用户要求详细分析时展开"
}
```

这是局部内容示例，实际保存还需补全 SKILL.md 中的范围、标题、状态、语言和真实来源，不是完整工具调用。

## 不合格示例

“用户看到仓库有 Makefile，所以喜欢所有项目都使用 Makefile。”

原因：把源码事实推成个人偏好，缺少用户证据，范围扩大。
