//! Rebuild the crate whenever the centralized locale resources change.
//!
//! `rust_i18n::i18n!` reads `../../locales` at compile time. Cargo only reruns a
//! proc macro when its input files are tracked, so list every resource file
//! explicitly; otherwise edits to YAML would silently leave a stale compiled
//! table in incremental builds. A content hash is also emitted as a `--cfg`
//! flag: `rerun-if-changed` reruns this build script, but compiler caches
//! (sccache / incremental) key the crate on its compiler invocation. Changing
//! the flag whenever locale content changes makes the cached object invalid.
use std::{
    collections::hash_map::DefaultHasher,
    env, fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"))
        .join("../../locales");
    println!("cargo:rerun-if-changed={}", root.display());
    let files = locale_files(&root);
    for file in &files {
        println!("cargo:rerun-if-changed={}", file.display());
    }
    let mut hasher = DefaultHasher::new();
    for file in &files {
        file.strip_prefix(&root)
            .unwrap_or(file)
            .to_string_lossy()
            .hash(&mut hasher);
        match fs::read(file) {
            Ok(bytes) => bytes.hash(&mut hasher),
            Err(error) => error.kind().hash(&mut hasher),
        }
    }
    println!("cargo:rustc-cfg=tron_locales_{:016x}", hasher.finish());
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
    files.sort();
    files
}
