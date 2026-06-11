mod auth;
mod management;
mod models;
mod shared;

use std::sync::{Arc, Mutex};

use crate::{application::user::UserService, shared::crypto::Aes256SecretBox};

use super::OpenAppRepository;

pub use models::{
    AuthenticatedOpenAppRequest, CapabilityGrantInput, CreateOpenAppCommand, CreatedOpenApp,
    ManagedOpenAppCapabilityGrant, ManagedOpenAppDetail, ManagedOpenAppSecretReveal,
    ManagedOpenAppSecretRevealResult, ManagedOpenAppSummary, ManagedOpenAppUserGrant,
    ReplaceCapabilityGrantsCommand, ReplaceUserGrantsCommand, SignedOpenApiRequest,
    UpdateOpenAppCommand,
};

pub(super) const DEFAULT_REPLAY_WINDOW_SECONDS: i64 = 300;

#[derive(Clone)]
pub struct OpenAppService {
    pub(super) repository: Arc<Mutex<Box<dyn OpenAppRepository>>>,
    pub(super) users: UserService,
    pub(super) secret_box: Aes256SecretBox,
    pub(super) replay_window_seconds: i64,
}

impl OpenAppService {
    pub fn new(
        repository: impl OpenAppRepository + 'static,
        users: UserService,
        secret_box: Aes256SecretBox,
    ) -> Self {
        Self::with_replay_window(repository, users, secret_box, DEFAULT_REPLAY_WINDOW_SECONDS)
    }

    pub fn with_replay_window(
        repository: impl OpenAppRepository + 'static,
        users: UserService,
        secret_box: Aes256SecretBox,
        replay_window_seconds: i64,
    ) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            users,
            secret_box,
            replay_window_seconds,
        }
    }
}
