use std::io::Bytes;

pub mod filesystem;
pub use filesystem::FilesystemDatastore;

pub trait Datastore {
  fn new(base_path: &str) -> Self;

  fn get_object(&self, path: String) -> Result<String, String>;
  fn list_objects(&self) -> Result<Vec<String>, String>;
  fn put_object(&self, object_name: &str, object_content: &[u8]) -> Result<(), String>;
}
