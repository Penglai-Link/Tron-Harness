use crate::domain::open_app::{
    OpenApiNonceRecord, OpenApp, OpenAppAuthState, OpenAppCapabilityGrant, OpenAppCredential,
    OpenAppError, OpenAppUserGrant,
};

pub trait OpenAppRepository: Send {
    fn next_id(&mut self) -> Result<i64, OpenAppError>;
    fn create_open_app(
        &mut self,
        open_app: &OpenApp,
        credential: &OpenAppCredential,
    ) -> Result<(), OpenAppError>;
    fn list_open_app_auth_states(&mut self) -> Result<Vec<OpenAppAuthState>, OpenAppError>;
    fn find_open_app_by_open_app_code(
        &mut self,
        open_app_code: &str,
    ) -> Result<Option<OpenApp>, OpenAppError>;
    fn update_open_app(&mut self, open_app: &OpenApp) -> Result<(), OpenAppError>;
    fn rotate_credential(
        &mut self,
        open_app_code: &str,
        revoked_at: &str,
        credential: &OpenAppCredential,
    ) -> Result<(), OpenAppError>;
    fn load_auth_state(
        &mut self,
        open_app_code: &str,
    ) -> Result<Option<OpenAppAuthState>, OpenAppError>;
    fn insert_user_grant(&mut self, grant: &OpenAppUserGrant) -> Result<(), OpenAppError>;
    fn insert_capability_grant(
        &mut self,
        grant: &OpenAppCapabilityGrant,
    ) -> Result<(), OpenAppError>;
    fn replace_user_grants(
        &mut self,
        open_app_code: &str,
        grants: &[OpenAppUserGrant],
    ) -> Result<(), OpenAppError>;
    fn replace_capability_grants(
        &mut self,
        open_app_code: &str,
        grants: &[OpenAppCapabilityGrant],
    ) -> Result<(), OpenAppError>;
    fn purge_expired_nonces(&mut self, now_epoch: i64) -> Result<(), OpenAppError>;
    fn insert_nonce(&mut self, record: &OpenApiNonceRecord) -> Result<(), OpenAppError>;
}
