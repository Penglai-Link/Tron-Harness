use std::{future::Future, sync::Arc};

use tokio::{task::JoinHandle, task_local};

use crate::domain::auth::{AccessTokenIdentity, CurrentSession};

#[derive(Debug, Clone)]
pub struct AuthHolderContext {
    pub access_token: String,
    pub locale: String,
    pub session: CurrentSession,
    pub token_identity: AccessTokenIdentity,
}

impl AuthHolderContext {
    pub fn user_code(&self) -> &str {
        &self.token_identity.user_code
    }

    pub fn session_code(&self) -> &str {
        &self.token_identity.session_code
    }
}

task_local! {
    static CURRENT_AUTH_HOLDER: Arc<AuthHolderContext>;
}

pub struct AuthHolder;

impl AuthHolder {
    async fn scope_shared<Fut>(context: Arc<AuthHolderContext>, future: Fut) -> Fut::Output
    where
        Fut: Future,
    {
        CURRENT_AUTH_HOLDER.scope(context, future).await
    }

    pub async fn scope<Fut>(context: AuthHolderContext, future: Fut) -> Fut::Output
    where
        Fut: Future,
    {
        Self::scope_shared(Arc::new(context), future).await
    }

    pub fn get() -> Option<Arc<AuthHolderContext>> {
        CURRENT_AUTH_HOLDER.try_with(Clone::clone).ok()
    }

    pub fn spawn<Fut>(future: Fut) -> JoinHandle<Fut::Output>
    where
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        let context = Self::get();
        crate::app::telemetry::spawn_traced(async move {
            match context {
                Some(context) => Self::scope_shared(context, future).await,
                None => future.await,
            }
        })
    }
}
