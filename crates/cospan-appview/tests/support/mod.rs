//! Test support utilities for cospan-appview integration tests.
//!
//! Provides:
//! - `test_pds`: a minimal Rust-based ATProto PDS running in-process
//! - `test_node`: spawns cospan-node instances for git source/destination
//! - `seed`: helpers for populating the DB with fixture data

#![allow(dead_code)] // Not all helpers used in every test module.

pub mod test_node;
pub mod test_pds;
