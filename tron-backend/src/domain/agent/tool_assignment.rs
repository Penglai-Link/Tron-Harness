//! Selection rules are independent of exposure, platform permission and executors.
use super::{ToolAssignmentMode, ToolAvailabilityState, ToolCatalogRecord, ToolOperationKind};

pub trait ToolAssignmentStrategy: Send + Sync {
    fn includes(&self, tool: &ToolCatalogRecord, selected_codes: &[String]) -> bool;
}

struct AllTools;
struct SelectedTools;
struct ReadOnlyTools;
struct Categories;
impl ToolAssignmentStrategy for Categories {
    fn includes(&self, tool: &ToolCatalogRecord, codes: &[String]) -> bool {
        codes.iter().any(|code| {
            (code == "readonly" && tool.operation_kind == ToolOperationKind::Query)
                || tool
                    .category
                    .as_ref()
                    .is_some_and(|category| &category.code == code)
        })
    }
}

impl ToolAssignmentStrategy for AllTools {
    fn includes(&self, _: &ToolCatalogRecord, _: &[String]) -> bool {
        true
    }
}
impl ToolAssignmentStrategy for SelectedTools {
    fn includes(&self, tool: &ToolCatalogRecord, selected_codes: &[String]) -> bool {
        selected_codes
            .iter()
            .any(|code| code.trim() == tool.tool_code)
    }
}
impl ToolAssignmentStrategy for ReadOnlyTools {
    fn includes(&self, tool: &ToolCatalogRecord, _: &[String]) -> bool {
        tool.operation_kind == ToolOperationKind::Query
    }
}

impl ToolAssignmentMode {
    pub fn strategy(self) -> &'static dyn ToolAssignmentStrategy {
        match self {
            Self::SelectedCategories => &Categories,
            Self::AllPlatformTools => &AllTools,
            Self::SelectedTools => &SelectedTools,
            Self::AllReadOnlyTools => &ReadOnlyTools,
        }
    }

    pub fn includes_policy(
        self,
        tool: &ToolCatalogRecord,
        tools: &[String],
        categories: &[String],
    ) -> bool {
        self.strategy().includes(
            tool,
            if self == Self::SelectedCategories {
                categories
            } else {
                tools
            },
        )
    }

    pub fn selects_policy(
        self,
        tool: &ToolCatalogRecord,
        tools: &[String],
        categories: &[String],
    ) -> bool {
        tool.availability_state == ToolAvailabilityState::Available
            && self.includes_policy(tool, tools, categories)
    }

    /// Shared availability checks apply to every selection strategy.
    pub fn selects(self, tool: &ToolCatalogRecord, selected_codes: &[String]) -> bool {
        tool.availability_state == ToolAvailabilityState::Available
            && self.strategy().includes(tool, selected_codes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn category_union_uses_declared_category_and_query_kind() {
        let mut tool =
            crate::application::agent::BusinessToolCatalog::tool("bash.exec", "en-US").unwrap();
        let codes = vec!["system.command".into(), "readonly".into()];
        assert!(ToolAssignmentMode::SelectedCategories.selects(&tool, &codes));
        tool.category = Some(eliteforge_agent_core::ToolCategory::localized(
            "other", "其他", "Other",
        ));
        assert!(!ToolAssignmentMode::SelectedCategories.selects(&tool, &codes));
        tool.operation_kind = ToolOperationKind::Query;
        assert!(ToolAssignmentMode::SelectedCategories.selects(&tool, &codes));
        tool.operation_kind = ToolOperationKind::Unknown;
        assert!(!ToolAssignmentMode::SelectedCategories.selects(&tool, &codes));
        tool.availability_state = ToolAvailabilityState::Disabled;
        assert!(!ToolAssignmentMode::SelectedCategories.selects(&tool, &vec!["other".into()]));
    }
}
