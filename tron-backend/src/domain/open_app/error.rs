use std::{error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenAppError {
    NotFound,
    NameRequired,
    DuplicateName,
    InvalidOpenAppCode,
    InvalidUserCode,
    InvalidCapability,
    InvalidOperation,
    OpenAppDisabled,
    Unauthorized,
    InvalidSignatureVersion,
    InvalidTimestamp,
    InvalidSignature,
    NonceReplayed,
    UserForbidden,
    OperationForbidden,
    SecretCryptoFailure,
    StorageFailure,
}

impl fmt::Display for OpenAppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => formatter.write_str("open-app was not found"),
            Self::NameRequired => formatter.write_str("open-app name is required"),
            Self::DuplicateName => formatter.write_str("open-app name already exists"),
            Self::InvalidOpenAppCode => formatter.write_str("open-app code is invalid"),
            Self::InvalidUserCode => formatter.write_str("userCode is invalid"),
            Self::InvalidCapability => formatter.write_str("capability is invalid"),
            Self::InvalidOperation => formatter.write_str("operation is invalid"),
            Self::OpenAppDisabled => formatter.write_str("open-app is not enabled"),
            Self::Unauthorized => formatter.write_str("open-api authentication failed"),
            Self::InvalidSignatureVersion => formatter.write_str("signature version is invalid"),
            Self::InvalidTimestamp => formatter.write_str("signature timestamp is invalid"),
            Self::InvalidSignature => formatter.write_str("signature is invalid"),
            Self::NonceReplayed => formatter.write_str("nonce was replayed"),
            Self::UserForbidden => formatter.write_str("acting user is not allowed"),
            Self::OperationForbidden => formatter.write_str("operation is not allowed"),
            Self::SecretCryptoFailure => formatter.write_str("open-app secret crypto failed"),
            Self::StorageFailure => formatter.write_str("open-app storage failure"),
        }
    }
}

impl Error for OpenAppError {}
