use std::{io::Error, path::Path};

use tokio::io::AsyncWrite;

pub mod filesystem;
pub use filesystem::FilesystemDatastore;

#[allow(unused)]
pub trait Datastore {
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
