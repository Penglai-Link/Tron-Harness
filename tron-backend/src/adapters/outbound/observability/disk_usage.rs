//! Cached local data usage. Scans run off the request path and never follow child symlinks.
use crate::{
    application::observability::{DiskUsageMeasurement, DiskUsageSnapshot},
    config::{AppConfig, ResourceStorageProfile},
};
use std::{
    fs::{self, Metadata},
    io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const REFRESH_INTERVAL: Duration = Duration::from_secs(60);
const SCAN_BUDGET: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub struct LocalDiskUsage(Arc<Inner>);

struct Inner {
    roots: Vec<PathBuf>,
    state: Mutex<SampleState>,
}

struct SampleState {
    snapshot: DiskUsageSnapshot,
    checked_at: Option<Instant>,
    refreshing: bool,
}

impl LocalDiskUsage {
    pub fn new(config: &AppConfig) -> Self {
        let roots = data_roots(config);
        Self(Arc::new(Inner {
            state: Mutex::new(SampleState {
                snapshot: DiskUsageSnapshot {
                    bytes: None,
                    sampled_at: None,
                    paths: roots.iter().map(|p| p.display().to_string()).collect(),
                    measurement: if cfg!(unix) {
                        DiskUsageMeasurement::Allocated
                    } else {
                        DiskUsageMeasurement::Logical
                    },
                },
                checked_at: None,
                refreshing: false,
            }),
            roots,
        }))
    }

    /// The first response can be pending; later polls reuse the last completed sample.
    pub fn snapshot(&self) -> DiskUsageSnapshot {
        let mut state = self.0.state.lock().unwrap_or_else(|e| e.into_inner());
        if !state.refreshing
            && state
                .checked_at
                .is_none_or(|at| at.elapsed() >= REFRESH_INTERVAL)
        {
            state.refreshing = true;
            let inner = self.0.clone();
            tokio::task::spawn_blocking(move || {
                let result = measure(&inner.roots);
                if let Err(error) = &result {
                    tracing::warn!(%error, "Could not sample Tron data disk usage");
                }
                let mut state = inner.state.lock().unwrap_or_else(|e| e.into_inner());
                // Unknown or incomplete scans must not look like zero usage.
                state.snapshot.bytes = result.ok();
                state.snapshot.sampled_at = Some(super::now());
                state.checked_at = Some(Instant::now());
                state.refreshing = false;
            });
        }
        state.snapshot.clone()
    }
}

fn data_roots(config: &AppConfig) -> Vec<PathBuf> {
    let config_file = PathBuf::from(&config.application_config_path);
    let config_dir = config_file.parent().unwrap_or_else(|| Path::new("."));
    let mut roots = if config_dir.file_name().is_some_and(|name| name == ".tron") {
        vec![config_dir.to_path_buf()]
    } else {
        // A custom YAML can live in a repository. Do not scan that entire repository.
        vec![
            config_file.clone(),
            config_dir.join("logs"),
            config_dir.join("skins"),
        ]
    };
    if !config.database_url.starts_with("file:") && config.database_url != ":memory:" {
        let database = PathBuf::from(&config.database_url);
        sqlite_roots(&database, &mut roots);
        sqlite_roots(&database.with_extension("telemetry.sqlite3"), &mut roots);
        for extension in [
            "artifacts",
            "capabilities.json",
            "capability-packs",
            "skills",
            "skills-catalog",
            "workspaces",
        ] {
            roots.push(database.with_extension(extension));
        }
    }
    if config.graph_storage_path != ":memory:" {
        let graph = PathBuf::from(&config.graph_storage_path);
        roots.push(graph.clone());
        roots.push(suffixed(&graph, ".wal"));
    }
    roots.push(PathBuf::from(&config.skills_registry_root));
    for profile in config.resource.storage.profiles.values() {
        if let ResourceStorageProfile::Local(local) = profile {
            roots.push(PathBuf::from(&local.root));
        }
    }
    // Resolve even missing paths through their existing ancestors, so macOS
    // /var and /private/var aliases cannot become duplicate roots later.
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut roots = roots
        .into_iter()
        .map(|path| {
            resolve_root(&if path.is_absolute() {
                path
            } else {
                cwd.join(path)
            })
        })
        .collect::<Vec<_>>();
    roots.sort_by_key(|path| (path.components().count(), path.clone()));
    let mut unique: Vec<PathBuf> = Vec::new();
    for root in roots {
        if !unique.iter().any(|parent| root.starts_with(parent)) {
            unique.push(root);
        }
    }
    unique
}

fn resolve_root(path: &Path) -> PathBuf {
    let mut ancestor = path;
    let mut tail = Vec::new();
    loop {
        if let Ok(mut resolved) = fs::canonicalize(ancestor) {
            for name in tail.into_iter().rev() {
                resolved.push(name);
            }
            return resolved;
        }
        match (ancestor.parent(), ancestor.file_name()) {
            (Some(parent), Some(name)) => {
                tail.push(name);
                ancestor = parent;
            }
            _ => return path.to_path_buf(),
        }
    }
}

fn sqlite_roots(path: &Path, roots: &mut Vec<PathBuf>) {
    roots.extend([
        path.to_path_buf(),
        suffixed(path, "-wal"),
        suffixed(path, "-shm"),
    ]);
}

fn suffixed(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    value.into()
}

fn measure(roots: &[PathBuf]) -> io::Result<u64> {
    let started = Instant::now();
    let mut bytes = 0u64;
    #[cfg(unix)]
    let mut hard_links = std::collections::BTreeSet::new();
    for root in roots {
        let mut directories = Vec::new();
        let mut visit = |path: &Path| -> io::Result<Option<fs::ReadDir>> {
            if started.elapsed() >= SCAN_BUDGET {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "disk usage scan exceeded its time budget",
                ));
            }
            let metadata = match fs::symlink_metadata(path) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
                Err(error) => return Err(error),
            };
            if metadata.file_type().is_symlink() || !(metadata.is_file() || metadata.is_dir()) {
                return Ok(None);
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                if metadata.is_file()
                    && metadata.nlink() > 1
                    && !hard_links.insert((metadata.dev(), metadata.ino()))
                {
                    return Ok(None);
                }
            }
            bytes = bytes
                .checked_add(file_bytes(&metadata))
                .ok_or_else(|| io::Error::other("disk usage overflow"))?;
            if metadata.is_dir() {
                fs::read_dir(path).map(Some)
            } else {
                Ok(None)
            }
        };
        if let Some(directory) = visit(root)? {
            directories.push(directory);
        }
        while let Some(directory) = directories.last_mut() {
            match directory.next() {
                Some(entry) => {
                    if let Some(child) = visit(&entry?.path())? {
                        directories.push(child);
                    }
                }
                None => {
                    directories.pop();
                }
            }
        }
    }
    Ok(bytes)
}

