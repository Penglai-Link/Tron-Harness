//! 混合加密信封
//!
//! 对应 Java: `client/crypto/` 包
//!
//! 加密协议:
//! 1. 生成 256 位 AES 密钥 + 12 字节 IV
//! 2. AES-256-GCM 加密业务 JSON -> ciphertext
//! 3. RSA-OAEP-SHA256 加密 AES 密钥 -> encrypted_key
//! 4. 三者 Base64 编码组装为 `ClientEncryptedEnvelope`

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use base64::Engine;
use rand::RngCore;
use rsa::pkcs8::DecodePublicKey;
use rsa::{Oaep, RsaPublicKey};
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::Sha256;

use crate::base::AuthorizedClientContext;
use crate::error::{Result, ServerApiError};

/// AES 密钥长度 (256 位 = 32 字节)
const AES_KEY_SIZE: usize = 32;

/// GCM IV 长度 (12 字节)
const GCM_IV_LENGTH: usize = 12;

/// 加密信封
///
/// 对应 Java: `ClientEncryptedEnvelope`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientEncryptedEnvelope {
    /// 信封类型标记，固定为 "encrypted"
    /// （服务端 WebSocket 处理器要求带上该标记；HTTP 网关过滤器会忽略多余字段，不受影响）
    #[serde(rename = "type", default = "default_envelope_type")]
    pub envelope_type: String,
    /// RSA 加密后的 AES 密钥 (Base64)
    #[serde(rename = "encrypted_key")]
    pub encrypted_key: String,
    /// AES-GCM 初始化向量 (Base64, 12 字节)
    pub iv: String,
    /// AES-GCM 加密后的业务 JSON (Base64)
    pub ciphertext: String,
}

fn default_envelope_type() -> String {
    "encrypted".to_string()
}

/// 信封加密器
///
/// 对应 Java: `ClientEnvelopeEncryptor`
///
/// 使用 RSA 公钥加密 AES 密钥，AES-256-GCM 加密业务数据，
/// 实现混合加密。
pub struct ClientEnvelopeEncryptor;

impl ClientEnvelopeEncryptor {
    /// 使用授权上下文中的公钥加密 JSON 对象
    pub fn encrypt_with_context(
        context: &AuthorizedClientContext,
        raw_json: &str,
    ) -> Result<ClientEncryptedEnvelope> {
        context.validate_gateway_access()?;
        let public_key = context.public_key.as_ref().unwrap();
        Self::encrypt(public_key, raw_json)
    }

    /// 使用指定公钥加密原始 JSON 字符串
    ///
    /// `public_key_pem` 应为 X.509 SubjectPublicKeyInfo PEM 格式:
    /// ```text
    /// -----BEGIN PUBLIC KEY-----
    /// ...
    /// -----END PUBLIC KEY-----
    /// ```
    pub fn encrypt(public_key_pem: &str, raw_json: &str) -> Result<ClientEncryptedEnvelope> {
        // 1. 解析 RSA 公钥
        let public_key =
            RsaPublicKey::from_public_key_pem(public_key_pem).map_err(|e| {
                ServerApiError::crypto(format!("解析 RSA 公钥失败: {e}"))
            })?;

        // 2. 生成随机 AES-256 密钥
        let mut aes_key = vec![0u8; AES_KEY_SIZE];
        rand::thread_rng().fill_bytes(&mut aes_key);

        // 3. 生成随机 12 字节 IV
        let mut iv = vec![0u8; GCM_IV_LENGTH];
        rand::thread_rng().fill_bytes(&mut iv);

        // 4. AES-256-GCM 加密业务 JSON
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&aes_key));
        let nonce = Nonce::from_slice(&iv);
        let ciphertext = cipher
            .encrypt(nonce, raw_json.as_bytes())
            .map_err(|e| ServerApiError::crypto(format!("AES-GCM 加密失败: {e}")))?;

        // 5. RSA-OAEP-SHA256 加密 AES 密钥
        let mut rng = rand::thread_rng();
        let padding = Oaep::new_with_mgf_hash::<Sha256, Sha1>();
        let encrypted_key = public_key
            .encrypt(&mut rng, padding, &aes_key)
            .map_err(|e| ServerApiError::crypto(format!("RSA 加密 AES 密钥失败: {e}")))?;

        // 6. Base64 编码
        let b64 = base64::engine::general_purpose::STANDARD;
        Ok(ClientEncryptedEnvelope {
            envelope_type: default_envelope_type(),
            encrypted_key: b64.encode(&encrypted_key),
            iv: b64.encode(&iv),
            ciphertext: b64.encode(&ciphertext),
        })
    }
}

