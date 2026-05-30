use serde::Deserialize;

use crate::application::agent::UpdatePlatformMainAgentCommand;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateMainAgentConfigRequest {
    pub personality: Option<String>,
    pub instructions: Option<String>,
}

impl From<UpdateMainAgentConfigRequest> for UpdatePlatformMainAgentCommand {
    fn from(request: UpdateMainAgentConfigRequest) -> Self {
        Self {
            personality: request.personality,
            instructions: request.instructions,
        }
    }
}
