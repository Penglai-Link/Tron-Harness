//! SDK submission identities survive local retries; a confirmed missing or
//! unbillable failed task gets a new identity, never a replay of its tombstone.
use super::*;
use crab_serverapi::error::ServerApiError;

pub(super) struct ParseSubmission<'a> {
    request: CreateParseTask,
    source: &'a [u8],
    remote_id: Option<String>,
    replacements: usize,
}
impl<'a> ParseSubmission<'a> {
    pub(super) fn new(
        filename: &str,
        bytes: &'a [u8],
        mode: ParseMode,
        prior: Option<&IngestionTask>,
    ) -> Self {
        let mut request = CreateParseTask::new(filename, Vec::new(), mode);
        if let Some(prior) = prior {
            // Older checkpoints used the local task ID. All new submissions
            // persist the SDK request ID before sending any bytes.
            request.request_id = prior
                .remote_file_id
                .clone()
                .unwrap_or_else(|| prior.id.clone());
        }
        Self {
            request,
            source: bytes,
            remote_id: prior.and_then(|task| task.remote_job_id.clone()),
            replacements: 0,
        }
    }
    pub(super) async fn load(
        &mut self,
        client: &DocumentParseClient,
        context: &crab_serverapi::AuthorizedClientContext,
        checkpoint: Option<&Arc<dyn ParseCheckpoint>>,
    ) -> Result<ParseTask, KnowledgeBaseError> {
        let Some(id) = self.remote_id.clone() else {
            return self.create(client, context, checkpoint).await;
        };
        match client.get_task(context, &id).await {
            Ok(task)
                if (task.is_settled()
                    && matches!(task.status.as_str(), "failed" | "partial" | "canceled"))
                    || result_expired(&task) =>
            {
                self.replace(client, context, checkpoint).await
            }
            Ok(task) => Ok(task),
            Err(error) if task_not_found(&error) => self.replace(client, context, checkpoint).await,
            Err(error) => Err(platform_error(error)),
        }
    }
    pub(super) async fn poll(
        &mut self,
        client: &DocumentParseClient,
        context: &crab_serverapi::AuthorizedClientContext,
        id: &str,
        checkpoint: Option<&Arc<dyn ParseCheckpoint>>,
    ) -> Result<ParseTask, KnowledgeBaseError> {
        match client.get_task(context, id).await {
            Ok(task) => Ok(task),
            Err(error) if task_not_found(&error) => self.replace(client, context, checkpoint).await,
            Err(error) => Err(platform_error(error)),
        }
    }
    async fn create(
        &mut self,
        client: &DocumentParseClient,
        context: &crab_serverapi::AuthorizedClientContext,
        checkpoint: Option<&Arc<dyn ParseCheckpoint>>,
    ) -> Result<ParseTask, KnowledgeBaseError> {
        let task = loop {
            if let Some(checkpoint) = checkpoint {
                checkpoint.save(Some(self.request.request_id.clone()), None, true)?;
            }
            // Keep only a borrow during long polling. The source is already
            // retained by the ingestion worker; an extra 50 MB copy per task
            // is needed only while submitting.
            self.request.file_bytes = self.source.to_vec();
            let response = client.create_task(context, &self.request).await;
            self.request.file_bytes = Vec::new();
            match response {
                Ok(task) => break task,
                // A lost create response may leave only request_id locally.
                // The platform retains its tombstone after admin deletion.
                Err(error) if request_retired(&error) => self.renew()?,
                Err(error) => return Err(platform_error(error)),
            }
        };
        if let Some(checkpoint) = checkpoint {
            checkpoint.save(
                Some(self.request.request_id.clone()),
                Some(task.id.clone()),
                false,
            )?;
        }
        self.remote_id = Some(task.id.clone());
        Ok(task)
    }
    pub(super) async fn replace(
        &mut self,
        client: &DocumentParseClient,
        context: &crab_serverapi::AuthorizedClientContext,
        checkpoint: Option<&Arc<dyn ParseCheckpoint>>,
    ) -> Result<ParseTask, KnowledgeBaseError> {
        self.renew()?;
        self.create(client, context, checkpoint).await
    }
    fn renew(&mut self) -> Result<(), KnowledgeBaseError> {
        // One recovery per worker invocation. Repeated upstream failures stop
        // instead of creating an unlimited series of jobs.
        if self.replacements >= 1 {
            return Err(KnowledgeBaseError::Invalid(
                "解析任务重新提交后仍不可用，请稍后重试；未发布不完整索引".into(),
            ));
        }
        self.replacements += 1;
        self.request =
            CreateParseTask::new(self.request.filename.clone(), Vec::new(), self.request.mode)
                .with_auto_cleanup(self.request.auto_cleanup);
        self.remote_id = None;
        Ok(())
    }
}

pub(super) fn task_not_found(error: &ServerApiError) -> bool {
    // A proxy HTTP 404 or model Gateway 404 is not proof that this task is gone.
    matches!(error, ServerApiError::Business { code: 404, .. })
}
fn request_retired(error: &ServerApiError) -> bool {
    matches!(error, ServerApiError::Business { code: 409, message }
        if matches!(message.as_str(),
            "原任务已删除或正在删除，请使用新的 request_id" |
            "原任务已清理，请使用新的 request_id"))
}
pub(super) fn result_expired(task: &ParseTask) -> bool {
    task.is_settled()
        && task
            .result_expires_at
            .as_deref()
            .and_then(|value| value.parse::<i64>().ok())
            .is_some_and(|expires| expires <= chrono::Utc::now().timestamp_millis())
}
pub(super) enum TaskProgress {
    Ready,
    Wait,
    Replace,
}
pub(super) fn progress(task: &ParseTask) -> Result<TaskProgress, KnowledgeBaseError> {
    if task.deleted || result_expired(task) {
        return Ok(TaskProgress::Replace);
    }
    if task.deletion_pending {
        return Ok(TaskProgress::Wait);
    }
    if task.status == "failed" && task.billing_status == "not_required" {
        return Ok(TaskProgress::Replace);
    }
    if task.billing_status == "balance_insufficient" {
        return Err(KnowledgeBaseError::Storage(
            "解析结果待结算：积分不足，请充值后重试原任务".into(),
        ));
    }
    if task.status == "submission_unknown" {
        return Err(KnowledgeBaseError::Storage(
            "平台暂未确认解析提交，稍后继续查询原任务".into(),
        ));
    }
    if task.is_settled() {
        if task.status == "completed" {
            return Ok(TaskProgress::Ready);
        }
        return Err(KnowledgeBaseError::Invalid(format!(
            "解析任务{}，未发布不完整的索引：{}",
            task.status,
            failure_reason(task)
        )));
    }
    Ok(TaskProgress::Wait)
}

fn failure_reason(task: &ParseTask) -> String {
    let nonempty = |value: &str| !value.trim().is_empty();
    task.error_message.as_deref().filter(|value| nonempty(value))
        .or_else(|| task.result.as_ref()?.get("error")?.as_str().filter(|value| nonempty(value)))
        .or_else(|| task.result.as_ref()?.get("segments")?.as_array()?.iter()
            .filter_map(|segment| segment.get("error")?.as_str())
            .find(|value| nonempty(value)))
        .unwrap_or("请检查解析结果")
        .chars().take(512).collect()
}
pub(super) fn result_manifest(task: &ParseTask) -> Result<&Value, KnowledgeBaseError> {
    task.result
        .as_ref()
        .and_then(|gateway| gateway.get("result"))
        .filter(|result| result.is_object())
        .ok_or_else(|| KnowledgeBaseError::Storage("平台已结算，但未返回解析产物清单".into()))
}
