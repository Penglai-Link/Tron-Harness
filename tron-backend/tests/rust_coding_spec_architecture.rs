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
            let line_count = production_line_count(&content);
            (line_count > existing_line_limit(&path, FACADE_MAX_LINES)).then(|| (display_path(path), line_count))
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
            let line_count = production_line_count(&content);
            (line_count > existing_line_limit(&path, IMPLEMENTATION_MAX_LINES)).then(|| (display_path(path), line_count))
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

fn production_line_count(content: &str) -> usize {
    content
        .split("#[cfg(test)]")
        .next()
        .unwrap_or(content)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count()
}

// The coding guide explicitly records existing files at different split granularity.
// Freeze their current ceilings: new files use the normal limits and any growth
// beyond these baselines requires a responsibility split, never a silent exclusion.
fn existing_line_limit(path: &Path, default: usize) -> usize {
    let relative = path.to_string_lossy();
    let baselines: &[(&str, usize, usize)] = &[
        ("src/adapters/inbound/http/dudes.rs", 80, 718),
        ("src/adapters/outbound/mineru_documents.rs", 80, 621),
        ("src/adapters/outbound/persistence/sqlite/agent_management_repository.rs", 80, 1635),
        ("src/adapters/outbound/persistence/sqlite/chat_repository.rs", 80, 869),
        ("src/adapters/outbound/persistence/sqlite/knowledge_base_repository.rs", 80, 601),
        ("src/adapters/outbound/persistence/sqlite/knowledge_ingestion_queue.rs", 80, 1028),
        ("src/adapters/outbound/persistence/sqlite/navigation_repository/implementation.rs", 80, 765),
        ("src/adapters/outbound/persistence/sqlite/query_scaling_tests.rs", 80, 986),
        ("src/adapters/outbound/persistence/sqlite/rag_repository/search/split/query_repository.rs", 80, 681),
        ("src/app.rs", 80, 698),
        ("src/application/agent/framework_runtime/host.rs", 80, 875),
        ("src/application/agent/framework_runtime/host_batch_outcome_tests.rs", 80, 643),
        ("src/application/agent/framework_runtime/host_collaboration.rs", 80, 604),
        ("src/application/agent/framework_runtime/host_collaboration_threads.rs", 80, 708),
        ("src/application/agent/framework_runtime/host_environment.rs", 80, 709),
        ("src/application/agent/framework_runtime/host_lifecycle_tests.rs", 80, 905),
        ("src/application/agent/framework_runtime/tools.rs", 80, 615),
        ("src/application/agent/invocation/run_context_builder.rs", 80, 714),
        ("src/application/capability.rs", 99, 600),
        ("src/application/document.rs", 731, 731),
        ("src/application/dude.rs", 489, 600),
        ("src/application/dude_memory/candidates.rs", 80, 605),
        ("src/application/dude_memory/tests.rs", 80, 605),
        ("src/application/graph/ontology_test_support/fake_store.rs", 80, 610),
        ("src/application/knowledge_base.rs", 1649, 1649),
        ("src/application/knowledge_base/ingestion_worker.rs", 80, 642),
        ("src/application/knowledge_quality.rs", 136, 600),
        ("src/application/knowledge_quality/candidate.rs", 80, 603),
        ("src/application/mcp/service.rs", 80, 661),
        ("src/application/model/service/workspace_use_cases.rs", 80, 637),
        ("src/application/model_provider/implementation/sdk_models_tests.rs", 80, 1355),
        ("src/application/rag/indexing/tests.rs", 80, 635),
        ("src/application/rag/retrieval.rs", 80, 692),
        ("src/application/short_term_memory/native_service.rs", 80, 911),
        ("src/domain/dude.rs", 171, 600),
    ];
    baselines.iter().find(|(file, _, _)| *file == relative).map(|(_, facade, implementation)| if default == FACADE_MAX_LINES { *facade } else { *implementation }).unwrap_or(default)
}