fn file_bytes(metadata: &Metadata) -> u64 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        metadata.blocks().saturating_mul(512)
    }
    #[cfg(not(unix))]
    {
        if metadata.is_file() {
            metadata.len()
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(root: &Path) -> AppConfig {
        AppConfig {
            application_config_path: root.join("application.yaml").display().to_string(),
            database_url: root.join("tron.sqlite3").display().to_string(),
            graph_storage_path: root.join("tron.graph.grafeo").display().to_string(),
            skills_registry_root: root.join("skills").display().to_string(),
            resource: crate::config::ResourceConfig::with_default_root(
                root.join("resources").display().to_string(),
            ),
            ..AppConfig::default()
        }
    }

    #[test]
    fn scope_deduplicates_nested_storage_and_excludes_repository_siblings() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join(".tron");
        fs::create_dir(&home).unwrap();
        assert_eq!(
            data_roots(&config(&home)),
            vec![fs::canonicalize(&home).unwrap()]
        );
        let repository = dir.path().join("repository");
        fs::create_dir(&repository).unwrap();
        fs::write(repository.join("application.yaml"), "configuration").unwrap();
        fs::write(repository.join("source.rs"), "unrelated source").unwrap();
        let roots = data_roots(&config(&repository));
        assert!(!roots.contains(&repository));
        assert!(roots.iter().all(|path| !path.ends_with("source.rs")));
        assert_eq!(
            measure(&roots).unwrap(),
            file_bytes(&fs::metadata(repository.join("application.yaml")).unwrap())
        );
    }

    #[cfg(unix)]
    #[test]
    fn counts_allocated_blocks_once_without_following_links_into_repositories() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        let repo = dir.path().join("repository");
        fs::create_dir(&data).unwrap();
        fs::create_dir(&repo).unwrap();
        fs::write(data.join("log"), vec![1u8; 8192]).unwrap();
        fs::hard_link(data.join("log"), data.join("same-log")).unwrap();
        fs::write(repo.join("source"), vec![1u8; 32768]).unwrap();
        symlink(&repo, data.join("project")).unwrap();
        symlink(&data, data.join("cycle")).unwrap();
        let expected = file_bytes(&fs::metadata(&data).unwrap())
            + file_bytes(&fs::metadata(data.join("log")).unwrap());
        assert_eq!(measure(&[data]).unwrap(), expected);
    }

    #[test]
    fn missing_files_are_empty_but_failed_scans_are_not_reported_as_zero() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(measure(&[dir.path().join("missing")]).unwrap(), 0);
        fs::write(dir.path().join("file"), "data").unwrap();
        assert!(measure(&[dir.path().join("file/invalid-child")]).is_err());
    }

    #[tokio::test]
    async fn background_samples_are_shared_and_cached_across_dashboard_polls() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join(".tron");
        fs::create_dir(&home).unwrap();
        fs::write(home.join("log"), vec![1u8; 8192]).unwrap();
        let sampler = LocalDiskUsage::new(&config(&home));
        assert_eq!(sampler.snapshot().bytes, None);
        let first = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let snapshot = sampler.snapshot();
                if snapshot.bytes.is_some() {
                    break snapshot;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(first.bytes.unwrap() > 0);
        fs::write(home.join("large-log"), vec![1u8; 32768]).unwrap();
        let cached = sampler.clone().snapshot();
        assert_eq!(cached.bytes, first.bytes);
        assert_eq!(cached.sampled_at, first.sampled_at);
        sampler.0.state.lock().unwrap().checked_at = Some(Instant::now() - REFRESH_INTERVAL);
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if sampler.snapshot().bytes > first.bytes {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }
}
