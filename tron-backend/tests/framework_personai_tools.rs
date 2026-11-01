//! Exercises the Framework PersonAI gateway through a local SDK-compatible server.
use async_trait::async_trait;
use eliteforge_agent_capability_personai::{
    AccountContext, PersonAiCapabilityPack, PersonAiConfig, PersonAiConnection,
    PersonAiContextHandle, sdk,
};
use eliteforge_agent_core::*;
use eliteforge_agent_test_support::TestBuilder;
use rsa::{RsaPrivateKey, pkcs8::EncodePublicKey};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

struct Server {
    url: String,
    requests: Arc<Mutex<Vec<String>>>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Server {
    async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = requests.clone();
        let task = tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                let recorded = recorded.clone();
                tokio::spawn(async move {
                    let mut request = Vec::new();
                    loop {
                        let mut buffer = [0_u8; 8192];
                        let n = socket.read(&mut buffer).await.unwrap();
                        if n == 0 {
                            return;
                        }
                        request.extend_from_slice(&buffer[..n]);
                        if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                            let headers = String::from_utf8_lossy(&request[..end]);
                            let length = headers
                                .lines()
                                .find_map(|line| {
                                    line.to_ascii_lowercase()
                                        .strip_prefix("content-length:")
                                        .and_then(|n| n.trim().parse::<usize>().ok())
                                })
                                .unwrap_or(0);
                            if request.len() >= end + 4 + length {
                                break;
                            }
                        }
                    }
                    let request = String::from_utf8(request).unwrap();
                    let path = request.lines().next().unwrap();
                    let data = if path.contains(" /api/v1/search ") {
                        json!({"query":"rust","count":1,"results":[{"title":"Rust","url":"https://rust-lang.org","snippet":"Rust language","source":"test"}],"providerCode":"test"})
                    } else if path.contains(" /api/v1/images/generations ") {
                        json!({"id":"image-task","media_type":0,"status":0,"progress":0})
                    } else if path.contains(" /api/v1/multimedia/tasks/image-task ") {
                        json!({"id":"image-task","media_type":0,"status":2,"progress":100,"result_url":"https://example.com/image.png"})
                    } else if path.contains(" /api/v1/aigc/detections/") {
                        json!({"id":"detection","detection_type":if path.contains("/image") {1} else {0},"model_code":"detector","status":0,"ai_confidence":0.9})
                    } else if path.contains("media_type=") {
                        let kind: i32 = path
                            .split("media_type=")
                            .nth(1)
                            .unwrap()
                            .split(['&', ' '])
                            .next()
                            .unwrap()
                            .parse()
                            .unwrap();
                        json!([{"model_id":format!("media-{kind}"),"model_code":format!("media-{kind}"),"media_type":kind}])
                    } else {
                        json!([])
                    };
                    recorded.lock().unwrap().push(request);
                    let body = json!({"code":0,"data":data}).to_string();
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    socket.write_all(response.as_bytes()).await.unwrap();
                });
            }
        });
        Self {
            url,
            requests,
            task,
        }
    }
}
async fn connection(server: &Server) -> (Arc<PersonAiConnection>, Arc<PersonAiContextHandle>) {
    let key = RsaPrivateKey::new(&mut rand08::thread_rng(), 2048)
        .unwrap()
        .to_public_key()
        .to_public_key_pem(rsa::pkcs8::LineEnding::LF)
        .unwrap();
    let context = Arc::new(PersonAiContextHandle::default());
    context.publish(AccountContext {
        account_scope: "alice".into(),
        revision: 1,
        authorization: sdk::base::AuthorizedClientContext::new()
            .with_token("token")
            .with_client_id("client")
            .with_public_key(key)
            .with_product_code("test-product")
            .with_product_version("1.0.0"),
    });
    let connection =
        PersonAiConnection::connect(PersonAiConfig::new(&server.url), context.clone(), None)
            .await
            .unwrap();
    (connection, context)
}
#[derive(Clone)]
struct Script {
    replies: Arc<Mutex<VecDeque<ChatResponse>>>,
    requests: Arc<Mutex<Vec<ChatRequest>>>,
}
#[async_trait]
impl ChatModel for Script {
    async fn generate(
        &self,
        _: &str,
        request: ChatRequest,
        _: &dyn ChatStream,
    ) -> Result<ChatResponse, ModelError> {
        self.requests.lock().unwrap().push(request);
        Ok(self
            .replies
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected model call"))
    }
}
fn call(name: &str, arguments: Value) -> ChatResponse {
    ChatResponse::tool_calls(vec![ToolCall::new(name, name, arguments)])
}
/// A `tool_execute` envelope addressing an inner tool discovered through
/// `tool_search`.
fn envelope(id: &str, tool: &str, arguments: Value) -> ChatResponse {
    ChatResponse::tool_calls(vec![ToolCall::new(
        id,
        "tool_execute",
        json!({"tool": tool, "arguments": {"items": [arguments]}}),
    )])
}
#[tokio::test]
async fn discovery_executes_all_remote_tools_with_structured_results() {
    let server = Server::start().await;
    let (connection, _) = connection(&server).await;
    let script = Script {
        requests: Arc::default(),
        replies: Arc::new(Mutex::new(VecDeque::from([
            call(
                "tool_search",
                json!({"items":[{"query":"personai","limit":10}]}),
            ),
            envelope("search", "web_search", json!({"query":"rust","count":1})),
            envelope(
                "generate",
                "personai_generate_image",
                json!({"prompt":"a cat"}),
            ),
            envelope(
                "query",
                "personai_query_image",
                json!({"task_id":"image-task"}),
            ),
            envelope("detect", "personai_detect_text", json!({"text":"hello"})),
            envelope(
                "detect-image",
                "personai_detect_image",
                json!({"image_url":"https://example.com/image.png"}),
            ),
            ChatResponse::text("done"),
        ]))),
    };
    let agent = TestBuilder::new()
        .capability_pack(PersonAiCapabilityPack::new(connection.clone()))
        .plugin(eliteforge_agent_test_support::TestStoragePlugin::default())
        .plugin(ChatModelPlugin::new(script.clone()))
        .build()
        .await
        .unwrap();
    assert_eq!(
        agent
            .run(RunRequest::new("remote-tools", "use PersonAI"))
            .await
            .unwrap()
            .output,
        "done"
    );
    let requests = script.requests.lock().unwrap().clone();
    // Cache-stability invariant: every request sees exactly the two
    // stable gateway tools, even after discovery and execution.
    for (index, request) in requests.iter().enumerate() {
        let names: Vec<_> = request.tools.iter().map(|t| t.call_name.as_str()).collect();
        assert_eq!(
            names,
            vec!["tool_execute", "tool_search"],
            "request {index} must keep the stable gateway catalog"
        );
    }
    // The search output carries the wrapped tools' schemas as plain text.
    assert!(
        requests[1]
            .messages
            .iter()
            .any(|m| m.role == Role::Tool && m.content.text().contains("web_search"))
    );
    let transcript = requests
        .last()
        .unwrap()
        .messages
        .iter()
        .map(|m| m.content.text())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(transcript.contains("Rust language"), "{transcript}");
    assert!(transcript.contains("image-task"), "{transcript}");
    assert!(
        transcript.contains("https://example.com/image.png"),
        "{transcript}"
    );
    assert!(transcript.contains("ai_confidence"), "{transcript}");
    drop(requests);
    let http = server.requests.lock().unwrap().clone();
    for endpoint in [
        "/search ",
        "/images/generations ",
        "/multimedia/tasks/image-task ",
        "/aigc/detections/text ",
        "/aigc/detections/image ",
    ] {
        let request = http
            .iter()
            .find(|r| r.lines().next().unwrap().contains(endpoint))
            .unwrap_or_else(|| panic!("missing {endpoint}"));
        assert!(
            request
                .to_ascii_lowercase()
                .contains("x-product-code: test-product")
        );
        assert!(
            request
                .to_ascii_lowercase()
                .contains("authorization: bearer token")
        );
    }
    drop(http);
    agent.close().await;
    connection.close().await.unwrap();
}

/// Explicitly opt in to real searches and text detection using the development profile.
/// Credentials are never printed and no application records are created.
#[tokio::test]
#[ignore = "requires TRON_PERSONAI_LIVE_DIRECTORY and a connected Person-AI profile"]
async fn live_personai_search_and_text_detection_through_the_agent_gateway() {
    use std::{path::PathBuf, time::Duration};
    use tron_backend::{
        adapters::outbound::crypto::Sm4GcmVariableCipher,
        application::model_provider::ModelProviderService, config::VariableCryptoConfig,
    };

    let directory = PathBuf::from(std::env::var("TRON_PERSONAI_LIVE_DIRECTORY").unwrap());
    let yaml: serde_yaml::Value =
        serde_yaml::from_str(&std::fs::read_to_string(directory.join("application.yaml")).unwrap())
            .unwrap();
    let cipher = Arc::new(
        Sm4GcmVariableCipher::from_config(VariableCryptoConfig {
            key_base64: yaml["variableCrypto"]["keyBase64"].as_str().unwrap().into(),
        })
        .unwrap(),
    );
    let providers =
        ModelProviderService::connect(directory.join("tron.sqlite3").to_str().unwrap(), cipher)
            .unwrap();
    let scope = providers.sdk_model_authorization_scope().unwrap();
    let (_, base_url, context) = providers.personai_tool_context(&scope).await.unwrap();
    let connection = PersonAiConnection::connect(PersonAiConfig::new(&base_url), context, None)
        .await
        .unwrap();
    let script = Script {
        requests: Arc::default(),
        replies: Arc::new(Mutex::new(VecDeque::from([
            envelope(
                "search",
                "web_search",
                json!({"query":"Rust programming language", "count":1}),
            ),
            envelope(
                "detect",
                "personai_detect_text",
                json!({"text":"A small garden can become a place of quiet discovery. Each morning the gardener checks the soil, waters the young plants, and watches the changing light. Over several weeks, green leaves appear and the first flowers open. The work requires patience, attention, and a willingness to learn from the weather and the seasons."}),
            ),
            envelope(
                "detect-image",
                "personai_detect_image",
                json!({"image_base64":"iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/a2kAAAAASUVORK5CYII="}),
            ),
            ChatResponse::text("done"),
        ]))),
    };
    let agent = TestBuilder::new()
        .capability_pack(PersonAiCapabilityPack::new(connection.clone()))
        .plugin(eliteforge_agent_test_support::TestStoragePlugin::default())
        .plugin(ChatModelPlugin::new(script.clone()))
        .build()
        .await
        .unwrap();
    let mut request = RunRequest::new("live-personai-search", "Search the web");
    request.model = Some(ModelRef::new("default", "default"));
    let result = tokio::time::timeout(Duration::from_secs(45), agent.run(request))
        .await
        .expect("Person-AI search timed out")
        .expect("agent run failed");
    assert_eq!(result.output, "done");
    let requests = script.requests.lock().unwrap();
    let messages = &requests.last().unwrap().messages;
    let tool_results: Vec<_> = messages.iter().filter(|m| m.role == Role::Tool).collect();
    assert_eq!(tool_results.len(), 3);
    for message in &tool_results[..2] {
        let output = message.content.text();
        let result: Value = serde_json::from_str(&output).expect("gateway returned invalid JSON");
        let item = &result["results"][0];
        assert_eq!(item["status"], "Completed");
        assert_eq!(item["isError"], false);
        assert!(item["result"].is_object());
    }
    let image_output = tool_results[2].content.text();
    let image_result: Value = serde_json::from_str(&image_output).unwrap();
    let image_item = &image_result["results"][0];
    let image_unavailable = image_item["isError"] == true;
    if image_unavailable {
        assert!(
            image_item.to_string().contains("model unavailable"),
            "{image_item}"
        );
    } else {
        assert_eq!(image_item["status"], "Completed");
        assert!(image_item["result"].is_object());
    }
    println!(
        "PERSONAI_LIVE_RECEIPT search=true text_detection=true image_detection_unavailable={image_unavailable}"
    );
    drop(requests);
    agent.close().await;
    connection.close().await.unwrap();
}

/// Creates one small image job in the development account, then checks it via
/// the same agent gateway. This incurs the provider's normal usage charges.
#[tokio::test]
#[ignore = "requires TRON_PERSONAI_LIVE_DIRECTORY and creates a paid image job"]
async fn live_personai_image_generation_and_query_through_the_agent_gateway() {
    use std::{path::PathBuf, time::Duration};
    use tron_backend::{
        adapters::outbound::crypto::Sm4GcmVariableCipher,
        application::model_provider::ModelProviderService, config::VariableCryptoConfig,
    };
    let directory = PathBuf::from(std::env::var("TRON_PERSONAI_LIVE_DIRECTORY").unwrap());
    let yaml: serde_yaml::Value =
        serde_yaml::from_str(&std::fs::read_to_string(directory.join("application.yaml")).unwrap())
            .unwrap();
    let cipher = Arc::new(
        Sm4GcmVariableCipher::from_config(VariableCryptoConfig {
            key_base64: yaml["variableCrypto"]["keyBase64"].as_str().unwrap().into(),
        })
        .unwrap(),
    );
    let providers =
        ModelProviderService::connect(directory.join("tron.sqlite3").to_str().unwrap(), cipher)
            .unwrap();
    let scope = providers.sdk_model_authorization_scope().unwrap();
    let (_, base_url, context) = providers.personai_tool_context(&scope).await.unwrap();
    let connection = PersonAiConnection::connect(PersonAiConfig::new(&base_url), context, None)
        .await
        .unwrap();

    async fn run_tool(connection: Arc<PersonAiConnection>, tool: &str, arguments: Value) -> Value {
        let script = Script {
            requests: Arc::default(),
            replies: Arc::new(Mutex::new(VecDeque::from([
                envelope("tool", tool, arguments),
                ChatResponse::text("done"),
            ]))),
        };
        let agent = TestBuilder::new()
            .capability_pack(PersonAiCapabilityPack::new(connection))
            .plugin(eliteforge_agent_test_support::TestStoragePlugin::default())
            .plugin(ChatModelPlugin::new(script.clone()))
            .build()
            .await
            .unwrap();
        let mut request = RunRequest::new(format!("live-{tool}"), "Run the requested tool");
        request.model = Some(ModelRef::new("default", "default"));
        let run = tokio::time::timeout(Duration::from_secs(60), agent.run(request))
            .await
            .expect("live tool timed out")
            .expect("agent run failed");
        assert_eq!(run.output, "done");
        let requests = script.requests.lock().unwrap();
        let message = requests
            .last()
            .unwrap()
            .messages
            .iter()
            .find(|m| m.role == Role::Tool)
            .unwrap();
        let parsed: Value = serde_json::from_str(&message.content.text()).unwrap();
        let item = &parsed["results"][0];
        assert_eq!(item["status"], "Completed", "tool {tool}: {item}");
        assert_eq!(item["isError"], false, "tool {tool}: {item}");
        let result = item["result"].clone();
        drop(requests);
        agent.close().await;
        result
    }
    let generated = run_tool(
        connection.clone(),
        "personai_generate_image",
        json!({
            "prompt":"A simple red circle on a white background", "count":1
        }),
    )
    .await;
    let task_id = generated["task_id"]
        .as_str()
        .expect("generation did not return a task ID");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    loop {
        let queried = run_tool(
            connection.clone(),
            "personai_query_image",
            json!({"task_id":task_id}),
        )
        .await;
        assert_eq!(queried["task_id"], task_id);
        let job = &queried["job"];
        match job["status"].as_str() {
            Some("Completed") => {
                assert!(
                    job["images"]
                        .as_array()
                        .is_some_and(|images| !images.is_empty()),
                    "completed image job has no images"
                );
                break;
            }
            Some("Failed" | "Cancelled") => {
                panic!("image job ended unsuccessfully: {}", job["error"])
            }
            Some("Queued" | "Running") => {}
            other => panic!("unexpected image status: {other:?}"),
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "image job did not finish in 120 seconds"
        );
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
    println!("PERSONAI_LIVE_IMAGE_RECEIPT submitted=true query=true completed=true images=true");
    connection.close().await.unwrap();
}

/// Use an actual Person-AI chat model as well as its real search service.
#[tokio::test]
#[ignore = "requires a connected Person-AI profile and incurs model/search usage"]
async fn live_personai_model_discovers_and_executes_search() {
    use std::{path::PathBuf, time::Duration};
    use tron_backend::{
        adapters::outbound::crypto::Sm4GcmVariableCipher,
        application::model_provider::ModelProviderService, config::VariableCryptoConfig,
    };
    let directory = PathBuf::from(std::env::var("TRON_PERSONAI_LIVE_DIRECTORY").unwrap());
    let model = std::env::var("TRON_PERSONAI_LIVE_MODEL").unwrap();
    let yaml: serde_yaml::Value =
        serde_yaml::from_str(&std::fs::read_to_string(directory.join("application.yaml")).unwrap())
            .unwrap();
    let cipher = Arc::new(
        Sm4GcmVariableCipher::from_config(VariableCryptoConfig {
            key_base64: yaml["variableCrypto"]["keyBase64"].as_str().unwrap().into(),
        })
        .unwrap(),
    );
    let providers =
        ModelProviderService::connect(directory.join("tron.sqlite3").to_str().unwrap(), cipher)
            .unwrap();
    let scope = providers.sdk_model_authorization_scope().unwrap();
    let (_, base_url, context) = providers.personai_tool_context(&scope).await.unwrap();
    let connection = PersonAiConnection::connect(PersonAiConfig::new(&base_url), context, None)
        .await
        .unwrap();
    let agent = TestBuilder::new()
        .capability_pack(PersonAiCapabilityPack::new(connection.clone()))
        .plugin(eliteforge_agent_test_support::TestStoragePlugin::default())
        .build()
        .await
        .unwrap();
    let session = "live-model-web-search";
    let mut request = RunRequest::new(
        session,
        "Use the available tools to search the web for the official Rust programming language website, then report its URL. You must actually run a search tool.",
    );
    request.model = Some(ModelRef::new("personai", model));
    request.limits.max_steps = 8;
    request.limits.max_tool_calls = 6;
    let result = tokio::time::timeout(Duration::from_secs(120), agent.run(request))
        .await
        .expect("live model conversation timed out")
        .expect("live model conversation failed");
    assert_eq!(result.status, RunStatus::Completed);
    assert!(!result.output.trim().is_empty());
    let conversation = agent
        .conversation(&ConversationKey::new(session))
        .await
        .unwrap();
    let search_call = conversation
        .messages
        .iter()
        .flat_map(|m| &m.tool_calls)
        .find(|call| {
            call.call_name == "web_search"
                || (call.call_name == "tool_execute" && call.arguments["tool"] == "web_search")
        })
        .expect("the real model did not call web_search");
    let search_result = conversation
        .messages
        .iter()
        .find(|message| message.tool_call_id.as_deref() == Some(search_call.id.as_str()))
        .expect("the search tool result was not persisted");
    assert!(
        search_result
            .tool_result
            .as_ref()
            .is_some_and(|result| !result.is_error)
    );
    let text = search_result.content.text();
    let structured: Value = serde_json::from_str(&text).expect("search result is not JSON");
    if search_call.call_name == "tool_execute" {
        assert_eq!(structured["results"][0]["isError"], false);
    }
    println!(
        "PERSONAI_LIVE_MODEL_RECEIPT completed=true searched=true tool_calls={}",
        result.tool_calls
    );
    agent.close().await;
    connection.close().await.unwrap();
}
