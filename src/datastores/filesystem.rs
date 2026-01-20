use crate::datastores::Datastore;
use regex::Regex;
use std::{
  fs::{File, create_dir_all, read_dir, remove_file},
  io::{Read, Write},
  path::{Path, PathBuf},
};

pub struct FilesystemDatastore {
  base_path: PathBuf,
}

impl Datastore for FilesystemDatastore {
  fn new(base_path: &str) -> Self {
    let mut instance = Self {
      base_path: PathBuf::from(base_path),
    };

    if !instance.base_path.is_dir() {
      panic!("Datastore is not a directory")
    }

    if !instance.base_path.exists() {
      let _ = create_dir_all(instance.base_path.clone())
        .map_err(|err| panic!("Cannot create datastore base directory: {}", err));
    }

    instance
  }

  fn get_object(&self, path: String) -> Result<String, String> {
    let full_path = self.base_path.join(Path::new(path.as_str()));

    let mut file = File::open(full_path.display().to_string())
      .map_err(|err| format!("Couldn't open file {}: {}", full_path.display(), err))?;

    let mut content = String::new();
    file
      .read_to_string(&mut content)
      .map_err(|err| format!("Couldn't read file {}: {}", full_path.display(), err))?;

    Ok(content)
  }

  fn list_objects(&self) -> Result<Vec<String>, String> {
    let backup_file_regex = Regex::new(r"^backup_\w+_[0-9]+\.json$").map_err(|e| e.to_string())?;
    let dir_content = read_dir(self.base_path.clone())
      .map_err(|err| format!("Cannot read read datastore directory content: {}", err))?
      .filter_map(|f| f.ok())
      .filter(|f| f.file_type().is_ok() && f.file_type().unwrap().is_file())
      .map(|f| f.file_name().to_string_lossy().to_string())
      .filter(|f| backup_file_regex.is_match(f))
      .collect();

    Ok(dir_content)
  }

  fn put_object(&self, object_name: &str, obj_content: &[u8]) -> Result<(), String> {
    let file_path = self.base_path.join(Path::new(object_name));

    if file_path.exists() {
      return Err(format!("File {} already exists", file_path.display()));
    }

    let mut file = File::create(file_path.clone())
      .map_err(|e| format!("Cannot create file {}: {}", file_path.display(), e))?;

    file
      .write(obj_content)
      .map_err(|e| format!("Cannot write file {}: {}", file_path.display(), e))?;

    Ok(())
  }

  fn delete_object(&self, object_name: &str) -> Result<(), String> {
    let file_path = self.base_path.join(Path::new(object_name));

    if !file_path.exists() {
      return Err(format!("File {} does not exists", file_path.display()));
    }

    let _ = remove_file(file_path.clone())
      .map_err(|e| format!("Cannot delete file {}: {}", file_path.display(), e))?;

    Ok(())
  }
}
