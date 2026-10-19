use eliteforge_agent_core::{
    AgentError, AutoRouteCandidate, AutoRouteContext, AutoRouteDataSource,
};

/// One host-authorized snapshot; model data and account ownership stay in Tron.
pub struct TronAutoRouteDataSource(pub Vec<AutoRouteCandidate>);

impl AutoRouteDataSource for TronAutoRouteDataSource {
    fn candidates(
        &self,
        _: &AutoRouteContext<'_>,
        available: &[AutoRouteCandidate],
    ) -> Result<Vec<AutoRouteCandidate>, AgentError> {
        let available = available
            .iter()
            .map(|c| &c.model)
            .collect::<std::collections::BTreeSet<_>>();
        Ok(self
            .0
            .iter()
            .filter(|candidate| available.contains(&candidate.model))
            .cloned()
            .collect())
    }
}
