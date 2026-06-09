use std::{fs, path::Path, process::Command};

#[test]
fn make_help_lists_backend_targets_and_existing_frontend_targets() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tron-backend must live under the repository root");

    let output = Command::new("make")
        .arg("help")
        .current_dir(repo_root)
        .output()
        .expect("make help should execute from the repository root");

    assert!(
        output.status.success(),
        "make help should exit successfully, stderr:\n{}",
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    for backend_target in [
        "make be-install",
        "make be-check",
        "make be-test",
        "make be-run",
    ] {
        assert!(
            stdout.contains(backend_target),
            "make help must list `{backend_target}`, stdout:\n{stdout}",
        );
    }

    for frontend_target in ["make fe-install", "make fe-check", "make fe-dev"] {
        assert!(
            stdout.contains(frontend_target),
            "make help must preserve existing `{frontend_target}`, stdout:\n{stdout}",
        );
    }
}

#[test]
fn make_be_run_port_override_is_wired_into_backend_config() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tron-backend must live under the repository root");

    let makefile = fs::read_to_string(repo_root.join("Makefile"))
        .expect("Makefile should be readable from the repository root");
    assert!(
        makefile.contains("TRON_BACKEND_PORT=$(PORT)"),
        "make be-run must export TRON_BACKEND_PORT when PORT is provided"
    );
    assert!(
        makefile.contains("TRON_BACKEND_GRAPH_STORAGE_PATH=$(GRAPH_STORAGE_PATH)"),
        "make be-run must export TRON_BACKEND_GRAPH_STORAGE_PATH when GRAPH_STORAGE_PATH is provided"
    );

    let config_source = fs::read_to_string(repo_root.join("tron-backend/src/config/parsing.rs"))
        .expect("backend config parsing source should be readable");
    assert!(
        config_source.contains("ENV_BACKEND_PORT")
            && config_source.contains("env_port_override")
            && config_source.contains("parse::<u16>()"),
        "backend config parsing must consume TRON_BACKEND_PORT as a u16 override"
    );

    let path_source = fs::read_to_string(repo_root.join("tron-backend/src/config/paths.rs"))
        .expect("backend config path source should be readable");
    assert!(
        path_source.contains("TRON_BACKEND_GRAPH_STORAGE_PATH")
            && path_source.contains("env_graph_storage_path_override"),
        "backend config path handling must consume TRON_BACKEND_GRAPH_STORAGE_PATH"
    );
}
