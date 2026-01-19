use std::{collections::HashMap, env, fs::File, io::Read, path::Path};

#[derive(Debug)]
enum BackupDatastoreType {
    FileSystem,
    S3,
}

#[derive(Debug)]
struct BackupDatastore {
    storage_type: BackupDatastoreType,
    path: String,
}

#[derive(Debug)]
struct BackupSchedule {
    enabled: bool,
    cron: String,
}

#[derive(Debug)]
struct Backup {
    display_name: String,
    connection_string: String,
    ignore_collections: Vec<String>,
    datastore: BackupDatastore,
    schedule: BackupSchedule,
    encryption_key: Option<String>,
}

#[derive(Debug)]
enum TomlValue {
    String(String),
    Int(i64),
    Bool(bool),
    Object(HashMap<String, TomlValue>),
    Array(Vec<TomlValue>),
}

impl TomlValue {
    fn as_string(&self) -> Result<String, String> {
        match self {
            TomlValue::String(s) => Ok(s.clone()),
            TomlValue::Array(v) if v.len() == 1 => v[0].as_string(),
            _ => Err("Expected string".into()),
        }
    }

    fn as_bool(&self) -> Result<bool, String> {
        match self {
            TomlValue::Bool(b) => Ok(*b),
            TomlValue::Array(v) if v.len() == 1 => v[0].as_bool(),
            _ => Err("Expected bool".into()),
        }
    }

    fn as_array(&self) -> Result<&Vec<TomlValue>, String> {
        match self {
            TomlValue::Array(v) => Ok(v),
            _ => Err("Expected array".into()),
        }
    }

    fn as_object(&self) -> Result<&std::collections::HashMap<String, TomlValue>, String> {
        match self {
            TomlValue::Object(m) => Ok(m),
            TomlValue::Array(v) if v.len() == 1 => v[0].as_object(),
            _ => Err("Expected object".into()),
        }
    }
}

enum Frame {
    Array(Vec<TomlValue>),
    Object(HashMap<String, TomlValue>),
}

#[derive(Debug)]
pub struct Config {
    backups: HashMap<String, Backup>,
}

impl Config {
    pub fn new() -> Self {
        let mut instance = Self {
            backups: HashMap::new(),
        };

        let config_file = env::var("CONFIG_FILE").unwrap_or("./config.toml".to_string());

        let path = Path::new(&config_file);
        let mut file = match File::open(path) {
            Ok(file) => file,
            Err(err) => panic!("Couldn't open config file {}: {}", path.display(), err),
        };

        let mut content = String::new();
        let _ = match file.read_to_string(&mut content) {
            Ok(_) => instance.parse_config(content.clone()),
            Err(err) => panic!("Couldn't read file {}: {}", path.display(), err),
        };

        instance
    }

    fn parse_config(&mut self, config: String) -> Result<(), String> {
        let mut result = HashMap::new();
        let mut section = String::new();
        let mut i = 0;

        let lines: Vec<String> = config.lines().map(|l| l.to_string()).collect();

        while i < lines.len() {
            let line = Self::strip_comment(&lines[i]);
            i += 1;

            if line.is_empty() {
                continue;
            }

            if line.starts_with('[') && line.ends_with(']') {
                section = line[1..line.len() - 1].to_string();
                result.insert(section.clone(), HashMap::new());
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
                    panic!("Unclosed multiline value");
                }
                let next = Self::strip_comment(&lines[i]);
                i += 1;
                value.push(' ');
                value.push_str(&next);
                Self::scan_symbols(&next, &mut stack);
            }

            let parsed = Self::parse_value(value.trim());

            if let Some(section_map) = result.get_mut(&section) {
                section_map.insert(key, parsed);
            } else {
                panic!("Found key '{}' outside of any section in config file", key);
            }
        }

        for (section, values) in result {
            if section.starts_with("backup.") {
                let backup = Self::parse_backup(&values)?;
                let key = if !backup.display_name.is_empty() {
                    backup.display_name.clone()
                } else {
                    section.clone()
                };
                self.backups.insert(key, backup);
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

        match stack.pop().unwrap() {
            Frame::Array(v) => TomlValue::Array(v),
            Frame::Object(m) => TomlValue::Object(m),
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
