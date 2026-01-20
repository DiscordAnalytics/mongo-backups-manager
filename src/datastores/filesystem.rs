use crate::datastores::Datastore;
use regex::Regex;
use std::{
  fs::{File, create_dir_all, read_dir, remove_file},
  io::{ErrorKind, Read, Write},
  path::{Path, PathBuf},
  sync::OnceLock,
};

static BACKUP_FILE_REGEX: OnceLock<Regex> = OnceLock::new();

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
    let backup_file_regex = BACKUP_FILE_REGEX
      .get_or_init(|| Regex::new(r"^backup_\w+_[0-9]+\.json$").expect("invalid regex"));
    let dir_content = read_dir(self.base_path.clone())
      .map_err(|err| format!("Cannot read read datastore directory content: {}", err))?
      .filter_map(Result::ok)
      .filter_map(|entry| {
        let name = entry.file_name();
        let name = name.to_str()?;
        backup_file_regex.is_match(name).then(|| name.to_string())
      })
      .map(|f| f)
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
      .write_all(obj_content)
      .map_err(|e| format!("Cannot write file {}: {}", file_path.display(), e))?;

    Ok(())
  }

  fn delete_object(&self, object_name: &str) -> Result<(), String> {
    let file_path = self.base_path.join(Path::new(object_name));

    let _ = remove_file(file_path.clone()).map_err(|e| {
      if e.kind() == ErrorKind::NotFound {
        format!("File {} does not exist", file_path.display())
      } else {
        format!("Cannot delete file {}: {}", file_path.display(), e)
      }
    })?;

    Ok(())
  }
}
