use crate::{
  utils::logger::Logger,
};

fn main() {
  Logger::info("Hello World!");
  Logger::highlight("Hello World!");
  Logger::warn("Hello World!");
  Logger::error("Hello World!");
}

mod utils;