use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    path::Path,
};

use opendal::{Operator, blocking, services};

use crate::{
    application::resource::ObjectStorage, config::S3ResourceStorageProfile,
    domain::resource::ResourceError,
};

/// S3-compatible object storage backed by Apache OpenDAL.
///
/// OpenDAL owns S3 authentication, retries, endpoint addressing and native
/// range requests. The adapter only translates the platform's synchronous
/// storage port to OpenDAL's blocking operator.
#[derive(Clone)]
pub struct S3ObjectStorage {
    operator: blocking::Operator,
}

impl S3ObjectStorage {
    pub fn from_config(profile: &S3ResourceStorageProfile) -> Result<Self, ResourceError> {
        let access_key_id = required_env(&profile.access_key_id_env)?;
        let secret_access_key = required_env(&profile.secret_access_key_env)?;
        let mut builder = services::S3::default()
            .bucket(profile.bucket.trim())
            .endpoint(profile.endpoint.trim())
            .region(profile.region.trim())
            .access_key_id(&access_key_id)
            .secret_access_key(&secret_access_key)
            .root(profile.prefix.trim_matches('/'));
        if let Some(name) = profile.session_token_env.as_deref()
            && let Some(token) = optional_env(name)?
        {
            builder = builder.session_token(&token);
        }
        if !profile.force_path_style {
            builder = builder.enable_virtual_host_style();
        }

        let operator = Operator::new(builder)
            .map_err(|_| ResourceError::StorageFailure)?
            .finish();
        let operator =
            blocking::Operator::new(operator).map_err(|_| ResourceError::StorageFailure)?;
        Ok(Self { operator })
    }

    fn path(&self, object_key: &str) -> Result<String, ResourceError> {
        validate_object_key(object_key)?;
        Ok(object_key.to_string())
    }

    fn put_reader_inner(
        &self,
        object_key: &str,
        source: &mut dyn Read,
    ) -> Result<u64, ResourceError> {
        let path = self.path(object_key)?;
        let mut writer = self.operator.writer(&path).map_err(map_storage_error)?;
        let mut buffer = [0_u8; 128 * 1024];
        let mut total = 0_u64;
        loop {
            let read = source
                .read(&mut buffer)
                .map_err(|_| ResourceError::StorageFailure)?;
            if read == 0 {
                break;
            }
            writer
                .write(buffer[..read].to_vec())
                .map_err(map_storage_error)?;
            total = total
                .checked_add(read as u64)
                .ok_or(ResourceError::StorageFailure)?;
        }
        writer.close().map_err(map_storage_error)?;
        Ok(total)
    }

    fn copy_to_writer<R: Read>(
        &self,
        mut source: R,
        target: &mut dyn Write,
    ) -> Result<u64, ResourceError> {
        let copied = io::copy(&mut source, target).map_err(|_| ResourceError::StorageFailure)?;
        target.flush().map_err(|_| ResourceError::StorageFailure)?;
        Ok(copied)
    }
}

impl ObjectStorage for S3ObjectStorage {
    fn put_file(&self, object_key: &str, source: &Path) -> Result<u64, ResourceError> {
        let mut source = File::open(source).map_err(|_| ResourceError::StorageFailure)?;
        self.put_reader_inner(object_key, &mut source)
    }

    fn put_reader(&self, object_key: &str, source: &mut dyn Read) -> Result<u64, ResourceError> {
        self.put_reader_inner(object_key, source)
    }

    fn get_file(&self, object_key: &str, target: &Path) -> Result<u64, ResourceError> {
        let output = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(target)
            .map_err(|_| ResourceError::StorageFailure)?;
        let mut output = io::BufWriter::new(output);
        let copied = self.get_writer(object_key, &mut output)?;
        output.flush().map_err(|_| ResourceError::StorageFailure)?;
        Ok(copied)
    }

    fn get_writer(&self, object_key: &str, target: &mut dyn Write) -> Result<u64, ResourceError> {
        let path = self.path(object_key)?;
        let reader = self
            .operator
            .reader(&path)
            .map_err(map_storage_error)?
            .into_std_read(..)
            .map_err(map_storage_error)?;
        self.copy_to_writer(reader, target)
    }

    fn get_range_writer(
        &self,
        object_key: &str,
        start_byte: u64,
        max_bytes: u64,
        target: &mut dyn Write,
    ) -> Result<u64, ResourceError> {
        if max_bytes == 0 {
            return Ok(0);
        }
        let end_byte = start_byte
            .checked_add(max_bytes)
            .ok_or(ResourceError::StorageFailure)?;
        let path = self.path(object_key)?;
        let reader = self
            .operator
            .reader(&path)
            .map_err(map_storage_error)?
            .into_std_read(start_byte..end_byte)
            .map_err(map_storage_error)?;
        self.copy_to_writer(reader, target)
    }

    fn delete(&self, object_key: &str) -> Result<(), ResourceError> {
        let path = self.path(object_key)?;
        match self.operator.delete(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == opendal::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(map_storage_error(error)),
        }
    }

    fn exists(&self, object_key: &str) -> Result<bool, ResourceError> {
        let path = self.path(object_key)?;
        self.operator.exists(&path).map_err(map_storage_error)
    }
}

fn required_env(name: &str) -> Result<String, ResourceError> {
    optional_env(name)?
        .filter(|value| !value.is_empty())
        .ok_or(ResourceError::StorageFailure)
}

fn optional_env(name: &str) -> Result<Option<String>, ResourceError> {
    match std::env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => Err(ResourceError::StorageFailure),
    }
}

fn validate_object_key(object_key: &str) -> Result<(), ResourceError> {
    if object_key.trim().is_empty()
        || object_key.starts_with('/')
        || object_key
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(ResourceError::StorageLocatorInvalid);
    }
    Ok(())
}

fn map_storage_error(error: opendal::Error) -> ResourceError {
    if error.kind() == opendal::ErrorKind::NotFound {
        ResourceError::MissingBinaryObject
    } else {
        ResourceError::StorageFailure
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_keys_are_restricted_to_relative_segments() {
        assert!(validate_object_key("chat-sessions/session/file.txt").is_ok());
        assert_eq!(
            validate_object_key("../secrets"),
            Err(ResourceError::StorageLocatorInvalid)
        );
        assert_eq!(
            validate_object_key("chat//file"),
            Err(ResourceError::StorageLocatorInvalid)
        );
    }
}
