use crate::utils::Config;
use crate::utils::backup_manager::BackupJob;

pub fn list() {
  let config = Config::new();

  if config.backups.is_empty() {
    println!("No backup jobs found")
  }

  let mut backup_jobs: Vec<&BackupJob> = config.backups.values().collect();
  backup_jobs.sort_by(|a, b| a.get_next_run().cmp(&b.get_next_run()));

  for backup_job in backup_jobs {
    println!(
      "- {} - Next run: {:?}",
      backup_job.identifier,
      backup_job.get_next_run()
    );
  }
}
