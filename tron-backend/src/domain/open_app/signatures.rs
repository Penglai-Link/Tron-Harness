use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

use super::{OpenApiCanonicalRequest, OpenAppError};

type HmacSha256 = Hmac<Sha256>;

pub fn open_app_secret_fingerprint(secret: &str) -> String {
    let digest = Sha256::digest(secret.as_bytes());
    hex::encode(digest)[..12].to_string()
}

pub fn normalize_query_string(raw_query: Option<&str>) -> String {
    let Some(raw_query) = raw_query.filter(|value| !value.trim().is_empty()) else {
        return String::new();
    };

    let mut pairs = url::form_urlencoded::parse(raw_query.as_bytes())
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    pairs.sort();

    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    for (key, value) in pairs {
        serializer.append_pair(&key, &value);
    }
    serializer.finish()
}

pub fn open_api_body_hash_hex(body: &[u8]) -> String {
    hex::encode(Sha256::digest(body))
}

pub fn build_open_api_canonical_request(
    request: &OpenApiCanonicalRequest<'_>,
) -> Result<String, OpenAppError> {
    let method = request.method.trim();
    let path = request.path.trim();
    let open_app_code = request.open_app_code.trim();
    let user_code = request.user_code.trim();
    let nonce = request.nonce.trim();
    if method.is_empty()
        || path.is_empty()
        || open_app_code.is_empty()
        || user_code.is_empty()
        || nonce.is_empty()
    {
        return Err(OpenAppError::Unauthorized);
    }

    Ok(format!(
        "{method}\n{path}\n{}\n{}\n{open_app_code}\n{user_code}\n{}\n{nonce}",
        normalize_query_string(request.raw_query),
        open_api_body_hash_hex(request.body),
        request.timestamp,
    ))
}

pub fn compute_open_api_signature(
    secret: &str,
    canonical_request: &str,
) -> Result<String, OpenAppError> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| OpenAppError::SecretCryptoFailure)?;
    mac.update(canonical_request.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

pub fn verify_open_api_signature(
    secret: &str,
    canonical_request: &str,
    provided_signature: &str,
) -> Result<bool, OpenAppError> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| OpenAppError::SecretCryptoFailure)?;
    mac.update(canonical_request.as_bytes());
    let provided_bytes =
        hex::decode(provided_signature.trim()).map_err(|_| OpenAppError::InvalidSignature)?;
    Ok(mac.verify_slice(&provided_bytes).is_ok())
}
