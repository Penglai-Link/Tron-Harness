use std::{collections::HashMap, sync::Arc};

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use gmcrypto_core::sm4::mode_gcm;
use rand::RngCore;

use crate::{
    application::variable::VariableCipher,
    config::VariableCryptoConfig,
    domain::variable::{EncryptedVariableValue, VARIABLE_CRYPTO_ALGORITHM, VariableError},
};

#[derive(Clone)]
pub struct Sm4GcmVariableCipher {
    active_key_version: Option<String>,
    keys: Arc<HashMap<String, [u8; 16]>>,
}

impl Sm4GcmVariableCipher {
    pub fn from_config(config: VariableCryptoConfig) -> Result<Self, VariableError> {
        let keys = config
            .keys
            .into_iter()
            .map(|key| {
                let bytes = BASE64
                    .decode(key.key_base64.as_bytes())
                    .map_err(|_| VariableError::EncryptionNotConfigured)?;
                let key_bytes: [u8; 16] = bytes
                    .try_into()
                    .map_err(|_| VariableError::EncryptionNotConfigured)?;
                Ok((key.version, key_bytes))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;

        Ok(Self {
            active_key_version: config.active_key_version,
            keys: Arc::new(keys),
        })
    }

    fn active_key(&self) -> Result<(&str, &[u8; 16]), VariableError> {
        let version = self
            .active_key_version
            .as_deref()
            .ok_or(VariableError::EncryptionNotConfigured)?;
        let key = self
            .keys
            .get(version)
            .ok_or(VariableError::EncryptionNotConfigured)?;
        Ok((version, key))
    }

    fn key_for_version(&self, version: &str) -> Result<&[u8; 16], VariableError> {
        self.keys
            .get(version)
            .ok_or(VariableError::EncryptionNotConfigured)
    }
}

impl VariableCipher for Sm4GcmVariableCipher {
    fn encrypt(
        &self,
        plaintext: &str,
        aad: &[u8],
    ) -> Result<EncryptedVariableValue, VariableError> {
        let (version, key) = self.active_key()?;
        let mut nonce = [0u8; 12];
        rand::rng().fill_bytes(&mut nonce);
        let (ciphertext, tag) = mode_gcm::encrypt(key, &nonce, aad, plaintext.as_bytes())
            .ok_or(VariableError::EncryptFailed)?;

        Ok(EncryptedVariableValue {
            ciphertext: BASE64.encode(ciphertext),
            nonce: BASE64.encode(nonce),
            tag: BASE64.encode(tag),
            algorithm: VARIABLE_CRYPTO_ALGORITHM.into(),
            key_version: version.to_string(),
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
        let key = self.key_for_version(&encrypted.key_version)?;
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
        let plaintext = mode_gcm::decrypt(key, &nonce, aad, &ciphertext, &tag)
            .ok_or(VariableError::DecryptFailed)?;
        String::from_utf8(plaintext).map_err(|_| VariableError::DecryptFailed)
    }
}
