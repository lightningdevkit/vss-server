//! Hosts API contract for VSS.
//!
//! VSS is an open-source project designed to offer a server-side cloud storage solution specifically
//! tailored for noncustodial Lightning supporting mobile wallets. Its primary objective is to
//! simplify the development process for Lightning wallets by providing a secure means to store
//! and manage the essential state required for Lightning Network (LN) operations.

#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]
#![deny(missing_docs)]

/// Contains interface for authorizer that is run before every request, and its corresponding implementations.
pub mod auth;
/// Implements the error type ([`error::VssError`]) which is eventually converted to [`ErrorResponse`] and returned to the client.
///
/// [`ErrorResponse`]: types::ErrorResponse
pub mod error;

/// Contains [`kv_store::KvStore`] interface which needs to be implemented by every backend implementation of VSS.
pub mod kv_store;

/// Contains request/response types generated from the API definition of VSS.
pub mod types;
