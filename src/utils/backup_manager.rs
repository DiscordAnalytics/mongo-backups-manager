use async_stream::stream;
use chrono::Local;
use cronexpr::Crontab;
use cronexpr::jiff::Zoned;
use futures::{Stream, StreamExt, TryStreamExt};
use mongodb::{
  Collection, Database, IndexModel,
  bson::{Document, doc},
  options::{CreateCollectionOptions, FindOptions},
  results::CollectionSpecification,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};
use tokio::io::AsyncWriteExt;

use crate::utils::config::BackupDatastore;
use crate::{
  datastores::{Datastore, DatastoreTrait, FilesystemDatastore, S3Datastore},
  db::DatabaseConnection,
  utils::{config::BackupDatastoreType, logger::StreamEvent},
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

      let connection = match DatabaseConnection::new().connect(self.connection_string.as_str()).await {
        Ok(value) => value,
        Err(err) => return yield StreamEvent::Error(format!("Failed to connected to MongoDB Server: {err}")),
      };

      let db = match connection.client() {
        Some(client) => client.database(&self.database_name.as_str()),
        None => return yield StreamEvent::Error("MongoDB client not initialized".to_string()),
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
    let collection_files = self.datastore.list_objects()?;
    let mut collection_hashes: HashMap<String, String> = HashMap::new();

    for file in collection_files {
      let file_name = file
        .split_once('.')
        .map(|(name, _)| name)
        .unwrap_or(&file)
        .to_string();
      let file_hash = self.datastore.get_object_hash(file)?;

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

  pub fn restore_backup_to_database(&self, backup_dir: String) -> impl Stream<Item = StreamEvent> {
    stream! {
      yield StreamEvent::Info(format!("Starting restore from backup directory: {}", backup_dir));

      // Connect to MongoDB
      let connection = match DatabaseConnection::new().connect(self.connection_string.as_str()).await {
        Ok(value) => value,
        Err(err) => return yield StreamEvent::Error(format!("Failed to connect to MongoDB Server: {err}")),
      };

      let client = match connection.client() {
        Some(client) => client,
        None => return yield StreamEvent::Error("MongoDB client not initialized".to_string()),
      };

      // Read the metadata file to get database information
      let metadata_path = format!("{}/.database.json", backup_dir);
      let metadata_content = match self.datastore.get_object(metadata_path) {
        Ok(content) => content,
        Err(err) => return yield StreamEvent::Error(format!("Failed to read metadata file: {err}")),
      };

      let metadata: DatabaseMetadata = match serde_json::from_str(&metadata_content) {
        Ok(meta) => meta,
        Err(err) => return yield StreamEvent::Error(format!("Failed to parse metadata file: {err}")),
      };

      yield StreamEvent::Info(format!("Restoring database: {}", metadata.name));

      let db = client.database(&self.database_name);

      // Restore each collection
      for (collection_name, _hash) in metadata.collection_hashes {
        yield StreamEvent::Info(format!("Restoring collection: {}", collection_name));

        let collection_file_path = format!("{}/{}.json", backup_dir, collection_name);
        let collection_content = match self.datastore.get_object(collection_file_path) {
          Ok(content) => content,
          Err(err) => {
            yield StreamEvent::Error(format!("Failed to read collection file: {err}"));
            continue;
          }
        };

        let collection_header: DatabaseCollectionHeader = match serde_json::from_str(&collection_content) {
          Ok(header) => header,
          Err(err) => {
            yield StreamEvent::Error(format!("Failed to parse collection file: {err}"));
            continue;
          }
        };

        // Drop the collection if it exists
        let collection: Collection<Document> = db.collection(&collection_header.name);
        if let Err(err) = collection.drop().await {
          // It's okay if the collection doesn't exist
          yield StreamEvent::Info(format!("Collection {} does not exist (will create new): {}", collection_header.name, err));
        }

        // Create the collection with options
        if let Err(err) = db.create_collection(&collection_header.name).with_options(collection_header.options).await {
          yield StreamEvent::Error(format!("Failed to create collection: {err}"));
          continue;
        }

        let collection: Collection<Document> = db.collection(&collection_header.name);

        // Create indexes
        if !collection_header.indexes.is_empty() {
          if let Err(err) = collection.create_indexes(collection_header.indexes).await {
            yield StreamEvent::Error(format!("Failed to create indexes: {err}"));
            continue;
          }
        }

        // Insert documents in batches
        if !collection_header.data.is_empty() {
          let total_docs = collection_header.data.len();
          let mut inserted = 0;

          for chunk in collection_header.data.chunks(DOCUMENTS_BATCH_SIZE as usize) {
            if let Err(err) = collection.insert_many(chunk).await {
              yield StreamEvent::Error(format!("Failed to insert documents: {err}"));
              continue;
            }
            inserted += chunk.len();
            yield StreamEvent::Info(format!("Inserted {}/{} documents", inserted, total_docs));
          }
        }

        yield StreamEvent::Info(format!("Restored collection: {}", collection_header.name));
      }

      yield StreamEvent::Info("Restore completed successfully".to_string());
    }
  }
}
