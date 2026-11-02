use super::*;
use crate::domain::model::{AutoRouteSources, AutoRouterKind};

impl ModelProviderService {
    pub fn automatic_route_sources(
        &self,
    ) -> Result<BTreeMap<String, AutoRouteSources>, ModelProviderError> {
        self.connection.call(
            |connection| {
                let mut statement = connection
                    .prepare("SELECT router_kind,sources_json FROM automatic_router_sources")
                    .map_err(|_| ModelProviderError::Storage)?;
                let rows = statement
                    .query_map([], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                    })
                    .map_err(|_| ModelProviderError::Storage)?;
                rows.map(|row| {
                    let (kind, json) = row.map_err(|_| ModelProviderError::Storage)?;
                    Ok((
                        kind,
                        serde_json::from_str(&json).map_err(|_| ModelProviderError::Storage)?,
                    ))
                })
                .collect()
            },
            || ModelProviderError::Storage,
        )
    }
    pub fn save_automatic_route_sources(
        &self,
        kind: AutoRouterKind,
        sources: AutoRouteSources,
    ) -> Result<(), ModelProviderError> {
        if !sources.valid() {
            return Err(ModelProviderError::InvalidSelection(
                "invalid route sources".into(),
            ));
        }
        let json = serde_json::to_string(&sources).map_err(|_| ModelProviderError::Storage)?;
        self.connection.call(move |connection| {
            connection.execute("INSERT INTO automatic_router_sources(router_kind,sources_json) VALUES(?1,?2) ON CONFLICT(router_kind) DO UPDATE SET sources_json=excluded.sources_json", params![kind.code(),json]).map_err(|_|ModelProviderError::Storage)?;
            Ok(())
        }, ||ModelProviderError::Storage)
    }
}
