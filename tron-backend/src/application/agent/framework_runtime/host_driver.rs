//! ReAct, External and Continuation agents select the same Framework environment.
use super::*;
impl FrameworkHost {
    pub(super) async fn build_driver(&self, external: bool) -> Result<Agent, fw::AgentError> {
        let framework = self.framework().await?;
        let (packs, extensions) = self.capability_state();
        let mut selection = self
            .main_selection(&framework, &packs, &extensions)
            .map_err(fw::AgentError::InvalidConfiguration)?;
        selection.agent_loop = Some(
            if external {
                "tron.external-loop"
            } else {
                "tron.continuation-loop"
            }
            .into(),
        );
        // These loops delegate execution and never invoke the Framework tool pipeline.
        selection.tool_policies = Some(vec![]);
        selection.tool_providers = Some(vec![]);
        selection.context_assembler = Some("tron.driver.context".into());
        framework.agent(selection).build().await
    }
}
