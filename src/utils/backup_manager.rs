use std::{collections::HashMap, path::Path};

use async_stream::stream;
use chrono::Local;
use cronexpr::{Crontab, jiff::Zoned};
use futures::{Stream, StreamExt, TryStreamExt};
use mongodb::{
  Collection, Database, IndexModel,
  bson::{Document, doc},
  options::{CreateCollectionOptions, FindOptions},
  results::CollectionSpecification,
};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader};

use crate::{
  datastores::{Datastore, DatastoreTrait, FilesystemDatastore, S3Datastore},
  db::DatabaseConnection,
  utils::{
    config::{BackupDatastore, BackupDatastoreType},
    logger::StreamEvent,
  },
};

const DOCUMENTS_BATCH_SIZE: u32 = 1000;

#[derive(Deserialize, Serialize)]
struct DatabaseCollectionHeader {
  name: String,
  options: CreateCollectionOptions,
  indexes: Vec<IndexModel>,
  documents_count: u64,
  data: Vec<Document>,
}

#[derive(Deserialize, Serialize)]
struct DatabaseCollectionHeaderWithoutData {
  name: String,
  options: CreateCollectionOptions,
  indexes: Vec<IndexModel>,
  documents_count: u64,
}

#[derive(Deserialize, Serialize)]
pub struct DatabaseMetadata {
  pub name: String,
  pub collection_hashes: HashMap<String, String>,
}
#[derive(Debug, Clone)]
pub struct BackupJob {
  pub identifier: String,
  pub display_name: String,
  pub database_name: String,
  pub ignore_collections: Vec<String>,
  pub schedule: Option<Crontab>,
  pub raw_schedule: Option<String>,
  connection_string: String,
  encryption_key: Option<String>,
  pub datastore: Datastore,
}

impl PartialEq for BackupJob {
  fn eq(&self, other: &Self) -> bool {
    self.identifier == other.identifier
      && self.display_name == other.display_name
      && self.database_name == other.database_name
      && self.ignore_collections == other.ignore_collections
      && self.connection_string == other.connection_string
      && self.encryption_key == other.encryption_key
      && self.datastore == other.datastore
  }
}

impl BackupJob {
  pub fn new(
    identifier: String,
    display_name: String,
    database_name: String,
    ignore_collections: Vec<String>,
    schedule: Option<String>,
    connection_string: String,
    encryption_key: Option<String>,
    datastore: BackupDatastore,
  ) -> Result<Self, String> {
    let datastore = match datastore.storage_type {
      BackupDatastoreType::FileSystem => {
        let store = FilesystemDatastore::new(Path::new(datastore.path.as_str()))
          .map_err(|e| format!("Failed to create datastore: {}", e))?;
        Datastore::FileSystem(store)
      }
      BackupDatastoreType::S3 => {
        let store = S3Datastore::new(Path::new(datastore.path.as_str()))
          .map_err(|e| format!("Failed to create datastore: {}", e))?;
        Datastore::S3(store)
      }
    };

    Ok(Self {
      identifier,
      display_name,
      database_name,
      ignore_collections,
      schedule: if let Some(schedule) = schedule.clone() {
        Some(cronexpr::parse_crontab(schedule.as_str()).map_err(|e| e.to_string())?)
      } else {
        None
      },
      raw_schedule: schedule,
      connection_string,
      encryption_key,
      datastore,
    })
  }

  pub fn get_next_run(&self) -> Option<Zoned> {
    let now = Local::now();
    if let Some(schedule) = self.schedule.clone() {
      schedule.find_next(now.to_rfc3339().as_str()).ok()
    } else {
      None
    }
  }

