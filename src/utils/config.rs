use std::{
  env,
  fs::File,
  path::Path,
  io::Read,
};
use std::collections::HashMap;
use fancy_regex::Regex;

#[derive(Debug)]
enum DatastoreType {
  FileSystem,
  S3
}

#[derive(Debug)]
struct Datastore {
  storage_type: DatastoreType,
  path: String,
}

#[derive(Debug)]
struct Backup {
  display_name: String,
  connection_string: String,
  store: Datastore,
  schedule: Option<String>,
  encryption_key: Option<String>,
}

#[derive(Debug)]
enum TomlValue {
  String(String),
  Int(i32),
  Bool(bool),
  Object(HashMap<String, TomlValue>),
  None
}

#[derive(Debug)]
struct TomlProperty {
  name: String,
  value: TomlValue,
}

pub struct Config {
  backups: HashMap<String, Backup>
}

impl Config {
  pub fn new () -> Self {
    let mut instance = Self {
      backups: HashMap::new()
    };

    let config_file = env::var("CONFIG_FILE")
      .unwrap_or("./config.toml".to_string());

    let path = Path::new(&config_file);
    let mut file = match File::open(path) {
      Err(err) => panic!("Couldn't open config file {}: {}", path.display(), err),
      Ok(file) => file,
    };

    let mut content = String::new();
    match file.read_to_string(&mut content) {
      Err(err) => panic!("Couldn't read file {}: {}", path.display(), err),
      Ok(_) => instance.parse_config(content),
    };

    instance
  }

  fn parse_config (&mut self, config: String) {
    let lines = config.lines();
    let table_regex = Regex::new(r"^\[([\w.]+)]$").unwrap();
    let mut blocs: HashMap<String, HashMap<String, TomlValue>> = HashMap::new();
    let mut current_bloc_key = "".to_string();
    blocs.entry(current_bloc_key.clone()).insert_entry(HashMap::new());

    for (index, line) in lines.enumerate() {
      let line = line.trim();

      if table_regex.is_match(line).unwrap() {
        let captures = table_regex.captures(line).unwrap().unwrap();
        let backup_name = captures.get(1).map_or("", |m| m.as_str().split(".").last().unwrap());

        blocs.entry(backup_name.to_string()).insert_entry(HashMap::new());
        current_bloc_key = backup_name.to_string();
      } else {
        let property = match Self::parse_property(line.trim(), index as u16) {
          Ok(value) => value,
          Err(error) => panic!("{}", error),
        };
        println!("{:?}", property);
        let current_bloc_key = current_bloc_key.clone();

        if !matches!(property.value, TomlValue::None) {
          blocs
            .get_mut(current_bloc_key.as_str())
            .unwrap()
            .entry(property.name)
            .insert_entry(property.value);
        }
      }
    }

    for (backup_name, properties) in blocs.iter() {
      if backup_name == "" {
        continue
      }

      let backup_name = match properties.get("display_name").unwrap_or(&TomlValue::String(backup_name.to_string())) {
        TomlValue::String(value) => value.to_string(),
        _ => panic!("Expected String value for property `display_name`")
      };
      let connection_string = match properties.get("connection_string") {
        Some(TomlValue::String(value)) => value.to_string(),
        _ => panic!("Expected String value for property `connection_string`")
      };
      let schedule = match properties.get("schedule") {
        Some(TomlValue::String(value)) => Option::from(value.to_string()),
        Some(TomlValue::None) | None => None,
        _ => panic!("Expected String or None value for property `schedule`")
      };
      let encryption_key = match properties.get("encryption") {
        Some(TomlValue::String(value)) => Option::from(value.to_string()),
        Some(TomlValue::None) | None => None,
        _ => panic!("Expected String or None value for property `encryption_key`")
      };
      let datastore = match properties.get("datastore") {
        Some(TomlValue::Object(datastore)) => {
          let storage_type = match datastore.get("type") {
            Some(TomlValue::String(value)) => match value.as_str() {
              "filesystem" => DatastoreType::FileSystem,
              "s3" => DatastoreType::S3,
              _ => panic!("Invalid datastore type")
            },
            _ => panic!("Expected String value for property `datastore.type`")
          };
          let path = match datastore.get("path") {
            Some(TomlValue::String(value)) => value.to_string(),
            _ => panic!("Expected String value for property `datastore.path`")
          };

          Datastore {
            storage_type,
            path
          }
        },
        _ => panic!("Expected Object value for property `datastore`")
      };

      if encryption_key.as_deref().is_some_and(|k| k.len() != 64) {
        panic!("`encryption_key` is invalid. You can use the `mdbmcli generate-key` command to get one.")
      }

      self.backups.entry(backup_name.clone()).insert_entry(Backup {
        display_name: backup_name,
        connection_string,
        store: datastore,
        schedule,
        encryption_key,
      });
    }

    println!("\n{:?}", self.backups)
  }

