//! Selection rules are independent of exposure, platform permission and executors.
use super::{ToolAssignmentMode, ToolAvailabilityState, ToolCatalogRecord, ToolOperationKind};

pub trait ToolAssignmentStrategy: Send + Sync {
    fn includes(&self, tool: &ToolCatalogRecord, selected_codes: &[String]) -> bool;
}

struct AllTools;
struct SelectedTools;
struct ReadOnlyTools;

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
            Self::AllPlatformTools => &AllTools,
            Self::SelectedTools => &SelectedTools,
            Self::AllReadOnlyTools => &ReadOnlyTools,
        }
    }

    /// Shared availability checks apply to every selection strategy.
    pub fn selects(self, tool: &ToolCatalogRecord, selected_codes: &[String]) -> bool {
        tool.availability_state == ToolAvailabilityState::Available
            && self.strategy().includes(tool, selected_codes)
    }
}
