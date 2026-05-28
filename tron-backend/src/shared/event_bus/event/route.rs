use crate::shared::id::SnowflakeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusRouteClass {
    Audit,
    Observe,
    Agui,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusPriority {
    Critical,
    Standard,
    Debug,
}

impl BusPriority {
    pub fn ingress_lane(self) -> BusIngressLane {
        match self {
            Self::Critical => BusIngressLane::Critical,
            Self::Standard | Self::Debug => BusIngressLane::Standard,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusOutcome {
    Success,
    Failure,
    Rejected,
    Started,
    Progress,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusIngressLane {
    Critical,
    Standard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishOutcome {
    Disabled,
    Accepted {
        event_id: SnowflakeId,
    },
    RejectedInvalid {
        field: &'static str,
    },
    RejectedOversize {
        field: &'static str,
        actual: usize,
        limit: usize,
    },
    DroppedIngressFull {
        lane: BusIngressLane,
    },
    DroppedIngressDisconnected {
        lane: BusIngressLane,
    },
}
