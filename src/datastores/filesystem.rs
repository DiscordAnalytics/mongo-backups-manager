use std::{
  fs::{File, create_dir_all, read_dir, remove_file},
  io::{self, Error, ErrorKind, Read, Write},
  path::{Path, PathBuf},
  sync::OnceLock,
};

use regex::Regex;
use sha2::{Digest, Sha256};
use tokio::{fs::File as TokioFile, io::AsyncWrite};

use crate::{datastores::DatastoreTrait, utils::backup_manager::DatabaseMetadata};

static BACKUP_FILE_REGEX: OnceLock<Regex> = OnceLock::new();
static BACKUP_DIR_REGEX: OnceLock<Regex> = OnceLock::new();

#[derive(Debug, PartialEq, Clone)]
pub struct FilesystemDatastore {
  pub base_path: PathBuf,
}

impl DatastoreTrait for FilesystemDatastore {
  fn new(base_path: &Path) -> Result<Self, Error> {
    let base_path = base_path.to_path_buf();

    if !base_path.exists() {
      create_dir_all(&base_path)?;
    }

    if !base_path.is_dir() {
      return Err(Error::new(
        ErrorKind::InvalidInput,
        "Datastore path is not a directory",
      ));
    }

    Ok(Self { base_path })
  }

  fn check_backup_integrity(&self) -> Result<bool, String> {
    let backup_summary_file = self.get_object(".database.json".to_string())?;
    let backup_summary = serde_json::from_str::<DatabaseMetadata>(backup_summary_file.as_str())
      .map_err(|e| format!("Failed to parse metadata file: {e}"))?;

    for (collection, hash) in backup_summary.collection_hashes {
      let real_hash = self.get_object_hash(format!("{collection}.json")).ok();

      if real_hash.is_none_or(|value| value != hash) {
        return Ok(false);
      }
    }

    Ok(true)
  }

  fn get_object(&self, path: String) -> Result<String, String> {
    let full_path = self.base_path.join(path.as_str());

    let mut file = File::open(full_path.display().to_string())
      .map_err(|err| format!("Couldn't open file {}: {}", full_path.display(), err))?;

    let mut content = String::new();
    file
      .read_to_string(&mut content)
      .map_err(|err| format!("Couldn't read file {}: {}", full_path.display(), err))?;

    Ok(content)
  }

  fn get_object_hash(&self, path: String) -> Result<String, String> {
    let full_path = self.base_path.join(path);
    let mut file = File::open(full_path).map_err(|e| format!("Failed to open file: {e}"))?;
    let mut sha256 = Sha256::new();

    io::copy(&mut file, &mut sha256).map_err(|e| format!("Failed to hash from file: {e}"))?;
    let hash = sha256.finalize();

    Ok(format!("{:x}", hash))
  }

  fn list_objects(&self, path: String) -> Result<Vec<String>, String> {
    let backup_file_regex =
      BACKUP_FILE_REGEX.get_or_init(|| Regex::new(r"\.?\w+\.json$").expect("invalid regex"));
    let dir_content = read_dir(self.base_path.clone().join(PathBuf::from(path)))
      .map_err(|err| format!("Cannot read datastore directory content: {}", err))?
      .filter_map(Result::ok)
      .filter_map(|entry| {
        let name = entry.file_name();
        let name = name.to_str()?;
        backup_file_regex.is_match(name).then(|| name.to_string())
      })
      .collect();

    Ok(dir_content)
  }

  fn list_backups(&self) -> Result<Vec<Self>, String> {
    let backup_dir_regex =
      BACKUP_DIR_REGEX.get_or_init(|| Regex::new(r"\w+_[0-9]+$").expect("invalid regex"));
    let backups = read_dir(self.base_path.clone())
      .map_err(|err| format!("Cannot read datastore directory content: {}", err))?
      .filter_map(Result::ok)
      .filter_map(|entry| {
        let name = entry.file_name();
        let name = name.to_str()?;
        backup_dir_regex
          .is_match(name)
          .then(|| Self::new(entry.path().as_path()))
      })
      .filter_map(Result::ok)
      .collect();

    Ok(backups)
  }

