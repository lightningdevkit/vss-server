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

use chrono::Utc;

/// Contains in-memory backend implementation for VSS, for testing purposes only.
pub mod in_memory_store;
/// Contains [PostgreSQL](https://www.postgresql.org/) based backend implementation for VSS.
pub mod postgres_store;

/// A record stored in the VSS database.
struct VssDbRecord {
	/// Token uniquely identifying the user that owns this record.
	user_token: String,
	/// Identifier for the store this record belongs to.
	store_id: String,
	/// Key under which the value is stored.
	key: String,
	/// Stored value as raw bytes.
	value: Vec<u8>,
	/// Version number for optimistic concurrency control.
	version: i64,
	/// Timestamp when the record was created (UTC).
	created_at: chrono::DateTime<Utc>,
	/// Timestamp when the record was last updated (UTC).
	last_updated_at: chrono::DateTime<Utc>,
}

/// The maximum number of key versions that can be returned in a single page.
///
/// This constant helps control memory and bandwidth usage for list operations,
/// preventing overly large payloads. If the number of results exceeds this limit,
/// the response will be paginated.
const LIST_KEY_VERSIONS_MAX_PAGE_SIZE: i32 = 100;

/// The maximum number of items allowed in a single `PutObjectRequest`.
///
/// Setting an upper bound on the number of items helps ensure that
/// each request stays within acceptable memory and performance limits.
/// Exceeding this value will result in request rejection through [`VssError::InvalidRequestError`].
const MAX_PUT_REQUEST_ITEM_COUNT: usize = 1000;

mod migrations;

#[macro_use]
extern crate api;
