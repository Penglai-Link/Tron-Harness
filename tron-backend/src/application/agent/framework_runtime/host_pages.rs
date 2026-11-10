//! Static, product-specific capability page resources.
use super::*;
#[path = "page_validation.rs"]
mod validation;
use base64::Engine;
use eliteforge_agent_capability_runtime::CapabilityPackManifest;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackPage {
    pub id: String,
    pub title: BTreeMap<String, String>,
    pub root: String,
    pub entry: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pages {
    version: u32,
    pages: Vec<PackPage>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageBundle {
    pub pack_id: String,
    pub version: String,
    pub digest: String,
    pub page_id: String,
    pub title: BTreeMap<String, String>,
    pub entry: String,
    pub assets: BTreeMap<String, String>,
}
fn relative(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.contains(['\\', ':', '?', '#', '%'])
        || Path::new(value)
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err("invalid capability page resource path".into());
    }
    Ok(())
}
fn inside(root: &Path, name: &str) -> Result<PathBuf, String> {
    relative(name)?;
    let path = root.join(name).canonicalize().map_err(|e| e.to_string())?;
    if !path.starts_with(root) {
        return Err("page resource escapes root".into());
    }
    Ok(path)
}
fn assets(root: &Path) -> Result<BTreeMap<String, Vec<u8>>, String> {
    fn visit(
        root: &Path,
        dir: &Path,
        out: &mut BTreeMap<String, Vec<u8>>,
        total: &mut usize,
    ) -> Result<(), String> {
        if dir.strip_prefix(root).map_err(|e| e.to_string())?.components().count() > 16 { return Err("page asset directory nesting exceeds 16".into()); }
        for item in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let item = item.map_err(|e| e.to_string())?;
            let kind = item.file_type().map_err(|e| e.to_string())?;
            if kind.is_symlink() {
                return Err("page symlinks are not supported".into());
            }
            if kind.is_dir() {
                visit(root, &item.path(), out, total)?;
                continue;
            }
            if !kind.is_file() {
                return Err("invalid page resource".into());
            }
            let name = item
                .path()
                .strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            relative(&name)?;
            if !matches!(
                item.path().extension().and_then(|e| e.to_str()),
                Some(
                    "html"
                        | "js"
                        | "css"
                        | "png"
                        | "jpg"
                        | "jpeg"
                        | "gif"
                        | "webp"
                        | "svg"
                        | "ico"
                        | "woff"
                        | "woff2"
                        | "ttf"
                        | "otf"
                )
            ) {
                return Err(format!("unsupported page asset: {name}"));
            }
            let size = item.metadata().map_err(|e| e.to_string())?.len();
            if size > 16 * 1024 * 1024
                || *total + size as usize > 16 * 1024 * 1024
                || out.len() >= 256
            {
                return Err("page assets exceed limits".into());
            }
            let bytes = fs::read(item.path()).map_err(|e| e.to_string())?;
            *total += bytes.len();
            out.insert(name, bytes);
        }
        Ok(())
    }
    let mut out = BTreeMap::new();
    visit(root, root, &mut out, &mut 0)?;
    Ok(out)
}
pub(crate) fn read_pages(
    path: &Path,
    manifest: &CapabilityPackManifest,
) -> Result<Vec<PackPage>, String> {
    if !manifest.resources.iter().any(|r| r == "tron/pages.json") {
        return Ok(vec![]);
    }
    let root = path.canonicalize().map_err(|e| e.to_string())?;
    let file = inside(&root, "tron/pages.json")?;
    if fs::metadata(&file).map_err(|e| e.to_string())?.len() > 1024 * 1024 {
        return Err("page declaration too large".into());
    }
    let pages: Pages = serde_json::from_slice(&fs::read(file).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if pages.version != 1 || pages.pages.len() > 32 {
        return Err("unsupported page declaration".into());
    }
    let mut ids = BTreeSet::new();
    for page in &pages.pages {
        if page.id.is_empty()
            || page.id.len() > 64
            || !page
                .id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            || !ids.insert(&page.id)
        {
            return Err("invalid or duplicate page id".into());
        }
        if ["zh-CN", "en-US"]
            .iter()
            .any(|k| page.title.get(*k).is_none_or(|v| v.trim().is_empty()))
        {
            return Err("page titles are required".into());
        }
        let dir = inside(&root, &page.root)?;
        let files = assets(&dir)?;
        relative(&page.entry)?;
        if !page.entry.ends_with(".html") || !files.contains_key(&page.entry) {
            return Err("page entry is missing".into());
        }
        validation::validate(&files, &page.entry)?;
    }
    Ok(pages.pages)
}
impl FrameworkHost {
    pub fn capability_page_catalog(&self, pack_id: &str) -> Result<Vec<PackPage>, String> {
        if !self
            .capability_state()
            .0
            .get(pack_id)
            .copied()
            .unwrap_or(false)
        {
            return Err("capability page is unavailable".into());
        }
        let manager =
            eliteforge_agent_capability_runtime::CapabilityPackManager::new(&self.0.pack_root)
                .map_err(|e| e.to_string())?;
        let pack = manager
            .installed()
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter(|p| p.manifest.id == pack_id)
            .max_by(|a, b| a.manifest.version.cmp(&b.manifest.version))
            .ok_or("capability pack not found")?;
        read_pages(&pack.path, &pack.manifest)
    }

    pub fn capability_page(&self, pack_id: &str, page_id: &str) -> Result<PageBundle, String> {
        if !self
            .capability_state()
            .0
            .get(pack_id)
            .copied()
            .unwrap_or(false)
        {
            return Err("capability page is unavailable".into());
        }
        let manager =
            eliteforge_agent_capability_runtime::CapabilityPackManager::new(&self.0.pack_root)
                .map_err(|e| e.to_string())?;
        let pack = manager
            .installed()
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter(|p| p.manifest.id == pack_id)
            .max_by(|a, b| a.manifest.version.cmp(&b.manifest.version))
            .ok_or("capability pack not found")?;
        let page = read_pages(&pack.path, &pack.manifest)?
            .into_iter()
            .find(|p| p.id == page_id)
            .ok_or("capability page not found")?;
        let root = pack.path.canonicalize().map_err(|e| e.to_string())?;
        let files = assets(&inside(&root, &page.root)?)?;
        let mut hash = Sha256::new();
        hash.update(serde_json::to_vec(&page).map_err(|e| e.to_string())?);
        let mut encoded = BTreeMap::new();
        for (name, bytes) in files {
            hash.update((name.len() as u64).to_le_bytes());
            hash.update(name.as_bytes());
            hash.update((bytes.len() as u64).to_le_bytes());
            hash.update(&bytes);
            encoded.insert(
                name,
                base64::engine::general_purpose::STANDARD.encode(bytes),
            );
        }
        Ok(PageBundle {
            pack_id: pack_id.into(),
            version: pack.manifest.version.to_string(),
            digest: format!("{:x}", hash.finalize()),
            page_id: page.id,
            title: page.title,
            entry: page.entry,
            assets: encoded,
        })
    }
}
