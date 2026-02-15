use std::{
  io::Error,
  path::{Path, PathBuf},
};

use tokio::io::AsyncWrite;

use crate::datastores::DatastoreTrait;

#[derive(Debug, PartialEq, Clone)]
pub struct S3Datastore {
  pub base_path: PathBuf,
}

#[allow(unused)]
impl DatastoreTrait for S3Datastore {
  fn new(base_path: &Path) -> Result<Self, Error> {
    todo!()
  }

  fn check_backup_integrity(&self) -> Result<bool, String> {
    todo!()
  }

  fn get_object(&self, path: String) -> Result<String, String> {
    todo!()
  }

  fn get_object_hash(&self, path: String) -> Result<String, String> {
    todo!()
  }

  fn list_objects(&self) -> Result<Vec<String>, String> {
    todo!()
  }

  fn list_backups(&self) -> Result<Vec<Self>, String> {
    todo!()
  }

  fn put_object(&self, object_name: &str, object_content: &[u8]) -> Result<(), String> {
    todo!()
  }

  fn delete_object(&self, object_name: &str) -> Result<(), String> {
    todo!()
  }

  async fn open_write_stream(
    &self,
    object_name: &str,
  ) -> Result<Box<dyn AsyncWrite + Unpin + Send>, String> {
    todo!()
  }
}
