use super::*;
use crate::{application::knowledge_base::ingestion::{ParseEndpointCapacity, IngestionWaitReason, WaitReasonCode}, config::MineruNode};

impl MineruKnowledgeDocumentParser {
    pub(super) async fn upload_source(
        &self,
        filename: &str,
        bytes: &[u8],
        extension: &str,
    ) -> Result<String, KnowledgeBaseError> {
        let mime = match extension {
            "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            _ => "application/pdf",
        };
        let upload = self.post("/v1/uploads", json!({"filename":filename,"bytes":bytes.len(),"mime_type":mime,"purpose":"parse","sha256sum":format!("{:x}",Sha256::digest(bytes))})).await?;
        let file = if upload["status"] == "completed" {
            upload["file"].clone()
        } else {
            let target = self
                .url("/")?
                .join(
                    upload["upload_url"]
                        .as_str()
                        .ok_or_else(|| invalid("MinerU未返回上传地址"))?,
                )
                .map_err(invalid)?;
            if target.origin() != self.url("/")?.origin()
                || !target.username().is_empty()
                || target.password().is_some()
            {
                return Err(invalid("MinerU返回跨域上传地址，拒绝发送资料"));
            }
            self.bytes(self.client.put(target).body(bytes.to_vec()))
                .await?;
            let id = Self::id(&upload, "id")?;
            self.post(&format!("/v1/uploads/{id}/complete"), json!({}))
                .await?["file"]
                .clone()
        };
        Ok(Self::id(&file, "id")?.into())
    }
    pub(super) fn nodes(&self) -> Result<Vec<MineruNode>, KnowledgeBaseError> {
        if !self.config.enabled {
            return Err(invalid("MinerU解析已禁用"));
        }
        let nodes = if self.config.nodes.is_empty() {
            vec![MineruNode {
                id: "default".into(),
                base_url: self.config.base_url.clone(),
                enabled: true,
                allow_remote: true,
                capacity: self.config.ingestion.parse_capacity,
                weight: 1,
                resource_group: "default-parser".into(),
                resource_group_capacity: self.config.ingestion.parse_capacity,
            }]
        } else {
            self.config
                .nodes
                .iter()
                .filter(|n| n.enabled)
                .cloned()
                .collect()
        };
        let mut ids = BTreeSet::new();
        for node in &nodes {
            if node.id.is_empty()
                || node.id == "local-markdown"
                || !node
                    .id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
                || !ids.insert(node.id.clone())
                || node.capacity == 0
                || node.weight == 0
                || node.resource_group_capacity == 0
            {
                return Err(invalid("MinerU节点身份或容量无效"));
            }
            let url = Url::parse(&node.base_url).map_err(invalid)?;
            if !matches!(url.scheme(), "http" | "https")
                || !url.username().is_empty()
                || url.password().is_some()
                || url.query().is_some()
                || url.fragment().is_some()
            {
                return Err(invalid("MinerU节点地址无效"));
            }
            let local = matches!(
                url.host_str(),
                Some("localhost" | "127.0.0.1" | "::1" | "[::1]")
            );
            if !local && !node.allow_remote {
                return Err(invalid("MinerU远程节点需要显式allowRemote授权"));
            }
        }
        Ok(nodes)
    }
    pub(super) fn on_endpoint(&self, id: &str) -> Result<Self, KnowledgeBaseError> {
        let node = self
            .nodes()?
            .into_iter()
            .find(|n| n.id == id)
            .ok_or_else(|| invalid("原MinerU节点已移除或禁用，不能切换在途任务"))?;
        let mut config = self.config.clone();
        config.base_url = node.base_url;
        config.nodes.clear();
        Ok(Self {
            config,
            client: self.client.clone(),
        })
    }
    async fn node_capabilities(
        &self,
        node: &MineruNode,
    ) -> Result<ParsingCapabilities, IngestionWaitReason> {
        let parser = self.on_endpoint(&node.id).map_err(|_| IngestionWaitReason::new(WaitReasonCode::InvalidNodeConfig))?;
        let health = parser.get("/v1/health").await.map_err(|_| IngestionWaitReason::new(WaitReasonCode::NodeUnreachable))?;
        if health["status"] != "ok" {
            return Err(IngestionWaitReason::new(WaitReasonCode::Unhealthy));
        }
        if health["version"] != self.config.expected_parser_version {
            return Err(IngestionWaitReason::with_params(WaitReasonCode::VersionMismatch, json!({"expected":self.config.expected_parser_version,"actual":health["version"].as_str()})));
        }
        let tiers = parser.get("/v1/tiers").await.map_err(|_| IngestionWaitReason::new(WaitReasonCode::NodeUnreachable))?;
        let available = tiers["data"]
            .as_array()
            .ok_or_else(|| IngestionWaitReason::new(WaitReasonCode::NoTiers))?
            .iter()
            .filter_map(|t| serde_json::from_value(t["id"].clone()).ok())
            .collect();
        Ok(ParsingCapabilities {
            default_tier: self.config.default_tier,
            available_tiers: available,
            parser_version: self.config.expected_parser_version.clone(),
            office_tier: ParsingTier::Flash,
        })
    }
    pub(super) async fn routing_capacities(
        &self,
    ) -> Result<Vec<ParseEndpointCapacity>, KnowledgeBaseError> {
        self.nodes()?;
        Ok(self.routing_status_with_reasons().await.endpoints)
    }
    pub(super) async fn routing_status_with_reasons(&self) -> crate::application::knowledge_base::parsing::ParserRoutingStatus {
        use crate::application::knowledge_base::parsing::ParserRoutingStatus;
        let mut status = ParserRoutingStatus::default();
        if !self.config.enabled {
            status.unavailable.push((None, IngestionWaitReason::new(WaitReasonCode::MineruDisabled)));
            return status;
        }
        let nodes = match self.nodes() {
            Ok(nodes) => nodes,
            Err(_) => { status.unavailable.push((None, IngestionWaitReason::new(WaitReasonCode::InvalidNodeConfig))); return status; }
        };
        if nodes.is_empty() { status.unavailable.push((None, IngestionWaitReason::new(WaitReasonCode::NoEnabledNodes))); }
        for node in nodes {
            match tokio::time::timeout(Duration::from_secs(2), self.node_capabilities(&node)).await {
                Ok(Ok(capabilities)) if !capabilities.available_tiers.is_empty() => status.endpoints.push(ParseEndpointCapacity {
                    supported_tiers: capabilities
                        .available_tiers
                        .iter()
                        .map(|t| t.as_str().into())
                        .collect(),
                    endpoint_id: node.id.clone(),
                    capacity: node.capacity,
                    resource_group: if node.resource_group.is_empty() {
                        node.id.clone()
                    } else {
                        node.resource_group.clone()
                    },
                    resource_group_capacity: node.resource_group_capacity,
                    weight: node.weight,
                }),
                Ok(Ok(_)) => status.unavailable.push((Some(node.id), IngestionWaitReason::new(WaitReasonCode::NoTiers))),
                Ok(Err(reason)) => status.unavailable.push((Some(node.id), reason)),
                Err(_) => status.unavailable.push((Some(node.id), IngestionWaitReason::new(WaitReasonCode::HealthTimeout))),
            }
        }
        status
    }
    pub(super) async fn pool_capabilities(
        &self,
    ) -> Result<ParsingCapabilities, KnowledgeBaseError> {
        let mut tiers = BTreeSet::new();
        for endpoint in self.routing_capacities().await? {
            tiers.extend(endpoint.supported_tiers);
        }
        if !tiers.contains(self.config.default_tier.as_str()) {
            return Err(invalid("没有健康节点支持平台默认解析质量"));
        }
        Ok(ParsingCapabilities {
            default_tier: self.config.default_tier,
            available_tiers: tiers
                .into_iter()
                .filter_map(|t| serde_json::from_value(json!(t)).ok())
                .collect(),
            parser_version: self.config.expected_parser_version.clone(),
            office_tier: ParsingTier::Flash,
        })
    }
    /// Never resend an ambiguous job POST. Reconcile on the recorded node, across pages.
    pub(super) async fn reconcile_job(
        &self,
        file: &str,
        tier: ParsingTier,
    ) -> Result<Value, KnowledgeBaseError> {
        let mut after: Option<String> = None;
        let mut seen = BTreeSet::new();
        for _ in 0..100 {
            let mut url = self.url("/v1/parse/jobs")?;
            url.query_pairs_mut().append_pair("limit", "100");
            if let Some(id) = &after {
                url.query_pairs_mut().append_pair("after", id);
            }
            let list: Value = serde_json::from_slice(&self.bytes(self.client.get(url)).await?)
                .map_err(invalid)?;
            for item in list["data"]
                .as_array()
                .ok_or_else(|| invalid("MinerU任务对账响应无效"))?
            {
                let id = Self::id(item, "job_id")?;
                let job = self.get(&format!("/v1/parse/jobs/{id}")).await?;
                if job["tier"] == tier.as_str()
                    && job["files"]
                        .as_array()
                        .is_some_and(|files| files.len() == 1 && files[0]["file_id"] == file)
                {
                    return Ok(job);
                }
            }
            if list["has_more"] != true {
                break;
            }
            let last = Self::id(&list, "last_id")?.to_owned();
            if !seen.insert(last.clone()) {
                break;
            }
            after = Some(last);
        }
        Err(KnowledgeBaseError::Storage(
            "MinerU提交响应丢失，原节点尚未对账到任务；不会重复提交".into(),
        ))
    }
}
