use std::{path::Path, time::Duration};

use chrono::Local;
use cronexpr::{Crontab, jiff::Zoned};
use futures::stream::TryStreamExt;
use mongodb::{
  Collection, Database, IndexModel,
  bson::{Document, doc},
  options::{CreateCollectionOptions, FindOptions},
  results::CollectionSpecification,
};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

use crate::{
  datastores::{Datastore, FilesystemDatastore},
  db::DatabaseConnection,
  utils::{
    config::{Backup, BackupDatastoreType, Config},
    logger::Logger,
  },
};

#[derive(Deserialize, Serialize)]
struct DatabaseCollectionHeader {
  name: String,
  options: CreateCollectionOptions,
  indexes: Vec<IndexModel>,
  documents_count: u64,
  data: Vec<Document>,
}

pub struct Daemon {}

impl Daemon {
  const DOCUMENTS_BATCH_SIZE: u32 = 1000;

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

              Self::start_backup_job(backup_clone.clone())
                .await
                .map_err(|e| format!("Backup job failed: {e}"))
                .unwrap();

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

  async fn start_backup_job(backup: Backup) -> Result<(), String> {
    let connection = DatabaseConnection::new()
      .connect(backup.connection_string.as_str())
      .await
      .map_err(|err| format!("Failed to connected to MongoDB Server: {err}"))?;

    let db = connection
      .client()
      .unwrap()
      .database(backup.database_name.as_str());

    let all_collection_names = db
      .list_collection_names()
      .await
      .map_err(|e| format!("Failed to fetch collection names: {e}"))?;
    let collections_count = all_collection_names.iter().count();
    Logger::info(
      format!(
        "Backing up {} collections from {}",
        collections_count, backup.database_name
      )
      .as_str(),
    );

    let mut collections_cursor = db
      .list_collections()
      .await
      .map_err(|e| format!("Failed to fetch collections: {e}"))?;
    let backup_timestamp = Local::now().timestamp();
    let datastore_path = Path::new(backup.datastore.path.as_str()).join(format!(
      "backup_{}_{}",
      backup.database_name, backup_timestamp
    ));
    let datastore = match backup.datastore.storage_type {
      BackupDatastoreType::FileSystem => FilesystemDatastore::new(datastore_path.to_str().unwrap()),
      BackupDatastoreType::S3 => todo!(),
    };

    while let Some(collection_specs) = collections_cursor
      .try_next()
      .await
      .map_err(|e| format!("Failed to read collection: {e}"))?
    {
      if collection_specs.name.starts_with("system.")
        || backup.ignore_collections.contains(&collection_specs.name)
      {
        continue;
      }

      Self::backup_collection(collection_specs.clone(), db.clone(), &datastore).await?;

      Logger::info(format!("Backed up collection {}", collection_specs.name).as_str());
    }

    Ok(())
  }

  async fn backup_collection(
    collection_specs: CollectionSpecification,
    db: Database,
    datastore: &impl Datastore,
  ) -> Result<(), String> {
    let collection: Collection<Document> = db.collection(collection_specs.name.as_str());

    let mut write_stream = datastore
      .open_write_stream(format!("{}.json", collection.name()).as_str())
      .await
      .map_err(|e| format!("Failed to open write stream: {e}"))?;

    let collection_indexes = collection
      .list_indexes()
      .await
      .map_err(|e| format!("Failed to fetch collection indexes: {e}"))?
      .try_collect()
      .await
      .map_err(|e| format!("Failed to fetch collection indexes: {e}"))?;

    let documents_count = collection
      .count_documents(doc! {})
      .await
      .map_err(|e| format!("Failed to count documents: {e}"))?;

    let collection_header = DatabaseCollectionHeader {
      name: collection_specs.name,
      options: collection_specs.options,
      indexes: collection_indexes,
      documents_count,
      data: Vec::new(),
    };

    let mut collection_header_string =
      serde_json::to_string(&collection_header).map_err(|e| e.to_string())?;
    collection_header_string.truncate(collection_header_string.len() - 2);

    write_stream
      .write_all(collection_header_string.as_ref())
      .await
      .map_err(|e| format!("Failed to write document: {e}"))?;

    let options = FindOptions::builder()
      .batch_size(Self::DOCUMENTS_BATCH_SIZE)
      .build();

    let mut cursor = collection
      .find(doc! {})
      .with_options(options)
      .await
      .map_err(|e| format!("Failed to fetch collection data: {e}"))?;

    let mut saved_documents = 0u64;
    while let Some(document) = cursor
      .try_next()
      .await
      .map_err(|e| format!("Failed to read document: {e}"))?
    {
      saved_documents += 1;
      let json_document = serde_json::to_value(document)
        .map_err(|e| format!("Failed to parse JSON from document: {e}"))?;

      let json_string = json_document.to_string()
        + if saved_documents == collection_header.documents_count {
          ""
        } else {
          ","
        };

      write_stream
        .write_all(json_string.as_ref())
        .await
        .map_err(|e| format!("Failed to write document: {e}"))?;
    }

    write_stream
      .write_all(b"]}")
      .await
      .map_err(|e| e.to_string())?;

    write_stream
      .flush()
      .await
      .map_err(|e| format!("Failed to close write stream: {e}"))?;

    Ok(())
  }
}
