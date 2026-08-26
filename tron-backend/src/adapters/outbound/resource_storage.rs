use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write},
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

    fn copy_reader(source: &mut dyn Read, target: &Path) -> Result<u64, ResourceError> {
        let output = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(target)
            .map_err(|_| ResourceError::StorageFailure)?;
        let mut writer = BufWriter::new(output);
        let copied = io::copy(source, &mut writer).map_err(|_| ResourceError::StorageFailure)?;
        writer.flush().map_err(|_| ResourceError::StorageFailure)?;
        Ok(copied)
    }

    fn copy_reader_atomically(source: &mut dyn Read, target: &Path) -> Result<u64, ResourceError> {
        let parent = target.parent().ok_or(ResourceError::StorageFailure)?;
        let mut temporary =
            tempfile::NamedTempFile::new_in(parent).map_err(|_| ResourceError::StorageFailure)?;

        let copied = {
            let mut writer = BufWriter::new(temporary.as_file_mut());
            match io::copy(source, &mut writer) {
                Ok(copied) => match writer.flush() {
                    Ok(()) => Ok(copied),
                    Err(_) => Err(ResourceError::StorageFailure),
                },
                Err(_) => Err(ResourceError::StorageFailure),
            }
        };

        match copied {
            Ok(copied) => match temporary.persist(target) {
                Ok(_) => Ok(copied),
                Err(error) => {
                    let _ = error.file.close();
                    Err(ResourceError::StorageFailure)
                }
            },
            Err(error) => {
                let _ = temporary.close();
                Err(error)
            }
        }
    }

    fn copy(source: &Path, target: &Path) -> Result<u64, ResourceError> {
        let input = File::open(source).map_err(|_| ResourceError::StorageFailure)?;
        Self::copy_reader(&mut BufReader::new(input), target)
    }
}

impl ObjectStorage for LocalObjectStorage {
    fn put_file(&self, object_key: &str, source: &Path) -> Result<u64, ResourceError> {
        let target = self.object_path(object_key)?;
        let parent = target.parent().ok_or(ResourceError::StorageFailure)?;
        fs::create_dir_all(parent).map_err(|_| ResourceError::StorageFailure)?;
        let input = File::open(source).map_err(|_| ResourceError::StorageFailure)?;
        Self::copy_reader_atomically(&mut BufReader::new(input), &target)
    }

    fn put_reader(&self, object_key: &str, source: &mut dyn Read) -> Result<u64, ResourceError> {
        let target = self.object_path(object_key)?;
        let parent = target.parent().ok_or(ResourceError::StorageFailure)?;
        fs::create_dir_all(parent).map_err(|_| ResourceError::StorageFailure)?;
        Self::copy_reader_atomically(source, &target)
    }

    fn get_file(&self, object_key: &str, target: &Path) -> Result<u64, ResourceError> {
        let source = self.object_path(object_key)?;
        if !source.is_file() {
            return Err(ResourceError::MissingBinaryObject);
        }
        Self::copy(&source, target)
    }

    fn get_writer(&self, object_key: &str, target: &mut dyn Write) -> Result<u64, ResourceError> {
        let source = self.object_path(object_key)?;
        if !source.is_file() {
            return Err(ResourceError::MissingBinaryObject);
        }
        let input = File::open(source).map_err(|_| ResourceError::StorageFailure)?;
        let copied = io::copy(&mut BufReader::new(input), target)
            .map_err(|_| ResourceError::StorageFailure)?;
        target.flush().map_err(|_| ResourceError::StorageFailure)?;
        Ok(copied)
    }

    fn get_range_writer(
        &self,
        object_key: &str,
        start_byte: u64,
        max_bytes: u64,
        target: &mut dyn Write,
    ) -> Result<u64, ResourceError> {
        let source = self.object_path(object_key)?;
        if !source.is_file() {
            return Err(ResourceError::MissingBinaryObject);
        }
        let mut input = File::open(source).map_err(|_| ResourceError::StorageFailure)?;
        input
            .seek(SeekFrom::Start(start_byte))
            .map_err(|_| ResourceError::StorageFailure)?;
        let copied = io::copy(&mut input.take(max_bytes), target)
            .map_err(|_| ResourceError::StorageFailure)?;
        target.flush().map_err(|_| ResourceError::StorageFailure)?;
        Ok(copied)
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

#[cfg(test)]
mod tests {
    use std::io::{self, Cursor, Read};

    use super::*;

    struct FailingReader {
        remaining: Option<Cursor<Vec<u8>>>,
    }

    impl FailingReader {
        fn after(bytes: impl Into<Vec<u8>>) -> Self {
            Self {
                remaining: Some(Cursor::new(bytes.into())),
            }
        }
    }

    impl Read for FailingReader {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if let Some(reader) = self.remaining.as_mut() {
                let copied = reader.read(buffer)?;
                if copied > 0 {
                    return Ok(copied);
                }
                self.remaining = None;
            }
            Err(io::Error::other("source stream failed"))
        }
    }

    #[test]
    fn failed_put_reader_preserves_the_existing_object_and_cleans_the_temp_file() {
        let directory = tempfile::tempdir().expect("temp storage root");
        let storage = LocalObjectStorage::new(directory.path()).expect("storage initializes");
        let mut original = Cursor::new(b"previous bytes".to_vec());
        storage
            .put_reader("nested/object.bin", &mut original)
            .expect("original object is stored");

        let mut failing_source = FailingReader::after(b"partial replacement".to_vec());
        assert_eq!(
            storage.put_reader("nested/object.bin", &mut failing_source),
            Err(ResourceError::StorageFailure)
        );

        assert_eq!(
            fs::read(directory.path().join("nested/object.bin")).expect("original object remains"),
            b"previous bytes"
        );
        let entries = fs::read_dir(directory.path().join("nested"))
            .expect("nested directory exists")
            .collect::<Result<Vec<_>, _>>()
            .expect("directory can be read");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].file_name(), "object.bin");
    }
}
