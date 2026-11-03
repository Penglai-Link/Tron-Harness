use chrono::{DateTime, FixedOffset, Utc};
use rusqlite::{OptionalExtension, params};

use super::{PersonAiAccountSummary, ProfileRegistryError, ProfileRegistryService};

// PersonAI check-in days follow UTC+8, independently of the UI time zone.
pub(super) fn checkin_day(at: DateTime<Utc>) -> String {
    at.with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap())
        .format("%Y-%m-%d")
        .to_string()
}

impl ProfileRegistryService {
    pub fn cached_account(
        &self,
        user: &str,
    ) -> Result<Option<PersonAiAccountSummary>, ProfileRegistryError> {
        self.cached_account_on(user, &checkin_day(Utc::now()))
    }

    fn cached_account_on(
        &self,
        user: &str,
        day: &str,
    ) -> Result<Option<PersonAiAccountSummary>, ProfileRegistryError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ProfileRegistryError::LockPoisoned)?;
        Ok(connection
            .query_row(
                "SELECT nickname,vip_level,remaining_points,account_updated_at,
             CASE WHEN checkin_date=?2 THEN today_checked_in END,
             CASE WHEN checkin_date=?2 THEN today_reward_points END
             FROM profile_account_cache WHERE person_ai_user_id=?1",
                params![user, day],
                |row| {
                    Ok(PersonAiAccountSummary {
                        nickname: row.get(0)?,
                        vip_level: row.get(1)?,
                        remaining_points: row.get(2)?,
                        updated_at: row.get(3)?,
                        today_checked_in: row.get(4)?,
                        today_reward_points: row.get(5)?,
                    })
                },
            )
            .optional()?)
    }

    pub(super) fn cache_account_info(
        &self,
        user: &str,
        nickname: Option<&str>,
        vip_level: Option<i32>,
        points: Option<f64>,
        at: i64,
    ) -> Result<(), ProfileRegistryError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ProfileRegistryError::LockPoisoned)?;
        connection.execute(
            "INSERT INTO profile_account_cache(person_ai_user_id,nickname,vip_level,remaining_points,account_updated_at)
             VALUES(?1,?2,?3,?4,?5) ON CONFLICT(person_ai_user_id) DO UPDATE SET
             nickname=excluded.nickname,vip_level=excluded.vip_level,remaining_points=excluded.remaining_points,account_updated_at=excluded.account_updated_at
             WHERE account_updated_at IS NULL OR account_updated_at<=excluded.account_updated_at",
            params![user,nickname,vip_level,points,at],
        )?;
        Ok(())
    }

    pub(super) fn cache_checkin(
        &self,
        user: &str,
        day: &str,
        checked: bool,
        reward: Option<f64>,
        at: i64,
    ) -> Result<(), ProfileRegistryError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ProfileRegistryError::LockPoisoned)?;
        connection.execute(
            "INSERT INTO profile_account_cache(person_ai_user_id,checkin_date,today_checked_in,today_reward_points,checkin_updated_at)
             VALUES(?1,?2,?3,?4,?5) ON CONFLICT(person_ai_user_id) DO UPDATE SET
             checkin_date=excluded.checkin_date,
             today_checked_in=CASE WHEN checkin_date=excluded.checkin_date THEN MAX(today_checked_in,excluded.today_checked_in) ELSE excluded.today_checked_in END,
             today_reward_points=CASE WHEN checkin_date=excluded.checkin_date AND today_checked_in=1 AND excluded.today_checked_in=0 THEN today_reward_points ELSE excluded.today_reward_points END,
             checkin_updated_at=excluded.checkin_updated_at
             WHERE checkin_updated_at IS NULL OR checkin_updated_at<=excluded.checkin_updated_at",
            params![user,day,checked,reward,at],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persisted_snapshots_are_isolated_and_expire_only_the_checkin_day() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("profiles.sqlite");
        let registry = ProfileRegistryService::new(path.to_str().unwrap()).unwrap();
        registry.register_or_update("alice", "Alice").unwrap();
        registry
            .cache_account_info("alice", Some("Nickname"), Some(1), Some(123.5), 100)
            .unwrap();
        registry
            .cache_checkin("alice", "2026-11-04", true, Some(10.0), 101)
            .unwrap();
        registry
            .cache_checkin("alice", "2026-11-04", false, None, 102)
            .unwrap();
        drop(registry);
        let registry = ProfileRegistryService::new(path.to_str().unwrap()).unwrap();
        let cached = registry
            .cached_account_on("alice", "2026-11-04")
            .unwrap()
            .unwrap();
        assert_eq!(cached.remaining_points, Some(123.5));
        assert_eq!(cached.today_checked_in, Some(true));
        assert_eq!(cached.today_reward_points, Some(10.0));
        assert!(registry.cached_account("bob").unwrap().is_none());
        let tomorrow = registry
            .cached_account_on("alice", "2026-11-05")
            .unwrap()
            .unwrap();
        assert_eq!(tomorrow.today_checked_in, None);
        assert_eq!(tomorrow.remaining_points, Some(123.5));
        registry
            .cache_checkin("alice", "2026-11-05", false, Some(20.0), 103)
            .unwrap();
        assert_eq!(
            registry
                .cached_account_on("alice", "2026-11-05")
                .unwrap()
                .unwrap()
                .today_checked_in,
            Some(false)
        );
        registry
            .cache_account_info("alice", None, None, Some(999.0), 99)
            .unwrap();
        assert_eq!(
            registry
                .cached_account("alice")
                .unwrap()
                .unwrap()
                .remaining_points,
            Some(123.5)
        );
    }

    #[test]
    fn checkin_success_preserves_balance_and_deleting_profile_removes_cache() {
        let registry = ProfileRegistryService::new(":memory:").unwrap();
        let profile = registry.register_or_update("alice", "Alice").unwrap();
        registry
            .cache_account_info("alice", None, Some(1), Some(0.0), 100)
            .unwrap();
        registry
            .cache_checkin("alice", &checkin_day(Utc::now()), true, Some(10.0), 101)
            .unwrap();
        assert_eq!(
            registry
                .bootstrap_status()
                .unwrap()
                .account
                .unwrap()
                .remaining_points,
            Some(0.0)
        );
        registry.remove(&profile.profile_id).unwrap();
        assert!(registry.cached_account("alice").unwrap().is_none());
        assert_eq!(
            checkin_day("2026-11-04T16:00:00Z".parse().unwrap()),
            "2026-11-05"
        );
    }

    #[tokio::test]
    async fn account_read_routes_use_persisted_data_without_provider_credentials() {
        use axum::{
            body::{Body, to_bytes},
            http::Request,
        };
        use std::sync::Arc;
        use tower::ServiceExt;
        let registry = ProfileRegistryService::new(":memory:").unwrap();
        registry.register_or_update("alice", "Alice").unwrap();
        registry
            .cache_account_info("alice", Some("Nickname"), Some(1), Some(123.5), 100)
            .unwrap();
        registry
            .cache_checkin("alice", &checkin_day(Utc::now()), true, Some(10.0), 101)
            .unwrap();
        let state = crate::app::AppState::new_with_profiles(
            crate::config::AppConfig::default(),
            Arc::new(crate::shared::id::FixedSnowflakeIdProvider::new(
                735_000_000_000_000_000,
            )),
            registry.clone(),
        )
        .unwrap();
        // Assemble only the real read routes: no background worker, no remote
        // authorization. Even a legacy refresh query must never fetch upstream.
        let router = crate::adapters::inbound::http::profiles::routes()
            .merge(
                crate::adapters::inbound::http::person_ai::routes().route_layer(
                    axum::middleware::from_fn_with_state(
                        registry.clone(),
                        crate::adapters::inbound::http::auth::require_profile,
                    ),
                ),
            )
            .with_state(state);
        for _ in 0..2 {
            let response = router
                .clone()
                .oneshot(
                    Request::get("/api/v1/bootstrap/status?refreshAccount=true")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), axum::http::StatusCode::OK);
            let body: serde_json::Value =
                serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
                    .unwrap();
            assert_eq!(body["data"]["account"]["remainingPoints"], 123.5);
            assert_eq!(body["data"]["account"]["todayCheckedIn"], true);
        }
        let response = router
            .clone()
            .oneshot(
                Request::get("/api/v1/account/checkin")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(body["data"]["todayCheckedIn"], true);
        registry.register_or_update("bob", "Bob").unwrap();
        let response = router
            .oneshot(
                Request::get("/api/v1/bootstrap/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(body["data"]["activeProfile"]["personAiUserId"], "bob");
        assert!(body["data"]["account"].is_null());
    }
}
