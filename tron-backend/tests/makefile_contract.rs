use std::{path::Path, process::Command};

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
