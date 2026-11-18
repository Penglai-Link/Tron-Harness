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
const BUILTINS: [&str; 3] = [
    include_str!("../../../resources/skins/default.json"),
    include_str!("../../../resources/skins/brand.json"),
    include_str!("../../../resources/skins/matrix.json"),
];
const RETIRED_BUILTINS: [&str; 3] = ["light", "matrix-dark", "matrix-particles"];

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
            }
        }
        Ok(())
    }

    fn read_skin(&self, id: &str) -> Result<Skin, SkinError> {
        if !valid_skin_id(id) {
            return Err(SkinError::Invalid);
        }
        // Reserved built-in skins follow the running application version.
        if let Some(content) = BUILTINS
            .iter()
            .find(|content| serde_json::from_str::<Skin>(content).is_ok_and(|skin| skin.id == id))
        {
            return serde_json::from_str(content).map_err(|_| SkinError::Storage);
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
                        return Err(SkinError::CssRejected(
                            crate::shared::i18n_message::MessageText::new(
                                "tron.skin.css_resource_must_be_css",
                            )
                            .arg("path", path),
                        ));
                    }
                    None => return Err(SkinError::Invalid),
                };
                if !mime.starts_with(expected) {
                    if expected == "text/css" {
                        return Err(SkinError::CssRejected(
                            crate::shared::i18n_message::MessageText::new(
                                "tron.skin.css_resource_must_be_css",
                            )
                            .arg("path", path),
                        ));
                    }
                    return Err(SkinError::Invalid);
                }
                if !staging.path().join(path).is_file() {
                    if expected == "text/css" {
                        return Err(SkinError::CssRejected(
                            crate::shared::i18n_message::MessageText::new(
                                "tron.skin.css_resource_missing",
                            )
                            .arg("path", path),
                        ));
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
                SkinError::CssRejected(
                    crate::shared::i18n_message::MessageText::new(
                        "tron.skin.css_resource_unreadable",
                    )
                    .arg("path", path),
                )
            })?;
            validate_theme_css(&css, staging.path()).map_err(|reason| {
                SkinError::CssRejected(
                    crate::shared::i18n_message::MessageText::new("tron.skin.css_rejected")
                        .arg("path", path)
                        .arg("detail", reason),
                )
            })?;
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
        let mut skins = vec![
            self.read_skin("default")?,
            self.read_skin("brand")?,
            self.read_skin("matrix")?,
        ];
        for entry in fs::read_dir(&self.root)
            .map_err(|_| SkinError::Storage)?
            .flatten()
        {
            let id = entry.file_name().to_string_lossy().to_string();
            if id == "default"
                || id == "brand"
                || id == "matrix"
                || RETIRED_BUILTINS.contains(&id.as_str())
            {
                continue;
            }
            if let Ok(skin) = self.read_skin(&id) {
                skins.push(skin);
            }
        }
        skins[3..].sort_by(|a, b| a.name.cmp(&b.name));
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
        atomic_write(&self.root.join("active.json"), br#"{"id":"default"}"#)?;
        self.read_skin("default")
    }

    fn activate(&self, id: &str) -> Result<Skin, SkinError> {
        if RETIRED_BUILTINS.contains(&id) {
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
        if id == "default" || id == "brand" || id == "matrix" || RETIRED_BUILTINS.contains(&id) {
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
            atomic_write(&active_path, br#"{"id":"default"}"#)?;
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
    if skin.version != 3 {
        return Err(SkinError::Incompatible(
            crate::shared::i18n_message::MessageText::new("tron.skin.incompatible_version")
                .arg("version", skin.version.to_string()),
        ));
    }
    if skin
        .resources
        .css
        .as_ref()
        .is_some_and(|path| !valid_resource_path(path))
    {
        return Err(SkinError::CssRejected(
            crate::shared::i18n_message::MessageText::new(
                "tron.skin.css_reference_outside_package",
            ),
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
