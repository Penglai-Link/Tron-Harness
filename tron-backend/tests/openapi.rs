use tron_backend::openapi::document;

#[test]
fn documents_system_endpoints() {
    let document = document();
    let yaml = document
        .to_yaml()
        .expect("OpenAPI document should serialize to YAML");

    assert!(document.paths.paths.contains_key("/api/v1/health"));
    assert!(document.paths.paths.contains_key("/api/v1/readiness"));
    assert!(yaml.contains("openapi:"));
}
