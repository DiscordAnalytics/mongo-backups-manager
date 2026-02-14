use std::process::exit;

use futures::StreamExt;

use crate::utils::{Config, Log, logger::StreamEvent};

pub async fn start(name: String) {
  let config = Config::new();
  let Some(backup_job) = config
    .backups
    .values()
    .find(|backup_job| backup_job.identifier == name)
  else {
    Log::error("Backup not found");
    exit(1);
  };

  Log::info(format!("Starting backup job {}...", backup_job.display_name).as_str());

  let mut events = Box::pin(backup_job.execute());

  let mut has_errors = false;
  while let Some(event) = events.next().await {
    if matches!(event, StreamEvent::Error(_)) && !has_errors {
      has_errors = true;
    }
    Log::from_stream_event(event);
  }

  if !has_errors {
    Log::highlight(
      format!(
        "Backup job {} successfully executed",
        backup_job.display_name
      )
      .as_str(),
    )
  }
}
