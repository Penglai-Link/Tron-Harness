use std::path::Path;

/// Read a package README for display without re-verifying the entire payload.
///
/// The package directory is canonicalized so a symlinked README or package path
/// cannot escape the installed version.
pub(super) fn installed_readme(path: &Path) -> Result<Option<String>, String> {
    let package = path.canonicalize().map_err(|error| error.to_string())?;
    let mut names = Vec::new();
    for entry in std::fs::read_dir(&package).map_err(|error| error.to_string())? {
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
    let file = package
        .join(name)
        .canonicalize()
        .map_err(|error| error.to_string())?;
    if !file.starts_with(&package) {
        return Err("capability README escapes its package".into());
    }
    let bytes = std::fs::read(&file).map_err(|error| error.to_string())?;
    let content =
        String::from_utf8(bytes).map_err(|error| format!("Invalid UTF-8 in {name}: {error}"))?;
    let content = content.trim_start_matches('\u{feff}');
    Ok((!content.trim().is_empty()).then(|| content.to_owned()))
}