  pub fn execute(&self) -> impl Stream<Item = StreamEvent> {
    stream! {
      let current_time = Local::now().timestamp();
      let backup_dir = format!("backup_{}_{}", self.identifier, current_time);

      let db = match self.connect_to_mongodb().await {
        Ok(c) => c.database(self.database_name.as_str()),
        Err(err) => return yield StreamEvent::Error(err),
      };

      let all_collection_names = match db.list_collection_names().await {
        Ok(names) => names,
        Err(err) => return yield StreamEvent::Error(format!("Failed to fetch collection names: {err}")),
      };
      let collections_count = all_collection_names.len();
      yield StreamEvent::Info(format!("Backup up {} collections from {}", collections_count, self.database_name));

      let mut collections_cursor = match db.list_collections().await {
        Ok(cursor) => cursor,
        Err(err) => return yield StreamEvent::Error(format!("Failed to fetch collections: {err}")),
      };

      while let Ok(Some(collection_specs)) = collections_cursor.try_next().await {
        if collection_specs.name.starts_with("system.")
          || self.ignore_collections.contains(&collection_specs.name)
        {
          continue;
        }
        yield StreamEvent::Info(format!("Backing up collection {}", collection_specs.name));

        let mut events = Box::pin(self.backup_collection(collection_specs.clone(), db.clone(), backup_dir.clone()));

        while let Some(event) = events.next().await {
          yield event;
        }

        yield StreamEvent::Info(format!("Backed up collection {}", collection_specs.name));
      }

      yield StreamEvent::Info(format!("Finalizing backup `{}`...", self.display_name));

      match self.finalize_backup(db, backup_dir).await {
        Ok(_) => {},
        Err(err) => return yield StreamEvent::Error(err),
      };
    }
  }

  fn backup_collection(
    &self,
    collection_specs: CollectionSpecification,
    db: Database,
    backup_dir: String,
  ) -> impl Stream<Item = StreamEvent> {
    stream! {
      let collection: Collection<Document> = db.collection(collection_specs.name.as_str());

      let mut write_stream = match self.datastore.open_write_stream(format!("{}/{}.json", backup_dir, collection.name()).as_str()).await {
        Ok(stream) => stream,
        Err(err) => return yield StreamEvent::Error(format!("Failed to open write stream: {err}")),
      };

      let collection_indexes: Vec<IndexModel> = match collection.list_indexes().await {
        Ok(indexes_cursor) => match indexes_cursor.try_collect().await {
          Ok(indexes) => indexes,
          Err(err) => return yield StreamEvent::Error(format!("Failed to fetch collection indexes: {err}")),
        },
        Err(err) => return yield StreamEvent::Error(format!("Failed to fetch collection indexes: {err}")),
      };

      let documents_count = match collection.count_documents(doc! {}).await {
        Ok(count) => count,
        Err(err) => return yield StreamEvent::Error(format!("Failed to count documents: {err}")),
      };

      let collection_header = DatabaseCollectionHeader {
        name: collection_specs.name,
        options: collection_specs.options,
        indexes: collection_indexes,
        documents_count,
        data: Vec::new(),
      };

      let mut collection_header_string = match serde_json::to_string(&collection_header) {
        Ok(value) => value,
        Err(err) => return yield StreamEvent::Error(err.to_string()),
      };
      collection_header_string.truncate(collection_header_string.len() - 2);

      if let Err(err) = write_stream.write_all(collection_header_string.as_ref()).await {
          return yield StreamEvent::Error(format!("Failed to write document: {err}"));
      };

      let options = FindOptions::builder()
        .batch_size(DOCUMENTS_BATCH_SIZE)
        .build();

      let mut cursor = match collection.find(doc! {}).with_options(options).await {
        Ok(cursor) => cursor,
        Err(err) => return yield StreamEvent::Error(format!("Failed to fetch collection data: {err}")),
      };

      let mut saved_documents = 0u64;
      while let Ok(Some(document)) = cursor.try_next().await {
        saved_documents += 1;
        let json_document = match serde_json::to_value(document) {
          Ok(value) => value,
          Err(err) => return yield StreamEvent::Error(format!("Failed to parse JSON from document: {err}")),
        };

        let json_string = json_document.to_string() + if saved_documents == collection_header.documents_count { "" } else { "," };

        if let Err(err) = write_stream.write_all(json_string.as_ref()).await {
          return yield StreamEvent::Error(format!("Failed to write document: {err}"));
        };
      }

      if let Err(err) = write_stream.write_all(b"]}").await {
        return yield StreamEvent::Error(format!("Failed to write document: {err}"));
      };

      if let Err(err) = write_stream.flush().await {
        return yield StreamEvent::Error(format!("Failed to write document: {err}"));
      };
    }
  }

  async fn finalize_backup(&self, db: Database, backup_dir: String) -> Result<(), String> {
    let collection_files = self.datastore.list_objects(backup_dir.clone())?;
    let mut collection_hashes: HashMap<String, String> = HashMap::new();

    for file in collection_files {
      let file_name = file
        .split_once('.')
        .map(|(name, _)| name)
        .unwrap_or(&file)
        .to_string();
      let file_hash = self
        .datastore
        .get_object_hash(format!("{}/{file}", backup_dir.clone()))?;

      collection_hashes.insert(file_name, file_hash);
    }

    let meta = DatabaseMetadata {
      name: db.name().to_string(),
      collection_hashes,
    };

    let json_string =
      serde_json::to_string(&meta).map_err(|e| format!("Failed to parse JSON: {e}"))?;

    self
      .datastore
      .put_object(&(backup_dir + "/.database.json"), json_string.as_ref())?;

    Ok(())
  }

