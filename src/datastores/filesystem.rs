use crate::datastores::Datastore;
use std::fs::{File, create_dir_all};
use std::io::{Bytes, Read};
use std::path::{Path, PathBuf};

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

        instance
    }

    fn get_object(&self, path: String) -> Result<String, String> {
        let full_path = self.base_path.join(Path::new(path.as_str()));

        let mut file = File::open(full_path.display().to_string())
            .map_err(|err| format!("Couldn't open file {}: {}", full_path.display(), err))?;

        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|err| format!("Couldn't read file {}: {}", full_path.display(), err))?;

        Ok(content)
    }

    fn list_objects(path: &str) -> Result<Vec<String>, String> {
        todo!()
    }

    fn put_object(path: &str, content: Vec<Bytes<u8>>) -> Result<(), String> {
        todo!()
    }
}
