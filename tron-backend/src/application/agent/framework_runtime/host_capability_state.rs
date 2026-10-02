use std::{collections::BTreeMap, fs, path::Path};

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct CapabilityState {
    packs: BTreeMap<String, bool>,
    extensions: BTreeMap<String, bool>,
}
pub(super) fn load_capability_state(
    path: &Path,
) -> (BTreeMap<String, bool>, BTreeMap<String, bool>) {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<CapabilityState>(&bytes).ok())
        .map(|state| (state.packs, state.extensions))
        .unwrap_or_default()
}
pub(super) fn persist_capability_state(
    path: &Path,
    packs: &BTreeMap<String, bool>,
    extensions: &BTreeMap<String, bool>,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let bytes = serde_json::to_vec(&CapabilityState {
        packs: packs.clone(),
        extensions: extensions.clone(),
    })
    .map_err(|error| error.to_string())?;
    let temp = path.with_extension("tmp");
    fs::write(&temp, bytes).map_err(|error| error.to_string())?;
    fs::rename(temp, path).map_err(|error| error.to_string())
}
