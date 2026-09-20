use std::{env, path::PathBuf};

const DEFAULT_DATABASE_URL_SUFFIX: &str = ".tron/tron.sqlite3";
const DEFAULT_APPLICATION_CONFIG_PATH_SUFFIX: &str = ".tron/application.yaml";
const DEFAULT_GRAPH_STORAGE_PATH_SUFFIX: &str = ".tron/tron.graph.grafeo";
const DEFAULT_SKILLS_REGISTRY_ROOT_SUFFIX: &str = ".tron/skills-registry";
const DEFAULT_RESOURCE_STORAGE_ROOT_SUFFIX: &str = ".tron/resources";
const ENV_DATABASE_URL: &str = "TRON_BACKEND_DATABASE_URL";
const ENV_GRAPH_STORAGE_PATH: &str = "TRON_BACKEND_GRAPH_STORAGE_PATH";

pub(super) fn default_database_url_from_home() -> String {
    default_path_from_home(env::var("HOME").ok(), DEFAULT_DATABASE_URL_SUFFIX)
}

pub(super) fn default_application_config_path_from_home() -> String {
    default_path_from_home(
        env::var("HOME").ok(),
        DEFAULT_APPLICATION_CONFIG_PATH_SUFFIX,
    )
}

pub(super) fn default_graph_storage_path_from_home() -> String {
    default_path_from_home(env::var("HOME").ok(), DEFAULT_GRAPH_STORAGE_PATH_SUFFIX)
}

pub(super) fn default_skills_registry_root_from_home() -> String {
    default_path_from_home(env::var("HOME").ok(), DEFAULT_SKILLS_REGISTRY_ROOT_SUFFIX)
}

pub(super) fn default_resource_storage_root_from_home() -> String {
    default_path_from_home(env::var("HOME").ok(), DEFAULT_RESOURCE_STORAGE_ROOT_SUFFIX)
}

pub(super) fn env_database_url_override() -> Option<String> {
    env::var(ENV_DATABASE_URL)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(normalize_storage_path)
}

pub(super) fn env_graph_storage_path_override() -> Option<String> {
    env::var(ENV_GRAPH_STORAGE_PATH)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(normalize_storage_path)
}

pub(super) fn normalize_storage_path(value: String) -> String {
    if value == ":memory:" || value.starts_with("file:") {
        return value;
    }
    expand_home_path(value)
}

fn expand_home_path(value: String) -> String {
    if let Ok(profile_dir) = env::var("TRON_PROFILE_DATA_DIR")
        && !profile_dir.trim().is_empty()
        && let Some(suffix) = value.strip_prefix("~/.tron")
    {
        return PathBuf::from(profile_dir.trim())
            .join(suffix.trim_start_matches('/'))
            .display()
            .to_string();
    }
    if value == "~" {
        return env::var("HOME")
            .map(|home| PathBuf::from(home).display().to_string())
            .unwrap_or_else(|_| ".".into());
    }

    let Some(rest) = value.strip_prefix("~/") else {
        return value;
    };

    default_path_from_home(env::var("HOME").ok(), rest)
}

fn default_path_from_home(home: Option<String>, suffix: &str) -> String {
    if let Ok(profile_dir) = env::var("TRON_PROFILE_DATA_DIR")
        && !profile_dir.trim().is_empty()
        && let Some(rest) = suffix.strip_prefix(".tron")
    {
        return PathBuf::from(profile_dir.trim())
            .join(rest.trim_start_matches('/'))
            .display()
            .to_string();
    }
    home.map(|home| PathBuf::from(home).join(suffix))
        .unwrap_or_else(|| PathBuf::from(suffix))
        .display()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::default_application_config_path_from_home;
    use std::{env, path::PathBuf};

    #[test]
    fn application_config_defaults_to_home_tron_directory() {
        let expected = PathBuf::from(env::var("HOME").expect("HOME should be set"))
            .join(".tron/application.yaml")
            .display()
            .to_string();
        assert_eq!(default_application_config_path_from_home(), expected);
    }
}
