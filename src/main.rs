use dotenvy::dotenv;
use crate::{
  utils::{
    logger::Logger,
    config::Config,
    crypto::generate_key,
  },
};

fn main() {
  dotenv().ok();

  Logger::info("Hello World!");
  Logger::highlight("Hello World!");
  Logger::warn("Hello World!");
  Logger::error("Hello World!");

  let config = Config::new();
  
  let key = generate_key();
  
  println!("{:?}", key);
}

mod utils;