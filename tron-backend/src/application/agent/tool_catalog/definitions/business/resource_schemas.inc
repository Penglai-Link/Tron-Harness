fn resource_kind_schema() -> serde_json::Value {
    json!({ "type": "string", "enum": ["File", "Image", "Pdf"] })
}

fn resource_file_kind_schema() -> serde_json::Value {
    json!({ "type": "string", "enum": ["File", "Image", "Pdf"] })
}

fn resource_workspace_code_schema() -> serde_json::Value {
    json!({ "type": "string", "minLength": 1 })
}

fn resource_code_schema() -> serde_json::Value {
    json!({ "type": "string", "minLength": 1, "maxLength": 64 })
}
