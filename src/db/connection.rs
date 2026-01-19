use mongodb::{
  Client,
  error::{Error, Result},
};

pub struct DatabaseConnection {
  client: Option<Client>,
}

impl DatabaseConnection {
  pub fn new() -> Self {
    Self { client: None }
  }

  pub async fn connect(&mut self, uri: &str) -> Result<()> {
    let client = Client::with_uri_str(uri).await?;
    self.client = Some(client);
    Ok(())
  }

  pub async fn disconnect(&mut self) -> Result<()> {
    if let Some(c) = self.client.take() {
      c.shutdown().await;
    }
    self.client = None;
    Ok(())
  }

  pub fn is_connected(&self) -> bool {
    self.client.is_some()
  }

  pub fn client(&self) -> Option<&Client> {
    self.client.as_ref()
  }

  pub async fn list_databases(&self) -> Result<Vec<String>> {
    if let Some(c) = &self.client {
      Ok(c.list_database_names().await?)
    } else {
      Err(Error::custom("No connected databases"))
    }
  }
}