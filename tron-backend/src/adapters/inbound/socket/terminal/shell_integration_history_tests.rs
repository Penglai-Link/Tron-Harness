use std::{fs, os::unix::fs::PermissionsExt, process::Command};

use super::*;

#[test]
fn local_bash_history_stays_owned_by_the_users_shell() {
    let home = tempfile::tempdir().expect("home directory should be created");
    fs::write(home.path().join(".bash_history"), "pre-existing-command\n")
        .expect("history should be written");
    fs::write(
        home.path().join(".bashrc"),
        "HISTFILE=$HOME/.bash_history\nHISTSIZE=100\nHISTFILESIZE=100\nhistory -r\n",
    )
    .expect("bashrc should be written");
    let directory = write_shell_files(ShellKind::Bash).expect("rc files should be written");

    let output = Command::new("/bin/bash")
        .args([
            "--noprofile",
            "--init-file",
            directory
                .path()
                .join("bashrc")
                .to_str()
                .expect("path should be UTF-8"),
            "-i",
            "-c",
            "history -s user-command; history",
        ])
        .env("HOME", home.path())
        .env("TERM", "dumb")
        .output()
        .expect("bash should run");
    let history = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "bash history command should succeed"
    );
    assert!(history.contains("pre-existing-command"));
    assert!(history.contains("user-command"));
    assert!(!history.contains("__TRON_INTEGRATION_BASE64"));
    assert!(!history.contains("eval \"$(printf %s"));
}

#[test]
fn container_helper_bootstraps_before_the_interactive_shell() {
    let directory = tempfile::tempdir().expect("temp directory should be created");
    let fake_runtime = directory.path().join("fake-runtime");
    fs::write(
        &fake_runtime,
        "#!/bin/sh\nfor argument do printf 'ARG=%s\\n' \"$argument\"; done\n",
    )
    .expect("fake runtime should be written");
    let mut permissions = fs::metadata(&fake_runtime)
        .expect("fake runtime metadata should exist")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&fake_runtime, permissions).expect("fake runtime should be executable");

    for (name, script) in [
        ("integration.sh", INTEGRATION_SCRIPT),
        ("ssh-helper.sh", SSH_HELPER_SCRIPT),
        ("ssh-bootstrap.sh", SSH_BOOTSTRAP_SCRIPT),
        ("container-helper.sh", CONTAINER_HELPER_SCRIPT),
    ] {
        fs::write(directory.path().join(name), script).expect("helper script should be written");
    }

    let output = Command::new("/bin/bash")
        .arg(directory.path().join("container-helper.sh"))
        .arg(&fake_runtime)
        .args(["exec", "-it", "api", "bash"])
        .env(
            "__TRON_SSH_INTEGRATION",
            directory.path().join("integration.sh"),
        )
        .env("__TRON_SSH_HELPER", directory.path().join("ssh-helper.sh"))
        .env(
            "__TRON_SSH_BOOTSTRAP",
            directory.path().join("ssh-bootstrap.sh"),
        )
        .env(
            "__TRON_CONTAINER_HELPER",
            directory.path().join("container-helper.sh"),
        )
        .output()
        .expect("container helper should run");
    let stdout = String::from_utf8(output.stdout).expect("output should be UTF-8");

    assert!(output.status.success());
    assert!(stdout.contains("ARG=exec\nARG=-it\n"));
    assert!(stdout.contains("ARG=__TRON_INTEGRATION_BASE64="));
    assert!(stdout.contains("__TRON_CONTAINER_HELPER_BASE64="));
    assert!(stdout.contains("ARG=api\nARG=/bin/sh\nARG=-c\n"));
    assert!(!stdout.contains("ARG=bash\n"));

    let bypassed = Command::new("/bin/bash")
        .arg(directory.path().join("container-helper.sh"))
        .arg(&fake_runtime)
        .args(["exec", "-i", "--", "api", "bash"])
        .output()
        .expect("unsupported syntax should pass through");
    let bypassed_stdout = String::from_utf8(bypassed.stdout).expect("output should be UTF-8");
    assert!(bypassed.status.success());
    assert!(bypassed_stdout.contains("ARG=exec\nARG=-i\nARG=--\nARG=api\nARG=bash\n"));
    assert!(!bypassed_stdout.contains("__TRON_INTEGRATION_BASE64"));
}
