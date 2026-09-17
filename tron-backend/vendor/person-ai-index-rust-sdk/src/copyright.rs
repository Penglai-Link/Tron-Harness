//! 区块版权存证客户端
//!
//! 客户端应用通过本模块将资源哈希提交至至信链完成版权存证：
//! 客户端本地计算资源 SM3 哈希（[`sm3_hex`]），连同用户 ID、资源 ID、资源名称
//! 一并提交，服务端负责签名与上链，返回存证 ID / 交易 ID / 区块高度等上链结果。
//!
//! 存证成功后可发起证书生成任务并查询证书下载地址。
//!
//! # 提交示例
//!
//! ```no_run
//! use crab_serverapi::copyright::{CopyrightClient, CopyrightEvidenceSubmitRequest, sm3_hex};
//!
//! # async fn example() -> crab_serverapi::Result<()> {
//! # let context: crab_serverapi::base::AuthorizedClientContext = todo!();
//! let client = CopyrightClient::new("https://personai.chat/person-ai-index")?;
//! let resource = b"resource bytes";
//! let req = CopyrightEvidenceSubmitRequest::of(
//!     "user-id",
//!     "resource-id",
//!     sm3_hex(resource),
//!     "资源名称",
//! );
//! let evidence = client.submit(&context, &req).await?;
//! println!("evId={:?}, txId={:?}", evidence.ev_id, evidence.tx_id);
//! # Ok(())
//! # }
//! ```

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::base::{AuthorizedClientApi, AuthorizedClientContext, PageResult};
use crate::error::{Result, ServerApiError};

/// 附加信息（extend_info）最大长度，超过服务端返回 422
pub const MAX_EXTEND_INFO_LENGTH: usize = 1024;

/// 资源哈希十六进制字符串最大长度（SM3 为 64 字符，预留更长算法）
pub const MAX_RESOURCE_HASH_LENGTH: usize = 128;

/// 腾讯至信链厂商编码（首期唯一支持的厂商）
pub const VENDOR_ZXCHAIN: &str = "zxchain";

// ─── 数据模型 ──────────────────────────────────────────────

/// 可用厂商选项（仅公开信息）
#[derive(Debug, Clone, Deserialize)]
pub struct CopyrightVendorOption {
    #[serde(default)]
    pub vendor_code: Option<String>,
    #[serde(default)]
    pub vendor_name: Option<String>,
}

/// 提交版权存证请求
///
/// 服务端校验 `user_id` / `resource_id` / `resource_hash` / `resource_name` 必填，
/// `resource_hash` 必须为十六进制字符串（≤[`MAX_RESOURCE_HASH_LENGTH`]），
/// `extend_info` ≤[`MAX_EXTEND_INFO_LENGTH`]。
#[derive(Debug, Clone, Serialize)]
pub struct CopyrightEvidenceSubmitRequest {
    /// 资源归属用户 ID（必填）
    pub user_id: String,
    /// 资源唯一编号（必填，客户端系统内的资源标识）
    pub resource_id: String,
    /// 资源哈希十六进制字符串（必填，建议使用 [`sm3_hex`] 本地计算）
    pub resource_hash: String,
    /// 资源名称（必填，用于展示）
    pub resource_name: String,
    /// 厂商编码（可选，未传时服务端取唯一启用厂商；首期即 [`VENDOR_ZXCHAIN`]）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor_code: Option<String>,
    /// 附加信息（可选，上链时的 extendInfo，≤[`MAX_EXTEND_INFO_LENGTH`]）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extend_info: Option<String>,
    /// 业务类型（可选，0=其他，1=文件存证，2=作品确权，默认 0）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub biz_type: Option<i32>,
}

impl CopyrightEvidenceSubmitRequest {
    /// 构建必填字段的存证提交请求
    pub fn of(
        user_id: impl Into<String>,
        resource_id: impl Into<String>,
        resource_hash: impl Into<String>,
        resource_name: impl Into<String>,
    ) -> Self {
        Self {
            user_id: user_id.into(),
            resource_id: resource_id.into(),
            resource_hash: resource_hash.into(),
            resource_name: resource_name.into(),
            vendor_code: None,
            extend_info: None,
            biz_type: None,
        }
    }

    /// 指定厂商编码
    pub fn with_vendor_code(mut self, vendor_code: impl Into<String>) -> Self {
        self.vendor_code = Some(vendor_code.into());
        self
    }

    /// 附带附加信息（上链时的 extendInfo）
    pub fn with_extend_info(mut self, extend_info: impl Into<String>) -> Self {
        self.extend_info = Some(extend_info.into());
        self
    }

    /// 指定业务类型（0=其他，1=文件存证，2=作品确权）
    pub fn with_biz_type(mut self, biz_type: i32) -> Self {
        self.biz_type = Some(biz_type);
        self
    }
}

