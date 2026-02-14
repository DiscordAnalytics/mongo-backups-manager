use std::{io::Error, path::Path};

use tokio::io::AsyncWrite;

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
  fn get_object(&self, path: String) -> Result<String, String>;
  fn get_object_hash(&self, path: String) -> Result<String, String>;
  fn list_objects(&self) -> Result<Vec<String>, String>;
  fn list_backups(&self) -> Result<Vec<Self>, String>
  where
    Self: Sized;
  fn put_object(&self, object_name: &str, object_content: &[u8]) -> Result<(), String>;
  fn delete_object(&self, object_name: &str) -> Result<(), String>;
  async fn open_write_stream(
    &self,
    object_name: &str,
  ) -> Result<Box<dyn AsyncWrite + Unpin + Send>, String>;
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
  pub fn check_backup_integrity(&self) -> Result<bool, String> {
    delegate_to_datastore!(self, check_backup_integrity())
  }

  pub fn get_object(&self, path: String) -> Result<String, String> {
    delegate_to_datastore!(self, get_object(path))
  }

  pub fn get_object_hash(&self, path: String) -> Result<String, String> {
    delegate_to_datastore!(self, get_object_hash(path))
  }

  pub fn list_objects(&self) -> Result<Vec<String>, String> {
    delegate_to_datastore!(self, list_objects())
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
}