/// 服务端解密工具
///
/// 对应 Java: `ClientCrypto` (服务端侧解密，客户端 SDK 包含用于测试/验证)
pub struct ClientCrypto;

impl ClientCrypto {
    /// 使用 RSA 私钥解密信封，返回明文字符串
    pub fn decrypt_to_string(
        private_key_pem: &str,
        encrypted_key_b64: &str,
        iv_b64: &str,
        ciphertext_b64: &str,
    ) -> Result<String> {
        let plaintext = Self::decrypt(private_key_pem, encrypted_key_b64, iv_b64, ciphertext_b64)?;
        String::from_utf8(plaintext)
            .map_err(|e| ServerApiError::crypto(format!("UTF-8 解码失败: {e}")))
    }

    /// 使用 RSA 私钥解密信封，返回明文字节
    pub fn decrypt(
        private_key_pem: &str,
        encrypted_key_b64: &str,
        iv_b64: &str,
        ciphertext_b64: &str,
    ) -> Result<Vec<u8>> {
        use rsa::pkcs8::DecodePrivateKey;
        use rsa::Oaep;

        let b64 = base64::engine::general_purpose::STANDARD;

        // 1. 解析 RSA 私钥 (PKCS#8 PEM)
        let private_key = rsa::RsaPrivateKey::from_pkcs8_pem(private_key_pem)
            .map_err(|e| ServerApiError::crypto(format!("解析 RSA 私钥失败: {e}")))?;

        // 2. Base64 解码
        let encrypted_key = b64.decode(encrypted_key_b64)?;
        let iv = b64.decode(iv_b64)?;
        let ciphertext = b64.decode(ciphertext_b64)?;

        // 3. RSA-OAEP-SHA256 解密 AES 密钥
        let padding = Oaep::new_with_mgf_hash::<Sha256, Sha1>();
        let aes_key = private_key
            .decrypt(padding, &encrypted_key)
            .map_err(|e| ServerApiError::crypto(format!("RSA 解密 AES 密钥失败: {e}")))?;

        // 4. AES-256-GCM 解密业务数据
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&aes_key));
        let nonce = Nonce::from_slice(&iv);
        let plaintext = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| ServerApiError::crypto(format!("AES-GCM 解密失败: {e}")))?;

        Ok(plaintext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate_rsa_keypair() -> (String, String) {
        use rsa::pkcs8::EncodePrivateKey;
        use rsa::pkcs8::EncodePublicKey;
        use rsa::RsaPrivateKey;

        let mut rng = rand::thread_rng();
        let bits = 2048;
        let priv_key = RsaPrivateKey::new(&mut rng, bits).unwrap();
        let pub_key = priv_key.to_public_key();

        let priv_pem = priv_key
            .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
            .unwrap()
            .to_string();
        let pub_pem = pub_key
            .to_public_key_pem(rsa::pkcs8::LineEnding::LF)
            .unwrap();

        (priv_pem, pub_pem)
    }

    #[test]
    fn should_encrypt_and_decrypt_successfully() {
        let (priv_pem, pub_pem) = generate_rsa_keypair();
        let plaintext = r#"{"message":"hello","count":42}"#;

        let envelope = ClientEnvelopeEncryptor::encrypt(&pub_pem, plaintext).unwrap();
        let decrypted = ClientCrypto::decrypt_to_string(
            &priv_pem,
            &envelope.encrypted_key,
            &envelope.iv,
            &envelope.ciphertext,
        )
        .unwrap();

        assert_eq!(decrypted, plaintext);
    }
}
