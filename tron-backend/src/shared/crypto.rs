use aes_gcm_siv::{
    Aes256GcmSiv, Nonce,
    aead::{Aead, KeyInit},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};

use crate::shared::random::random_bytes;

#[derive(Clone)]
pub struct Aes256SecretBox {
    key: [u8; 32],
}

impl Aes256SecretBox {
    pub fn new(key_material: &str) -> Self {
        let digest = Sha256::digest(key_material.as_bytes());
        let mut key = [0_u8; 32];
        key.copy_from_slice(&digest[..32]);
        Self { key }
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<String, ()> {
        let cipher = Aes256GcmSiv::new_from_slice(&self.key).map_err(|_| ())?;
        let nonce_bytes = random_bytes(12);
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce_bytes), plaintext.as_bytes())
            .map_err(|_| ())?;
        let mut payload = nonce_bytes;
        payload.extend_from_slice(&ciphertext);
        Ok(URL_SAFE_NO_PAD.encode(payload))
    }

    pub fn decrypt(&self, encoded: &str) -> Result<String, ()> {
        let payload = URL_SAFE_NO_PAD.decode(encoded).map_err(|_| ())?;
        if payload.len() < 13 {
            return Err(());
        }
        let (nonce_bytes, ciphertext) = payload.split_at(12);
        let cipher = Aes256GcmSiv::new_from_slice(&self.key).map_err(|_| ())?;
        let plaintext = cipher
            .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
            .map_err(|_| ())?;
        String::from_utf8(plaintext).map_err(|_| ())
    }
}
