use super::{ModelProviderError, ModelProviderService, ServerApiError, server_api_error};
use crab_serverapi::capability::{
    CapabilityCenterClient, CapabilityDetail, CapabilityLikeResult, CapabilityPage, CapabilityQuery,
};

impl ModelProviderService {
    pub async fn capability_center_page(
        &self,
        user: &str,
        query: &CapabilityQuery,
    ) -> Result<CapabilityPage, ModelProviderError> {
        let (context, base) = self.person_ai_profile_session(user).await?;
        let client = CapabilityCenterClient::new(base).map_err(server_api_error)?;
        self.person_ai_service_result(client.list(&context, query).await)
    }

    pub async fn capability_center_categories(
        &self,
        user: &str,
    ) -> Result<Vec<String>, ModelProviderError> {
        let (context, base) = self.person_ai_profile_session(user).await?;
        let client = CapabilityCenterClient::new(base).map_err(server_api_error)?;
        self.person_ai_service_result(client.categories(&context).await)
    }

    pub async fn capability_center_set_like(
        &self,
        user: &str,
        pack_id: &str,
        liked: bool,
    ) -> Result<CapabilityLikeResult, ModelProviderError> {
        let (context, base) = self.person_ai_profile_session(user).await?;
        let client = CapabilityCenterClient::new(base).map_err(server_api_error)?;
        self.capability_market_result(
            client.set_like(&context, pack_id, liked).await,
            ModelProviderError::CapabilityPackUnavailable,
        )
    }

    pub async fn capability_center_detail(
        &self,
        user: &str,
        id: &str,
    ) -> Result<CapabilityDetail, ModelProviderError> {
        let (context, base) = self.person_ai_profile_session(user).await?;
        let client = CapabilityCenterClient::new(base).map_err(server_api_error)?;
        self.capability_release_result(client.detail(&context, id).await)
    }

    pub async fn capability_center_download(
        &self,
        user: &str,
        detail: &CapabilityDetail,
    ) -> Result<Vec<u8>, ModelProviderError> {
        let (context, base) = self.person_ai_profile_session(user).await?;
        let client = CapabilityCenterClient::new(base).map_err(server_api_error)?;
        let download =
            self.capability_release_result(client.download(&context, &detail.summary.id).await)?;
        // Bind the credential to the immutable release the user reviewed.
        if download.id != detail.summary.id
            || download.pack_id != detail.summary.pack_id
            || download.version != detail.summary.version
            || download.sha256 != detail.summary.sha256
            || download.file_size != detail.summary.file_size
        {
            return Err(ModelProviderError::Remote(
                "Capability download does not match the selected release".into(),
            ));
        }
        self.capability_release_result(client.download_zip(&download).await)
    }

    // Preserve release availability separately from transport and authentication failures.
    // Do not apply this mapping to catalog/category endpoints, where 404 may mean an absent API.
    fn capability_release_result<T>(
        &self,
        result: Result<T, ServerApiError>,
    ) -> Result<T, ModelProviderError> {
        self.capability_market_result(result, ModelProviderError::CapabilityReleaseUnavailable)
    }

    fn capability_market_result<T>(
        &self,
        result: Result<T, ServerApiError>,
        unavailable: ModelProviderError,
    ) -> Result<T, ModelProviderError> {
        match result {
            Err(ServerApiError::Business { code: 404, .. })
            | Err(ServerApiError::Gateway { status: 404, .. }) => Err(unavailable),
            Err(ServerApiError::Http(error))
                if error.status() == Some(reqwest::StatusCode::NOT_FOUND) =>
            {
                Err(unavailable)
            }
            other => self.person_ai_service_result(other),
        }
    }
}
