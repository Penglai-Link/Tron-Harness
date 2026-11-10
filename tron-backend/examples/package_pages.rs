//! Build the importable static-pages example with Framework's archive writer.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/capability-packs/pages");
    let archive = eliteforge_agent_capability_runtime::build_capability_archive(&source, None)?;
    let target = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../output/capability-pages-example.zip");
    std::fs::create_dir_all(target.parent().ok_or("missing output directory")?)?;
    std::fs::write(&target, archive)?;
    println!("{}", target.display());
    Ok(())
}
