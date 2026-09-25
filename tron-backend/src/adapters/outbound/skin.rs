use crate::{
    application::skin::{SkinError, SkinStore},
    domain::skin::{Skin, resource_mime, valid_resource_path, valid_skin_id},
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
            } else if skin.id == "matrix-dark" {
                // Upgrade only the unmodified shipped palette, preserving authored skins.
                let previous = self
                    .checked_resource(&skin.id, "skin.json")
                    .ok()
                    .and_then(|path| fs::read(path).ok())
                    .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
                let original: serde_json::Value =
                    serde_json::from_str(ORIGINAL_MATRIX).map_err(|_| SkinError::Storage)?;
                if previous.as_ref() == Some(&original) {
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
                    Ok(skin)
                } else {
                    Err(SkinError::Invalid)
                }
            });
        result.or_else(|error| {
            BUILTINS
                .iter()
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
            if id == "light" || id == "matrix-dark" {
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
        atomic_write(&self.root.join("active.json"), br#"{"id":"light"}"#)?;
        self.read_skin("light")
    }

    fn activate(&self, id: &str) -> Result<Skin, SkinError> {
        let _guard = self.lock.lock().map_err(|_| SkinError::Storage)?;
        self.initialize()?;
        let skin = self.read_skin(id)?;
        for path in [&skin.resources.background, &skin.resources.font]
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
        if bytes.len() > MAX_PACKAGE {
            return Err(SkinError::Invalid);
        }
        let _guard = self.lock.lock().map_err(|_| SkinError::Storage)?;
        self.initialize()?;
        let staging = tempfile::tempdir_in(&self.root).map_err(|_| SkinError::Storage)?;
        let skin = if bytes.starts_with(b"PK") {
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
        for (path, is_font) in [
            (&skin.resources.background, false),
            (&skin.resources.font, true),
        ] {
            if let Some(path) = path {
                if path.starts_with("https://") {
                    continue;
                }
                let mime = resource_mime(path).ok_or(SkinError::Invalid)?;
                if mime.starts_with("font/") != is_font || !staging.path().join(path).is_file() {
                    return Err(SkinError::Invalid);
                }
            }
        }
        fs::rename(staging.path(), self.root.join(&skin.id)).map_err(|_| SkinError::Storage)?;
        Ok(skin)
    }

    fn uninstall(&self, id: &str) -> Result<(), SkinError> {
        if id == "light" || id == "matrix-dark" {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn upgrades_original_matrix_and_preserves_customized_palette() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("matrix-dark");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("skin.json"), ORIGINAL_MATRIX).unwrap();
        fs::write(temp.path().join("active.json"), br#"{"id":"matrix-dark"}"#).unwrap();
        let store = FileSkinStore::new(temp.path().into());
        let current: Skin = serde_json::from_str(BUILTINS[1]).unwrap();
        assert_eq!(store.active().unwrap().colors, current.colors);
        let persisted = parse_skin(&fs::read(dir.join("skin.json")).unwrap()).unwrap();
        assert_eq!(persisted.colors, current.colors);

        let mut customized: Skin = serde_json::from_str(ORIGINAL_MATRIX).unwrap();
        customized
            .colors
            .insert("--foreground".into(), "#abcdef".into());
        let bytes = serde_json::to_vec(&customized).unwrap();
        fs::write(dir.join("skin.json"), &bytes).unwrap();
        assert_eq!(store.active().unwrap().colors, customized.colors);
        assert_eq!(fs::read(dir.join("skin.json")).unwrap(), bytes);
    }

    #[test]
    fn authoring_skill_templates_install_and_activate() {
        let temp = tempfile::tempdir().unwrap();
        let store = FileSkinStore::new(temp.path().into());
        for bytes in [
            include_bytes!("../../../../tron-skill/tron-theme-authoring/assets/light/skin.json")
                .as_slice(),
            include_bytes!("../../../../tron-skill/tron-theme-authoring/assets/dark/skin.json")
                .as_slice(),
        ] {
            let skin = store.install(bytes).unwrap();
            assert!(skin.layout.contains_key("--tron-table-row-height"));
            assert_eq!(store.activate(&skin.id).unwrap().id, skin.id);
        }
    }
    fn custom() -> Skin {
        let mut skin: Skin = serde_json::from_str(BUILTINS[0]).unwrap();
        skin.id = "ocean".into();
        skin
    }
    #[test]
    fn skins_survive_restart_and_missing_active_falls_back() {
        let temp = tempfile::tempdir().unwrap();
        let store = FileSkinStore::new(temp.path().into());
        assert_eq!(store.active().unwrap().id, "light");
        store
            .install(&serde_json::to_vec(&custom()).unwrap())
            .unwrap();
        store.activate("ocean").unwrap();
        let reopened = FileSkinStore::new(temp.path().into());
        assert_eq!(reopened.active().unwrap().id, "ocean");
        assert_eq!(reopened.list().unwrap().len(), 3);
        fs::remove_dir_all(temp.path().join("ocean")).unwrap();
        assert_eq!(reopened.active().unwrap().id, "light");
    }
    #[test]
    fn rejects_overwrite_injection_and_missing_resources() {
        let temp = tempfile::tempdir().unwrap();
        let store = FileSkinStore::new(temp.path().into());
        assert!(matches!(
            store.install(BUILTINS[0].as_bytes()),
            Err(SkinError::Conflict)
        ));
        let mut skin = custom();
        skin.colors
            .insert("--color-primary".into(), "red;}body{display:none".into());
        assert!(matches!(
            store.install(&serde_json::to_vec(&skin).unwrap()),
            Err(SkinError::Invalid)
        ));
        let mut skin = custom();
        skin.resources.background = Some("missing.png".into());
        assert!(store.install(&serde_json::to_vec(&skin).unwrap()).is_err());
        assert!(store.activate("../light").is_err());
        assert!(store.resource("light", "../../active.json").is_err());
    }
    #[test]
    fn broken_builtin_and_active_files_recover() {
        let temp = tempfile::tempdir().unwrap();
        let store = FileSkinStore::new(temp.path().into());
        store.active().unwrap();
        fs::write(temp.path().join("light/skin.json"), b"broken").unwrap();
        fs::write(temp.path().join("active.json"), b"broken").unwrap();
        assert_eq!(store.active().unwrap().id, "light");
        assert_eq!(store.list().unwrap().len(), 2);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_resource_and_manifest_symlinks_outside_skin_root() {
        let temp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let store = FileSkinStore::new(temp.path().into());
        store.active().unwrap();
        fs::write(outside.path().join("secret.png"), b"secret").unwrap();
        fs::write(
            outside.path().join("skin.json"),
            serde_json::to_vec(&custom()).unwrap(),
        )
        .unwrap();
        std::os::unix::fs::symlink(outside.path(), temp.path().join("ocean")).unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("secret.png"),
            temp.path().join("light/secret.png"),
        )
        .unwrap();
        assert!(store.resource("light", "secret.png").is_err());
        assert!(store.activate("ocean").is_err());
    }

    #[test]
    fn installs_zip_and_rejects_zip_traversal() {
        let temp = tempfile::tempdir().unwrap();
        let store = FileSkinStore::new(temp.path().into());
        for traversal in [true, false] {
            let mut skin = custom();
            skin.resources.background = Some("assets/bg.png".into());
            let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
            let options = zip::write::SimpleFileOptions::default();
            zip.start_file("skin.json", options).unwrap();
            zip.write_all(&serde_json::to_vec(&skin).unwrap()).unwrap();
            zip.start_file(
                if traversal {
                    "../bg.png"
                } else {
                    "assets/bg.png"
                },
                options,
            )
            .unwrap();
            zip.write_all(b"image").unwrap();
            let bytes = zip.finish().unwrap().into_inner();
            if traversal {
                assert!(store.install(&bytes).is_err());
            } else {
                store.install(&bytes).unwrap();
                store.activate("ocean").unwrap();
                assert_eq!(store.resource("ocean", "assets/bg.png").unwrap(), b"image");
            }
        }
    }
}
