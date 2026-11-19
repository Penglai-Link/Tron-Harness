//! Join the published catalog to this profile's installed runtime, before paging local filters.
use crate::application::{
    agent::{CapabilityPackInfo, FrameworkHost, query_installed_capability_packs},
    model_provider::{ModelProviderError, ModelProviderService},
};
use crab_serverapi::capability::{CapabilityDetail, CapabilityQuery, CapabilitySummary};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CenterStatus {
    #[default]
    All,
    Installed,
    Updatable,
    Enabled,
    Disabled,
}

pub struct CenterQuery {
    pub current: u64,
    pub size: u64,
    pub keyword: Option<String>,
    pub category: Option<String>,
    pub source: Option<String>,
    pub status: CenterStatus,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CenterPack {
    pub pack_id: String,
    pub release_id: Option<String>,
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub source: String,
    pub category: Option<String>,
    pub installed_version: Option<String>,
    pub enabled: bool,
    pub builtin: bool,
    pub update_available: bool,
    pub update_check_failed: bool,
    pub file_size: Option<u64>,
    pub downloads: Option<u64>,
    pub like_count: Option<u64>,
    pub liked: Option<bool>,
}

impl CenterPack {
    pub fn from_release(release: CapabilitySummary, local: Option<&CapabilityPackInfo>) -> Self {
        let update_available =
            local.is_some_and(|pack| !pack.builtin && newer(&release.version, &pack.version));
        Self {
            pack_id: release.pack_id,
            release_id: Some(release.id),
            name: release.name,
            description: release.description,
            author: release.author,
            version: release.version,
            source: release.source,
            category: Some(release.category),
            installed_version: local.map(|p| p.version.clone()),
            enabled: local.is_some_and(|p| p.effective),
            builtin: local.is_some_and(|p| p.builtin),
            update_available,
            update_check_failed: false,
            file_size: Some(release.file_size),
            downloads: Some(release.download_count),
            like_count: Some(release.like_count),
            liked: Some(release.liked),
        }
    }
    fn local(pack: CapabilityPackInfo, failed: bool) -> Self {
        let builtin = pack.builtin;
        Self {
            pack_id: pack.id,
            release_id: None,
            name: pack.name,
            description: pack.description,
            author: pack.provider,
            version: pack.version.clone(),
            source: if builtin { "builtin" } else { "local" }.into(),
            category: None,
            installed_version: Some(pack.version),
            enabled: pack.effective,
            builtin,
            update_available: false,
            update_check_failed: failed,
            file_size: None,
            downloads: None,
            like_count: None,
            liked: None,
        }
    }
}

pub fn newer(remote: &str, local: &str) -> bool {
    match (
        semver::Version::parse(remote),
        semver::Version::parse(local),
    ) {
        (Ok(remote), Ok(local)) => remote.cmp_precedence(&local).is_gt(),
        _ => false,
    }
}

pub async fn installed(
    host: &FrameworkHost,
    locale: &str,
) -> Result<Vec<CapabilityPackInfo>, ModelProviderError> {
    let root = host.pack_root().to_path_buf();
    let locale = locale.to_owned();
    let (enabled, _) = host.capability_state();
    let mut records = tokio::task::spawn_blocking(move || {
        query_installed_capability_packs(&root, &locale, false, None)
    })
    .await
    .map_err(|e| ModelProviderError::Remote(e.to_string()))?
    .map_err(ModelProviderError::Remote)?;
    records.retain(|p| p.kind == "capability");
    for pack in &mut records {
        pack.effective = enabled.get(&pack.id).copied().unwrap_or(pack.effective);
    }
    Ok(records)
}

pub async fn page(
    provider: &ModelProviderService,
    host: &FrameworkHost,
    user: &str,
    locale: &str,
    query: CenterQuery,
) -> Result<(Vec<CenterPack>, u64, bool), ModelProviderError> {
    if !(1..=1_000_000).contains(&query.current)
        || !(1..=100).contains(&query.size)
        || query
            .source
            .as_ref()
            .is_some_and(|s| !matches!(s.as_str(), "official" | "third_party"))
    {
        return Err(ModelProviderError::InvalidSelection(
            tron_i18n::Message::new("tron.model_provider.capability_center_query_invalid"),
        ));
    }
    let local = installed(host, locale).await?;
    if query.status == CenterStatus::All {
        let result = provider
            .capability_center_page(
                user,
                &CapabilityQuery {
                    keyword: query.keyword,
                    source: query.source,
                    category: query.category,
                    ..CapabilityQuery::of(query.current, query.size)
                },
            )
            .await?;
        return Ok((
            result
                .records
                .into_iter()
                .map(|r| {
                    let pack = local.iter().find(|p| p.id == r.pack_id);
                    CenterPack::from_release(r, pack)
                })
                .collect(),
            result.total,
            false,
        ));
    }
    // Resolve each installed ID, not just releases visible on the current market page.
    // Bounded concurrency avoids a request burst for large local installations.
    let mut records = Vec::new();
    let mut tasks = tokio::task::JoinSet::new();
    for pack in local {
        if (query.status == CenterStatus::Enabled && !pack.effective)
            || (query.status == CenterStatus::Disabled && pack.effective)
        {
            continue;
        }
        if pack.builtin {
            records.push(CenterPack::local(pack, false));
            continue;
        }
        let provider = provider.clone();
        let user = user.to_owned();
        tasks.spawn(async move {
            let result = provider
                .capability_center_page(
                    &user,
                    &CapabilityQuery {
                        pack_id: Some(pack.id.clone()),
                        ..CapabilityQuery::of(1, 1)
                    },
                )
                .await;
            (pack, result)
        });
        if tasks.len() >= 6 {
            records.push(resolve_local(tasks.join_next().await, query.status)?);
        }
    }
    while !tasks.is_empty() {
        records.push(resolve_local(tasks.join_next().await, query.status)?);
    }
    let incomplete = records.iter().any(|r| r.update_check_failed);
    let keyword = query.keyword.unwrap_or_default().trim().to_lowercase();
    records.retain(|r| {
        (query.status != CenterStatus::Updatable || r.update_available)
            && query.source.as_ref().is_none_or(|s| &r.source == s)
            && query
                .category
                .as_ref()
                .is_none_or(|c| r.category.as_ref() == Some(c))
            && (keyword.is_empty()
                || format!("{} {} {} {}", r.pack_id, r.name, r.description, r.author)
                    .to_lowercase()
                    .contains(&keyword))
    });
    records.sort_by(|a, b| {
        b.update_available
            .cmp(&a.update_available)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.pack_id.cmp(&b.pack_id))
    });
    let total = records.len() as u64;
    Ok((
        records
            .into_iter()
            .skip(((query.current - 1) * query.size) as usize)
            .take(query.size as usize)
            .collect(),
        total,
        incomplete,
    ))
}

