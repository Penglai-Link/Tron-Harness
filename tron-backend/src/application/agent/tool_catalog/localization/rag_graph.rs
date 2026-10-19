use crate::shared::locale::SupportedLocale;
pub(super) fn translate(locale: SupportedLocale, key: &str) -> Option<&'static str> {
    Some(match (locale, key) {
        (SupportedLocale::ZhCn, "agent.toolCatalog.ragGraph.search.name") => "检索知识图谱",
        (SupportedLocale::EnUs, "agent.toolCatalog.ragGraph.search.name") => {
            "Search knowledge graphs"
        }
        (SupportedLocale::ZhCn, "agent.toolCatalog.ragGraph.search.description") => {
            "在当前智能体获授权的知识库、记忆和技能中，通过节点全文与向量检索选取实体，并分别沿入边、出边默认扩展三层。返回节点、关系及带文件名和位置的原文引文。正文事实必须以references为依据；生成说明不是原文。需要展示卡片时由你调用可用的A2UI工具，引用返回的来源，不得编造图片、文件或位置。"
        }
        (SupportedLocale::EnUs, "agent.toolCatalog.ragGraph.search.description") => {
            "Search node fulltext and vectors in the Agent's authorized knowledge bases, memory and skills, then expand incoming and outgoing edges separately, three hops by default. Returns nodes, relationships and original quotes with filenames and locations. Ground claims in references; generated descriptions are not originals. To present cards, call an available A2UI tool yourself using the returned evidence; never invent files, images or locations."
        }
        (SupportedLocale::ZhCn, "agent.toolCatalog.ragGraph.search.summary") => {
            "返回有原文依据的子图、命中原因及可继续加载的游标。"
        }
        (SupportedLocale::EnUs, "agent.toolCatalog.ragGraph.search.summary") => {
            "Returns a source-grounded subgraph, hit reasons and a continuation cursor."
        }
        (SupportedLocale::ZhCn, "agent.toolCatalog.ragGraph.expand.name") => "展开知识图谱",
        (SupportedLocale::EnUs, "agent.toolCatalog.ragGraph.expand.name") => {
            "Expand knowledge graphs"
        }
        (SupportedLocale::ZhCn, "agent.toolCatalog.ragGraph.expand.description") => {
            "从已返回的nodeIds继续读取入边或出边，支持关系类型过滤且不重复调用向量模型。继续分页只传cursor，不能修改查询选项。每次加载重新检查权限及发布快照；游标过期或快照变化须重新检索。达到预算时查看unfinished与truncatedBy，不得把部分子图描述为全图。"
        }
        (SupportedLocale::EnUs, "agent.toolCatalog.ragGraph.expand.description") => {
            "Expand incoming or outgoing edges from returned nodeIds, optionally filtering relationship types, without another embedding call. For pagination send cursor alone; do not modify options. Each load rechecks authorization and publication. Search again if the cursor expires or snapshot changes. Inspect unfinished and truncatedBy when bounded; never describe a partial graph as complete."
        }
        (SupportedLocale::ZhCn, "agent.toolCatalog.ragGraph.expand.summary") => {
            "返回增量子图、原文来源和剩余展开位置。"
        }
        (SupportedLocale::EnUs, "agent.toolCatalog.ragGraph.expand.summary") => {
            "Returns an incremental subgraph, original sources and remaining expansion positions."
        }
        _ => return None,
    })
}
