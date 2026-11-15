use serde::{Deserialize, Serialize};

/// Network access settings used by the mobile client.
///
/// The token is the only gate for non-loopback requests. When remote access is
/// enabled the listener binds to all interfaces (unless an explicit non-loopback
/// address is configured), so the token must never be exposed by a remote
/// endpoint.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteAccessConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub token: String,
    /// Public URL used in pairing payloads, for example an FRP or reverse-proxy
    /// address. Empty means "detect the LAN address".
    #[serde(default)]
    pub public_base_url: String,
    /// Display name shown in the mobile client. Empty means "derive from the OS".
    #[serde(default)]
    pub machine_name: String,
    /// Dedicated listener port for remote clients. 0 means "main port + 1".
    /// Keeping remote access on its own listener means an FRP tunnel that
    /// connects from localhost still has to present the pairing token.
    #[serde(default)]
    pub port: u16,
    /// Remote listener address. Empty means "all interfaces".
    #[serde(default)]
    pub bind_address: String,
    /// Product identity published to mobile clients. Tron keeps the PersonAI
    /// SDK product code `tron`; other products reuse the same capability pack
    /// with their own code.
    #[serde(default)]
    pub product_code: String,
    #[serde(default)]
    pub product_name: String,
    /// Stable node identity used by the mobile product -> node -> session flow.
    #[serde(default)]
    pub node_id: String,
    /// Plain HTTP is rejected unless the operator explicitly opts in for a
    /// trusted local network. Production must terminate TLS at FRP or a
    /// reverse proxy and forward `X-Forwarded-Proto: https`.
    #[serde(default)]
    pub allow_insecure_http: bool,
    /// Lifetime of the short-lived access JWT handed to mobile clients.
    #[serde(default)]
    pub jwt_ttl_seconds: u64,
}

impl RemoteAccessConfig {
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn has_token(&self) -> bool {
        !self.token.trim().is_empty()
    }

    pub fn listen_port(&self, main_port: u16) -> u16 {
        if self.port == 0 {
            main_port.saturating_add(1)
        } else {
            self.port
        }
    }

    pub fn listen_address(&self) -> &str {
        match self.bind_address.trim() {
            "" => "0.0.0.0",
            value => value,
        }
    }

    pub fn product_code(&self) -> &str {
        match self.product_code.trim() {
            "" => "tron",
            value => value,
        }
    }

    pub fn product_name(&self) -> &str {
        match self.product_name.trim() {
            "" => "Tron",
            value => value,
        }
    }

    pub fn jwt_ttl_seconds(&self) -> u64 {
        match self.jwt_ttl_seconds {
            0 => 43_200,
            value => value.clamp(300, 30 * 24 * 60 * 60),
        }
    }
}
