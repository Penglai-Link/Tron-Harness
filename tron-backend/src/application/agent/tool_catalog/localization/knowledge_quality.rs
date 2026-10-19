use crate::shared::locale::SupportedLocale;
pub(super) fn translate(locale: SupportedLocale, key: &str) -> Option<&'static str> {
    Some(match (locale, key) {
        (SupportedLocale::ZhCn, "agent.toolCatalog.quality.sources.name") => "读取评测原文",
        (SupportedLocale::ZhCn, "agent.toolCatalog.quality.sources.description") => {
            "不填libraryId时列出当前用户知识库与切片策略；填入后分页读取已入库资料的原文及稳定证据范围。仅复用解析产物，不创建解析任务。"
        }
        (SupportedLocale::ZhCn, "agent.toolCatalog.quality.sources.summary") => {
            "返回原文、来源范围和资料指纹。"
        }
        (SupportedLocale::ZhCn, "agent.toolCatalog.quality.cases.name") => "管理检索评测题",
        (SupportedLocale::ZhCn, "agent.toolCatalog.quality.cases.description") => {
            "不填cases时读取题集；填写sourceFingerprint与cases时保存8至64题，至少4道training与4道holdout，证据范围不能重叠。工具创建的题标记为generated，不得冒充人工标注。"
        }
        (SupportedLocale::ZhCn, "agent.toolCatalog.quality.cases.summary") => {
            "返回题集及其来源指纹、版本和标注来源。"
        }
        (SupportedLocale::ZhCn, "agent.toolCatalog.quality.start.name") => "评测并优化知识库",
        (SupportedLocale::ZhCn, "agent.toolCatalog.quality.start.description") => {
            "启动有预算上限的后台评测与切片优化。没有题集时自动生成原文自测题，训练选优后仅用独立验证题验证获胜方案。确认改善后自动发布并清理旧索引，复用已有解析和已评测向量。相同请求重试沿用requestId。预算耗尽后停止，不要循环创建新任务绕过预算。"
        }
        (SupportedLocale::ZhCn, "agent.toolCatalog.quality.start.summary") => {
            "返回优化任务ID，使用rag_quality_status查询实际指标与发布结果。"
        }
        (SupportedLocale::ZhCn, "agent.toolCatalog.quality.status.name") => "查看优化结果",
        (SupportedLocale::ZhCn, "agent.toolCatalog.quality.status.description") => {
            "按libraryId查询最近优化；提供runId读取基线、候选、独立验证、未命中问题、预算与发布结果。waitSeconds最多25秒。自动生成题的指标必须称为自测召回率。"
        }
        (SupportedLocale::ZhCn, "agent.toolCatalog.quality.status.summary") => {
            "返回持久化的优化进度与结果。"
        }
        (SupportedLocale::ZhCn, "agent.toolCatalog.quality.cancel.name") => "停止知识库优化",
        (SupportedLocale::ZhCn, "agent.toolCatalog.quality.cancel.description") => {
            "停止指定知识库的优化任务，候选未发布时继续使用当前索引。"
        }
        (SupportedLocale::ZhCn, "agent.toolCatalog.quality.cancel.summary") => "返回取消状态。",
        (SupportedLocale::EnUs, "agent.toolCatalog.quality.sources.name") => {
            "Read evaluation evidence"
        }
        (SupportedLocale::EnUs, "agent.toolCatalog.quality.sources.description") => {
            "Without libraryId, list owned knowledge bases and registered chunk strategies. Otherwise read paged evidence with stable original ranges. Reuse existing extraction; never submit parsing."
        }
        (SupportedLocale::EnUs, "agent.toolCatalog.quality.sources.summary") => {
            "Returns original evidence, source spans and a source fingerprint."
        }
        (SupportedLocale::EnUs, "agent.toolCatalog.quality.cases.name") => {
            "Manage retrieval evaluation cases"
        }
        (SupportedLocale::EnUs, "agent.toolCatalog.quality.cases.description") => {
            "Read the dataset when cases is omitted. Save 8–64 cases with sourceFingerprint otherwise, including at least four training and four holdout cases with non-overlapping evidence. Tool-created cases are generated, never human-labeled."
        }
        (SupportedLocale::EnUs, "agent.toolCatalog.quality.cases.summary") => {
            "Returns cases, provenance, source fingerprint and dataset revision."
        }
        (SupportedLocale::EnUs, "agent.toolCatalog.quality.start.name") => {
            "Evaluate and optimize a knowledge base"
        }
        (SupportedLocale::EnUs, "agent.toolCatalog.quality.start.description") => {
            "Start a bounded background experiment. Generate source-grounded self-test cases if needed, choose candidates on training, and validate only the winner on holdout. Automatically publish confirmed improvements, delete old indexes, and reuse parsed output and tested vectors. Reuse requestId when retrying. Stop at the budget; never create repeated runs to bypass it."
        }
        (SupportedLocale::EnUs, "agent.toolCatalog.quality.start.summary") => {
            "Returns a run ID. Read actual scores and publication using rag_quality_status."
        }
        (SupportedLocale::EnUs, "agent.toolCatalog.quality.status.name") => {
            "Read optimization results"
        }
        (SupportedLocale::EnUs, "agent.toolCatalog.quality.status.description") => {
            "List recent library runs, or use runId for baseline, trials, holdout, missing evidence, budget and publication results. waitSeconds is at most 25. Generated-case scores must be described as self-test recall."
        }
        (SupportedLocale::EnUs, "agent.toolCatalog.quality.status.summary") => {
            "Returns persisted experiment progress and results."
        }
        (SupportedLocale::EnUs, "agent.toolCatalog.quality.cancel.name") => {
            "Stop knowledge base optimization"
        }
        (SupportedLocale::EnUs, "agent.toolCatalog.quality.cancel.description") => {
            "Stop the specified optimization. Keep the current index when the candidate has not been published."
        }
        (SupportedLocale::EnUs, "agent.toolCatalog.quality.cancel.summary") => {
            "Returns cancellation state."
        }
        _ => return None,
    })
}
