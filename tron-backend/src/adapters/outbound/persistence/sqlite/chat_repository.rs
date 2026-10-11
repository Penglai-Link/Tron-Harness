//! Tron owns only workspace/business links. Session lifecycle and presentation
//! history are stored through Framework services on the injected connection.
use super::framework_storage::{blocking, chat_error};
use crate::{
    application::{agent::FrameworkHost, chat::ChatRepository},
    domain::chat::*,
};
use eliteforge_agent_core as fw;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub struct SqliteChatRepository {
    host: FrameworkHost,
}
#[derive(Clone, Serialize, Deserialize)]
struct Link {
    id: i64,
    workspace_code: String,
    project_code: Option<String>,
    user_code: Option<String>,
    app_code: Option<String>,
    task_code: Option<String>,
    source_type: ChatSessionSourceType,
    source_code: Option<String>,
    agent_code: Option<String>,
}
impl Link {
    fn from_session(s: &ChatSession) -> Self {
        Self {
            id: s.id,
            workspace_code: s.workspace_code.clone(),
            project_code: s.project_code.clone(),
            user_code: s.user_code.clone(),
            app_code: s.app_code.clone(),
            task_code: s.task_code.clone(),
            source_type: s.source_type,
            source_code: s.source_code.clone(),
            agent_code: s.agent_code.clone(),
        }
    }
}
impl SqliteChatRepository {
    pub fn connect(database_url: &str) -> Result<Self, ChatError> {
        let host = if database_url == ":memory:" {
            FrameworkHost::temporary(None)
        } else {
            Ok(FrameworkHost::persistent(database_url, None))
        }
        .map_err(|_| ChatError::StorageFailure)?;
        Self::with_host(host)
    }
    pub fn with_host(host: FrameworkHost) -> Result<Self, ChatError> {
        blocking(async {
            host.storage_framework()
                .await
                .map_err(|_| ChatError::StorageFailure)?;
            host.shared_database()
                .await
                .map_err(chat_error)?
                .call(|c| super::schema::initialize_business(c))
                .await
                .map_err(|_| ChatError::StorageFailure)?;
            Ok(Self { host })
        })
    }
    pub(crate) async fn ensure_run_session(
        host: &FrameworkHost,
        context: &crate::domain::agent::AgentRunContext,
    ) -> Result<(), ChatError> {
        host.shared_database()
            .await
            .map_err(chat_error)?
            .call(|c| super::schema::initialize_business(c))
            .await
            .map_err(|_| ChatError::StorageFailure)?;
        let repository = Self { host: host.clone() };
        let id = context
            .conversation_code
            .as_deref()
            .unwrap_or(&context.run_code);
        match repository.session(id).await {
            Ok(session) => {
                if session.workspace_code != context.workspace_code {
                    return Err(ChatError::SessionNotFound);
                }
                if session.status != ChatSessionStatus::Active {
                    return Err(ChatError::SessionArchived);
                }
                return Ok(());
            }
            Err(ChatError::SessionNotFound) if context.conversation_code.is_none() => {}
            Err(error) => return Err(error),
        }
        let mut record = fw::SessionRecord::new("临时运行");
        record.id = id.into();
        let link = Link {
            id: 0,
            workspace_code: context.workspace_code.clone(),
            project_code: None,
            user_code: context.user_code.clone(),
            app_code: context.execution_scope.app_code.clone(),
            task_code: context.execution_scope.task_code.clone(),
            source_type: ChatSessionSourceType::Manual,
            source_code: None,
            agent_code: context
                .short_term_memory
                .as_ref()
                .and_then(|m| m.agent_code.clone()),
        };
        let value = serde_json::to_string(&link).map_err(|_| ChatError::StorageFailure)?;
        let id = id.to_owned();
        let db = host.shared_database().await.map_err(chat_error)?;
        let sessions = repository
            .framework()
            .await?
            .sessions()
            .map_err(|_| ChatError::StorageFailure)?;
        db.transaction(async {
            sessions.create(record).await?;
            db.call(move |c| {
                c.execute(
                    "INSERT INTO workspace_session_links VALUES (?1,?2,?3)",
                    params![id, link.workspace_code, value],
                )
            })
            .await
            .map_err(|e| fw::StateError::Backend(e.to_string()))?;
            Ok(())
        })
        .await
        .map_err(chat_error)
    }
    async fn framework(&self) -> Result<eliteforge_agent_framework::Framework, ChatError> {
        self.host
            .storage_framework()
            .await
            .map_err(|_| ChatError::StorageFailure)
    }
    async fn link(&self, id: &str) -> Result<Link, ChatError> {
        let id = id.to_owned();
        let json: Option<String> = self
            .host
            .shared_database()
            .await
            .map_err(chat_error)?
            .call(move |c| {
                c.query_row(
                    "SELECT business_json FROM workspace_session_links WHERE session_id=?1",
                    [id],
                    |r| r.get(0),
                )
                .optional()
            })
            .await
            .map_err(|_| ChatError::StorageFailure)?;
        serde_json::from_str(&json.ok_or(ChatError::SessionNotFound)?)
            .map_err(|_| ChatError::StorageFailure)
    }
    async fn session(&self, id: &str) -> Result<ChatSession, ChatError> {
        let link = self.link(id).await?;
        let framework = self.framework().await?;
        let sessions = framework
            .sessions()
            .map_err(|_| ChatError::StorageFailure)?;
        let session = sessions.get(id).await.map_err(chat_error)?;
        let runs = framework
            .runs()
            .map_err(|_| ChatError::StorageFailure)?
            .list(id)
            .await
            .map_err(chat_error)?;
        let last = runs
            .iter()
            .filter(|run| run.kind != "SUMMARY")
            .max_by_key(|r| (r.started_at_ms, r.id));
        let runtime = sessions
            .read_context(id, "tron.external-reference")
            .await
            .map_err(chat_error)?
            .map(|(_, v)| v);
        Ok(ChatSession {
            id: link.id,
            session_code: session.id,
            workspace_code: link.workspace_code,
            project_code: link.project_code,
            user_code: link.user_code,
            app_code: link.app_code,
            task_code: link.task_code,
            source_type: link.source_type,
            source_code: link.source_code,
            agent_code: link.agent_code,
            system_agent_session_id: runtime
                .and_then(|v| v["sessionId"].as_str().map(str::to_owned)),
            title: session.title,
            status: match session.status {
                fw::SessionStatus::Active => ChatSessionStatus::Active,
                fw::SessionStatus::Archived => ChatSessionStatus::Archived,
            },
            last_run_code: last.map(|r| r.id.to_string()),
            last_run_status: last.map(|r| {
                match r.status {
                    fw::ManagedRunStatus::Completed => "COMPLETED",
                    fw::ManagedRunStatus::AwaitingInput => "AWAITING_USER_INPUT",
                    fw::ManagedRunStatus::Running => "RUNNING",
                    fw::ManagedRunStatus::Cancelled => "STOPPED",
                    _ => "FAILED",
                }
                .into()
            }),
            created_at: crate::shared::time::timestamp_from_millis(session.created_at_ms as i64),
            updated_at: crate::shared::time::timestamp_from_millis(session.updated_at_ms as i64),
        })
    }
    async fn events(&self, session: &str, kind: &str) -> Result<Vec<fw::SessionEvent>, ChatError> {
        self.link(session).await?;
        let store = self
            .framework()
            .await?
            .events()
            .map_err(|_| ChatError::StorageFailure)?;
        let mut result = Vec::new();
        let mut after = 0;
        loop {
            let page = store.list(session, after, 1000).await.map_err(chat_error)?;
            if page.is_empty() {
                break;
            }
            after = page.last().unwrap().sequence;
            result.extend(page.into_iter().filter(|e| e.kind == kind));
        }
        Ok(result)
    }
    async fn put_context(&self, id: &str, key: &str, value: Value) -> Result<(), ChatError> {
        let s = self
            .framework()
            .await?
            .sessions()
            .map_err(|_| ChatError::StorageFailure)?;
        let expected = s
            .read_context(id, key)
            .await
            .map_err(chat_error)?
            .map(|(revision, _)| revision);
        s.write_context(id, key, expected, value)
            .await
            .map_err(chat_error)?;
        Ok(())
    }
}
impl ChatRepository for SqliteChatRepository {
    fn create_session(&mut self, session: &ChatSession) -> Result<(), ChatError> {
        let mut record = fw::SessionRecord::new(&session.title);
        record.id = session.session_code.clone();
        let link = Link::from_session(session);
        let id = record.id.clone();
        let value = serde_json::to_string(&link).map_err(|_| ChatError::StorageFailure)?;
        blocking(async {
            let db = self.host.shared_database().await.map_err(chat_error)?;
            let sessions = self
                .framework()
                .await?
                .sessions()
                .map_err(|_| ChatError::StorageFailure)?;
            db.transaction(async {
                sessions.create(record).await?;
                db.call(move|c|c.execute("INSERT INTO workspace_session_links(session_id,workspace_code,business_json) VALUES(?1,?2,?3)",params![id,link.workspace_code,value]))
                    .await.map_err(|e|fw::StateError::Backend(e.to_string()))?; Ok(())
            }).await.map_err(chat_error)
        })
    }
    fn delete_session(&mut self, id: &str) -> Result<(), ChatError> {
        blocking(async {
            self.link(id).await?;
            let db = self.host.shared_database().await.map_err(chat_error)?;
            let sessions = self
                .framework()
                .await?
                .sessions()
                .map_err(|_| ChatError::StorageFailure)?;
            let code = id.to_owned();
            db.transaction(async {
                let record=sessions.get(id).await?;
                if record.status!=fw::SessionStatus::Archived {return Err(fw::StateError::Invalid("session must be archived".into()));}
                if self.framework().await.map_err(|e|fw::StateError::Backend(e.to_string()))?.goals().map_err(|e|fw::StateError::Backend(e.to_string()))?.active(id).await?.is_some_and(|goal| goal.status == fw::GoalStatus::Active) {
                    return Err(fw::StateError::Busy);
                }
                sessions.delete(id).await?;
                db.call(move|c| {
                    c.execute("DELETE FROM resource_chat_session_attachments WHERE session_code=?1",[&code])?;
                    for (table, sql) in [
                        ("chat_session_auto_model_routes", "DELETE FROM chat_session_auto_model_routes WHERE session_code=?1"),
                        ("provider_model_run_snapshots", "DELETE FROM provider_model_run_snapshots WHERE session_code=?1"),
                        ("automation_execution_records", "UPDATE automation_execution_records SET session_code=NULL WHERE session_code=?1"),
                    ] {
                        if c.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",[table],|r|r.get::<_,bool>(0))? { c.execute(sql,[&code])?; }
                    }
                    c.execute("DELETE FROM workspace_session_links WHERE session_id=?1",[&code])?;
                    Ok::<_,rusqlite::Error>(())
                }).await.map_err(|e|fw::StateError::Backend(e.to_string()))?; Ok(())
            }).await.map_err(chat_error)
        })
    }
    fn list_sessions(&mut self) -> Result<Vec<ChatSession>, ChatError> {
        blocking(async {
            let db = self.host.shared_database().await.map_err(chat_error)?;
            let codes = db
                .call(|c| {
                    let mut stmt = c.prepare(
                        "SELECT session_id FROM workspace_session_links ORDER BY session_id",
                    )?;
                    stmt.query_map([], |r| r.get::<_, String>(0))?
                        .collect::<Result<Vec<_>, _>>()
                })
                .await
                .map_err(|_| ChatError::StorageFailure)?;
            let mut result = Vec::new();
            for code in codes {
                result.push(self.session(&code).await?);
            }
            result.sort_by(|a, b| {
                b.updated_at
                    .cmp(&a.updated_at)
                    .then_with(|| b.session_code.cmp(&a.session_code))
            });
            Ok(result)
        })
    }
    fn get_session(&mut self, id: &str) -> Result<ChatSession, ChatError> {
        blocking(self.session(id))
    }
    fn update_session_status(
        &mut self,
        id: &str,
        status: ChatSessionStatus,
        _at: &str,
    ) -> Result<(), ChatError> {
        blocking(async {
            self.link(id).await?;
            let f = self.framework().await?;
            let s = f.sessions().map_err(|_| ChatError::StorageFailure)?;
            let db = self.host.shared_database().await.map_err(chat_error)?;
            db.transaction(async {
                if status == ChatSessionStatus::Archived {
                    if f.goals()
                        .map_err(|e| fw::StateError::Backend(e.to_string()))?
                        .active(id)
                        .await?
                        .is_some_and(|goal| goal.status == fw::GoalStatus::Active)
                    {
                        return Err(fw::StateError::Busy);
                    }
                    s.archive(id).await?;
                } else {
                    s.restore(id).await?;
                }
                Ok(())
            })
            .await
            .map_err(chat_error)?;
            Ok(())
        })
    }
    fn update_session_title(&mut self, id: &str, title: &str, _at: &str) -> Result<(), ChatError> {
        blocking(async {
            self.link(id).await?;
            let s = self
                .framework()
                .await?
                .sessions()
                .map_err(|_| ChatError::StorageFailure)?;
            let mut record = s.get(id).await.map_err(chat_error)?;
            let expected = record.revision;
            record.title = title.into();
            s.update(record, expected).await.map_err(chat_error)?;
            Ok(())
        })
    }
    fn update_session_run(
        &mut self,
        id: &str,
        _run: &str,
        _status: &str,
        agent: Option<&str>,
        runtime: Option<&str>,
        _at: &str,
    ) -> Result<(), ChatError> {
        blocking(async {
            let db = self.host.shared_database().await.map_err(chat_error)?;
            let sessions = self
                .framework()
                .await?
                .sessions()
                .map_err(|_| ChatError::StorageFailure)?;
            db.transaction(async {
                let mut link = self
                    .link(id)
                    .await
                    .map_err(|e| fw::StateError::Backend(e.to_string()))?;
                if let Some(agent) = agent {
                    link.agent_code = Some(agent.into());
                }
                let value = serde_json::to_string(&link)
                    .map_err(|e| fw::StateError::Backend(e.to_string()))?;
                let code = id.to_owned();
                db.call(move |c| {
                    c.execute(
                        "UPDATE workspace_session_links SET business_json=?2 WHERE session_id=?1",
                        params![code, value],
                    )
                })
                .await
                .map_err(|e| fw::StateError::Backend(e.to_string()))?;
                if let Some(runtime) = runtime {
                    let expected = sessions
                        .read_context(id, "tron.external-reference")
                        .await?
                        .map(|(revision, _)| revision);
                    sessions
                        .write_context(
                            id,
                            "tron.external-reference",
                            expected,
                            json!({"sessionId":runtime}),
                        )
                        .await?;
                }
                Ok(())
            })
            .await
            .map_err(chat_error)
        })
    }