type LocalLookup = Option<
    Result<
        (
            CapabilityPackInfo,
            Result<crab_serverapi::capability::CapabilityPage, ModelProviderError>,
        ),
        tokio::task::JoinError,
    >,
>;
fn resolve_local(
    result: LocalLookup,
    status: CenterStatus,
) -> Result<CenterPack, ModelProviderError> {
    let (pack, remote) = result
        .ok_or(ModelProviderError::Storage)?
        .map_err(|e| ModelProviderError::Remote(e.to_string()))?;
    match remote {
        Ok(page) => Ok(page
            .records
            .into_iter()
            .next()
            .map(|r| CenterPack::from_release(r, Some(&pack)))
            .unwrap_or_else(|| CenterPack::local(pack, false))),
        Err(error) if status == CenterStatus::Updatable => Err(error),
        Err(_) => Ok(CenterPack::local(pack, true)),
    }
}

pub enum InstallBlock {
    Manifest(String),
    Identity,
    Dependency(String),
    Platform(String),
    Builtin,
    NotNewer,
}

pub fn validate_release(
    detail: &CapabilityDetail,
    local: &[CapabilityPackInfo],
) -> Result<(), InstallBlock> {
    let manifest: eliteforge_agent_capability_runtime::CapabilityPackManifest =
        serde_json::from_value(detail.manifest.clone())
            .map_err(|e| InstallBlock::Manifest(e.to_string()))?;
    manifest
        .validate()
        .map_err(|e| InstallBlock::Manifest(e.to_string()))?;
    if manifest.id != detail.summary.pack_id
        || manifest.version.to_string() != detail.summary.version
    {
        return Err(InstallBlock::Identity);
    }
    for dependency in &manifest.dependencies {
        if !local.iter().any(|p| {
            p.id == dependency.id
                && semver::Version::parse(&p.version).is_ok_and(|v| dependency.version.matches(&v))
        }) {
            return Err(InstallBlock::Dependency(format!(
                "{} {}",
                dependency.id, dependency.version
            )));
        }
    }
    if let Some(backend) = &manifest.backend {
        let target = eliteforge_agent_capability_runtime::platform_key();
        // Use the native runtime target key, not the product host OS.
        if !backend.executables.contains_key(&target) {
            return Err(InstallBlock::Platform(target));
        }
    }
    if let Some(pack) = local.iter().find(|p| p.id == manifest.id) {
        if pack.builtin {
            return Err(InstallBlock::Builtin);
        }
        if !newer(&detail.summary.version, &pack.version) {
            return Err(InstallBlock::NotNewer);
        }
    }
    Ok(())
}
