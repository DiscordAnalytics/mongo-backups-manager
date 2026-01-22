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

    if !instance.base_path.exists() {
      let _ = create_dir_all(instance.base_path.clone())
        .map_err(|err| panic!("Cannot create datastore base directory: {}", err));
    }

    if !instance.base_path.is_dir() {
      panic!("Datastore is not a directory")
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

#[cfg(test)]
mod tests {
  use crate::{
    datastores::{Datastore, FilesystemDatastore},
    tests::{clean_test_dir, get_test_dir_path},
  };
  use chrono::Timelike;
  use std::fs::{create_dir_all, write};

  #[test]
  fn fs_datastore_no_dir_initialization() {
    let test_dir_path = get_test_dir_path("fs_datastore_no_dir_initialization");
    clean_test_dir(test_dir_path.clone());

    let datastore = FilesystemDatastore::new(test_dir_path.as_str());

    assert!(datastore.base_path.exists());

    clean_test_dir(test_dir_path);
  }

  #[test]
  #[should_panic]
  fn fs_datastore_file_initialization() {
    let test_dir_path = get_test_dir_path("fs_datastore_file_initialization");
    let dump_file_path = format!("{}/test.txt", test_dir_path.clone());
    clean_test_dir(test_dir_path.clone());

    let _ = create_dir_all(test_dir_path.clone());
    let _ = write(dump_file_path.clone(), b"test file :)");
    let _ = FilesystemDatastore::new(dump_file_path.as_str());

    clean_test_dir(test_dir_path);
  }

  #[test]
  fn fs_datastore_put_in_dev_dir() {
    let datastore = FilesystemDatastore::new("/dev");
    let res = datastore.put_object("test.txt", &[0]);

    assert!(res.is_err());
  }

  #[test]
  fn fs_datastore_put_existing_object() {
    let test_dir_path = get_test_dir_path("fs_datastore_put_existing_object");
    clean_test_dir(test_dir_path.clone());

    let datastore = FilesystemDatastore::new(test_dir_path.as_str());
    let res = datastore.put_object("test.txt", &[0]);
    let res = datastore.put_object("test.txt", &[0]);

    assert!(res.is_err());

    clean_test_dir(test_dir_path);
  }

  #[test]
  fn fs_datastore_put_object() {
    let test_dir_path = get_test_dir_path("fs_datastore_put_object");
    clean_test_dir(test_dir_path.clone());

    let datastore = FilesystemDatastore::new(test_dir_path.as_str());

    let res = datastore.put_object("test.txt", &[4]);

    assert!(res.is_ok());

    clean_test_dir(test_dir_path);
  }

  #[test]
  fn fs_datastore_get_object() {
    let test_dir_path = get_test_dir_path("fs_datastore_get_object");
    clean_test_dir(test_dir_path.clone());

    let datastore = FilesystemDatastore::new(test_dir_path.as_str());
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
    let datastore = FilesystemDatastore::new(test_dir_path.as_str());

    let res = datastore.get_object("test.txt".to_string());
    assert!(res.is_err());

    clean_test_dir(test_dir_path);
  }

  #[test]
  fn fs_datastore_get_dir() {
    let test_dir_path = get_test_dir_path("fs_datastore_get_dir");
    clean_test_dir(test_dir_path.clone());
    let datastore = FilesystemDatastore::new(test_dir_path.as_str());

    let res = datastore.get_object("".to_string());
    assert!(res.is_err());

    clean_test_dir(test_dir_path);
  }

  #[test]
  fn fs_datastore_list_objects() {
    let test_dir_path = get_test_dir_path("fs_datastore_list_objects");
    clean_test_dir(test_dir_path.clone());
    let datastore = FilesystemDatastore::new(test_dir_path.as_str());

    let mut files: Vec<u32> = vec![];
    for _ in 0..3 {
      let timestamp = chrono::Local::now().nanosecond();
      let file_name = format!("backup_cool_{timestamp}.json");
      let _ = datastore.put_object(file_name.as_str(), b"test");
      files.push(timestamp);
    }

    let res = datastore.list_objects();
    assert!(res.is_ok());
    let res = res.unwrap();

    for i in 0..3 {
      assert!(res.contains(&format!("backup_cool_{}.json", files[i])));
    }

    clean_test_dir(test_dir_path);
  }

  #[test]
  fn fs_datastore_list_unknown_objects() {
    let test_dir_path = get_test_dir_path("fs_datastore_list_unknown_objects");
    clean_test_dir(test_dir_path.clone());
    let datastore = FilesystemDatastore::new(test_dir_path.as_str());

    let mut files: Vec<u32> = vec![];
    for _ in 0..3 {
      let timestamp = chrono::Local::now().nanosecond();
      let file_name = format!("fake_backup_{timestamp}.json");
      let _ = datastore.put_object(file_name.as_str(), b"test");
      files.push(timestamp);
    }

    let res = datastore.list_objects();
    assert!(res.is_ok());
    let res = res.unwrap();

    assert_eq!(res.len(), 0);

    clean_test_dir(test_dir_path);
  }

  #[test]
  fn fs_datastore_delete_object() {
    let test_dir_path = get_test_dir_path("fs_datastore_delete_object");
    clean_test_dir(test_dir_path.clone());
    let datastore = FilesystemDatastore::new(test_dir_path.as_str());

    let _ = datastore.put_object("test.txt", b"Awesome test :)");
    let res = datastore.delete_object("test.txt");

    assert!(res.is_ok());

    clean_test_dir(test_dir_path)
  }

  #[test]
  fn fs_datastore_delete_unknown_object() {
    let test_dir_path = get_test_dir_path("fs_datastore_delete_unknown_object");
    clean_test_dir(test_dir_path.clone());
    let datastore = FilesystemDatastore::new(test_dir_path.as_str());

    let res = datastore.delete_object("test.txt");

    assert!(res.is_err());

    clean_test_dir(test_dir_path)
  }
}