/// 存证记录
///
/// `ev_id` 非空表示已上链成功；`cert_status` 标识证书进度
/// （0=未生成，1=生成中，2=已生成，3=生成失败），`cert_url` 为证书下载地址。
#[derive(Debug, Clone, Deserialize)]
pub struct CopyrightEvidence {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub vendor_code: Option<String>,
    #[serde(default)]
    pub resource_id: Option<String>,
    #[serde(default)]
    pub resource_name: Option<String>,
    #[serde(default)]
    pub resource_hash: Option<String>,
    /// 哈希算法（SM3/SHA256 等）
    #[serde(default)]
    pub hash_algo: Option<String>,
    /// 业务类型（0=其他，1=文件存证，2=作品确权）
    #[serde(default)]
    pub biz_type: Option<i32>,
    #[serde(default)]
    pub extend_info: Option<String>,
    /// 厂商存证 ID（上链成功后返回；非空即上链成功）
    #[serde(default)]
    pub ev_id: Option<String>,
    /// 链上交易 ID
    #[serde(default)]
    pub tx_id: Option<String>,
    /// 区块高度
    #[serde(default)]
    pub block_height: Option<String>,
    /// 上链时间
    #[serde(default)]
    pub tx_time: Option<String>,
    /// 证书状态（0=未生成，1=生成中，2=已生成，3=生成失败）
    #[serde(default)]
    pub cert_status: Option<i32>,
    /// 证书下载地址（已生成后返回）
    #[serde(default)]
    pub cert_url: Option<String>,
    /// 提交次数（含重试）
    #[serde(default)]
    pub submit_count: Option<i32>,
    /// 最近一次提交失败的错误信息（成功后为空）
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

impl CopyrightEvidence {
    /// 是否已上链成功
    pub fn is_on_chain(&self) -> bool {
        self.ev_id.as_deref().map(|s| !s.is_empty()).unwrap_or(false)
    }

    /// 证书是否已生成
    pub fn is_cert_ready(&self) -> bool {
        self.cert_status == Some(2)
    }
}

/// 存证记录分页查询参数
#[derive(Debug, Clone, Serialize, Default)]
pub struct CopyrightEvidenceQuery {
    /// 厂商编码过滤（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor_code: Option<String>,
    pub page: Option<i32>,
    #[serde(rename = "page_size")]
    pub page_size: Option<i32>,
}

impl CopyrightEvidenceQuery {
    pub fn of(page: i32, page_size: i32) -> Self {
        Self {
            vendor_code: None,
            page: Some(page),
            page_size: Some(page_size),
        }
    }

    /// 按厂商编码过滤
    pub fn with_vendor_code(mut self, vendor_code: impl Into<String>) -> Self {
        self.vendor_code = Some(vendor_code.into());
        self
    }
}

// ─── 哈希工具 ──────────────────────────────────────────────

/// 计算资源内容的 SM3 哈希（小写十六进制，64 字符）
///
/// 与服务端 `hash_algo=SM3` 约定一致，提交存证前用本函数计算资源哈希。
pub fn sm3_hex(data: &[u8]) -> String {
    use sm3::{Digest, Sm3};
    let mut hasher = Sm3::new();
    hasher.update(data);
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for b in digest {
        hex.push_str(&format!("{b:02x}"));
    }
    hex
}

// ─── 客户端 ────────────────────────────────────────────────

/// 区块版权存证客户端
///
/// 需要 Bearer token 认证
pub struct CopyrightClient {
    api: AuthorizedClientApi,
}

impl CopyrightClient {
    pub fn new(server_base_url: impl Into<String>) -> Result<Self> {
        Ok(Self {
            api: AuthorizedClientApi::new(server_base_url)?,
        })
    }

    /// 查询可用厂商列表（仅已启用厂商，仅公开信息）
    ///
    /// `GET /api/v1/copyright/vendors`
    pub async fn list_vendors(
        &self,
        context: &AuthorizedClientContext,
    ) -> Result<Vec<CopyrightVendorOption>> {
        let request = self
            .api
            .new_authorized_request(Method::GET, "/copyright/vendors", context)?;
        self.api
            .execute_for_list(request, Some(context), "查询版权厂商列表")
            .await
    }

    /// 提交版权存证（服务端签名并上链）
    ///
    /// `POST /api/v1/copyright/evidence`
    ///
    /// 上链失败时服务端返回 502，错误信息同时记录在存证记录的 `last_error` 字段；
    /// `resource_hash` 必须为十六进制字符串，`extend_info` 超过
    /// [`MAX_EXTEND_INFO_LENGTH`] 时服务端返回 422。
    pub async fn submit(
        &self,
        context: &AuthorizedClientContext,
        req: &CopyrightEvidenceSubmitRequest,
    ) -> Result<CopyrightEvidence> {
        if req.resource_hash.len() > MAX_RESOURCE_HASH_LENGTH {
            return Err(ServerApiError::Validation(format!(
                "resource_hash 长度 {} 超过上限 {}",
                req.resource_hash.len(),
                MAX_RESOURCE_HASH_LENGTH
            )));
        }
        if let Some(info) = &req.extend_info {
            if info.len() > MAX_EXTEND_INFO_LENGTH {
                return Err(ServerApiError::Validation(format!(
                    "extend_info 长度 {} 超过上限 {}",
                    info.len(),
                    MAX_EXTEND_INFO_LENGTH
                )));
            }
        }
        let request = self
            .api
            .new_authorized_request(Method::POST, "/copyright/evidence", context)?
            .json(req);
        self.api
            .execute_for_object(request, Some(context), "提交版权存证")
            .await
    }