  fn parse_property (line: &str, index: u16) -> Result<TomlProperty, String> {
    let property_regex = Regex::new(r"^\s*(([\w-]+)\s*=\s*(.*))\s*$").map_err(|e| e.to_string())?;

    if line.starts_with("#") || line.is_empty() {
      return Ok(TomlProperty {
        name: "".to_string(),
        value: TomlValue::None
      })
    }

    let captures = property_regex.captures(line).unwrap_or(None);
    if captures.is_none() {
      return Err(format!("Invalid configuration at line {}", index + 1))
    }
    let captures = captures.unwrap();

    let property_name = captures.get(2).map_or("", |m| m.as_str());
    let property_raw_value = captures.get(3).map_or("", |m| m.as_str());
    match Self::parse_property_value(property_raw_value) {
      Ok(value) => Ok(TomlProperty {
        name: property_name.to_string(),
        value
      }),
      Err(_) => Err(format!("Invalid property type at line {}", index + 1))
    }
  }

  fn parse_property_value (raw_value: &str) -> Result<TomlValue, String> {
    let string_value_regex = Regex::new(r#"^(['"])([^'"]*)\1$"#).map_err(|e| e.to_string())?;
    let int_value_regex = Regex::new(r"^[0-9]+$").map_err(|e| e.to_string())?;
    let bool_value_regex = Regex::new(r"^(true)|(false)$").map_err(|e| e.to_string())?;
    let obj_value_regex = Regex::new(r"^\{(.*)\}$").map_err(|e| e.to_string())?;

    if string_value_regex.is_match(raw_value).unwrap() {
      let raw_value = string_value_regex.captures(raw_value).unwrap().unwrap();
      let property_value = raw_value.get(2).map_or("", |m| m.as_str());

      Ok(TomlValue::String(property_value.to_string()))
    } else if int_value_regex.is_match(raw_value).unwrap() {
      let raw_value = int_value_regex.captures(raw_value).unwrap().unwrap();
      let property_value = raw_value.get(0).map_or(0, |m| m.as_str().parse().unwrap());

      Ok(TomlValue::Int(property_value))
    } else if bool_value_regex.is_match(raw_value).unwrap() {
      let raw_value = bool_value_regex.captures(raw_value).unwrap().unwrap();
      let property_value = raw_value.get(0).map_or(false, |m| m.as_str() == "true");

      Ok(TomlValue::Bool(property_value))
    } else if obj_value_regex.is_match(raw_value).unwrap() {
      let raw_value = obj_value_regex.captures(raw_value).unwrap().unwrap();
      let obj_content = raw_value.get(0).map_or("", |m| m.as_str());

      let property_regex = Regex::new(r#"\w+\s*[=:]\s*(?:"[^"]*"|'[^']*'|\d+(?:\.\d+)?|true|false)"#).map_err(|e| e.to_string())?;
      let mut obj_properties: HashMap<String, TomlValue> = HashMap::new();

      for cap in property_regex.find_iter(obj_content) {
        if let Ok(property) = cap {
          let property = Self::parse_property(property.as_str(), 0);

          if property.is_ok() {
            let parsed_property = property?;
            obj_properties.entry(parsed_property.name).insert_entry(parsed_property.value);
          }
        }
      }

      Ok(TomlValue::Object(obj_properties))
    } else {
      Err("Invalid property type".to_string())
    }
  }
}