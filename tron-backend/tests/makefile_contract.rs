use std::{fs, path::Path, process::Command};

fn frontend_api_data_source_assignments(repo_root: &Path) -> Vec<String> {
    let service_config =
        fs::read_to_string(repo_root.join("tron-frontend/src/shared/api/serviceConfig.ts"))
            .expect("frontend service config should be readable");
    let mut assignments = vec![
        "VITE_TRON_DATA_SOURCE=api".to_string(),
        "VITE_TRON_AUTH_DATA_SOURCE=api".to_string(),
    ];
    let mut in_domain_env_keys = false;

    for line in service_config.lines() {
        if line.contains("const domainEnvKeys") {
            in_domain_env_keys = true;
            continue;
        }
        if in_domain_env_keys && line.contains("};") {
            break;
        }
        if !in_domain_env_keys {
            continue;
        }
        if let Some((_, value)) = line.split_once(": \"") {
            if let Some((env_key, _)) = value.split_once('"') {
                assignments.push(format!("VITE_TRON_{env_key}_DATA_SOURCE=api"));
            }
        }
    }

    assert!(
        assignments.len() > 2,
        "frontend service config must expose domain data source keys",
    );
    assignments
}

#[test]
fn make_help_lists_common_integrated_backend_and_frontend_targets() {
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

    for common_target in ["make install", "make clean"] {
        assert!(
            stdout.contains(common_target),
            "make help must list common `{common_target}`, stdout:\n{stdout}",
        );
    }

    for backend_target in [
        "make be-install",
        "make be-check",
        "make be-test",
        "make be-dev",
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

    for web_target in ["make web-build", "make web-dev", "make web-clean"] {
        assert!(
            stdout.contains(web_target),
            "make help must list integrated `{web_target}`, stdout:\n{stdout}",
        );
    }
}

#[test]
fn integrated_frontend_build_defaults_integrated_runtime_to_api_mode() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tron-backend must live under the repository root");
    let makefile = fs::read_to_string(repo_root.join("Makefile"))
        .expect("Makefile should be readable from the repository root");

    assert!(
        makefile.contains("TRON_FRONTEND_OUT_DIR="),
        "web-build must direct Vite output into backend frontend resources",
    );
    assert!(
        makefile.contains("VITE_TRON_API_BASE_URL="),
        "web-build must keep API calls same-origin for integrated runtime",
    );
    assert!(
        makefile
            .contains("APP_FRONTEND_ENV = VITE_TRON_API_BASE_URL= $(FRONTEND_API_DATA_SOURCE_ENV)"),
        "web-build must reuse the shared frontend API data source env",
    );
    for required in frontend_api_data_source_assignments(repo_root) {
        assert!(
            makefile.contains(&required),
            "integrated app build must default integrated runtime to API mode via `{required}`",
        );
    }
}

#[test]
fn dev_api_frontend_start_forces_every_frontend_module_to_api_mode() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tron-backend must live under the repository root");
    let makefile = fs::read_to_string(repo_root.join("Makefile"))
        .expect("Makefile should be readable from the repository root");

    assert!(
        makefile.contains("FE_API_ENV = VITE_TRON_API_BASE_URL= $(FRONTEND_API_DATA_SOURCE_ENV)"),
        "fe-dev-api must reuse the shared frontend API data source env",
    );
    for required in frontend_api_data_source_assignments(repo_root) {
        assert!(
            makefile.contains(&required),
            "dev-api must force frontend module API mode via `{required}`",
        );
    }
}

