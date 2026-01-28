// fileName: src/lib.rs

//! KoShelf library crate.
//! This crate backs the `koshelf` binary.

// Declaração dos módulos
pub mod app;
pub mod cli;
pub mod config;
pub mod i18n;
pub mod koreader;
pub mod library;
pub mod models;
pub mod parsers;
pub mod server;
pub mod share;
pub mod site_generator;
pub mod templates;
pub mod time_config;
pub mod utils;

// Re-exportações (Isso permite que o main.rs use "koshelf::run" e "koshelf::Cli")
pub use app::run;
pub use cli::Cli;

#[cfg(test)]
mod tests;