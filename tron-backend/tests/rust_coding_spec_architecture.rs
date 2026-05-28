use std::{fs, path::Path};

const BACKEND_SRC: &str = "src";
const FACADE_MAX_LINES: usize = 80;
const IMPLEMENTATION_MAX_LINES: usize = 600;
const FORBIDDEN_DOMAIN_TOKENS: &[&str] = &[
    "use axum",
    "axum::",
    "use rusqlite",
    "rusqlite::",
    "use reqwest",
    "reqwest::",
    "StatusCode",
    "ApiResponse",
    "adapters::",
    "application::",
    "shared::event_bus",
];

#[test]
fn source_tree_uses_modern_rust_module_layout_without_mod_rs() {
    let offenders = collect_files(Path::new(BACKEND_SRC))
        .into_iter()
        .filter(|path| path.file_name().is_some_and(|name| name == "mod.rs"))
        .map(display_path)
        .collect::<Vec<_>>();

    assert!(
        offenders.is_empty(),
        "rust-coding-spec forbids mod.rs module roots: {offenders:#?}"
    );
}

#[test]
fn domain_layer_does_not_import_infrastructure_or_application_boundaries() {
    let offenders = collect_files(Path::new(BACKEND_SRC).join("domain").as_path())
        .into_iter()
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .filter_map(|path| {
            let content = fs::read_to_string(&path).expect("source file should be readable");
            let forbidden = FORBIDDEN_DOMAIN_TOKENS
                .iter()
                .filter(|token| content.contains(**token))
                .copied()
                .collect::<Vec<_>>();
            (!forbidden.is_empty()).then(|| (display_path(path), forbidden))
        })
        .collect::<Vec<_>>();

    assert!(
        offenders.is_empty(),
        "domain layer must stay pure and not import infrastructure/application boundaries: {offenders:#?}"
    );
}

#[test]
fn bounded_context_roots_are_small_facades() {
    let roots = ["application", "domain"]
        .into_iter()
        .flat_map(|layer| {
            collect_files(Path::new(BACKEND_SRC).join(layer).as_path())
                .into_iter()
                .filter(move |path| {
                    path.extension().is_some_and(|extension| extension == "rs")
                        && path.parent().is_some_and(|parent| {
                            parent.ends_with(Path::new(BACKEND_SRC).join(layer))
                        })
                })
        })
        .filter_map(|path| {
            let content = fs::read_to_string(&path).expect("source file should be readable");
            let line_count = content.lines().count();
            (line_count > FACADE_MAX_LINES).then(|| (display_path(path), line_count))
        })
        .collect::<Vec<_>>();

    assert!(
        roots.is_empty(),
        "application/domain context root files should be small facade modules: {roots:#?}"
    );
}

#[test]
fn backend_source_files_do_not_become_large_implementation_buckets() {
    let offenders = collect_files(Path::new(BACKEND_SRC))
        .into_iter()
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .filter_map(|path| {
            let content = fs::read_to_string(&path).expect("source file should be readable");
            let line_count = content.lines().count();
            (line_count > IMPLEMENTATION_MAX_LINES).then(|| (display_path(path), line_count))
        })
        .collect::<Vec<_>>();

    assert!(
        offenders.is_empty(),
        "backend implementation files should be split by DDD or adapter responsibility before they become large buckets: {offenders:#?}"
    );
}

fn collect_files(root: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    collect_files_into(root, &mut files);
    files
}

fn collect_files_into(root: &Path, files: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(root).expect("source directory should be readable") {
        let path = entry
            .expect("source directory entry should be readable")
            .path();
        if path.is_dir() {
            collect_files_into(&path, files);
        } else {
            files.push(path);
        }
    }
}

fn display_path(path: impl AsRef<Path>) -> String {
    path.as_ref().display().to_string()
}
