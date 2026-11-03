use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use chrono::Utc;
use tokio::sync::{Mutex, Notify};

use super::{ProfileRecord, ProfileRegistryService, account_cache::checkin_day};
use crate::application::model_provider::{ModelProviderError, ModelProviderService};

#[derive(Clone)]
pub struct ProfileAccountSync(Arc<Inner>);

struct Inner {
    profiles: ProfileRegistryService,
    providers: ModelProviderService,
    client: reqwest::Client,
    lock: Mutex<()>,
    wake: Notify,
    started: AtomicBool,
}

impl ProfileAccountSync {
    pub fn new(
        profiles: ProfileRegistryService,
        providers: ModelProviderService,
        client: reqwest::Client,
    ) -> Self {
        Self(Arc::new(Inner {
            profiles,
            providers,
            client,
            lock: Mutex::new(()),
            wake: Notify::new(),
            started: AtomicBool::new(false),
        }))
    }

    pub fn request_sync(&self) {
        self.0.wake.notify_one();
    }

    pub fn start(&self) {
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            return;
        };
        if self.0.started.swap(true, Ordering::AcqRel) {
            return;
        }
        let weak = Arc::downgrade(&self.0);
        runtime.spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                let Some(inner) = weak.upgrade() else {
                    break;
                };
                tokio::select! { _ = interval.tick() => {}, _ = inner.wake.notified() => {} }
                let service = Self(inner);
                if let Err(error) = service.refresh().await
                    && !matches!(
                        error,
                        ModelProviderError::AuthenticationRequired
                            | ModelProviderError::ConnectionNotReady
                    )
                {
                    tracing::warn!("PersonAI account snapshot refresh failed: {error}");
                }
            }
        });
    }

    async fn refresh(&self) -> Result<(), ModelProviderError> {
        let _guard = self.0.lock.lock().await;
        let Some(profile) = self
            .0
            .profiles
            .active()
            .map_err(|_| ModelProviderError::Storage)?
        else {
            return Ok(());
        };
        self.refresh_profile(&profile).await
    }

    async fn refresh_profile(&self, profile: &ProfileRecord) -> Result<(), ModelProviderError> {
        let day = checkin_day(Utc::now());
        let (account, checkin) = tokio::time::timeout(
            Duration::from_secs(20),
            self.0
                .providers
                .profile_account_snapshot(&profile.person_ai_user_id),
        )
        .await
        .map_err(|_| ModelProviderError::Remote("Account synchronization timed out".into()))??;
        if account.username.as_deref().map(str::trim) != Some(profile.person_ai_username.as_str()) {
            return Err(ModelProviderError::AuthenticationRequired);
        }
        let now = Utc::now();
        self.0
            .profiles
            .cache_account_info(
                &profile.person_ai_user_id,
                account.nickname.as_deref(),
                account.vip_level,
                account.remaining_tokens,
                now.timestamp_millis(),
            )
            .map_err(|_| ModelProviderError::Storage)?;
        // Do not label a response crossing midnight as belonging to either day.
        if day == checkin_day(now) {
            self.0
                .profiles
                .cache_checkin(
                    &profile.person_ai_user_id,
                    &day,
                    checkin.today_checked_in,
                    checkin.today_reward_points,
                    now.timestamp_millis(),
                )
                .map_err(|_| ModelProviderError::Storage)?;
        }
        let _ = tokio::time::timeout(
            Duration::from_secs(5),
            self.refresh_avatar(profile, account.avatar_url.as_deref()),
        )
        .await;
        Ok(())
    }

    pub async fn checkin(
        &self,
        user: &str,
    ) -> Result<crab_serverapi::checkin::CheckinRecord, ModelProviderError> {
        let _guard = self.0.lock.lock().await;
        self.0
            .profiles
            .active()
            .map_err(|_| ModelProviderError::Storage)?
            .filter(|profile| profile.person_ai_user_id == user)
            .ok_or(ModelProviderError::AuthenticationRequired)?;
        match self.0.providers.checkin_profile(user).await {
            Ok(record) => {
                // Persist the successful receipt before any further network call.
                // Never infer a new balance by adding a reward to an old balance.
                let now = Utc::now();
                let date = record
                    .checkin_date
                    .clone()
                    .unwrap_or_else(|| checkin_day(now));
                self.0
                    .profiles
                    .cache_checkin(user, &date, true, record.points, now.timestamp_millis())
                    .map_err(|_| ModelProviderError::Storage)?;
                self.request_sync();
                Ok(record)
            }
            Err(error) => {
                // A check-in made elsewhere may have won the race. Refresh through
                // the single background writer, without replacing known UI state.
                self.request_sync();
                Err(error)
            }
        }
    }

    async fn refresh_avatar(&self, profile: &ProfileRecord, source: Option<&str>) {
        let Some(source) = source.filter(|url| !url.trim().is_empty()) else {
            let _ = self
                .0
                .profiles
                .clear_cached_avatar(&profile.person_ai_user_id);
            return;
        };
        let Ok(url) = url::Url::parse(source) else {
            return;
        };
        if url.scheme() != "https" {
            return;
        }
        let Ok(mut response) = self.0.client.get(url).send().await else {
            return;
        };
        const MAX_BYTES: usize = 2 * 1024 * 1024;
        if response.url().scheme() != "https"
            || !response.status().is_success()
            || response
                .content_length()
                .is_some_and(|length| length > MAX_BYTES as u64)
        {
            return;
        }
        let Some(content_type) = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(';').next())
            .map(str::trim)
            .filter(|value| value.starts_with("image/"))
            .map(str::to_owned)
        else {
            return;
        };
        let mut bytes = Vec::new();
        loop {
            match response.chunk().await {
                Ok(Some(chunk)) if bytes.len().saturating_add(chunk.len()) <= MAX_BYTES => {
                    bytes.extend_from_slice(&chunk)
                }
                Ok(None) => break,
                _ => return,
            }
        }
        if !bytes.is_empty() {
            let _ = self
                .0
                .profiles
                .cache_avatar(&profile.person_ai_user_id, &content_type, &bytes);
        }
    }
}