#[test]
fn make_be_dev_port_override_is_wired_into_backend_config() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tron-backend must live under the repository root");

    let makefile = fs::read_to_string(repo_root.join("Makefile"))
        .expect("Makefile should be readable from the repository root");
    assert!(
        makefile.contains("TRON_BACKEND_PORT=$(PORT)"),
        "make be-dev must export TRON_BACKEND_PORT when PORT is provided"
    );
    assert!(
        makefile.contains("TRON_BACKEND_GRAPH_STORAGE_PATH=$(GRAPH_STORAGE_PATH)"),
        "make be-dev must export TRON_BACKEND_GRAPH_STORAGE_PATH when GRAPH_STORAGE_PATH is provided"
    );
    assert!(
        makefile.contains("--bin tron-backend"),
        "make be-dev must explicitly select the server binary",
    );

    let config_source = fs::read_to_string(repo_root.join("tron-backend/src/config/parsing.rs"))
        .expect("backend config parsing source should be readable");
    assert!(
        config_source.contains("TRON_BACKEND_PORT")
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

#[test]
fn make_be_clean_removes_backend_tmp_scratch_files() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tron-backend must live under the repository root");

    let makefile = fs::read_to_string(repo_root.join("Makefile"))
        .expect("Makefile should be readable from the repository root");
    assert!(
        makefile.contains("rm -rf /tmp/tron*"),
        "make be-clean must remove /tmp/tron* scratch files"
    );
}

#[test]
fn desktop_targets_use_current_electron_directory() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tron-backend must live under the repository root");

    let makefile = fs::read_to_string(repo_root.join("Makefile"))
        .expect("Makefile should be readable from the repository root");

    assert!(
        makefile.contains("ELECTRON_DIR := electron"),
        "desktop targets must use the repository's electron directory",
    );
    assert!(
        !makefile.contains("ELECTRON_DIR := src-electron"),
        "desktop targets must not recreate the old src-electron directory",
    );
}

#[test]
fn desktop_build_copies_only_current_platform_sidecar() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tron-backend must live under the repository root");

    let makefile = fs::read_to_string(repo_root.join("Makefile"))
        .expect("Makefile should be readable from the repository root");

    for required in [
        "HOST_EXE_SUFFIX := $(if $(filter Windows_NT,$(OS)),.exe,)",
        "BACKEND_RELEASE_BINARY := tron-backend$(HOST_EXE_SUFFIX)",
        "BACKEND_RELEASE_PATH := $(BACKEND_DIR)/target/release/$(BACKEND_RELEASE_BINARY)",
        "DESKTOP_SIDECAR_PATH := $(ELECTRON_DIR)/resources/binaries/$(BACKEND_RELEASE_BINARY)",
        "cp -- \"$(BACKEND_DIR)/application.yaml\" \"$(ELECTRON_DIR)/resources/application.yaml\"",
        "ELECTRON_INSTALL_ENV = $(if $(strip $(ELECTRON_MIRROR)),ELECTRON_MIRROR=$(ELECTRON_MIRROR))",
        "test -f $(BACKEND_RELEASE_PATH)",
        "cp $(BACKEND_RELEASE_PATH) $(DESKTOP_SIDECAR_PATH)",
        "cd $(ELECTRON_DIR) && $(ELECTRON_INSTALL_ENV) $(ELECTRON_NPM) install",
        "cd $(ELECTRON_DIR) && $(ELECTRON_INSTALL_ENV) $(ELECTRON_NPM) run -s ensure-electron",
        "cd $(ELECTRON_DIR) && $(ELECTRON_NPM) run -s package",
    ] {
        assert!(
            makefile.contains(required),
            "desktop-build must use platform-specific sidecar handling via `{required}`",
        );
    }

    let config_source = fs::read_to_string(repo_root.join("tron-backend/src/config/file.rs"))
        .expect("backend config file source should be readable");
    assert!(
        config_source.contains("TRON_BACKEND_APPLICATION_CONFIG_PATH"),
        "packaged backend must accept an explicit application config path",
    );

    assert!(
        !makefile.contains(
            "tron-backend.exe $(ELECTRON_DIR)/resources/binaries/tron-backend.exe || true"
        ),
        "desktop-build must not unconditionally copy the Windows sidecar on non-Windows hosts",
    );
}