  fn put_object(&self, object_name: &str, obj_content: &[u8]) -> Result<(), String> {
    let file_path = self.base_path.join(object_name);

    if file_path.exists() {
      return Err(format!("File {} already exists", file_path.display()));
    }

    if let Some(parent) = file_path.parent() {
      create_dir_all(parent).map_err(|e| format!("Failed to create parent directory: {e}"))?;
    }

    let mut file = File::create(file_path.clone())
      .map_err(|e| format!("Cannot create file {}: {}", file_path.display(), e))?;

    file
      .write_all(obj_content)
      .map_err(|e| format!("Cannot write file {}: {}", file_path.display(), e))?;

    Ok(())
  }

  fn delete_object(&self, object_name: &str) -> Result<(), String> {
    let file_path = self.base_path.join(object_name);

    remove_file(file_path.clone()).map_err(|e| {
      if e.kind() == ErrorKind::NotFound {
        format!("File {} does not exist", file_path.display())
      } else {
        format!("Cannot delete file {}: {}", file_path.display(), e)
      }
    })?;

    Ok(())
  }

  async fn open_write_stream(
    &self,
    object_name: &str,
  ) -> Result<Box<dyn AsyncWrite + Unpin + Send>, String> {
    let full_path = self.base_path.join(object_name);

    self
      .put_object(object_name, b"")
      .map_err(|e| format!("Failed to create file: {}", e))?;

    let file = TokioFile::create(full_path)
      .await
      .map_err(|e| format!("Failed to create file: {}", e))?;

    Ok(Box::new(file))
  }
}

#[cfg(test)]
mod tests {
  use std::{
    fs::{create_dir_all, write},
    path::Path,
  };

  use chrono::Timelike;
  use tokio::io::AsyncWriteExt;

  use crate::{
    datastores::{DatastoreTrait, FilesystemDatastore},
    tests::{clean_test_dir, get_test_dir_path},
  };

  #[test]
  fn fs_datastore_no_dir_initialization() {
    let test_dir_path = get_test_dir_path("fs_datastore_no_dir_initialization");
    clean_test_dir(test_dir_path.clone());

    let datastore = FilesystemDatastore::new(test_dir_path.as_path()).unwrap();

    assert!(datastore.base_path.exists());

    clean_test_dir(test_dir_path);
  }

  #[test]
  #[should_panic]
  fn fs_datastore_file_initialization() {
    let test_dir_path = get_test_dir_path("fs_datastore_file_initialization");
    let dump_file_path = test_dir_path.clone().join(Path::new("test.txt"));
    clean_test_dir(test_dir_path.clone());

    let _ = create_dir_all(test_dir_path.clone());
    let _ = write(dump_file_path.clone(), b"test file :)");
    let _ = FilesystemDatastore::new(dump_file_path.as_path()).unwrap();

    clean_test_dir(test_dir_path);
  }

  #[test]
  fn fs_datastore_put_in_dev_dir() {
    let datastore = FilesystemDatastore::new(Path::new("/dev")).unwrap();
    let res = datastore.put_object("test.txt", &[0]);

    assert!(res.is_err());
  }

  #[test]
  fn fs_datastore_put_existing_object() {
    let test_dir_path = get_test_dir_path("fs_datastore_put_existing_object");
    clean_test_dir(test_dir_path.clone());

    let datastore = FilesystemDatastore::new(test_dir_path.as_path()).unwrap();
    let _ = datastore.put_object("test.txt", &[0]);
    let res = datastore.put_object("test.txt", &[0]);

    assert!(res.is_err());

    clean_test_dir(test_dir_path);
  }

  #[test]
  fn fs_datastore_put_object() {
    let test_dir_path = get_test_dir_path("fs_datastore_put_object");
    clean_test_dir(test_dir_path.clone());

    let datastore = FilesystemDatastore::new(test_dir_path.as_path()).unwrap();

    let res = datastore.put_object("test.txt", &[4]);

    assert!(res.is_ok());

    clean_test_dir(test_dir_path);
  }

