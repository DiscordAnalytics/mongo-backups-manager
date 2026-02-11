use std::collections::HashMap;

use cronexpr::jiff::Zoned;

use crate::{
  Daemon,
  utils::{config::Config, logger::Logger},
};

pub fn list() {
  let config = Config::new();

  if config.backups.len() == 0 {
    println!("No backup jobs found")
  }

  let mut next_schedules: HashMap<String, Zoned> = HashMap::new();

  for (backup_name, backup) in config.backups.iter() {
    let backup_schedule = match cronexpr::parse_crontab(backup.schedule.cron.as_str()) {
      Ok(b) => b,
      Err(err) => {
        Logger::error(err.to_string().as_str());
        Logger::error(format!("Invalid cron string for backup `{}`", backup.display_name).as_str());
        continue;
      }
    };

    let next = match Daemon::get_next_cron_run(&backup_schedule) {
      Ok(n) => n,
      Err(e) => {
        Logger::error(format!("Invalid cron schedule: {e}").as_str());
        return;
      }
    };

    next_schedules.insert(backup_name.clone(), next);
  }

  let mut sorted_schedules: Vec<(String, Zoned)> = next_schedules.into_iter().collect();
  sorted_schedules.sort_by(|a, b| a.1.cmp(&b.1));

  for (backup_name, next_schedule) in sorted_schedules {
    println!("- {backup_name} - Next run: {:?}", next_schedule);
  }
}
