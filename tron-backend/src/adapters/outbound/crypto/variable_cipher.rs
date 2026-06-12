use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use gmcrypto_core::sm4::mode_gcm;
use rand::RngCore;

use crate::{
    application::variable::VariableCipher,
    config::VariableCryptoConfig,
    domain::variable::{EncryptedVariableValue, VARIABLE_CRYPTO_ALGORITHM, VariableError},
};

const VARIABLE_CRYPTO_KEY_VERSION: &str = "v1";

#[derive(Clone)]
pub struct Sm4GcmVariableCipher {
    key: [u8; 16],
}

impl Sm4GcmVariableCipher {
    pub fn from_config(config: VariableCryptoConfig) -> Result<Self, VariableError> {
        let key_base64 = config.key_base64.trim();
        if key_base64.is_empty() {
            return Err(VariableError::EncryptionNotConfigured);
        }
        let bytes = BASE64
            .decode(key_base64.as_bytes())
            .map_err(|_| VariableError::EncryptionNotConfigured)?;
        let key: [u8; 16] = bytes
            .try_into()
            .map_err(|_| VariableError::EncryptionNotConfigured)?;
        Ok(Self { key })
    }
}

impl VariableCipher for Sm4GcmVariableCipher {
    fn encrypt(
        &self,
        plaintext: &str,
        aad: &[u8],
    ) -> Result<EncryptedVariableValue, VariableError> {
        let mut nonce = [0u8; 12];
        rand::rng().fill_bytes(&mut nonce);
        let (ciphertext, tag) = mode_gcm::encrypt(&self.key, &nonce, aad, plaintext.as_bytes())
            .ok_or(VariableError::EncryptFailed)?;

        Ok(EncryptedVariableValue {
            ciphertext: BASE64.encode(ciphertext),
            nonce: BASE64.encode(nonce),
            tag: BASE64.encode(tag),
            algorithm: VARIABLE_CRYPTO_ALGORITHM.into(),
            key_version: VARIABLE_CRYPTO_KEY_VERSION.into(),
        })
    }

    fn decrypt(
        &self,
        encrypted: &EncryptedVariableValue,
        aad: &[u8],
    ) -> Result<String, VariableError> {
        if encrypted.algorithm != VARIABLE_CRYPTO_ALGORITHM {
            return Err(VariableError::DecryptFailed);
        }
        let nonce = BASE64
            .decode(encrypted.nonce.as_bytes())
            .map_err(|_| VariableError::DecryptFailed)?;
        let ciphertext = BASE64
            .decode(encrypted.ciphertext.as_bytes())
            .map_err(|_| VariableError::DecryptFailed)?;
        let tag_bytes = BASE64
            .decode(encrypted.tag.as_bytes())
            .map_err(|_| VariableError::DecryptFailed)?;
        let tag: [u8; 16] = tag_bytes
            .try_into()
            .map_err(|_| VariableError::DecryptFailed)?;
        let plaintext = mode_gcm::decrypt(&self.key, &nonce, aad, &ciphertext, &tag)
            .ok_or(VariableError::DecryptFailed)?;
        String::from_utf8(plaintext).map_err(|_| VariableError::DecryptFailed)
    }
}
