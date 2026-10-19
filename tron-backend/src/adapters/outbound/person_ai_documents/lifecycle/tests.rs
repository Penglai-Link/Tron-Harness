use super::*;
use crab_serverapi::{AuthorizedClientContext, base::AuthorizedClientApi};
use std::sync::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct Checkpoint(Mutex<IngestionTask>);
impl Checkpoint {
    fn new(remote: Option<&str>) -> Arc<dyn ParseCheckpoint> {
        let mut task = IngestionTask::queued(
            "local-task".into(),
            "kb".into(),
            "doc".into(),
            "user".into(),
            "PLATFORM".into(),
            1,
            1,
            "hash".into(),
            json!({}),
        );
        task.remote_file_id = Some("original-request".into());
        task.remote_job_id = remote.map(String::from);
        Arc::new(Self(Mutex::new(task)))
    }
}
impl ParseCheckpoint for Checkpoint {
    fn task(&self) -> Result<IngestionTask, KnowledgeBaseError> {
        Ok(self.0.lock().unwrap().clone())
    }
    fn save(
        &self,
        file: Option<String>,
        job: Option<String>,
        reconciling: bool,
    ) -> Result<(), KnowledgeBaseError> {
        let mut task = self.0.lock().unwrap();
        task.remote_file_id = file;
        task.remote_job_id = job;
        task.reconciling = reconciling;
        Ok(())
    }
}
fn context() -> AuthorizedClientContext {
    AuthorizedClientContext::new()
        .with_token("fixture-token")
        .with_product_code("Tron")
        .with_product_version("test")
}
fn task(id: &str, status: &str, billing: &str) -> Value {
    json!({"id":id,"user_id":"user","filename":"a.pdf","parse_mode":"text","status":status,"billing_status":billing,"successful_pages":0,"page_coefficient":1,"points":0,"error_message":null,"result":null})
}
fn response(id: &str, status: &str, billing: &str) -> Value {
    json!({"code":0,"data":task(id,status,billing)})
}
async fn mock(
    responses: Vec<Option<Value>>,
    checkpoint: Option<Arc<dyn ParseCheckpoint>>,
) -> (DocumentParseClient, tokio::task::JoinHandle<Vec<String>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let mut requests = Vec::new();
        for response in responses {
            let (mut stream, _) = tokio::time::timeout(Duration::from_secs(5), listener.accept())
                .await
                .unwrap()
                .unwrap();
            let mut bytes = Vec::new();
            loop {
                let mut buffer = [0; 4096];
                let n = stream.read(&mut buffer).await.unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&buffer[..n]);
                if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..end]).to_lowercase();
                    let size: usize = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length: "))
                        .unwrap_or("0")
                        .parse()
                        .unwrap();
                    if bytes.len() >= end + 4 + size {
                        break;
                    }
                }
            }
            let request = String::from_utf8(bytes).unwrap();
            if request.starts_with("POST ") {
                if let Some(checkpoint) = &checkpoint {
                    let saved = checkpoint.task().unwrap();
                    assert!(saved.reconciling);
                    assert!(saved.remote_job_id.is_none());
                    assert!(
                        request.contains(saved.remote_file_id.as_deref().unwrap()),
                        "persist request ID before sending"
                    );
                }
            }
            requests.push(request);
            if let Some(response) = response {
                let body = response.to_string();
                let headers = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                stream.write_all(headers.as_bytes()).await.unwrap();
                stream.write_all(body.as_bytes()).await.unwrap();
            }
        }
        requests
    });
    let http = reqwest::Client::builder().no_proxy().build().unwrap();
    (
        DocumentParseClient::with_api(AuthorizedClientApi::with_client(
            format!("http://{address}"),
            http,
        )),
        handle,
    )
}

