use std::{io::Error, path::Path};

use tokio::io::{AsyncRead, AsyncWrite};

mod filesystem;
mod s3;

pub use filesystem::FilesystemDatastore;
pub use s3::S3Datastore;

#[allow(unused)]
pub trait DatastoreTrait {
  fn new(base_path: &Path) -> Result<Self, Error>
  where
    Self: Sized;

  fn check_backup_integrity(&self) -> Result<bool, String>;
  fn get_object(&self, path: impl AsRef<Path>) -> Result<String, String>;
  fn get_object_hash(&self, path: impl AsRef<Path>) -> Result<String, String>;
  fn list_objects(&self, path: impl AsRef<Path>) -> Result<Vec<String>, String>;
  fn list_backups(&self) -> Result<Vec<Self>, String>
  where
    Self: Sized;
  fn put_object(&self, object_name: &str, object_content: &[u8]) -> Result<(), String>;
  fn delete_object(&self, object_name: &str) -> Result<(), String>;
  async fn open_write_stream(
    &self,
    object_name: &str,
  ) -> Result<Box<dyn AsyncWrite + Unpin + Send>, String>;
  async fn open_read_stream(
    &self,
    object_name: &str,
  ) -> Result<Box<dyn AsyncRead + Unpin + Send>, String>;
  fn create_parent_dir(&self, path: &Path) -> Result<(), String>;
}

#[derive(Debug, PartialEq, Clone)]
pub enum Datastore {
  FileSystem(FilesystemDatastore),
  S3(S3Datastore),
}

macro_rules! delegate_to_datastore {
  ($self:ident, $method:ident($($arg:expr),*)) => {
    match $self {
      Datastore::FileSystem(ds) => ds.$method($($arg),*),
      Datastore::S3(ds) => ds.$method($($arg),*),
    }
  };
}

#[allow(unused)]
impl Datastore {
  pub fn as_str(&self) -> &'static str {
    match self {
      Datastore::FileSystem(_) => "filesystem",
      Datastore::S3(_) => "s3",
    }
  }

  pub fn get_base_path(&self) -> String {
    match self {
      Datastore::FileSystem(store) => store.base_path.display().to_string(),
      Datastore::S3(store) => store.base_path.display().to_string(),
    }
  }

  pub fn check_backup_integrity(&self) -> Result<bool, String> {
    delegate_to_datastore!(self, check_backup_integrity())
  }

  pub fn get_object(&self, path: impl AsRef<Path>) -> Result<String, String> {
    delegate_to_datastore!(self, get_object(path))
  }

  pub fn get_object_hash(&self, path: impl AsRef<Path>) -> Result<String, String> {
    delegate_to_datastore!(self, get_object_hash(path))
  }

  pub fn list_objects(&self, path: impl AsRef<Path>) -> Result<Vec<String>, String> {
    delegate_to_datastore!(self, list_objects(path))
  }

  pub fn put_object(&self, object_name: &str, object_content: &[u8]) -> Result<(), String> {
    delegate_to_datastore!(self, put_object(object_name, object_content))
  }

  pub fn delete_object(&self, object_name: &str) -> Result<(), String> {
    delegate_to_datastore!(self, delete_object(object_name))
  }

  pub async fn open_write_stream(
    &self,
    object_name: &str,
  ) -> Result<Box<dyn AsyncWrite + Unpin + Send>, String> {
    match self {
      Datastore::FileSystem(store) => store.open_write_stream(object_name).await,
      Datastore::S3(store) => store.open_write_stream(object_name).await,
    }
  }

  pub async fn open_read_stream(
    &self,
    object_name: &str,
  ) -> Result<Box<dyn AsyncRead + Unpin + Send>, String> {
    match self {
      Datastore::FileSystem(store) => store.open_read_stream(object_name).await,
      Datastore::S3(store) => store.open_read_stream(object_name).await,
    }
  }

  pub fn get_backups(&self, backup_identifier: &str) -> Vec<(String, Datastore)> {
    let backup_dir_prefix = format!("backup_{backup_identifier}_");

    match self {
      Datastore::FileSystem(store) => store
        .list_backups()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|store| {
          let dir_name = store.base_path.file_name()?.to_str()?.to_string();

          if dir_name.starts_with(&backup_dir_prefix) {
            Some((dir_name, Datastore::FileSystem(store)))
          } else {
            None
          }
        })
        .collect::<Vec<_>>(),

      Datastore::S3(store) => store
        .list_backups()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|store| {
          let dir_name = store.base_path.file_name()?.to_str()?.to_string();

          if dir_name.starts_with(&backup_dir_prefix) {
            Some((dir_name, Datastore::S3(store)))
          } else {
            None
          }
        })
        .collect::<Vec<_>>(),
    }
  }
}
