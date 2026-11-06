use super::speech::SpeechInputError;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;

// Retain cancelled/completed IDs longer than the 120s operation deadline.
// A DELETE can arrive before the corresponding POST has finished uploading.
const RETENTION: Duration = Duration::from_secs(150);
const MAX_RECENT: usize = 4096;
const MAX_ACTIVE_PER_USER: usize = 8;

#[derive(Clone, Default)]
pub(super) struct SpeechSessions(Arc<Mutex<HashMap<(String, String), Entry>>>);
struct Entry {
    token: CancellationToken,
    expires: Instant,
}
pub(super) struct SpeechSession(CancellationToken);
impl SpeechSession {
    pub async fn cancelled(&self) {
        self.0.cancelled().await;
    }
}
impl Drop for SpeechSession {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

impl SpeechSessions {
    pub fn start(&self, user: &str, request_id: &str) -> Result<SpeechSession, SpeechInputError> {
        let key = (user.to_owned(), request_key(request_id)?);
        let mut entries = self.0.lock().map_err(|_| SpeechInputError::Storage)?;
        entries.retain(|_, entry| entry.expires > Instant::now());
        if let Some(entry) = entries.get(&key) {
            return Err(if entry.token.is_cancelled() {
                SpeechInputError::Cancelled
            } else {
                SpeechInputError::RequestConflict
            });
        }
        if entries.len() >= MAX_RECENT
            || entries
                .iter()
                .filter(|((owner, _), entry)| owner == user && !entry.token.is_cancelled())
                .count()
                >= MAX_ACTIVE_PER_USER
        {
            return Err(SpeechInputError::TooManyRequests);
        }
        let token = CancellationToken::new();
        entries.insert(
            key,
            Entry {
                token: token.clone(),
                expires: Instant::now() + RETENTION,
            },
        );
        Ok(SpeechSession(token))
    }

    pub fn cancel(&self, user: &str, request_id: &str) -> Result<(), SpeechInputError> {
        let key = (user.to_owned(), request_key(request_id)?);
        let mut entries = self.0.lock().map_err(|_| SpeechInputError::Storage)?;
        entries.retain(|_, entry| entry.expires > Instant::now());
        if let Some(entry) = entries.get(&key) {
            entry.token.cancel();
        } else {
            if entries.len() >= MAX_RECENT {
                return Err(SpeechInputError::TooManyRequests);
            }
            let token = CancellationToken::new();
            token.cancel();
            entries.insert(
                key,
                Entry {
                    token,
                    expires: Instant::now() + RETENTION,
                },
            );
        }
        Ok(())
    }
}

fn request_key(id: &str) -> Result<String, SpeechInputError> {
    if id.len() != 36
        || !id.bytes().enumerate().all(|(i, c)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                c == b'-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
    {
        return Err(SpeechInputError::InvalidRequestId);
    }
    Ok(id.to_ascii_lowercase())
}