  pub fn is_encryption_enabled(&self) -> bool {
    self.encryption_key.is_some()
  }

  pub fn restore_backup_to_database(
    &self,
    backup_dir: String,
    target_database_name: Option<String>,
  ) -> impl Stream<Item = StreamEvent> {
    stream! {
      yield StreamEvent::Info(format!("Starting restore from backup directory: {}", backup_dir));

      let backup_datastore = match self.create_backup_datastore(&backup_dir) {
        Ok(store) => store,
        Err(err) => return yield StreamEvent::Error(err),
      };

      yield StreamEvent::Info("Checking backup integrity...".to_string());
      match backup_datastore.check_backup_integrity() {
        Ok(true) => yield StreamEvent::Info("Backup integrity check passed".to_string()),
        Ok(false) => return yield StreamEvent::Error("Backup integrity check failed: corrupted backup".to_string()),
        Err(err) => return yield StreamEvent::Error(format!("Backup integrity check failed: {err}")),
      };

      let client = match self.connect_to_mongodb().await {
        Ok(c) => c,
        Err(err) => return yield StreamEvent::Error(err),
      };

      let metadata: DatabaseMetadata = match backup_datastore.get_object(".database.json".to_string()) {
        Ok(content) => match serde_json::from_str(&content) {
          Ok(meta) => meta,
          Err(err) => return yield StreamEvent::Error(format!("Failed to parse metadata: {err}")),
        },
        Err(err) => return yield StreamEvent::Error(format!("Failed to read metadata: {err}")),
      };

      let target_db_name = target_database_name.unwrap_or_else(|| self.database_name.clone());
      yield StreamEvent::Info(format!("Restoring database '{}' to '{}'", metadata.name, target_db_name));

      let db = client.database(&target_db_name);

      for (collection_name, _hash) in metadata.collection_hashes {
        yield StreamEvent::Info(format!("Restoring collection: {}", collection_name));

        let mut events = Box::pin(self.restore_collection(backup_datastore.clone(), db.clone(), &collection_name));
        while let Some(event) = events.next().await {
          yield event;
        }
      }

      yield StreamEvent::Info("Restore completed successfully".to_string());
    }
  }

  fn create_backup_datastore(&self, backup_dir: &str) -> Result<Datastore, String> {
    match &self.datastore {
      Datastore::FileSystem(ds) => {
        let backup_path = ds.base_path.join(backup_dir);
        FilesystemDatastore::new(backup_path.as_path())
          .map(Datastore::FileSystem)
          .map_err(|e| format!("Failed to create backup datastore: {e}"))
      }
      Datastore::S3(ds) => {
        let backup_path = ds.base_path.join(backup_dir);
        S3Datastore::new(backup_path.as_path())
          .map(Datastore::S3)
          .map_err(|e| format!("Failed to create backup datastore: {e}"))
      }
    }
  }

  async fn connect_to_mongodb(&self) -> Result<mongodb::Client, String> {
    let connection = DatabaseConnection::new()
      .connect(&self.connection_string)
      .await
      .map_err(|e| format!("Failed to connect to MongoDB: {e}"))?;

    connection
      .client()
      .cloned()
      .ok_or_else(|| "MongoDB client not initialized".to_string())
  }

