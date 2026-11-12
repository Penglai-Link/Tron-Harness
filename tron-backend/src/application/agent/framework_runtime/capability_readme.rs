use eliteforge_agent_capability_runtime::{CapabilityPackManager, InstalledCapabilityPack};

pub(super) fn installed_readme(
    manager: &CapabilityPackManager,
    pack: &InstalledCapabilityPack,
) -> Result<Option<String>, String> {
    let mut names = Vec::new();
    for entry in std::fs::read_dir(&pack.path).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if name.eq_ignore_ascii_case("readme.md")
            && entry
                .file_type()
                .map_err(|error| error.to_string())?
                .is_file()
        {
            names.push(name.to_owned());
        }
    }
    // Keep selection deterministic on case-sensitive filesystems.
    names.sort();
    let Some(name) = names.first() else {
        return Ok(None);
    };
    // Use the package manager's version/digest and path-boundary checks.
    let bytes = manager
        .read_resource(pack, name)
        .map_err(|error| error.to_string())?;
    let content =
        String::from_utf8(bytes).map_err(|error| format!("Invalid UTF-8 in {name}: {error}"))?;
    let content = content.trim_start_matches('\u{feff}');
    Ok((!content.trim().is_empty()).then(|| content.to_owned()))
}
