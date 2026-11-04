use crate::{
    application::skin::{SkinError, SkinStore},
    domain::skin::{Skin, resource_mime, valid_resource_path, valid_skin_id, validate_theme_css},
};
use std::{
    collections::HashSet,
    fs,
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

const MAX_PACKAGE: usize = 20 * 1024 * 1024;
const MAX_EXPANDED: u64 = 40 * 1024 * 1024;
const ORIGINAL_MATRIX: &str =
    include_str!("../../../resources/skins/migrations/matrix-dark-original.json");
const PREVIOUS_MATRIX: &str = include_str!("../../../resources/matrix-dark-shipped-v1.json");
const LEGACY_MATRIX_PARTICLES: &str =
    include_str!("../../../resources/skins/migrations/matrix-particles.json");
const BUILTINS: [&str; 2] = [
    include_str!("../../../resources/skins/light.json"),
    include_str!("../../../resources/skins/matrix-dark.json"),
];

pub struct FileSkinStore {
    root: PathBuf,
    lock: Mutex<()>,
}

impl FileSkinStore {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            lock: Mutex::new(()),
        }
    }

    fn initialize(&self) -> Result<(), SkinError> {
        fs::create_dir_all(&self.root).map_err(|_| SkinError::Storage)?;
        for content in BUILTINS {
            let skin: Skin = serde_json::from_str(content).map_err(|_| SkinError::Storage)?;
            let dir = self.root.join(&skin.id);
            if !dir.exists() {
                fs::create_dir_all(&dir).map_err(|_| SkinError::Storage)?;
                atomic_write(&dir.join("skin.json"), content.as_bytes())?;
            } else if skin.id == "light" {
                // Refresh only the former shipped chart palette; retain all authored colors.
                let previous = self
                    .checked_resource(&skin.id, "skin.json")
                    .ok()
                    .and_then(|path| fs::read(path).ok())
                    .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
                if let Some(mut previous) = previous
                    && let Some(colors) = previous
                        .get_mut("colors")
                        .and_then(|value| value.as_object_mut())
                {
                    let legacy = [
                        ("--chart-1", "oklch(0.87 0 0)"),
                        ("--chart-2", "oklch(0.556 0 0)"),
                        ("--chart-3", "oklch(0.439 0 0)"),
                        ("--chart-4", "oklch(0.371 0 0)"),
                        ("--chart-5", "oklch(0.269 0 0)"),
                    ];
                    if legacy.iter().all(|(key, value)| {
                        colors.get(*key).and_then(|color| color.as_str()) == Some(*value)
                    }) {
                        for (key, _) in legacy {
                            if let Some(value) = skin.colors.get(key) {
                                colors.insert(key.into(), serde_json::Value::String(value.clone()));
                            }
                        }
                        let bytes =
                            serde_json::to_vec_pretty(&previous).map_err(|_| SkinError::Storage)?;
                        atomic_write(&dir.join("skin.json"), &bytes)?;
                    }
                }
            } else if skin.id == "matrix-dark" {
                // Upgrade only the unmodified shipped palette, preserving authored skins.
                let previous = self
                    .checked_resource(&skin.id, "skin.json")
                    .ok()
                    .and_then(|path| fs::read(path).ok())
                    .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
                let original: serde_json::Value =
                    serde_json::from_str(ORIGINAL_MATRIX).map_err(|_| SkinError::Storage)?;
                let previous_shipped: serde_json::Value =
                    serde_json::from_str(PREVIOUS_MATRIX).map_err(|_| SkinError::Storage)?;
                if previous.as_ref() == Some(&original)
                    || previous.as_ref() == Some(&previous_shipped)
                {
                    atomic_write(&dir.join("skin.json"), content.as_bytes())?;
                }
            }
        }
        Ok(())
    }

    fn read_skin(&self, id: &str) -> Result<Skin, SkinError> {
        if !valid_skin_id(id) {
            return Err(SkinError::Invalid);
        }
        let result = self
            .checked_resource(id, "skin.json")
            .and_then(|path| {
                if fs::metadata(&path).map_err(|_| SkinError::NotFound)?.len() > 256 * 1024 {
                    return Err(SkinError::Invalid);
                }
                fs::read(path).map_err(|_| SkinError::NotFound)
            })
            .and_then(|bytes| parse_skin(&bytes))
            .and_then(|skin| {
                if skin.id == id {
                    let mut skin = skin;
                    skin.market_id = self
                        .checked_resource(id, "market-source.json")
                        .ok()
                        .and_then(|path| fs::read(path).ok())
                        .and_then(|bytes| serde_json::from_slice::<String>(&bytes).ok());
                    Ok(skin)
                } else {
                    Err(SkinError::Invalid)
                }
            });
        result.or_else(|error| {
            BUILTINS
                .iter()
                .copied()
                .chain(std::iter::once(LEGACY_MATRIX_PARTICLES))
                .filter_map(|content| serde_json::from_str::<Skin>(content).ok())
                .find(|skin| skin.id == id)
                .ok_or(error)
        })
    }

    fn checked_resource(&self, id: &str, path: &str) -> Result<PathBuf, SkinError> {
        if !valid_skin_id(id) || !valid_resource_path(path) {
            return Err(SkinError::Invalid);
        }
        let root = self.root.canonicalize().map_err(|_| SkinError::Storage)?;
        let dir = self
            .root
            .join(id)
            .canonicalize()
            .map_err(|_| SkinError::NotFound)?;
        let resource = dir
            .join(path)
            .canonicalize()
            .map_err(|_| SkinError::NotFound)?;
        if !dir.starts_with(root) || !resource.starts_with(&dir) {
            return Err(SkinError::Invalid);
        }
        Ok(resource)
    }
}

