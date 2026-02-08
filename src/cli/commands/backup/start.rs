use std::process::exit;

use crate::{
  Daemon,
  utils::{config::Config, logger::Logger},
};

pub async fn start(name: String) {
  let config = Config::new();
  let backup_config = config
    .backups
    .iter()
    .find(|(backup_name, _)| **backup_name == format!("backup.{name}"));

  if backup_config.is_none() {
    Logger::error("Backup not found");
    exit(1);
  }
  let backup_config = backup_config.unwrap().1;

  Logger::info(format!("Starting backup job {}...", backup_config.display_name).as_str());
  match Daemon::start_backup_job(backup_config.clone()).await {
    Ok(_) => Logger::highlight(
      format!(
        "Backup job {} successfully executed",
        backup_config.display_name
      )
      .as_str(),
    ),
    Err(err) => Logger::error(err.as_str()),
  };
}
