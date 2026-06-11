use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngCore;

pub fn random_bytes(byte_len: usize) -> Vec<u8> {
    let mut bytes = vec![0_u8; byte_len];
    rand::rng().fill_bytes(&mut bytes);
    bytes
}

pub fn random_hex(byte_len: usize) -> String {
    hex::encode(random_bytes(byte_len))
}

pub fn random_base64url(byte_len: usize) -> String {
    URL_SAFE_NO_PAD.encode(random_bytes(byte_len))
}
