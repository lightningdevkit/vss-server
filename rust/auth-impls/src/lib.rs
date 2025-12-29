//! Hosts VSS protocol compliant [`Authorizer`] implementations.
//!
//! VSS is an open-source project designed to offer a server-side cloud storage solution specifically
//! tailored for noncustodial Lightning supporting mobile wallets. Its primary objective is to
//! simplify the development process for Lightning wallets by providing a secure means to store
//! and manage the essential state required for Lightning Network (LN) operations.
//!
//! [`Authorizer`]: api::auth::Authorizer

#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]
#![deny(missing_docs)]

#[cfg(feature = "jwt")]
pub mod jwt;

#[cfg(feature = "sigs")]
pub mod signature;
