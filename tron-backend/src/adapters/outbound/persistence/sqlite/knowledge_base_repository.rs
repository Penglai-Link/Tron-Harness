use crate::application::knowledge_base::{
    KnowledgeBase, KnowledgeBaseDocument, KnowledgeBaseError,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Serialize, de::DeserializeOwned};
mod agent_grants;
pub(super) mod deletion;
mod intake;
mod platform_catalog;
struct SqliteKnowledgeBaseRepositorySql<'a> {
    connection: &'a mut Connection,
}
#[derive(Clone)]
pub struct SqliteKnowledgeBaseRepository {
    connection: crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase,
}
impl SqliteKnowledgeBaseRepositorySql<'_> {
    pub fn get(&self, id: &str) -> Result<KnowledgeBase, KnowledgeBaseError> {
        let conn = &*self.connection;
        let raw: Option<String> = conn
            .query_row(
                "SELECT payload_json FROM knowledge_bases WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()
            .map_err(storage)?;
        decode(&raw.ok_or(KnowledgeBaseError::NotFound)?)
    }
    pub fn insert(&self, kb: &KnowledgeBase, user: &str) -> Result<(), KnowledgeBaseError> {
        self.connection
            .execute(
                "INSERT INTO knowledge_bases(id,owner_user_code,payload_json) VALUES(?1,?2,?3)",
                params![kb.id, user, encode(kb)?],
            )
            .map_err(storage)?;
        Ok(())
    }
    pub fn documents(&self, id: &str) -> Result<Vec<KnowledgeBaseDocument>, KnowledgeBaseError> {
        let conn = &*self.connection;
        let mut stmt=conn.prepare("SELECT payload_json FROM knowledge_base_documents WHERE knowledge_base_id=?1 ORDER BY id").map_err(storage)?;
        let raw = stmt
            .query_map([id], |r| r.get::<_, String>(0))
            .map_err(storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage)?;
        raw.iter().map(|s| decode(s)).collect()
    }
    pub fn document(
        &self,
        kb: &str,
        id: &str,
    ) -> Result<KnowledgeBaseDocument, KnowledgeBaseError> {
        self.documents(kb)?
            .into_iter()
            .find(|d| d.id == id)
            .ok_or(KnowledgeBaseError::NotFound)
    }
    pub fn insert_document(
        &self,
        kb: &str,
        doc: &KnowledgeBaseDocument,
    ) -> Result<(), KnowledgeBaseError> {
        self.insert_document_with_intent(kb, doc, false)
    }
    pub fn upload_request(
        &self,
        request_id: &str,
        identity: &str,
    ) -> Result<Option<(String, String)>, KnowledgeBaseError> {
        let saved: Option<(String, String, String)> = self.connection.query_row("SELECT content_identity,knowledge_base_id,document_id FROM knowledge_base_upload_requests WHERE request_id=?1", [request_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(storage)?;
        match saved {
            Some((prior, kb, doc)) if prior == identity => Ok(Some((kb, doc))),
            Some(_) => Err(KnowledgeBaseError::Conflict),
            None => Ok(None),
        }
    }
    pub fn remember_upload_request(
        &self,
        request_id: &str,
        identity: &str,
        kb: &str,
        doc: &str,
    ) -> Result<(), KnowledgeBaseError> {
        self.connection
            .execute(
                "INSERT OR IGNORE INTO knowledge_base_upload_requests VALUES(?1,?2,?3,?4)",
                params![request_id, identity, kb, doc],
            )
            .map_err(storage)?;
        self.upload_request(request_id, identity)?
            .ok_or(KnowledgeBaseError::Conflict)?;
        Ok(())
    }
    pub fn save_document(
        &self,
        kb: &str,
        doc: &KnowledgeBaseDocument,
        revision: u64,
    ) -> Result<(), KnowledgeBaseError> {
        let changed=self.connection.execute("UPDATE knowledge_base_documents SET payload_json=?1,revision=revision+1 WHERE id=?2 AND knowledge_base_id=?3 AND revision=?4 AND json_extract(payload_json,'$.resourceCode')=?5 AND json_extract(payload_json,'$.resourceRevision')=?6",params![encode(doc)?,doc.id,kb,revision,doc.resource_code,doc.resource_revision]).map_err(storage)?;
        if changed != 1 {
            return Err(KnowledgeBaseError::Conflict);
        }
        Ok(())
    }
    /// Cache publication checks original identity as well as generation; id/revision may repeat after deletion.
    pub fn save_parsing_artifact(
        &self,
        kb: &str,
        doc: &KnowledgeBaseDocument,
    ) -> Result<(), KnowledgeBaseError> {
        let changed=self.connection.execute("UPDATE knowledge_base_documents SET payload_json=json_set(payload_json,'$.parsingArtifact',json(?1),'$.derivedResourceCodes',json(?5)) WHERE id=?2 AND knowledge_base_id=?3 AND revision=?4 AND json_extract(payload_json,'$.resourceCode')=?6 AND json_extract(payload_json,'$.resourceRevision')=?7",params![encode(&doc.parsing_artifact)?,doc.id,kb,doc.revision,encode(&doc.derived_resource_codes)?,doc.resource_code,doc.resource_revision]).map_err(storage)?;
        if changed != 1 {
            return Err(KnowledgeBaseError::Conflict);
        }
        Ok(())
    }
    /// Same-generation fidelity findings are publication guards, never a content edit.
    pub fn save_fidelity(
        &self,
        kb: &str,
        doc: &KnowledgeBaseDocument,
    ) -> Result<(), KnowledgeBaseError> {
        let blocked = crate::application::knowledge_base::fidelity::blocked(doc);
        let status = if blocked
            && !matches!(
                doc.status.as_str(),
                "QUEUED" | "PARSING" | "CHUNKING" | "EMBEDDING" | "PUBLISHING"
            ) {
            "REVIEW_REQUIRED"
        } else {
            doc.status.as_str()
        };
        let error = if blocked {
            Some("数据保真核验发现未解决字形或符号，禁止发布索引；请核对原件及图片".to_owned())
        } else {
            doc.error.clone()
        };
        let changed=self.connection.execute("UPDATE knowledge_base_documents SET payload_json=json_set(payload_json,'$.parsingArtifact',json(?1),'$.status',?2,'$.error',?3) WHERE id=?4 AND knowledge_base_id=?5 AND revision=?6 AND json_extract(payload_json,'$.resourceCode')=?7 AND json_extract(payload_json,'$.resourceRevision')=?8",params![encode(&doc.parsing_artifact)?,status,error,doc.id,kb,doc.revision,doc.resource_code,doc.resource_revision]).map_err(storage)?;
        if changed != 1 {
            return Err(KnowledgeBaseError::Conflict);
        }
        Ok(())
    }
    pub fn save_image_description_artifacts(
        &self,
        kb: &str,
        doc: &KnowledgeBaseDocument,
    ) -> Result<(), KnowledgeBaseError> {
        let changed = self.connection.execute("UPDATE knowledge_base_documents SET payload_json=json_set(payload_json,'$.imageDescriptionArtifacts',json(?1),'$.derivedResourceCodes',json(?2)) WHERE id=?3 AND knowledge_base_id=?4 AND revision=?5 AND json_extract(payload_json,'$.resourceCode')=?6 AND json_extract(payload_json,'$.resourceRevision')=?7", params![encode(&doc.image_description_artifacts)?, encode(&doc.derived_resource_codes)?, doc.id, kb, doc.revision,doc.resource_code,doc.resource_revision]).map_err(storage)?;
        if changed != 1 {
            return Err(KnowledgeBaseError::Conflict);
        }
        Ok(())
    }
    pub fn delete_document(&self, kb: &str, id: &str) -> Result<(), KnowledgeBaseError> {
        self.connection
            .execute(
                "DELETE FROM knowledge_base_documents WHERE id=?1 AND knowledge_base_id=?2",
                params![id, kb],
            )
            .map_err(storage)?;
        Ok(())
    }
    pub fn owns(&self, user: &str, id: &str) -> bool {
        self.connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM knowledge_bases WHERE id=?1 AND owner_user_code=?2 )",
                params![id, user],
                |r| r.get::<_, bool>(0),
            )
            .unwrap_or(false)
    }
    pub fn grants(&self, id: &str) -> Result<Vec<String>, KnowledgeBaseError> {
        let conn = &*self.connection;
        let mut stmt=conn.prepare("SELECT agent_code FROM knowledge_base_grants WHERE knowledge_base_id=?1 ORDER BY agent_code").map_err(storage)?;
        stmt.query_map([id], |r| r.get(0))
            .map_err(storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage)
    }
    pub fn set_grants(&self, id: &str, agents: &[String]) -> Result<(), KnowledgeBaseError> {
        let conn = &*self.connection;
        let tx = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&conn, rusqlite::TransactionBehavior::Immediate).map_err(storage)?;
        tx.execute(
            "DELETE FROM knowledge_base_grants WHERE knowledge_base_id=?1",
            [id],
        )
        .map_err(storage)?;
        for agent in agents {
            tx.execute(
                "INSERT OR IGNORE INTO knowledge_base_grants VALUES(?1,?2)",
                params![id, agent],
            )
            .map_err(storage)?;
        }
        tx.execute(
            "UPDATE knowledge_bases SET acl_revision=acl_revision+1 WHERE id=?1",
            [id],
        )
        .map_err(storage)?;
        tx.commit().map_err(storage)
    }
    pub fn acl_revision(&self, id: &str) -> Result<u64, KnowledgeBaseError> {
        self.connection
            .query_row(
                "SELECT acl_revision FROM knowledge_bases WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .map_err(storage)
    }
    pub fn mark_private(
        &self,
        user: &str,
        workspace: &str,
        resource: &str,
    ) -> Result<(), KnowledgeBaseError> {
        self.connection
            .execute(
                "INSERT INTO knowledge_base_resource_privacy VALUES(?1,?2,?3)",
                params![workspace, resource, user],
            )
            .map_err(storage)?;
        Ok(())
    }
}
impl SqliteKnowledgeBaseRepositorySql<'_> {
    fn can_read(&self, user: &str, workspace: &str, resource: &str) -> bool {
        let owner=self.connection.query_row("SELECT owner_user_code FROM knowledge_base_resource_privacy WHERE workspace_code=?1 AND resource_code=?2",params![workspace,resource],|r|r.get::<_,String>(0)).optional();
        match owner {
            Ok(None) => true,
            Ok(Some(owner)) => owner == user,
            Err(_) => false,
        }
    }
    fn can_read_agent(
        &self,
        user: &str,
        workspace: &str,
        resource: &str,
        agent: Option<&str>,
    ) -> bool {
        if !self.is_protected(workspace, resource) {
            return true;
        }
        let Some(agent) = agent else { return false };
        self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM knowledge_base_resource_privacy p JOIN knowledge_bases k ON k.owner_user_code=p.owner_user_code JOIN knowledge_base_documents d ON d.knowledge_base_id=k.id JOIN knowledge_base_grants g ON g.knowledge_base_id=k.id WHERE p.workspace_code=?1 AND p.resource_code=?2 AND p.owner_user_code=?3 AND g.agent_code=?4 AND coalesce(json_extract(d.payload_json,'$.status'),'') NOT IN ('REVIEW_REQUIRED') AND coalesce(json_array_length(d.payload_json,'$.parsingArtifact.fidelityIssues'),0)=0 AND json_extract(d.payload_json,'$.parserFailure') IS NULL AND coalesce(json_array_length(d.payload_json,'$.visualSource.originalFindings'),0)=0 AND (json_extract(d.payload_json,'$.resourceCode')=p.resource_code OR json_extract(d.payload_json,'$.parsingArtifact.resourceCode')=p.resource_code OR EXISTS(SELECT 1 FROM json_each(d.payload_json,'$.parsingArtifact.imageResourceCodes') a WHERE a.value=p.resource_code) OR EXISTS(SELECT 1 FROM json_each(d.payload_json,'$.imageDescriptionArtifacts') a WHERE json_extract(a.value,'$.resourceCode')=p.resource_code AND EXISTS(SELECT 1 FROM json_each(d.payload_json,'$.parsingArtifact.imageResourceCodes') i WHERE i.value=json_extract(a.value,'$.imageResourceCode')))))",
            params![workspace,resource,user,agent],|r|r.get::<_,bool>(0)).unwrap_or(false)
    }
    fn is_protected(&self, workspace: &str, resource: &str) -> bool {
        self.connection.query_row("SELECT EXISTS(SELECT 1 FROM knowledge_base_resource_privacy WHERE workspace_code=?1 AND resource_code=?2)",params![workspace,resource],|r|r.get::<_,bool>(0)).unwrap_or(true)
    }
}
fn storage(error: impl std::fmt::Display) -> KnowledgeBaseError {
    KnowledgeBaseError::Storage(error.to_string())
}
fn encode<T: Serialize>(v: &T) -> Result<String, KnowledgeBaseError> {
    serde_json::to_string(v).map_err(storage)
}
fn decode<T: DeserializeOwned>(v: &str) -> Result<T, KnowledgeBaseError> {
    serde_json::from_str(v).map_err(storage)
}

