#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalRuntimeEvent {
    RunRequested,
    RunStarted,
    ModelRequested,
    ModelResponded,
    ReasoningDelta,
    ReasoningRawDelta,
    OutputStarted,
    OutputDelta,
    OutputCompleted,
    ToolSelected,
    ToolStarted,
    ToolActivity,
    ToolCompleted,
    ToolFailed,
    ObservationCreated,
    RunCompleted,
    RunFailed,
}

impl CanonicalRuntimeEvent {
    pub fn code(self) -> &'static str {
        match self {
            Self::RunRequested => "RUN_REQUESTED",
            Self::RunStarted => "RUN_STARTED",
            Self::ModelRequested => "MODEL_REQUESTED",
            Self::ModelResponded => "MODEL_RESPONDED",
            Self::ReasoningDelta => "REASONING_DELTA",
            Self::ReasoningRawDelta => "REASONING_RAW_DELTA",
            Self::OutputStarted => "OUTPUT_STARTED",
            Self::OutputDelta => "OUTPUT_DELTA",
            Self::OutputCompleted => "OUTPUT_COMPLETED",
            Self::ToolSelected => "TOOL_SELECTED",
            Self::ToolStarted => "TOOL_STARTED",
            Self::ToolActivity => "TOOL_ACTIVITY",
            Self::ToolCompleted => "TOOL_COMPLETED",
            Self::ToolFailed => "TOOL_FAILED",
            Self::ObservationCreated => "OBSERVATION_CREATED",
            Self::RunCompleted => "RUN_COMPLETED",
            Self::RunFailed => "RUN_FAILED",
        }
    }

    pub fn from_bus_event_name(event_name: &str) -> Option<Self> {
        match event_name {
            "agent.run.requested" => Some(Self::RunRequested),
            "agent.run.started" => Some(Self::RunStarted),
            "agent.model.requested" => Some(Self::ModelRequested),
            "agent.model.responded" => Some(Self::ModelResponded),
            "agent.reasoning.delta" => Some(Self::ReasoningDelta),
            "agent.reasoning.raw_delta" => Some(Self::ReasoningRawDelta),
            "agent.output.started" => Some(Self::OutputStarted),
            "agent.output.delta" => Some(Self::OutputDelta),
            "agent.output.created" => Some(Self::OutputCompleted),
            "agent.tool.selected" => Some(Self::ToolSelected),
            "agent.tool.started" => Some(Self::ToolStarted),
            "agent.tool.activity" => Some(Self::ToolActivity),
            "agent.tool.completed" => Some(Self::ToolCompleted),
            "agent.tool.failed" => Some(Self::ToolFailed),
            "agent.observation.created" => Some(Self::ObservationCreated),
            "agent.run.completed" => Some(Self::RunCompleted),
            "agent.run.failed" => Some(Self::RunFailed),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CanonicalRuntimeEvent;

    #[test]
    fn maps_bus_event_names_to_canonical_codes() {
        let cases = [
            ("agent.run.requested", "RUN_REQUESTED"),
            ("agent.run.started", "RUN_STARTED"),
            ("agent.model.requested", "MODEL_REQUESTED"),
            ("agent.model.responded", "MODEL_RESPONDED"),
            ("agent.reasoning.delta", "REASONING_DELTA"),
            ("agent.reasoning.raw_delta", "REASONING_RAW_DELTA"),
            ("agent.output.started", "OUTPUT_STARTED"),
            ("agent.output.delta", "OUTPUT_DELTA"),
            ("agent.output.created", "OUTPUT_COMPLETED"),
            ("agent.tool.selected", "TOOL_SELECTED"),
            ("agent.tool.started", "TOOL_STARTED"),
            ("agent.tool.activity", "TOOL_ACTIVITY"),
            ("agent.tool.completed", "TOOL_COMPLETED"),
            ("agent.tool.failed", "TOOL_FAILED"),
            ("agent.observation.created", "OBSERVATION_CREATED"),
            ("agent.run.completed", "RUN_COMPLETED"),
            ("agent.run.failed", "RUN_FAILED"),
        ];

        for (event_name, expected_code) in cases {
            let canonical = CanonicalRuntimeEvent::from_bus_event_name(event_name)
                .expect("expected canonical mapping");
            assert_eq!(canonical.code(), expected_code);
        }
    }

    #[test]
    fn unknown_bus_event_name_returns_none() {
        assert!(CanonicalRuntimeEvent::from_bus_event_name("agent.unknown").is_none());
    }
}
