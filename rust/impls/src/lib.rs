//! Hosts VSS protocol compliant [`KvStore`] implementations for various backends.
//!
//! VSS is an open-source project designed to offer a server-side cloud storage solution specifically
//! tailored for noncustodial Lightning supporting mobile wallets. Its primary objective is to
//! simplify the development process for Lightning wallets by providing a secure means to store
//! and manage the essential state required for Lightning Network (LN) operations.
//!
//! [`KvStore`]: api::kv_store::KvStore

#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]
#![deny(missing_docs)]

mod migrations;
/// Contains [PostgreSQL](https://www.postgresql.org/) based backend implementation for VSS.
pub mod postgres_store;

#[macro_use]
extern crate api;
