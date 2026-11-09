//! Ephemeral credentials are separate from native interactions and history.
use super::{FrameworkHost, Host};
use eliteforge_agent_core as fw;
use eliteforge_agent_official_plugin_tools_local as local;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, Weak},
    time::Instant,
};
use tokio::sync::oneshot;

#[derive(Clone)]
pub(crate) struct PendingSudoInput {
    pub id: String,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
}
struct Pending {
    session: String,
    user: String,
    view: PendingSudoInput,
    sender: oneshot::Sender<Option<local::SudoPassword>>,
    expires: Instant,
}
#[derive(Default)]
pub(super) struct SudoInputs(Mutex<BTreeMap<String, Pending>>);

#[derive(Clone, Copy)]
pub(crate) enum SudoInputError {
    Invalid,
    Unavailable,
}
impl SudoInputError {
    pub fn descriptor(self) -> crate::application::chat::history_error::ApplicationErrorDescriptor {
        let (code, message_key) = match self {
            Self::Invalid => ("101035001", "tron.chat.sudo_input_invalid"),
            Self::Unavailable => ("101035002", "tron.chat.sudo_input_unavailable"),
        };
        crate::application::chat::history_error::ApplicationErrorDescriptor { code, message_key }
    }
}
impl FrameworkHost {
    pub(crate) fn pending_sudo_inputs(&self, session: &str, user: &str) -> Vec<PendingSudoInput> {
        let mut inputs = self
            .0
            .sudo_inputs
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        inputs.retain(|_, input| !input.sender.is_closed() && input.expires > Instant::now());
        inputs
            .values()
            .filter(|input| input.session == session && input.user == user)
            .map(|input| input.view.clone())
            .collect()
    }
    pub(crate) fn answer_sudo_input(
        &self,
        session: &str,
        user: &str,
        id: &str,
        password: Option<local::SudoPassword>,
    ) -> Result<(), SudoInputError> {
        if password.as_ref().is_some_and(|value| !value.is_valid()) {
            return Err(SudoInputError::Invalid);
        }
        let mut inputs = self
            .0
            .sudo_inputs
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        inputs
            .get(id)
            .filter(|input| {
                input.session == session
                    && input.user == user
                    && input.expires > Instant::now()
                    && !input.sender.is_closed()
            })
            .ok_or(SudoInputError::Unavailable)?;
        let input = inputs.remove(id).ok_or(SudoInputError::Unavailable)?;
        input
            .sender
            .send(password)
            .map_err(|_| SudoInputError::Unavailable)
    }
}

pub(super) struct HostSudoInput {
    pub host: Weak<Host>,
    pub session: String,
    pub user: String,
}
#[async_trait::async_trait]
impl local::SudoPasswordProvider for HostSudoInput {
    async fn request(
        &self,
        command: &str,
        args: &[String],
        cwd: Option<&std::path::Path>,
        _run: &fw::RunContext,
    ) -> Result<Option<local::SudoPassword>, fw::ToolError> {
        let host = self
            .host
            .upgrade()
            .ok_or_else(|| fw::ToolError::Failed("sudo input unavailable".into()))?;
        let id = format!("sudo-{:032x}", rand::random::<u128>());
        let (sender, receiver) = oneshot::channel();
        {
            let mut inputs = host.sudo_inputs.0.lock().unwrap_or_else(|e| e.into_inner());
            if inputs.len() >= 32 {
                return Err(fw::ToolError::Failed("too many pending sudo inputs".into()));
            }
            inputs.insert(
                id.clone(),
                Pending {
                    session: self.session.clone(),
                    user: self.user.clone(),
                    view: PendingSudoInput {
                        id: id.clone(),
                        command: command.into(),
                        args: args.into(),
                        cwd: cwd.map(|p| p.display().to_string()),
                    },
                    sender,
                    expires: Instant::now() + std::time::Duration::from_secs(45),
                },
            );
        }
        struct Remove(Arc<Host>, String);
        impl Drop for Remove {
            fn drop(&mut self) {
                self.0
                    .sudo_inputs
                    .0
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .remove(&self.1);
            }
        }
        let _remove = Remove(host, id);
        receiver
            .await
            .map_err(|_| fw::ToolError::Failed("sudo input expired".into()))
    }
}