impl SqliteKnowledgeBaseRepository {
    pub fn get(&self, id: &str) -> Result<KnowledgeBase, KnowledgeBaseError> {
        let id = id.to_owned();
        self.connection.call(
            move |connection| SqliteKnowledgeBaseRepositorySql { connection }.get(&id),
            || storage("database worker stopped"),
        )
    }
    pub fn insert(&self, kb: &KnowledgeBase, user: &str) -> Result<(), KnowledgeBaseError> {
        let kb = kb.to_owned();
        let user = user.to_owned();
        self.connection.call(
            move |connection| SqliteKnowledgeBaseRepositorySql { connection }.insert(&kb, &user),
            || storage("database worker stopped"),
        )
    }
    pub fn documents(&self, id: &str) -> Result<Vec<KnowledgeBaseDocument>, KnowledgeBaseError> {
        let id = id.to_owned();
        self.connection.call(
            move |connection| SqliteKnowledgeBaseRepositorySql { connection }.documents(&id),
            || storage("database worker stopped"),
        )
    }
    pub fn document(
        &self,
        kb: &str,
        id: &str,
    ) -> Result<KnowledgeBaseDocument, KnowledgeBaseError> {
        let kb = kb.to_owned();
        let id = id.to_owned();
        self.connection.call(
            move |connection| SqliteKnowledgeBaseRepositorySql { connection }.document(&kb, &id),
            || storage("database worker stopped"),
        )
    }
    pub fn insert_document(
        &self,
        kb: &str,
        doc: &KnowledgeBaseDocument,
    ) -> Result<(), KnowledgeBaseError> {
        let kb = kb.to_owned();
        let doc = doc.to_owned();
        self.connection.call(
            move |connection| {
                SqliteKnowledgeBaseRepositorySql { connection }.insert_document(&kb, &doc)
            },
            || storage("database worker stopped"),
        )
    }
    pub fn upload_request(
        &self,
        request_id: &str,
        identity: &str,
    ) -> Result<Option<(String, String)>, KnowledgeBaseError> {
        let request_id = request_id.to_owned();
        let identity = identity.to_owned();
        self.connection.call(
            move |connection| {
                SqliteKnowledgeBaseRepositorySql { connection }
                    .upload_request(&request_id, &identity)
            },
            || storage("database worker stopped"),
        )
    }
    pub fn remember_upload_request(
        &self,
        request_id: &str,
        identity: &str,
        kb: &str,
        doc: &str,
    ) -> Result<(), KnowledgeBaseError> {
        let request_id = request_id.to_owned();
        let identity = identity.to_owned();
        let kb = kb.to_owned();
        let doc = doc.to_owned();
        self.connection.call(
            move |connection| {
                SqliteKnowledgeBaseRepositorySql { connection }.remember_upload_request(
                    &request_id,
                    &identity,
                    &kb,
                    &doc,
                )
            },
            || storage("database worker stopped"),
        )
    }
    pub fn save_document(
        &self,
        kb: &str,
        doc: &KnowledgeBaseDocument,
        revision: u64,
    ) -> Result<(), KnowledgeBaseError> {
        let kb = kb.to_owned();
        let doc = doc.to_owned();
        self.connection.call(
            move |connection| {
                SqliteKnowledgeBaseRepositorySql { connection }.save_document(&kb, &doc, revision)
            },
            || storage("database worker stopped"),
        )
    }
    pub fn save_parsing_artifact(
        &self,
        kb: &str,
        doc: &KnowledgeBaseDocument,
    ) -> Result<(), KnowledgeBaseError> {
        let kb = kb.to_owned();
        let doc = doc.to_owned();
        self.connection.call(
            move |connection| {
                SqliteKnowledgeBaseRepositorySql { connection }.save_parsing_artifact(&kb, &doc)
            },
            || storage("database worker stopped"),
        )
    }
    pub fn save_fidelity(
        &self,
        kb: &str,
        doc: &KnowledgeBaseDocument,
    ) -> Result<(), KnowledgeBaseError> {
        let kb = kb.to_owned();
        let doc = doc.to_owned();
        self.connection.call(
            move |connection| {
                SqliteKnowledgeBaseRepositorySql { connection }.save_fidelity(&kb, &doc)
            },
            || storage("database worker stopped"),
        )
    }
    pub fn save_image_description_artifacts(
        &self,
        kb: &str,
        doc: &KnowledgeBaseDocument,
    ) -> Result<(), KnowledgeBaseError> {
        let kb = kb.to_owned();
        let doc = doc.to_owned();
        self.connection.call(
            move |connection| {
                SqliteKnowledgeBaseRepositorySql { connection }
                    .save_image_description_artifacts(&kb, &doc)
            },
            || storage("database worker stopped"),
        )
    }
    pub fn delete_document(&self, kb: &str, id: &str) -> Result<(), KnowledgeBaseError> {
        let kb = kb.to_owned();
        let id = id.to_owned();
        self.connection.call(
            move |connection| {
                SqliteKnowledgeBaseRepositorySql { connection }.delete_document(&kb, &id)
            },
            || storage("database worker stopped"),
        )
    }
    pub fn owns(&self, user: &str, id: &str) -> bool {
        let user = user.to_owned();
        let id = id.to_owned();
        self.connection
            .call(
                move |connection| {
                    Ok::<_, ()>(SqliteKnowledgeBaseRepositorySql { connection }.owns(&user, &id))
                },
                || (),
            )
            .unwrap_or(false)
    }
    pub fn grants(&self, id: &str) -> Result<Vec<String>, KnowledgeBaseError> {
        let id = id.to_owned();
        self.connection.call(
            move |connection| SqliteKnowledgeBaseRepositorySql { connection }.grants(&id),
            || storage("database worker stopped"),
        )
    }
    pub fn set_grants(&self, id: &str, agents: &[String]) -> Result<(), KnowledgeBaseError> {
        let id = id.to_owned();
        let agents = agents.to_owned();
        self.connection.call(
            move |connection| {
                SqliteKnowledgeBaseRepositorySql { connection }.set_grants(&id, &agents)
            },
            || storage("database worker stopped"),
        )
    }
    pub fn acl_revision(&self, id: &str) -> Result<u64, KnowledgeBaseError> {
        let id = id.to_owned();
        self.connection.call(
            move |connection| SqliteKnowledgeBaseRepositorySql { connection }.acl_revision(&id),
            || storage("database worker stopped"),
        )
    }
    pub fn mark_private(
        &self,
        user: &str,
        workspace: &str,
        resource: &str,
    ) -> Result<(), KnowledgeBaseError> {
        let user = user.to_owned();
        let workspace = workspace.to_owned();
        let resource = resource.to_owned();
        self.connection.call(
            move |connection| {
                SqliteKnowledgeBaseRepositorySql { connection }
                    .mark_private(&user, &workspace, &resource)
            },
            || storage("database worker stopped"),
        )
    }
}
impl crate::application::resource::ResourcePrivacyPolicy for SqliteKnowledgeBaseRepository {
    fn can_read(&self, user: &str, workspace: &str, resource: &str) -> bool {
        let user = user.to_owned();
        let workspace = workspace.to_owned();
        let resource = resource.to_owned();
        self.connection
            .call(
                move |connection| {
                    Ok::<_, ()>(
                        SqliteKnowledgeBaseRepositorySql { connection }
                            .can_read(&user, &workspace, &resource),
                    )
                },
                || (),
            )
            .unwrap_or(false)
    }
    fn can_read_agent(
        &self,
        user: &str,
        workspace: &str,
        resource: &str,
        agent: Option<&str>,
    ) -> bool {
        let user = user.to_owned();
        let workspace = workspace.to_owned();
        let resource = resource.to_owned();
        let agent = agent.map(ToOwned::to_owned);
        self.connection
            .call(
                move |connection| {
                    Ok::<_, ()>(
                        SqliteKnowledgeBaseRepositorySql { connection }.can_read_agent(
                            &user,
                            &workspace,
                            &resource,
                            agent.as_deref(),
                        ),
                    )
                },
                || (),
            )
            .unwrap_or(false)
    }
    fn can_read_agent_in_workspace(
        &self,
        user: &str,
        storage_scope: &str,
        resource: &str,
        agent: Option<&str>,
        run_workspace: Option<&str>,
    ) -> bool {
        let (user, scope, resource) = (
            user.to_owned(),
            storage_scope.to_owned(),
            resource.to_owned(),
        );
        let agent = agent.map(str::to_owned);
        let run_workspace = run_workspace.map(str::to_owned);
        self.connection.call(move |connection| {
            let repository = SqliteKnowledgeBaseRepositorySql { connection };
            if !repository.is_protected(&scope, &resource) { return Ok::<_, ()>(true); }
            let (Some(agent), Some(workspace)) = (agent.as_deref(), run_workspace.as_deref()) else { return Ok(false); };
            // Resource storage scope is independent of the Agent's active, assigned workspace.
            let assigned: bool = repository.connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM workspaces WHERE workspace_code=?1 AND status='active') AND (?2='main-agent' OR (EXISTS(SELECT 1 FROM platform_internal_agent_configs WHERE agent_code=?2) AND (NOT EXISTS(SELECT 1 FROM agent_workspace_grant_policies WHERE agent_code=?2) OR EXISTS(SELECT 1 FROM agent_workspace_grants WHERE agent_code=?2 AND workspace_code=?1))))",
                params![workspace,agent], |row| row.get(0)).unwrap_or(false);
            Ok(assigned && repository.can_read_agent(&user, &scope, &resource, Some(agent)))
        }, || ()).unwrap_or(false)
    }
    fn is_protected(&self, workspace: &str, resource: &str) -> bool {
        let workspace = workspace.to_owned();
        let resource = resource.to_owned();
        self.connection
            .call(
                move |connection| {
                    Ok::<_, ()>(
                        SqliteKnowledgeBaseRepositorySql { connection }
                            .is_protected(&workspace, &resource),
                    )
                },
                || (),
            )
            .unwrap_or(false)
    }
}

impl SqliteKnowledgeBaseRepository {
    pub fn ingestion_queue(
        &self,
    ) -> Result<
        super::knowledge_ingestion_queue::SqliteKnowledgeIngestionQueue,
        crate::application::knowledge_base::ingestion::IngestionQueueError,
    > {
        super::knowledge_ingestion_queue::SqliteKnowledgeIngestionQueue::from_connection(
            self.connection.clone(),
        )
    }
    pub fn connect(url: &str) -> Result<Self, KnowledgeBaseError> {
        let connection=crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase::open(url).map_err(storage)?;
        let repository = Self { connection };
        repository
            .ingestion_queue()
            .map_err(|e| storage(format!("{e:?}")))?;
        Ok(repository)
    }
}