    fn runtime_session_id(
        &mut self,
        id: &str,
        adapter: &str,
        agent: &str,
    ) -> Result<Option<String>, ChatError> {
        blocking(async {
            self.link(id).await?;
            let key = format!("tron.external/{adapter}/{agent}");
            Ok(self
                .framework()
                .await?
                .sessions()
                .map_err(|_| ChatError::StorageFailure)?
                .read_context(id, &key)
                .await
                .map_err(chat_error)?
                .and_then(|(_, v)| v.as_str().map(str::to_owned)))
        })
    }
    fn upsert_runtime_session(
        &mut self,
        id: &str,
        adapter: &str,
        agent: &str,
        runtime: &str,
        _run: Option<&str>,
        _at: &str,
    ) -> Result<(), ChatError> {
        blocking(self.put_context(
            id,
            &format!("tron.external/{adapter}/{agent}"),
            json!(runtime),
        ))
    }
    fn create_message(&mut self, message: &ChatMessage) -> Result<(), ChatError> {
        blocking(async {
            self.link(&message.session_code).await?;
            let mut event = fw::SessionEvent::new(
                &message.session_code,
                "tron.message",
                serde_json::to_value(message).map_err(|_| ChatError::StorageFailure)?,
            );
            event.run_id = message
                .run_code
                .as_deref()
                .and_then(|id| fw::RunId::parse_str(id).ok());
            self.framework()
                .await?
                .sessions()
                .map_err(|_| ChatError::StorageFailure)?
                .store
                .append_event(event)
                .await
                .map_err(chat_error)?;
            Ok(())
        })
    }
    fn list_messages(&mut self, id: &str) -> Result<Vec<ChatMessage>, ChatError> {
        blocking(async {
            self.events(id, "tron.message")
                .await?
                .into_iter()
                .map(|e| serde_json::from_value(e.payload).map_err(|_| ChatError::StorageFailure))
                .collect()
        })
    }
    fn list_user_input_requests(
        &mut self,
        id: &str,
    ) -> Result<Vec<ChatUserInputRequest>, ChatError> {
        blocking(async {
            let mut requests = std::collections::BTreeMap::new();
            for e in self.events(id, "tron.interaction-view").await? {
                let request: ChatUserInputRequest =
                    serde_json::from_value(e.payload).map_err(|_| ChatError::StorageFailure)?;
                requests.insert(request.request_code.clone(), request);
            }
            let services = self
                .framework()
                .await?
                .services()
                .map_err(|_| ChatError::StorageFailure)?;
            let interactions = services
                .get::<fw::InteractionStoreService>()
                .ok_or(ChatError::StorageFailure)?;
            let mut views = Vec::new();
            for mut request in requests.into_values() {
                let interaction = fw::RunId::parse_str(&request.request_code)
                    .map_err(|_| ChatError::StorageFailure)?;
                let record = interactions
                    .0
                    .get(id, interaction)
                    .await
                    .map_err(chat_error)?;
                request.status = if record.response.is_some() {
                    ChatUserInputRequestStatus::Answered
                } else {
                    ChatUserInputRequestStatus::Pending
                };
                request.answered_at = record
                    .answered_at_unix_ms
                    .map(|v| crate::shared::time::timestamp_from_millis(v as i64));
                request.answers = record
                    .response
                    .and_then(|r| r.value.get("answers").and_then(Value::as_object).cloned())
                    .map(|answers| {
                        answers
                            .into_iter()
                            .map(|(question_code, value)| ChatUserInputAnswer {
                                question_code,
                                answer_kind: if value.get("option_id").is_some() {
                                    ChatUserInputAnswerKind::Option
                                } else if value.get("text").is_some() {
                                    ChatUserInputAnswerKind::Other
                                } else {
                                    ChatUserInputAnswerKind::Skipped
                                },
                                option_code: value["option_id"].as_str().map(str::to_owned),
                                other_text: value["text"].as_str().map(str::to_owned),
                            })
                            .collect()
                    });
                views.push(request);
            }
            Ok(views)
        })
    }
    fn create_user_input_request(
        &mut self,
        request: &ChatUserInputRequest,
    ) -> Result<(), ChatError> {
        blocking(async {
            self.framework()
                .await?
                .sessions()
                .map_err(|_| ChatError::StorageFailure)?
                .store
                .append_event(fw::SessionEvent::new(
                    &request.session_code,
                    "tron.interaction-view",
                    serde_json::to_value(request).map_err(|_| ChatError::StorageFailure)?,
                ))
                .await
                .map_err(chat_error)?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod shared_transaction_tests {
    use super::*;
    use crate::adapters::outbound::persistence::sqlite::SqliteResourceRepository;

    fn fixture() -> (tempfile::TempDir, SqliteChatRepository) {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("tron.sqlite3");
        let path = path.to_str().unwrap();
        SqliteResourceRepository::connect(path).unwrap();
        let repository = SqliteChatRepository::connect(path).unwrap();
        (directory, repository)
    }
    fn session(id: &str) -> ChatSession {
        ChatSession {
            id: 1,
            session_code: id.into(),
            workspace_code: "ws-a".into(),
            project_code: None,
            user_code: None,
            app_code: None,
            task_code: None,
            source_type: ChatSessionSourceType::Manual,
            source_code: None,
            agent_code: None,
            system_agent_session_id: None,
            title: "transaction".into(),
            status: ChatSessionStatus::Active,
            last_run_code: None,
            last_run_status: None,
            created_at: "".into(),
            updated_at: "".into(),
        }
    }
    #[test]
    fn ordinary_archived_history_excludes_dude_work_even_in_same_workspace() {
        let (_directory, mut repository) = fixture();
        let mut user = session("archived-user");
        user.status = ChatSessionStatus::Archived;
        repository.create_session(&user).unwrap();
        let mut dude = session("archived-dude");
        dude.status = ChatSessionStatus::Archived;
        dude.source_code = Some("__dude:main-dude".into());
        repository.create_session(&dude).unwrap();
        let (items, total) = repository
            .list_archived_sessions(Some("ws-a"), 0, 100)
            .unwrap();
        assert_eq!(total, 1);
        assert_eq!(items[0].session_code, user.session_code);
        let (_, total) = repository.list_archived_sessions(None, 0, 100).unwrap();
        assert_eq!(total, 1);
        assert!(repository.get_session("archived-dude").is_ok());
    }

    #[test]
    fn workspace_link_failure_rolls_back_framework_session_and_events() {
        let (_directory, mut repository) = fixture();
        let framework = blocking(repository.framework()).unwrap();
        let database = blocking(repository.host.shared_database()).unwrap();
        database.call_blocking(|c| c.execute_batch("CREATE TRIGGER reject_link BEFORE INSERT ON workspace_session_links BEGIN SELECT RAISE(ABORT,'injected link failure'); END;")).unwrap();
        assert!(
            repository
                .create_session(&session("atomic-create"))
                .is_err()
        );
        assert!(matches!(
            blocking(framework.sessions().unwrap().get("atomic-create")),
            Err(fw::StateError::NotFound)
        ));
        database
            .call_blocking(|c| c.execute_batch("DROP TRIGGER reject_link"))
            .unwrap();
        repository
            .create_session(&session("atomic-create"))
            .unwrap();
        assert_eq!(
            repository
                .get_session("atomic-create")
                .unwrap()
                .workspace_code,
            "ws-a"
        );
    }
    #[test]
    fn delete_failure_rolls_back_session_history_and_business_link_then_can_retry() {
        let (_directory, mut repository) = fixture();
        repository
            .create_session(&session("atomic-delete"))
            .unwrap();
        let framework = blocking(repository.framework()).unwrap();
        blocking(
            framework
                .sessions()
                .unwrap()
                .store
                .append_event(fw::SessionEvent::new(
                    "atomic-delete",
                    "test.history",
                    json!({"text":"preserve"}),
                )),
        )
        .unwrap();
        repository
            .update_session_status("atomic-delete", ChatSessionStatus::Archived, "")
            .unwrap();
        let database = blocking(repository.host.shared_database()).unwrap();
        database.call_blocking(|c| c.execute_batch("CREATE TRIGGER reject_delete BEFORE DELETE ON workspace_session_links BEGIN SELECT RAISE(ABORT,'injected delete failure'); END;")).unwrap();
        assert!(repository.delete_session("atomic-delete").is_err());
        assert_eq!(
            repository.get_session("atomic-delete").unwrap().status,
            ChatSessionStatus::Archived
        );
        assert!(
            blocking(framework.events().unwrap().list("atomic-delete", 0, 100))
                .unwrap()
                .iter()
                .any(|e| e.kind == "test.history")
        );
        database
            .call_blocking(|c| c.execute_batch("DROP TRIGGER reject_delete"))
            .unwrap();
        repository.delete_session("atomic-delete").unwrap();
        assert!(matches!(
            blocking(framework.sessions().unwrap().get("atomic-delete")),
            Err(fw::StateError::NotFound)
        ));
        assert!(matches!(
            repository.get_session("atomic-delete"),
            Err(ChatError::SessionNotFound)
        ));
    }
    #[test]
    fn synchronous_business_sql_joins_the_framework_transaction() {
        let (_directory, repository) = fixture();
        let database = blocking(repository.host.shared_database()).unwrap();
        let shared = super::super::framework_storage::SharedDatabase::open(
            directory_path(&_directory).to_str().unwrap(),
        )
        .unwrap();
        let framework = blocking(repository.framework()).unwrap();
        let result: Result<(), fw::StateError> = blocking(database.transaction(async {
            framework
                .sessions()
                .unwrap()
                .create(fw::SessionRecord::new("rollback"))
                .await?;
            shared
                .call(
                    |c| {
                        c.execute(
                            "INSERT INTO workspace_session_links VALUES('joined','ws-a','{}')",
                            [],
                        )
                    },
                    || rusqlite::Error::InvalidQuery,
                )
                .unwrap();
            Err(fw::StateError::Invalid("rollback whole unit".into()))
        }));
        assert!(result.is_err());
        let count: i64 = database
            .call_blocking(|c| {
                c.query_row(
                    "SELECT count(*) FROM workspace_session_links WHERE session_id='joined'",
                    [],
                    |r| r.get(0),
                )
            })
            .unwrap();
        assert_eq!(count, 0);
        assert!(
            blocking(framework.sessions().unwrap().list(None, 100))
                .unwrap()
                .is_empty()
        );
    }
    fn directory_path(directory: &tempfile::TempDir) -> std::path::PathBuf {
        directory.path().join("tron.sqlite3")
    }
}