  fn restore_collection(
    &self,
    backup_datastore: Datastore,
    db: Database,
    collection_name: &str,
  ) -> impl Stream<Item = StreamEvent> {
    stream! {
      let read_stream = match backup_datastore.open_read_stream(&format!("{}.json", collection_name)).await {
        Ok(s) => s,
        Err(err) => return yield StreamEvent::Error(format!("Failed to open collection file: {err}")),
      };

      let mut reader = BufReader::new(read_stream);
      let mut buffer = String::new();
      let mut chunk = vec![0u8; 64 * 1024];

      let data_marker = "\"data\":[";
      loop {
        match reader.read(&mut chunk).await {
          Ok(0) => return yield StreamEvent::Error("Invalid backup: missing data array".to_string()),
          Ok(n) => buffer.push_str(&String::from_utf8_lossy(&chunk[..n])),
          Err(err) => return yield StreamEvent::Error(format!("Failed to read: {err}")),
        }

        if let Some(pos) = buffer.find(data_marker) {
          let header_json = buffer[..pos].trim_end_matches(',');
          let header_complete = format!("{}}}", header_json);

          let header: DatabaseCollectionHeaderWithoutData = match serde_json::from_str(&header_complete) {
            Ok(h) => h,
            Err(err) => return yield StreamEvent::Error(format!("Failed to parse header: {err}")),
          };

          let collection: Collection<Document> = db.collection(collection_name);
          let _ = collection.drop().await;

          if let Err(err) = db.create_collection(collection_name).with_options(header.options).await {
            return yield StreamEvent::Error(format!("Failed to create collection: {err}"));
          }

          let collection: Collection<Document> = db.collection(collection_name);

          if !header.indexes.is_empty() {
            if let Err(err) = collection.create_indexes(header.indexes).await {
              return yield StreamEvent::Error(format!("Failed to create indexes: {err}"));
            }
          }

          let data_start = pos + data_marker.len();
          let remaining = buffer[data_start..].to_string();

          let mut events = Box::pin(Self::stream_documents_from_reader(reader, remaining, collection, header.documents_count));
          while let Some(event) = events.next().await {
            yield event;
          }

          break;
        }
      }

      yield StreamEvent::Info(format!("Restored collection: {}", collection_name));
    }
  }

  fn stream_documents_from_reader<R: AsyncRead + Unpin + Send + 'static>(
    mut reader: BufReader<R>,
    initial_data: String,
    collection: Collection<Document>,
    total_docs: u64,
  ) -> impl Stream<Item = StreamEvent> {
    stream! {
      let mut buffer = initial_data;
      let mut batch: Vec<Document> = Vec::with_capacity(DOCUMENTS_BATCH_SIZE as usize);
      let mut inserted = 0u64;

      loop {
        let trimmed_start = buffer.trim_start_matches(|c: char| c == ',' || c.is_whitespace());

        if trimmed_start.starts_with(']') || trimmed_start.is_empty() {
          let mut chunk = vec![0u8; 64 * 1024];
          match reader.read(&mut chunk).await {
            Ok(0) => break, // EOF reached
            Ok(n) => {
              buffer = format!("{}{}", trimmed_start, String::from_utf8_lossy(&chunk[..n]));
              continue;
            }
            Err(err) => return yield StreamEvent::Error(format!("Failed to read: {err}")),
          }
        }

        let mut deserializer = serde_json::Deserializer::from_str(trimmed_start).into_iter::<Document>();
        let mut consumed = 0usize;
        let mut parsed_any = false;

        loop {
          match deserializer.next() {
            Some(Ok(doc)) => {
              consumed = deserializer.byte_offset();
              parsed_any = true;
              batch.push(doc);

              if batch.len() >= DOCUMENTS_BATCH_SIZE as usize {
                if let Err(err) = collection.insert_many(&batch).await {
                  return yield StreamEvent::Error(format!("Failed to insert documents: {err}"));
                }
                inserted += batch.len() as u64;
                yield StreamEvent::Info(format!("Inserted {}/{} documents", inserted, total_docs));
                batch.clear();
              }
            }
            Some(Err(_)) => break, // Incomplete JSON, need more data
            None => break, // No more documents in buffer
          }
        }

        // Keep unparsed portion
        buffer = trimmed_start[consumed..].to_string();

        // If we didn't parse anything, we need more data
        if !parsed_any || buffer.trim_start_matches(|c: char| c == ',' || c.is_whitespace()).is_empty() {
          let mut chunk = vec![0u8; 64 * 1024];
          match reader.read(&mut chunk).await {
            Ok(0) => break, // EOF
            Ok(n) => buffer.push_str(&String::from_utf8_lossy(&chunk[..n])),
            Err(err) => return yield StreamEvent::Error(format!("Failed to read: {err}")),
          }
        }
      }

      // Insert remaining batch
      if !batch.is_empty() {
        if let Err(err) = collection.insert_many(&batch).await {
          return yield StreamEvent::Error(format!("Failed to insert documents: {err}"));
        }
        inserted += batch.len() as u64;
        yield StreamEvent::Info(format!("Inserted {}/{} documents", inserted, total_docs));
      }
    }
  }
}
