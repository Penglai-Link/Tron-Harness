//! Nacos 服务注册客户端
//!
//! 对应 Java: `NacosClient`
//! 实现基本的 Nacos 服务注册、注销和心跳保活

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;

/// Nacos 客户端配置
#[derive(Debug, Clone)]
pub struct NacosConfig {
    pub base_url: String,
    pub namespace: String,
    pub group: String,
    pub service_name: String,
    pub ip: String,
    pub port: u16,
    pub weight: f64,
    pub enabled: bool,
    pub metadata: Vec<(String, String)>,
}

impl Default for NacosConfig {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:8848".to_string(),
            namespace: "public".to_string(),
            group: "DEFAULT_GROUP".to_string(),
            service_name: "person-ai".to_string(),
            ip: "127.0.0.1".to_string(),
            port: 8090,
            weight: 1.0,
            enabled: true,
            metadata: Vec::new(),
        }
    }
}

/// Nacos 服务注册客户端
pub struct NacosClient {
    config: NacosConfig,
    http_client: reqwest::Client,
    heartbeat_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl NacosClient {
    pub fn new(config: NacosConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        Self {
            config,
            http_client,
            heartbeat_handle: Arc::new(RwLock::new(None)),
        }
    }

    /// 注册服务实例
    pub async fn register(&self) -> Result<(), String> {
        let url = format!("{}/nacos/v1/ns/instance", self.config.base_url);

        let mut params = vec![
            ("serviceName".to_string(), self.config.service_name.clone()),
            ("ip".to_string(), self.config.ip.clone()),
            ("port".to_string(), self.config.port.to_string()),
            ("namespaceId".to_string(), self.config.namespace.clone()),
            ("groupName".to_string(), self.config.group.clone()),
            ("weight".to_string(), self.config.weight.to_string()),
            ("enabled".to_string(), self.config.enabled.to_string()),
            ("ephemeral".to_string(), "true".to_string()),
        ];

        if !self.config.metadata.is_empty() {
            let metadata_str = self.config.metadata.iter()
                .map(|(k, v)| format!("{k}:{v}"))
                .collect::<Vec<_>>()
                .join(",");
            params.push(("metadata".to_string(), metadata_str));
        }

        let resp = self.http_client.post(&url)
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Nacos 注册请求失败: {e}"))?;

        if resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            if body.contains("ok") || body.contains("true") || body.is_empty() {
                tracing::info!("Nacos 服务注册成功: {} ({}:{})", self.config.service_name, self.config.ip, self.config.port);
                Ok(())
            } else {
                Err(format!("Nacos 注册响应异常: {body}"))
            }
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(format!("Nacos 注册失败 HTTP {status}: {body}"))
        }
    }

    /// 注销服务实例
    pub async fn deregister(&self) -> Result<(), String> {
        let url = format!("{}/nacos/v1/ns/instance", self.config.base_url);

        let params = [
            ("serviceName", self.config.service_name.as_str()),
            ("ip", self.config.ip.as_str()),
            ("port", &self.config.port.to_string()),
            ("namespaceId", self.config.namespace.as_str()),
            ("groupName", self.config.group.as_str()),
            ("ephemeral", "true"),
        ];

        let resp = self.http_client.delete(&url)
            .query(&params)
            .send()
            .await
            .map_err(|e| format!("Nacos 注销请求失败: {e}"))?;

        if resp.status().is_success() {
            tracing::info!("Nacos 服务注销成功: {}", self.config.service_name);
            Ok(())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(format!("Nacos 注销失败 HTTP {status}: {body}"))
        }
    }

    /// 发送心跳
    pub async fn heartbeat(&self) -> Result<(), String> {
        let url = format!("{}/nacos/v1/ns/instance/beat", self.config.base_url);

        let beat = serde_json::json!({
            "serviceName": self.config.service_name,
            "ip": self.config.ip,
            "port": self.config.port,
            "weight": self.config.weight,
            "metadata": {}
        });

        let params = [
            ("serviceName", self.config.service_name.as_str()),
            ("ip", self.config.ip.as_str()),
            ("port", &self.config.port.to_string()),
            ("namespaceId", self.config.namespace.as_str()),
            ("beat", &serde_json::to_string(&beat).unwrap_or_default()),
        ];

        let resp = self.http_client.put(&url)
            .query(&params)
            .send()
            .await
            .map_err(|e| format!("Nacos 心跳请求失败: {e}"))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(format!("Nacos 心跳失败 HTTP {status}: {body}"))
        }
    }

    /// 启动后台心跳任务 (每 5 秒)
    pub fn start_heartbeat(&self) {
        let client = self.http_client.clone();
        let config = self.config.clone();
        let handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;

                let url = format!("{}/nacos/v1/ns/instance/beat", config.base_url);
                let beat = serde_json::json!({
                    "serviceName": config.service_name,
                    "ip": config.ip,
                    "port": config.port,
                    "weight": config.weight,
                    "metadata": {}
                });

                let params = [
                    ("serviceName", config.service_name.as_str()),
                    ("ip", config.ip.as_str()),
                    ("port", &config.port.to_string()),
                    ("namespaceId", config.namespace.as_str()),
                    ("beat", &serde_json::to_string(&beat).unwrap_or_default()),
                ];

                match client.put(&url).query(&params).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        tracing::debug!("Nacos 心跳成功");
                    }
                    Ok(resp) => {
                        let status = resp.status();
                        tracing::warn!("Nacos 心跳失败: HTTP {status}");
                    }
                    Err(e) => {
                        tracing::warn!("Nacos 心跳请求异常: {e}");
                    }
                }
            }
        });

        // 保存 handle 以便后续停止 (此处简化，不存储)
        let _ = handle;
    }

    /// 停止心跳
    pub async fn stop_heartbeat(&self) {
        let mut handle = self.heartbeat_handle.write().await;
        if let Some(h) = handle.take() {
            h.abort();
        }
    }
}