impl FileSkinStore {
    fn install_package(&self, bytes: &[u8], market_id: Option<&str>) -> Result<Skin, SkinError> {
        if bytes.len() > MAX_PACKAGE {
            return Err(SkinError::Invalid);
        }
        let _guard = self.lock.lock().map_err(|_| SkinError::Storage)?;
        self.initialize()?;
        let staging = tempfile::tempdir_in(&self.root).map_err(|_| SkinError::Storage)?;
        let mut skin = if bytes.starts_with(b"PK") {
            let mut archive =
                zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| SkinError::Invalid)?;
            if archive.len() > 128 {
                return Err(SkinError::Invalid);
            }
            let mut total = 0;
            let mut names = HashSet::new();
            for index in 0..archive.len() {
                let mut file = archive.by_index(index).map_err(|_| SkinError::Invalid)?;
                let name = file.name().trim_end_matches('/').to_owned();
                if !valid_resource_path(&name)
                    || !names.insert(name.clone())
                    || file
                        .unix_mode()
                        .is_some_and(|mode| mode & 0o170000 == 0o120000)
                {
                    return Err(SkinError::Invalid);
                }
                if file.is_dir() {
                    continue;
                }
                total += file.size();
                if total > MAX_EXPANDED {
                    return Err(SkinError::Invalid);
                }
                if name != "skin.json" && resource_mime(&name).is_none() {
                    return Err(SkinError::Invalid);
                }
                let mut content = Vec::new();
                (&mut file)
                    .take(MAX_EXPANDED + 1)
                    .read_to_end(&mut content)
                    .map_err(|_| SkinError::Invalid)?;
                if content.len() as u64 != file.size() {
                    return Err(SkinError::Invalid);
                }
                let dest = staging.path().join(name);
                fs::create_dir_all(dest.parent().ok_or(SkinError::Invalid)?)
                    .map_err(|_| SkinError::Storage)?;
                fs::write(dest, content).map_err(|_| SkinError::Storage)?;
            }
            parse_skin(
                &fs::read(staging.path().join("skin.json")).map_err(|_| SkinError::Invalid)?,
            )?
        } else {
            let skin = parse_skin(bytes)?;
            fs::write(staging.path().join("skin.json"), bytes).map_err(|_| SkinError::Storage)?;
            skin
        };
        if self.root.join(&skin.id).exists() {
            return Err(SkinError::Conflict);
        }
        for (path, expected) in [
            (&skin.resources.background, "image/"),
            (&skin.resources.font, "font/"),
            (&skin.resources.css, "text/css"),
        ] {
            if let Some(path) = path {
                if path.starts_with("https://") {
                    continue;
                }
                let mime = match resource_mime(path) {
                    Some(mime) => mime,
                    None if expected == "text/css" => {
                        return Err(SkinError::CssRejected(format!(
                            "{path}: CSS resource must use a .css file"
                        )));
                    }
                    None => return Err(SkinError::Invalid),
                };
                if !mime.starts_with(expected) {
                    if expected == "text/css" {
                        return Err(SkinError::CssRejected(format!(
                            "{path}: CSS resource must use a .css file"
                        )));
                    }
                    return Err(SkinError::Invalid);
                }
                if !staging.path().join(path).is_file() {
                    if expected == "text/css" {
                        return Err(SkinError::CssRejected(format!(
                            "{path}: CSS file is missing from the package"
                        )));
                    }
                    return Err(SkinError::Invalid);
                }
            }
        }
        if let Some(path) = &skin.resources.css
            && resource_mime(path) != Some("text/css; charset=utf-8")
        {
            return Err(SkinError::Invalid);
        }
        if let Some(path) = &skin.resources.css {
            let css = fs::read_to_string(staging.path().join(path)).map_err(|_| {
                SkinError::CssRejected(format!("{path}: CSS file is missing or is not UTF-8"))
            })?;
            validate_theme_css(&css, staging.path())
                .map_err(|reason| SkinError::CssRejected(format!("{path}: {reason}")))?;
        }
        if let Some(id) = market_id {
            let metadata = serde_json::to_vec(id).map_err(|_| SkinError::Storage)?;
            fs::write(staging.path().join("market-source.json"), metadata)
                .map_err(|_| SkinError::Storage)?;
            skin.market_id = Some(id.to_owned());
        }
        fs::rename(staging.path(), self.root.join(&skin.id)).map_err(|_| SkinError::Storage)?;
        Ok(skin)
    }
}

