//! # crab-serverapi
//!
//! Person-Ai 服务端 API 客户端 SDK，对应 Java 模块 `crab-ai-engine-serverapi`。
//!
//! 仅出站调用，不启动任何 HTTP 服务。功能包括：
//!
//! - **认证** (`auth`): 登录会话、Token 刷新、心跳、账户信息
//! - **网关** (`gateway`): 对话补全、模型列表查询、检索、向量嵌入（请求体混合加密）
//! - **技能仓库** (`skill`): 技能列表、详情、下载
//! - **多智能体仓库** (`repository`): 列表、上传、下载
//! - **角色仓库** (`agent_role`): 角色列表、详情
//! - **智能体页面发布** (`agent_site`): 发布静态页面、查询项目列表、删除项目
//! - **皮肤市场** (`skin`): 皮肤列表、详情、下载
//! - **音频转写** (`audio`): HTTP 文件转写、WebSocket 实时音频转写
//! - **消息中继** (`relay`): WebSocket 中继客户端（手机端远程对话）
//! - **版本更新** (`version`): 最新版本查询、安装包下载地址、下载计数上报
//! - **用户反馈** (`feedback`): 提交用户反馈（文本 + base64 图片）
//! - **文件管理** (`file`): 获取对象存储上传凭证、客户端直传、保存文件信息转正、详情查询、删除
//! - **区块版权** (`copyright`): 可用厂商查询、版权存证提交（SM3 哈希）、存证记录查询、证书生成与查询

pub mod agent_role;
pub mod agent_site;
pub mod audio;
pub mod auth;
pub mod base;
pub mod copyright;
pub mod crypto;
pub mod error;
pub mod feedback;
pub mod file;
pub mod gateway;
pub mod multimedia;
pub mod nacos;
pub mod relay;
pub mod repository;
pub mod skill;
pub mod skin;
pub mod version;

// 重新导出常用类型
pub use base::{ApiResponseEnvelope, AuthorizedClientApi, AuthorizedClientContext, PageResult};
pub use crypto::{ClientEncryptedEnvelope, ClientEnvelopeEncryptor};
pub use error::{Result, ServerApiError};
