use std::path::Path;
#[path = "support/model_architecture.rs"]
mod production_sources;
#[test]
fn production_model_network_and_retry_execution_belongs_to_sdk() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let sources = production_sources::production_sources(&root)
        .expect("production Rust must be parsed and classified without excluding unknown source");
    let dispatch_sources = production_sources::production_dispatch_sources(&root)
        .expect("model port forwarding must be classified by its complete production AST contract");
    let source = |relative: &str| {
        sources
            .iter()
            .find(|(path, _)| path == Path::new(relative))
            .map(|(_, source)| source.as_str())
            .expect("required production source must remain scanned")
    };
    for (path, production) in &sources {
        let relative = path.to_str().unwrap();
        if production.contains(".post(") {
            assert!(
                relative.starts_with("adapters/inbound/")
                    || relative == "application/rag/mineru.rs",
                "unexpected outbound POST in {relative}"
            );
        }
        for retired in [
            ".create_stream(",
            "EmbeddingsClient",
            "ResponsesClient",
            "String::from_utf8_lossy(&chunk)",
            "parse_sse_payloads",
        ] {
            assert!(
                !production.contains(retired),
                "retired model transport {retired} in {relative}"
            );
        }
        if production.contains("HttpProtocolClient::new") {
            assert_eq!(relative, "adapters/outbound/model/http_model_client.rs");
        }
        let dispatch = &dispatch_sources
            .iter()
            .find(|(source_path, _)| source_path == path)
            .expect("dispatch projection must preserve every production source")
            .1;
        if dispatch.contains(".generate_once(") {
            assert!(
                matches!(
                    relative,
                    "application/agent/runtime/framework_model.rs"
                        | "application/agent/runtime_compaction.rs"
                        | "shared/id/snowflake.rs"
                ),
                "uncontrolled single-attempt model caller: {relative}"
            );
        }
        if dispatch.contains(".generate_stream(") {
            assert!(
                matches!(
                    relative,
                    "application/agent/runtime/framework_model.rs"
                        | "adapters/outbound/model/http_model_client.rs"
                ),
                "uncontrolled model stream caller: {relative}"
            );
        }
    }
    for retired in [
        "adapters/outbound/model/http_model_stream_accumulator.rs",
        "application/model_provider/implementation/requests.rs",
        "adapters/outbound/model/protocol_adapters/chat_completions.rs",
        "adapters/outbound/model/protocol_adapters/responses.rs",
    ] {
        assert!(
            !root.join(retired).exists(),
            "retired production model control remains"
        );
    }
    let probe = source("application/model/service/verification_flow.rs");
    assert!(
        probe.contains("ChatModelJob")
            && probe
                .split_whitespace()
                .collect::<String>()
                .contains("model_jobs.get()")
    );
    assert!(!probe.contains("for_in"));
    let client = source("adapters/outbound/model/http_model_client.rs");
    assert!(
        client.contains("LazyModelJobClient")
            && client
                .split_whitespace()
                .collect::<String>()
                .contains("self.jobs.close_observed().await")
            && client
                .split_whitespace()
                .collect::<String>()
                .contains("workflow:Some")
    );
    let llm = source("adapters/outbound/model/http_llm_runtime_client.rs");
    assert!(llm.contains(".embed(") && llm.contains(".rerank("));
    let goal = source("application/chat/goal_loop.rs");
    assert!(!goal.contains("forattemptin"));
    assert!(goal.contains("ModelJobRepair"));
    let app = source("app.rs");
    assert!(
        app.contains("with_jobs(model_providers.model_jobs())")
            && app.contains("models.with_model_jobs(model_providers.model_jobs())")
    );
}
