use std::time::Duration;

use crate::utils::{Config, Log};
use chrono::Local;
use futures::StreamExt;

pub struct Daemon;

impl Daemon {
  pub async fn start() {
    let config = Config::new();
    Log::info(format!("Loaded {} backups from config file", config.backups.len()).as_str());

    for (_, backup_job) in config.backups.into_iter() {
      if backup_job.schedule.is_some() {
        let backup_job_clone = backup_job.clone();
        tokio::spawn(async move {
          let mut next = backup_job_clone.get_next_run();

          loop {
            let now = Local::now();
            if next.is_some() && now.timestamp() == next.clone().unwrap().timestamp().as_second() {
              Log::info(
                format!("Starting backup job `{}`", backup_job_clone.display_name).as_str(),
              );

              let job = backup_job_clone.clone();
              let mut events = Box::pin(job.execute());

              while let Some(event) = events.next().await {
                Log::from_stream_event(event);
              }

              next = backup_job_clone.get_next_run();
              Log::info(
                format!(
                  "Backup job `{}` done. Next run: {:?}",
                  backup_job_clone.display_name, next
                )
                .as_str(),
              );
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
          }
        });

        match backup_job.get_next_run() {
          Some(date) => Log::info(
            format!(
              "Scheduled backup `{}`. Next run: {}",
              backup_job.display_name, date
            )
            .as_str(),
          ),
          None => {
            Log::error(format!("Failed to get next run for `{}`", backup_job.display_name).as_str())
          }
        }
      } else {
        Log::info(format!("Skipped backup `{}` schedule", backup_job.display_name).as_str());
      }
    }

    loop {
      tokio::time::sleep(Duration::from_secs(60)).await;
    }
  }
}
