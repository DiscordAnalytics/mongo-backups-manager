use std::{ffi::OsStr, path::Path, process::exit};

use chrono::DateTime;
use cronexpr::jiff::Zoned;

use crate::{
  cli::commands::Daemon,
  datastores::{Datastore, FilesystemDatastore},
  utils::{
    config::{BackupDatastoreType, Config},
    logger::Logger,
  },
};

pub fn inspect(name: String) {
  let config = Config::new();
  let Some((_, backup_config)) = config
    .backups
    .iter()
    .find(|(backup_name, _)| **backup_name == format!("backup.{name}"))
  else {
    Logger::error("Backup not found");
    exit(1);
  };

  let datastore_path = Path::new(backup_config.datastore.path.as_str());
  let datastore = match backup_config.datastore.storage_type {
    BackupDatastoreType::FileSystem => FilesystemDatastore::new(datastore_path).unwrap(),
    BackupDatastoreType::S3 => todo!(),
  };

  let backup_schedule = cronexpr::parse_crontab(backup_config.schedule.cron.as_str()).ok();

  let next: Option<Zoned> = match backup_schedule {
    Some(value) => match Daemon::get_next_cron_run(&value) {
      Ok(n) => Some(n),
      Err(e) => {
        Logger::error(format!("Invalid cron schedule: {e}").as_str());
        None
      }
    },
    None => None,
  };

  let backup_dir_prefix = format!("backup_{name}_");
  let backups = datastore
    .list_backups()
    .unwrap_or_else(|_| Vec::new())
    .into_iter()
    .filter_map(|bckp| {
      let dir_name = bckp
        .base_path
        .file_name()
        .unwrap_or(OsStr::new(""))
        .to_str()
        .unwrap_or("")
        .to_string();

      if dir_name.starts_with(&backup_dir_prefix) {
        Some((dir_name, bckp))
      } else {
        None
      }
    });

  println!("-- {} --", backup_config.display_name);
  println!("Datastore:");
  println!("\tType: {:?}", backup_config.datastore.storage_type);
  println!("\tPath: {:?}", backup_config.datastore.path);
  println!(
    "Schedule: {}",
    if backup_config.schedule.enabled {
      backup_config.schedule.cron.clone()
    } else {
      "disabled".to_string()
    }
  );
  if let Some(next) = next {
    println!("Next run: {:?}", next);
  }
  println!(
    "Encryption: {}",
    if backup_config.encryption_key.is_some() {
      "enabled"
    } else {
      "disabled"
    }
  );
  println!("Available backups:");
  for (dir_name, datastore) in backups {
    let health_state = datastore.check_backup_integrity().is_ok_and(|res| res);
    let timestamp = dir_name
      .split("_")
      .last()
      .unwrap_or("0")
      .parse::<i64>()
      .unwrap_or(0);
    let date = match DateTime::from_timestamp_secs(timestamp) {
      Some(value) => value.to_rfc3339(),
      None => "Unknown date".to_string(),
    };
    println!(
      "\t{} - {}",
      date,
      if health_state { "Healthy" } else { "Unhealthy" }
    )
  }
}
