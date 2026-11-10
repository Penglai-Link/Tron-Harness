use std::{env, fs, path::PathBuf};

fn main() {
    let lock_path = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("Cargo.lock");
    println!("cargo:rerun-if-changed={}", lock_path.display());
    let lock = fs::read_to_string(&lock_path).expect("read Cargo.lock for Framework version");
    let versions: Vec<_> = lock
        .split("[[package]]")
        .filter_map(|package| {
            let mut name = None;
            let mut version = None;
            for line in package.lines() {
                if let Some(value) = line.strip_prefix("name = \"").and_then(|v| v.strip_suffix('"')) {
                    name = Some(value);
                } else if let Some(value) = line.strip_prefix("version = \"").and_then(|v| v.strip_suffix('"')) {
                    version = Some(value);
                }
            }
            (name == Some("eliteforge-agent-core")).then_some(version).flatten()
        })
        .collect();
    let [version] = versions.as_slice() else {
        panic!("expected exactly one eliteforge-agent-core version in Cargo.lock");
    };
    println!("cargo:rustc-env=TRON_FRAMEWORK_VERSION={version}");
}
