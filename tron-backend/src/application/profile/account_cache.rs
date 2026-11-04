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
