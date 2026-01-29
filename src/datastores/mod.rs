use std::format;
use std::fs::File;
use std::io::Bytes;
use tokio::io::AsyncWrite;

pub mod filesystem;
pub use filesystem::FilesystemDatastore;

pub trait Datastore {
  fn new(base_path: &str) -> Self;

  fn get_object(&self, path: String) -> Result<String, String>;
  fn list_objects(&self) -> Result<Vec<String>, String>;
  fn put_object(&self, object_name: &str, object_content: &[u8]) -> Result<(), String>;
  fn delete_object(&self, object_name: &str) -> Result<(), String>;
  async fn open_write_stream(
    &self,
    object_name: &str,
  ) -> Result<Box<dyn AsyncWrite + Unpin + Send>, String>;
}