impl SkinStore for FileSkinStore {
    fn list(&self) -> Result<Vec<Skin>, SkinError> {
        let _guard = self.lock.lock().map_err(|_| SkinError::Storage)?;
        self.initialize()?;
        let mut skins = vec![self.read_skin("light")?, self.read_skin("matrix-dark")?];
        for entry in fs::read_dir(&self.root)
            .map_err(|_| SkinError::Storage)?
            .flatten()
        {
            let id = entry.file_name().to_string_lossy().to_string();
            if id == "light" || id == "matrix-dark" || id == "matrix-particles" {
                continue;
            }
            if let Ok(skin) = self.read_skin(&id) {
                skins.push(skin);
            }
        }
        skins[2..].sort_by(|a, b| a.name.cmp(&b.name));
        Ok(skins)
    }

    fn active(&self) -> Result<Skin, SkinError> {
        let _guard = self.lock.lock().map_err(|_| SkinError::Storage)?;
        self.initialize()?;
        let id = fs::read(self.root.join("active.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
            .and_then(|value| {
                value
                    .get("id")
                    .and_then(|id| id.as_str())
                    .map(str::to_owned)
            });
        if let Some(skin) = id.and_then(|id| self.read_skin(&id).ok()) {
            return Ok(skin);
        }
        atomic_write(&self.root.join("active.json"), br#"{"id":"matrix-dark"}"#)?;
        self.read_skin("matrix-dark")
    }

    fn activate(&self, id: &str) -> Result<Skin, SkinError> {
        if id == "matrix-particles" {
            return Err(SkinError::NotFound);
        }
        let _guard = self.lock.lock().map_err(|_| SkinError::Storage)?;
        self.initialize()?;
        let skin = self.read_skin(id)?;
        for path in [
            &skin.resources.background,
            &skin.resources.font,
            &skin.resources.css,
        ]
        .into_iter()
        .flatten()
        {
            if !path.starts_with("https://") {
                self.checked_resource(id, path)?;
            }
        }
        atomic_write(
            &self.root.join("active.json"),
            &serde_json::to_vec(&serde_json::json!({"id": id})).map_err(|_| SkinError::Storage)?,
        )?;
        Ok(skin)
    }

    fn install(&self, bytes: &[u8]) -> Result<Skin, SkinError> {
        self.install_package(bytes, None)
    }

    fn install_market(&self, bytes: &[u8], market_id: &str) -> Result<Skin, SkinError> {
        self.install_package(bytes, Some(market_id))
    }

    fn uninstall(&self, id: &str) -> Result<(), SkinError> {
        if id == "light" || id == "matrix-dark" || id == "matrix-particles" {
            return Err(SkinError::Protected);
        }
        if !valid_skin_id(id) {
            return Err(SkinError::Invalid);
        }
        let _guard = self.lock.lock().map_err(|_| SkinError::Storage)?;
        self.initialize()?;
        self.read_skin(id)?;

        let active_path = self.root.join("active.json");
        let active_id = fs::read(&active_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
            .and_then(|value| {
                value
                    .get("id")
                    .and_then(|id| id.as_str())
                    .map(str::to_owned)
            });
        let was_active = active_id.as_deref() == Some(id);
        if was_active {
            atomic_write(&active_path, br#"{"id":"light"}"#)?;
        }

        if fs::remove_dir_all(self.root.join(id)).is_err() {
            if was_active {
                let active = serde_json::to_vec(&serde_json::json!({"id": id}))
                    .map_err(|_| SkinError::Storage)?;
                atomic_write(&active_path, &active)?;
            }
            return Err(SkinError::Storage);
        }
        Ok(())
    }

    fn resource(&self, id: &str, path: &str) -> Result<Vec<u8>, SkinError> {
        resource_mime(path).ok_or(SkinError::Invalid)?;
        let resource = self.checked_resource(id, path)?;
        if fs::metadata(&resource)
            .map_err(|_| SkinError::NotFound)?
            .len()
            > MAX_EXPANDED
        {
            return Err(SkinError::Invalid);
        }
        fs::read(resource).map_err(|_| SkinError::NotFound)
    }
}

fn parse_skin(bytes: &[u8]) -> Result<Skin, SkinError> {
    if bytes.len() > 256 * 1024 {
        return Err(SkinError::Invalid);
    }
    let skin: Skin = serde_json::from_slice(bytes).map_err(|_| SkinError::Invalid)?;
    if !matches!(skin.version, 1 | 2) {
        return Err(SkinError::Incompatible(format!(
            "skin.json: unsupported theme format version {}",
            skin.version
        )));
    }
    if skin.resources.css.is_some() && skin.version != 2 {
        return Err(SkinError::Incompatible(
            "skin.json: resources.css requires theme format version 2".into(),
        ));
    }
    if skin
        .resources
        .css
        .as_ref()
        .is_some_and(|path| !valid_resource_path(path))
    {
        return Err(SkinError::CssRejected(
            "skin.json: resources.css must reference a file inside the theme package".into(),
        ));
    }
    if !skin.validate() {
        return Err(SkinError::Invalid);
    }
    Ok(skin)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), SkinError> {
    let mut file = tempfile::NamedTempFile::new_in(path.parent().ok_or(SkinError::Storage)?)
        .map_err(|_| SkinError::Storage)?;
    file.write_all(bytes).map_err(|_| SkinError::Storage)?;
    file.as_file().sync_all().map_err(|_| SkinError::Storage)?;
    file.persist(path).map_err(|_| SkinError::Storage)?;
    Ok(())
}