#[tokio::test]
async fn deleted_platform_task_is_replaced_with_a_persisted_new_request_id() {
    let checkpoint = Checkpoint::new(Some("deleted-task"));
    let (client, server) = mock(
        vec![
            Some(json!({"code":404,"message":"任务不存在"})),
            Some(response("replacement", "queued", "pending")),
        ],
        Some(checkpoint.clone()),
    )
    .await;
    let mut submission = ParseSubmission::new(
        "a.pdf",
        b"original-pdf",
        ParseMode::Ocr,
        Some(&checkpoint.task().unwrap()),
    );
    let result = submission
        .load(&client, &context(), Some(&checkpoint))
        .await
        .unwrap();
    assert_eq!(result.id, "replacement");
    let saved = checkpoint.task().unwrap();
    assert_ne!(saved.remote_file_id.as_deref(), Some("original-request"));
    assert_eq!(saved.remote_job_id.as_deref(), Some("replacement"));
    assert!(!saved.reconciling);
    let requests = server.await.unwrap();
    assert!(requests[0].contains("id=deleted-task"));
    assert!(requests[1].contains("original-pdf"));
    assert!(requests[1].contains("name=\"mode\"\r\n\r\nocr"));
    assert!(requests[1].contains("name=\"auto_cleanup\"\r\n\r\nfalse"));
    assert!(requests[1].to_lowercase().contains("x-product-code: tron"));
}
#[tokio::test]
async fn deleted_request_without_a_saved_task_id_can_be_resubmitted_but_parameter_conflicts_cannot()
{
    for message in [
        "原任务已删除或正在删除，请使用新的 request_id",
        "原任务已清理，请使用新的 request_id",
        "相同幂等键不能用于不同文件或解析参数",
    ] {
        let retired = message != "相同幂等键不能用于不同文件或解析参数";
        let checkpoint = Checkpoint::new(None);
        let mut responses = vec![Some(json!({"code":409,"message":message}))];
        if retired {
            responses.push(Some(response("replacement", "queued", "pending")));
        }
        let (client, server) = mock(responses, Some(checkpoint.clone())).await;
        let mut submission = ParseSubmission::new(
            "a.pdf",
            b"pdf",
            ParseMode::Text,
            Some(&checkpoint.task().unwrap()),
        );
        let result = submission
            .load(&client, &context(), Some(&checkpoint))
            .await;
        assert_eq!(result.is_ok(), retired);
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), if retired { 2 } else { 1 });
        assert!(requests[0].contains("original-request"));
        if retired {
            assert!(!requests[1].contains("original-request"));
            assert_eq!(
                checkpoint.task().unwrap().remote_job_id.as_deref(),
                Some("replacement")
            );
        } else {
            assert_eq!(
                checkpoint.task().unwrap().remote_file_id.as_deref(),
                Some("original-request")
            );
        }
    }
}
#[tokio::test]
async fn lost_create_response_replays_the_same_saved_request_instead_of_creating_another_job() {
    let checkpoint = Checkpoint::new(None);
    let (client, server) = mock(vec![None], Some(checkpoint.clone())).await;
    let mut submission = ParseSubmission::new(
        "a.pdf",
        b"pdf",
        ParseMode::Text,
        Some(&checkpoint.task().unwrap()),
    );
    assert!(
        submission
            .load(&client, &context(), Some(&checkpoint))
            .await
            .is_err()
    );
    let first = server.await.unwrap();
    let saved = checkpoint.task().unwrap();
    assert!(saved.reconciling);
    assert!(saved.remote_job_id.is_none());
    let (client, server) = mock(
        vec![Some(response("original-created-job", "queued", "pending"))],
        Some(checkpoint.clone()),
    )
    .await;
    let mut resumed = ParseSubmission::new("a.pdf", b"pdf", ParseMode::Text, Some(&saved));
    assert_eq!(
        resumed
            .load(&client, &context(), Some(&checkpoint))
            .await
            .unwrap()
            .id,
        "original-created-job"
    );
    let second = server.await.unwrap();
    for request in [first[0].as_str(), second[0].as_str()] {
        assert!(request.contains(saved.remote_file_id.as_deref().unwrap()));
    }
}
#[tokio::test]
async fn unbillable_failure_can_be_replaced_once_and_repeated_failure_stops() {
    let checkpoint = Checkpoint::new(None);
    let (client, server) = mock(
        vec![
            Some(response("failed-first", "failed", "not_required")),
            Some(response("failed-second", "failed", "not_required")),
        ],
        Some(checkpoint.clone()),
    )
    .await;
    let mut submission = ParseSubmission::new(
        "a.pdf",
        b"pdf",
        ParseMode::Text,
        Some(&checkpoint.task().unwrap()),
    );
    let first = submission
        .load(&client, &context(), Some(&checkpoint))
        .await
        .unwrap();
    assert!(matches!(progress(&first).unwrap(), TaskProgress::Replace));
    let second = submission
        .replace(&client, &context(), Some(&checkpoint))
        .await
        .unwrap();
    assert!(matches!(progress(&second).unwrap(), TaskProgress::Replace));
    assert!(matches!(
        submission
            .replace(&client, &context(), Some(&checkpoint))
            .await,
        Err(KnowledgeBaseError::Invalid(_))
    ));
    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[0].contains("original-request"));
    assert!(!requests[1].contains("original-request"));
}
#[tokio::test]
async fn missing_task_during_polling_gets_a_new_submission() {
    let checkpoint = Checkpoint::new(Some("running-job"));
    let (client, server) = mock(
        vec![
            Some(json!({"code":404,"message":"任务不存在"})),
            Some(response("replacement", "queued", "pending")),
        ],
        Some(checkpoint.clone()),
    )
    .await;
    let mut submission = ParseSubmission::new(
        "a.pdf",
        b"pdf",
        ParseMode::Text,
        Some(&checkpoint.task().unwrap()),
    );
    assert_eq!(
        submission
            .poll(&client, &context(), "running-job", Some(&checkpoint))
            .await
            .unwrap()
            .id,
        "replacement"
    );
    assert_eq!(server.await.unwrap().len(), 2);
}
#[tokio::test]
async fn forbidden_task_lookup_does_not_submit_a_new_job() {
    let checkpoint = Checkpoint::new(Some("old-job"));
    let (client, server) = mock(vec![Some(json!({"code":403,"message":"无权限"}))], None).await;
    let mut submission = ParseSubmission::new(
        "a.pdf",
        b"pdf",
        ParseMode::Text,
        Some(&checkpoint.task().unwrap()),
    );
    assert!(
        submission
            .load(&client, &context(), Some(&checkpoint))
            .await
            .is_err()
    );
    assert_eq!(
        checkpoint.task().unwrap().remote_job_id.as_deref(),
        Some("old-job")
    );
    assert_eq!(server.await.unwrap().len(), 1);
}
#[test]
fn billing_waits_and_unknown_submission_never_become_a_fresh_request() {
    let balance: ParseTask =
        serde_json::from_value(task("job", "completed", "balance_insufficient")).unwrap();
    assert!(matches!(
        progress(&balance),
        Err(KnowledgeBaseError::Storage(_))
    ));
    let unknown: ParseTask =
        serde_json::from_value(task("job", "submission_unknown", "pending")).unwrap();
    assert!(matches!(
        progress(&unknown),
        Err(KnowledgeBaseError::Storage(_))
    ));
    let pending: ParseTask = serde_json::from_value(task("job", "failed", "pending")).unwrap();
    assert!(matches!(progress(&pending).unwrap(), TaskProgress::Wait));
    assert!(!task_not_found(&ServerApiError::gateway(
        404,
        "no model",
        "gateway_error"
    )));
    assert!(!task_not_found(&ServerApiError::Other("任务不存在".into())));
}
#[test]
fn deleted_or_expired_tasks_are_not_downloaded_and_nested_manifest_is_required() {
    let mut data = task("job", "completed", "settled");
    data["deletion_pending"] = json!(true);
    let pending: ParseTask = serde_json::from_value(data.clone()).unwrap();
    assert!(matches!(progress(&pending).unwrap(), TaskProgress::Wait));
    data["deletion_pending"] = json!(false);
    data["deleted"] = json!(true);
    let deleted: ParseTask = serde_json::from_value(data.clone()).unwrap();
    assert!(matches!(progress(&deleted).unwrap(), TaskProgress::Replace));
    data["deleted"] = json!(false);
    data["result_expires_at"] = json!("1");
    let expired: ParseTask = serde_json::from_value(data.clone()).unwrap();
    assert!(result_expired(&expired));
    data["result_expires_at"] = Value::Null;
    data["result"] =
        json!({"status":"completed","result":{"files":[{"filename":"a.pdf"}],"zip":"task.zip"}});
    let ready: ParseTask = serde_json::from_value(data).unwrap();
    assert!(matches!(progress(&ready).unwrap(), TaskProgress::Ready));
    assert_eq!(
        result_manifest(&ready).unwrap()["files"][0]["filename"],
        "a.pdf"
    );
}
#[test]
fn failed_gateway_segment_error_is_visible_when_platform_error_is_empty() {
    let mut value = task("job", "failed", "settled");
    value["result"] = json!({"error":"", "segments":[{"error":"PDF 拆页失败，未提交不完整文件"}]});
    let task: ParseTask = serde_json::from_value(value).unwrap();
    let message = match progress(&task) {
        Err(error) => error.to_string(),
        Ok(_) => panic!("a failed task must not be published"),
    };
    assert!(message.contains("PDF 拆页失败"));
    assert!(!message.contains("请检查解析结果"));
}
