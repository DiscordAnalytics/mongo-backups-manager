use std::{collections::HashMap, env, fs::File, io::Read, path::Path};

#[derive(Debug, PartialEq, Clone)]
enum BackupDatastoreType {
  FileSystem,
  S3,
}

#[derive(Debug, PartialEq, Clone)]
struct BackupDatastore {
  storage_type: BackupDatastoreType,
  path: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct BackupSchedule {
  pub enabled: bool,
  pub cron: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Backup {
  pub display_name: String,
  pub connection_string: String,
  pub ignore_collections: Vec<String>,
  pub datastore: BackupDatastore,
  pub schedule: BackupSchedule,
  pub encryption_key: Option<String>,
}

#[derive(Debug)]
enum TomlValue {
  String(String),
  Int(i64),
  Float(f64),
  Bool(bool),
  Object(HashMap<String, TomlValue>),
  Array(Vec<TomlValue>),
}

impl TomlValue {
  fn type_name(&self) -> &'static str {
    match self {
      TomlValue::String(_) => "string",
      TomlValue::Int(_) => "int",
      TomlValue::Float(_) => "float",
      TomlValue::Bool(_) => "bool",
      TomlValue::Array(_) => "array",
      TomlValue::Object(_) => "object",
    }
  }

  fn debug_value(&self) -> String {
    match self {
      TomlValue::String(s) => format!("\"{}\"", s),
      TomlValue::Int(n) => n.to_string(),
      TomlValue::Float(f) => f.to_string(),
      TomlValue::Bool(b) => b.to_string(),
      TomlValue::Array(_) => "[...]".to_string(),
      TomlValue::Object(_) => "{...}".to_string(),
    }
  }

  fn as_string(&self) -> Result<String, String> {
    match self {
      TomlValue::String(s) => Ok(s.clone()),
      TomlValue::Array(v) if v.len() == 1 => v[0].as_string(),
      _ => Err(format!(
        "Expected string, found {} ({})",
        self.type_name(),
        self.debug_value()
      )),
    }
  }

  fn as_int(&self) -> Result<i64, String> {
    match self {
      TomlValue::Int(i) => Ok(*i),
      TomlValue::Array(v) if v.len() == 1 => v[0].as_int(),
      _ => Err(format!(
        "Expected int, found {} ({})",
        self.type_name(),
        self.debug_value()
      )),
    }
  }

  fn as_float(&self) -> Result<f64, String> {
    match self {
      TomlValue::Float(f) => Ok(*f),
      TomlValue::Int(n) => Ok(*n as f64),
      TomlValue::Array(v) if v.len() == 1 => v[0].as_float(),
      _ => Err(format!(
        "Expected float, found {} ({})",
        self.type_name(),
        self.debug_value()
      )),
    }
  }

  fn as_bool(&self) -> Result<bool, String> {
    match self {
      TomlValue::Bool(b) => Ok(*b),
      TomlValue::Array(v) if v.len() == 1 => v[0].as_bool(),
      _ => Err(format!(
        "Expected bool, found {} ({})",
        self.type_name(),
        self.debug_value()
      )),
    }
  }

  fn as_array(&self) -> Result<&Vec<TomlValue>, String> {
    match self {
      TomlValue::Array(v) => Ok(v),
      _ => Err(format!(
        "Expected array, found {} ({})",
        self.type_name(),
        self.debug_value()
      )),
    }
  }

  fn as_object(&self) -> Result<&HashMap<String, TomlValue>, String> {
    match self {
      TomlValue::Object(m) => Ok(m),
      TomlValue::Array(v) if v.len() == 1 => v[0].as_object(),
      _ => Err(format!(
        "Expected object, found {} ({})",
        self.type_name(),
        self.debug_value()
      )),
    }
  }
}

#[derive(Debug)]
enum Frame {
  Array(Vec<TomlValue>),
  Object(HashMap<String, TomlValue>),
}

#[derive(Debug)]
pub struct Config {
  pub backups: HashMap<String, Backup>,
}

impl Config {
  pub fn new() -> Self {
    let mut instance = Self {
      backups: HashMap::new(),
    };

    let config_file = env::var("CONFIG_FILE").unwrap_or("./config.toml".to_string());

    let path = Path::new(&config_file);
    let mut file = File::open(path)
      .unwrap_or_else(|err| panic!("Couldn't open config file {}: {}", path.display(), err));

    let mut content = String::new();
    file
      .read_to_string(&mut content)
      .unwrap_or_else(|err| panic!("Couldn't read file {}: {}", path.display(), err));

    instance
      .parse_config(content)
      .expect("Failed to parse config file");

    instance
  }