  #[test]
  fn fs_datastore_get_object() {
    let test_dir_path = get_test_dir_path("fs_datastore_get_object");
    clean_test_dir(test_dir_path.clone());

    let datastore = FilesystemDatastore::new(test_dir_path.as_path()).unwrap();
    let _ = datastore.put_object("test.txt", b"This is the best test :)");

    let res = datastore.get_object("test.txt".to_string());
    assert!(res.is_ok());
    let res = res.unwrap();

    assert_eq!(res, "This is the best test :)");

    clean_test_dir(test_dir_path);
  }

  #[test]
  fn fs_datastore_get_unknown_object() {
    let test_dir_path = get_test_dir_path("fs_datastore_get_unknown_object");
    clean_test_dir(test_dir_path.clone());
    let datastore = FilesystemDatastore::new(test_dir_path.as_path()).unwrap();

    let res = datastore.get_object("test.txt".to_string());
    assert!(res.is_err());

    clean_test_dir(test_dir_path);
  }

  #[test]
  fn fs_datastore_get_dir() {
    let test_dir_path = get_test_dir_path("fs_datastore_get_dir");
    clean_test_dir(test_dir_path.clone());
    let datastore = FilesystemDatastore::new(test_dir_path.as_path()).unwrap();

    let res = datastore.get_object("".to_string());
    assert!(res.is_err());

    clean_test_dir(test_dir_path);
  }

  #[test]
  fn fs_datastore_list_objects() {
    let test_dir_path = get_test_dir_path("fs_datastore_list_objects");
    clean_test_dir(test_dir_path.clone());
    let datastore = FilesystemDatastore::new(test_dir_path.as_path()).unwrap();

    let mut files: Vec<u32> = vec![];
    for _ in 0..3 {
      let timestamp = chrono::Local::now().nanosecond();
      let file_name = format!("backup_cool_{timestamp}.json");
      let _ = datastore.put_object(file_name.as_str(), b"test");
      files.push(timestamp);
    }

    let res = datastore.list_objects(".".to_string());
    assert!(res.is_ok());
    let res = res.unwrap();

    for file in files {
      assert!(res.contains(&format!("backup_cool_{}.json", file)));
    }

    clean_test_dir(test_dir_path);
  }

  #[test]
  fn fs_datastore_list_unknown_objects() {
    let test_dir_path = get_test_dir_path("fs_datastore_list_unknown_objects");
    clean_test_dir(test_dir_path.clone());
    let datastore = FilesystemDatastore::new(test_dir_path.as_path()).unwrap();

    for i in 0..3 {
      let file_name = format!("fake_backup_{i}");
      let _ = datastore.put_object(file_name.as_str(), b"test");
    }

    let res = datastore.list_objects(".".to_string());
    assert!(res.is_ok());
    let res = res.unwrap();

    assert_eq!(res.len(), 0);

    clean_test_dir(test_dir_path);
  }

  #[test]
  fn fs_datastore_delete_object() {
    let test_dir_path = get_test_dir_path("fs_datastore_delete_object");
    clean_test_dir(test_dir_path.clone());
    let datastore = FilesystemDatastore::new(test_dir_path.as_path()).unwrap();

    let _ = datastore.put_object("test.txt", b"Awesome test :)");
    let res = datastore.delete_object("test.txt");

    assert!(res.is_ok());

    clean_test_dir(test_dir_path)
  }

  #[test]
  fn fs_datastore_delete_unknown_object() {
    let test_dir_path = get_test_dir_path("fs_datastore_delete_unknown_object");
    clean_test_dir(test_dir_path.clone());
    let datastore = FilesystemDatastore::new(test_dir_path.as_path()).unwrap();

    let res = datastore.delete_object("test.txt");

    assert!(res.is_err());

    clean_test_dir(test_dir_path)
  }

  #[tokio::test]
  async fn fs_datastore_open_write_stream() {
    let test_dir_path = get_test_dir_path("fs_datastore_delete_unknown_object");
    clean_test_dir(test_dir_path.clone());
    let datastore = FilesystemDatastore::new(test_dir_path.as_path()).unwrap();

    let mut stream = datastore
      .open_write_stream("test.txt")
      .await
      .expect("Failed to open write stream");

    stream.write_all(b"first test :)").await.unwrap();
    stream.write_all(b"second test :)").await.unwrap();
    stream.flush().await.unwrap();

    clean_test_dir(test_dir_path.clone());
  }
}
