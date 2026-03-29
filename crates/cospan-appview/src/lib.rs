pub mod auth;
pub mod cache;
pub mod config;
pub mod db;
pub mod error;
pub mod indexer;
pub mod interop;
pub mod middleware;
pub mod node_proxy;
pub mod state;
pub mod xrpc;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");