  fn parse_config(&mut self, config: String) -> Result<(), String> {
    let mut result = HashMap::new();
    let mut table = String::new();
    let mut i = 0;

    let lines: Vec<String> = config.lines().map(|l| l.to_string()).collect();

    while i < lines.len() {
      let line = Self::strip_comment(&lines[i]);
      i += 1;

      if line.is_empty() {
        continue;
      }

      if line.starts_with('[') && line.ends_with(']') {
        table = line[1..line.len() - 1].to_string();
        result.insert(table.clone(), HashMap::new());
        continue;
      }

      if !line.contains('=') {
        continue;
      }

      let parts: Vec<&str> = line.splitn(2, '=').collect();
      let key = parts[0].trim().to_string();
      let mut value = parts[1].trim().to_string();

      let mut stack = Vec::new();
      Self::scan_symbols(&value, &mut stack);

      while !stack.is_empty() {
        if i >= lines.len() {
          panic!("Unclosed multiline value at line {}", i);
        }
        let next = Self::strip_comment(&lines[i]);
        i += 1;
        value.push(' ');
        value.push_str(&next);
        Self::scan_symbols(&next, &mut stack);
      }

      let parsed = Self::parse_value(value.trim());

      if let Some(table_map) = result.get_mut(&table) {
        table_map.insert(key, parsed);
      } else {
        panic!(
          "Found key '{}' outside of any table in config file at line {}",
          key, i
        );
      }
    }

    for (table, values) in result {
      if table.starts_with("backup.") {
        let backup = Self::parse_backup(&values)?;
        self.backups.insert(table, backup);
      }
    }

    Ok(())
  }

  fn parse_backup(map: &HashMap<String, TomlValue>) -> Result<Backup, String> {
    Ok(Backup {
      display_name: map
        .get("display_name")
        .ok_or("missing display_name")?
        .as_string()?,
      connection_string: map
        .get("connection_string")
        .ok_or("missing connection_string")?
        .as_string()?,
      ignore_collections: map
        .get("ignore_collections")
        .ok_or("missing ignore_collections")?
        .as_array()?
        .iter()
        .map(|v| v.as_string())
        .collect::<Result<_, _>>()?,
      datastore: Self::parse_datastore(map.get("datastore").ok_or("missing datastore")?)?,
      schedule: Self::parse_schedule(map.get("schedule").ok_or("missing schedule")?)?,
      encryption_key: map
        .get("encryption_key")
        .map(|v| v.as_string())
        .transpose()?,
    })
  }

  fn parse_datastore(v: &TomlValue) -> Result<BackupDatastore, String> {
    let obj = v.as_object()?;
    let t = obj
      .get("type")
      .ok_or("missing datastore.type")?
      .as_string()?;
    let storage_type = match t.as_str() {
      "filesystem" => BackupDatastoreType::FileSystem,
      "s3" => BackupDatastoreType::S3,
      _ => return Err("unknown datastore type".into()),
    };

    Ok(BackupDatastore {
      storage_type,
      path: obj
        .get("path")
        .ok_or("missing datastore.path")?
        .as_string()?,
    })
  }

  fn parse_schedule(v: &TomlValue) -> Result<BackupSchedule, String> {
    let obj = v.as_object()?;
    Ok(BackupSchedule {
      enabled: obj
        .get("enabled")
        .ok_or("missing schedule.enabled")?
        .as_bool()?,
      cron: obj
        .get("cron")
        .ok_or("missing schedule.cron")?
        .as_string()?,
    })
  }

  fn strip_comment(line: &str) -> String {
    line.split('#').next().unwrap().trim().to_string()
  }

  fn scan_symbols(text: &str, stack: &mut Vec<char>) {
    let mut in_string = false;
    let mut escape = false;

    for ch in text.chars() {
      if in_string {
        if escape {
          escape = false;
        } else if ch == '\\' {
          escape = true;
        } else if ch == '"' {
          in_string = false;
          stack.pop();
        }
      } else {
        match ch {
          '"' => {
            in_string = true;
            stack.push('"');
          }
          '{' | '[' => stack.push(ch),
          '}' => {
            if stack.last() == Some(&'{') {
              stack.pop();
            }
          }
          ']' => {
            if stack.last() == Some(&'[') {
              stack.pop();
            }
          }
          _ => {}
        }
      }
    }
  }

  fn parse_value(text: &str) -> TomlValue {
    let mut chars = text.trim().chars().peekable();
    let mut stack: Vec<Frame> = Vec::new();
    let mut current_key: Option<String> = None;

    while let Some(c) = chars.next() {
      match c {
        '"' => {
          let mut s = String::new();
          while let Some(ch) = chars.next() {
            if ch == '"' {
              break;
            }
            s.push(ch);
          }
          Self::push_value(&mut stack, &mut current_key, TomlValue::String(s));
        }
        '[' => stack.push(Frame::Array(Vec::new())),
        '{' => stack.push(Frame::Object(HashMap::new())),
        ']' | '}' => {
          let frame = stack.pop().unwrap();
          let value = match frame {
            Frame::Array(v) => TomlValue::Array(v),
            Frame::Object(m) => TomlValue::Object(m),
          };
          Self::push_value(&mut stack, &mut current_key, value);
        }
        ',' => current_key = None,
        c if c == '=' || c.is_whitespace() => {}
        _ => {
          let mut token = String::new();
          token.push(c);
          while let Some(&ch) = chars.peek() {
            if ch == ',' || ch == ']' || ch == '}' || ch.is_whitespace() {
              break;
            }
            token.push(chars.next().unwrap());
          }

          let value = if token == "true" {
            TomlValue::Bool(true)
          } else if token == "false" {
            TomlValue::Bool(false)
          } else if let Ok(n) = token.parse::<i64>() {
            TomlValue::Int(n)
          } else if let Ok(f) = token.parse::<f64>() {
            TomlValue::Float(f)
          } else {
            if matches!(stack.last(), Some(Frame::Object(_))) && current_key.is_none() {
              current_key = Some(token);
              continue;
            }
            TomlValue::String(token)
          };

          Self::push_value(&mut stack, &mut current_key, value);
        }
      }
    }

    if stack.len() == 1 {
      match stack.pop().unwrap() {
        Frame::Array(v) => {
          if v.len() == 1 {
            v.into_iter().next().unwrap()
          } else {
            TomlValue::Array(v)
          }
        }
        Frame::Object(m) => TomlValue::Object(m),
      }
    } else if stack.is_empty() {
      TomlValue::String(text.to_string())
    } else {
      panic!("Malformed value parsing: {:?}", stack);
    }
  }

