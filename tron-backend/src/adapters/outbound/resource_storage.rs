use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufReader, BufWriter},
    path::{Component, Path, PathBuf},
};

use crate::{application::resource::ObjectStorage, domain::resource::ResourceError};

#[derive(Clone, Debug)]
pub struct LocalObjectStorage {
    root: PathBuf,
}

impl LocalObjectStorage {
    pub fn new(root: impl AsRef<Path>) -> Result<Self, ResourceError> {
        fs::create_dir_all(root.as_ref()).map_err(|_| ResourceError::StorageFailure)?;
        let root = root
            .as_ref()
            .canonicalize()
            .map_err(|_| ResourceError::StorageFailure)?;
        Ok(Self { root })
    }

    fn object_path(&self, object_key: &str) -> Result<PathBuf, ResourceError> {
        let key = Path::new(object_key);
        if object_key.trim().is_empty()
            || key.is_absolute()
            || key.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(ResourceError::StorageLocatorInvalid);
        }
        Ok(self.root.join(key))
    }

    fn copy(source: &Path, target: &Path) -> Result<u64, ResourceError> {
        let input = File::open(source).map_err(|_| ResourceError::StorageFailure)?;
        let output = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(target)
            .map_err(|_| ResourceError::StorageFailure)?;
        let mut writer = BufWriter::new(output);
        let copied = io::copy(&mut BufReader::new(input), &mut writer)
            .map_err(|_| ResourceError::StorageFailure)?;
        use std::io::Write;
        writer.flush().map_err(|_| ResourceError::StorageFailure)?;
        Ok(copied)
    }
}

impl ObjectStorage for LocalObjectStorage {
    fn put_file(&self, object_key: &str, source: &Path) -> Result<u64, ResourceError> {
        let target = self.object_path(object_key)?;
        let parent = target.parent().ok_or(ResourceError::StorageFailure)?;
        fs::create_dir_all(parent).map_err(|_| ResourceError::StorageFailure)?;
        Self::copy(source, &target)
    }

    fn get_file(&self, object_key: &str, target: &Path) -> Result<u64, ResourceError> {
        let source = self.object_path(object_key)?;
        if !source.is_file() {
            return Err(ResourceError::MissingBinaryObject);
        }
        Self::copy(&source, target)
    }

    fn delete(&self, object_key: &str) -> Result<(), ResourceError> {
        let path = self.object_path(object_key)?;
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(ResourceError::StorageFailure),
        }
    }

    fn exists(&self, object_key: &str) -> Result<bool, ResourceError> {
        Ok(self.object_path(object_key)?.is_file())
    }
}
