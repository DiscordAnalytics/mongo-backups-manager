use std::time::Duration;

use chrono::Local;
use cronexpr::{Crontab, jiff::Zoned};

use crate::utils::{config::Config, logger::Logger};

pub struct Daemon {}

impl Daemon {
  pub async fn start() {
    let config = Config::new();
    Logger::info(format!("Loaded {} backups from config file", config.backups.len()).as_str());

    for (_, backup) in config.backups.into_iter() {
      if backup.schedule.enabled {
        let backup_schedule = cronexpr::parse_crontab(backup.schedule.cron.as_str());
        if backup_schedule.is_err() {
          Logger::error(backup_schedule.err().unwrap().to_string().as_str());
          Logger::error(
            format!(
              "Invalid cron string for backup `{}`. Skipped schedule",
              backup.display_name
            )
            .as_str(),
          );
          continue;
        }
        let backup_schedule = backup_schedule.unwrap();

        let backup_schedule_clone = backup_schedule.clone();
        let backup_clone = backup.clone();
        tokio::spawn(async move {
          let mut next = Self::get_next_cron_run(backup_schedule_clone.clone());

          loop {
            let now = Local::now();
            if now.timestamp() == next.clone().unwrap().timestamp().as_second() {
              Logger::info(format!("Starting backup job `{}`", backup_clone.display_name).as_str());
              next = Self::get_next_cron_run(backup_schedule_clone.clone());
              Logger::info(
                format!(
                  "Backup job `{}` done. Next run: {}",
                  backup_clone.display_name,
                  next.clone().unwrap()
                )
                .as_str(),
              );
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
          }
        });

        let date = Self::get_next_cron_run(backup_schedule.clone());
        Logger::info(
          format!(
            "Scheduled backup `{}`. Next run: {}",
            backup.display_name,
            date.unwrap()
          )
          .as_str(),
        );
      } else {
        Logger::info(format!("Skipped backup `{}` schedule", backup.display_name).as_str());
      }
    }

    loop {
      tokio::time::sleep(Duration::from_secs(60)).await;
    }
  }

  fn get_next_cron_run(schedule: Crontab) -> Result<Zoned, cronexpr::Error> {
    let now = Local::now();
    schedule.find_next(now.to_rfc3339().as_str())
  }
}