    /// 分页查询本人存证记录
    ///
    /// `GET /api/v1/copyright/evidence?vendor_code=...&page=...&page_size=...`
    pub async fn list(
        &self,
        context: &AuthorizedClientContext,
        query: &CopyrightEvidenceQuery,
    ) -> Result<PageResult<CopyrightEvidence>> {
        let request = self
            .api
            .new_authorized_request(Method::GET, "/copyright/evidence", context)?;
        let mut request = request.query(&[("page", query.page.unwrap_or(1).to_string())]);
        request = request.query(&[(
            "page_size",
            query.page_size.unwrap_or(20).to_string(),
        )]);
        if let Some(ref vendor_code) = query.vendor_code {
            request = request.query(&[("vendor_code", vendor_code.as_str())]);
        }
        self.api
            .execute_for_page(request, Some(context), "查询存证记录列表")
            .await
    }

    /// 获取单条存证记录详情
    ///
    /// `GET /api/v1/copyright/evidence/{id}`
    pub async fn detail(
        &self,
        context: &AuthorizedClientContext,
        evidence_id: &str,
    ) -> Result<CopyrightEvidence> {
        let path = format!("/copyright/evidence/{evidence_id}");
        let request = self
            .api
            .new_authorized_request(Method::GET, &path, context)?;
        self.api
            .execute_for_object(request, Some(context), "获取存证记录详情")
            .await
    }

    /// 发起存证证书生成任务（需存证已上链）
    ///
    /// `POST /api/v1/copyright/evidence/{id}/cert`
    ///
    /// 证书生成是异步任务，返回的记录中 `cert_status=1`（生成中）；
    /// 稍后调用 [`Self::query_cert`] 轮询结果。
    pub async fn create_cert(
        &self,
        context: &AuthorizedClientContext,
        evidence_id: &str,
    ) -> Result<CopyrightEvidence> {
        let path = format!("/copyright/evidence/{evidence_id}/cert");
        let request = self
            .api
            .new_authorized_request(Method::POST, &path, context)?
            .json(&serde_json::json!({}));
        self.api
            .execute_for_object(request, Some(context), "发起证书生成")
            .await
    }

    /// 查询证书状态并获取下载地址（需存证已上链）
    ///
    /// `GET /api/v1/copyright/evidence/{id}/cert`
    ///
    /// 证书生成完成后返回记录中 `cert_status=2` 且 `cert_url` 非空。
    pub async fn query_cert(
        &self,
        context: &AuthorizedClientContext,
        evidence_id: &str,
    ) -> Result<CopyrightEvidence> {
        let path = format!("/copyright/evidence/{evidence_id}/cert");
        let request = self
            .api
            .new_authorized_request(Method::GET, &path, context)?;
        self.api
            .execute_for_object(request, Some(context), "查询证书状态")
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sm3_hex() {
        // SM3("abc") 标准测试向量
        assert_eq!(
            sm3_hex(b"abc"),
            "66c7f0f462eeedd9d1f2d46bdc10e4e24167c4875cf2f7a2297da02b8f4ba8e0"
        );
        // 空串
        assert_eq!(
            sm3_hex(b""),
            "1ab21d8355cfa17f8e61194831e81a8f22bec8c728fefb747ed035eb5082aa2b"
        );
    }

    #[test]
    fn test_submit_request_builder() {
        let req = CopyrightEvidenceSubmitRequest::of("u1", "r1", "aabb", "文件.txt")
            .with_vendor_code(VENDOR_ZXCHAIN)
            .with_extend_info("测试附加信息")
            .with_biz_type(1);
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["user_id"], "u1");
        assert_eq!(json["resource_id"], "r1");
        assert_eq!(json["resource_hash"], "aabb");
        assert_eq!(json["resource_name"], "文件.txt");
        assert_eq!(json["vendor_code"], VENDOR_ZXCHAIN);
        assert_eq!(json["extend_info"], "测试附加信息");
        assert_eq!(json["biz_type"], 1);
    }

    #[test]
    fn test_evidence_state_helpers() {
        let mut evidence = CopyrightEvidence {
            id: None,
            user_id: None,
            vendor_code: None,
            resource_id: None,
            resource_name: None,
            resource_hash: None,
            hash_algo: None,
            biz_type: None,
            extend_info: None,
            ev_id: Some("ev-1".into()),
            tx_id: None,
            block_height: None,
            tx_time: None,
            cert_status: Some(2),
            cert_url: Some("https://example.com/cert.pdf".into()),
            submit_count: None,
            last_error: None,
            created_at: None,
            updated_at: None,
        };
        assert!(evidence.is_on_chain());
        assert!(evidence.is_cert_ready());
        evidence.ev_id = None;
        assert!(!evidence.is_on_chain());
    }
}