  fn push_value(stack: &mut Vec<Frame>, current_key: &mut Option<String>, value: TomlValue) {
    if let Some(frame) = stack.last_mut() {
      match frame {
        Frame::Array(v) => v.push(value),
        Frame::Object(m) => {
          let key = current_key.take().expect("Missing key for object value");
          m.insert(key, value);
        }
      }
    } else {
      stack.push(Frame::Array(vec![value]));
    }
  }
}

#[cfg(test)]
mod tests {
  use std::{collections::HashMap, fs::write};

  use crate::utils::config::{
    Backup, BackupDatastore, BackupDatastoreType, BackupSchedule, Config,
  };

  const CONFIG_1: &str = r#"[backup.cool]
display_name = "Cool Backup"
connection_string = "mongodb://root:password@mongodb.example.com/database"
ignore_collections = [ "GlobalStats" ]
datastore = { type = "filesystem", path = "/data/mongo-backups" }
schedule = { enabled = true, cron = "0 0 * * *" }
encryption_key = "azertyuiop""#;
  const CONFIG_2: &str = r#"[backup.awesome]
display_name = "Awesome Backup"
connection_string = "mongodb://root:password@mongodb.awesome.com/database"
ignore_collections = [ "Collection123" ]
datastore = { type = "s3", path = "/backups-dir" }
schedule = { enabled = true, cron = "0 */5 * * *" }
encryption_key = "poiuytreza""#;

  #[test]
  fn config_parse_config() {
    let _ = write("./config.toml", CONFIG_1);
    let config = Config::new();
    let mut expected_backups: HashMap<String, Backup> = HashMap::new();
    expected_backups
      .entry("backup.cool".to_string())
      .insert_entry(Backup {
        display_name: String::from("Cool Backup"),
        connection_string: String::from("mongodb://root:password@mongodb.example.com/database"),
        ignore_collections: Vec::from([String::from("GlobalStats")]),
        datastore: BackupDatastore {
          path: String::from("/data/mongo-backups"),
          storage_type: BackupDatastoreType::FileSystem,
        },
        schedule: BackupSchedule {
          enabled: true,
          cron: String::from("0 0 * * *"),
        },
        encryption_key: Some(String::from("azertyuiop")),
      });

    for (key, _) in expected_backups.iter() {
      assert_eq!(config.backups.get(key), expected_backups.get(key))
    }
  }

  /*#[test]
  fn config_parse_config_multiple_backups() {
    let _ = write("./config.toml", format!("{CONFIG_1}\n\n{CONFIG_2}"));
    let config = Config::new();
    let mut expected_backups: HashMap<String, Backup> = HashMap::new();
    expected_backups
      .entry("backup.cool".to_string())
      .insert_entry(Backup {
        display_name: String::from("Cool Backup"),
        connection_string: String::from("mongodb://root:password@mongodb.example.com/database"),
        ignore_collections: Vec::from([String::from("GlobalStats")]),
        datastore: BackupDatastore {
          path: String::from("/data/mongo-backups"),
          storage_type: BackupDatastoreType::FileSystem,
        },
        schedule: BackupSchedule {
          enabled: true,
          cron: String::from("0 0 * * *"),
        },
        encryption_key: Some(String::from("azertyuiop")),
      });
    expected_backups
      .entry("backup.awesome".to_string())
      .insert_entry(Backup {
        display_name: String::from("Awesome Backup"),
        connection_string: String::from("mongodb://root:password@mongodb.awesome.com/database"),
        ignore_collections: Vec::from([String::from("Collection123")]),
        datastore: BackupDatastore {
          path: String::from("/backups-dir"),
          storage_type: BackupDatastoreType::S3,
        },
        schedule: BackupSchedule {
          enabled: true,
          cron: String::from("0 *\/5 * * *"),
        },
        encryption_key: Some(String::from("poiuytreza")),
      });

    for (key, _) in expected_backups.iter() {
      assert_eq!(config.backups.get(key), expected_backups.get(key))
    }
  }

  #[test]
  #[should_panic]
  fn config_parse_config_unknown_file() {
    let _ = remove_file("./config.toml");
    let _ = Config::new();
  }*/
}
