//! Rebuild the crate whenever the centralized locale resources change.
//!
//! `rust_i18n::i18n!` reads `../../locales` at compile time. Cargo only reruns a
//! proc macro when its input files are tracked, so list every resource file
//! explicitly; otherwise edits to YAML would silently leave a stale compiled
//! table in incremental builds.
use std::{env, fs, path::Path, path::PathBuf};

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"))
        .join("../../locales");
    println!("cargo:rerun-if-changed={}", root.display());
    for file in locale_files(&root) {
        println!("cargo:rerun-if-changed={}", file.display());
    }
}

fn locale_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return files;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(locale_files(&path));
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| matches!(ext, "yml" | "yaml" | "json" | "toml"))
        {
            files.push(path);
        }
    }
    files
}
