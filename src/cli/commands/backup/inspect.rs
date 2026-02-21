use std::process::exit;

use chrono::DateTime;

use crate::{
  datastores::{Datastore, DatastoreTrait},
  utils::{Config, Log},
};

pub fn inspect(name: String) {
  let config = Config::new();
  let Some(backup_job) = config
    .backups
    .values()
    .find(|backup_job| backup_job.identifier == name)
  else {
    Log::error("Backup not found");
    exit(1);
  };

  let backup_dir_prefix = format!("backup_{name}_");
  let backups = match &backup_job.datastore {
    Datastore::FileSystem(store) => store
      .list_backups()
      .unwrap_or_default()
      .into_iter()
      .filter_map(|store| {
        let dir_name = store.base_path.file_name()?.to_str()?.to_string();

        if dir_name.starts_with(&backup_dir_prefix) {
          Some((dir_name, Datastore::FileSystem(store)))
        } else {
          None
        }
      })
      .collect::<Vec<_>>(),

    Datastore::S3(store) => store
      .list_backups()
      .unwrap_or_default()
      .into_iter()
      .filter_map(|store| {
        let dir_name = store.base_path.file_name()?.to_str()?.to_string();

        if dir_name.starts_with(&backup_dir_prefix) {
          Some((dir_name, Datastore::S3(store)))
        } else {
          None
        }
      })
      .collect::<Vec<_>>(),
  };

  let datastore_type = backup_job.datastore.as_str();
  let datastore_base_path = match &backup_job.datastore {
    Datastore::FileSystem(store) => store.base_path.display().to_string(),
    Datastore::S3(store) => store.base_path.display().to_string(),
  };

  println!("-- {} --", backup_job.display_name);
  println!("Datastore:");
  println!("\tType: {}", datastore_type);
  println!("\tPath: {}", datastore_base_path);
  println!(
    "Schedule: {:?}",
    if let Some(schedule) = backup_job.clone().raw_schedule {
      schedule
    } else {
      "disabled".to_string()
    }
  );
  if let Some(next) = backup_job.get_next_run() {
    println!("Next run: {:?}", next);
  }
  println!(
    "Encryption: {}",
    if backup_job.is_encryption_enabled() {
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
      "\t{} - {} - {}",
      dir_name,
      date,
      if health_state { "Healthy" } else { "Unhealthy" }
    )
  }
}
