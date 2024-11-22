use api::error::VssError;
use api::kv_store::KvStore;
use api::types::{
	DeleteObjectRequest, DeleteObjectResponse, GetObjectRequest, GetObjectResponse,
	ListKeyVersionsRequest, ListKeyVersionsResponse, PutObjectRequest, PutObjectResponse,
};
use async_trait::async_trait;
use std::io::Error;

/// A [PostgreSQL](https://www.postgresql.org/) based backend implementation for VSS.
pub struct PostgresBackendImpl {}

impl PostgresBackendImpl {
	/// Constructs a [`PostgresBackendImpl`] using `dsn` for PostgreSQL connection information.
	pub async fn new(dsn: &str) -> Result<Self, Error> {
		todo!("pending implementation.");
	}
}

#[async_trait]
impl KvStore for PostgresBackendImpl {
	async fn get(
		&self, user_token: String, request: GetObjectRequest,
	) -> Result<GetObjectResponse, VssError> {
		todo!("pending implementation.");
	}

	async fn put(
		&self, user_token: String, request: PutObjectRequest,
	) -> Result<PutObjectResponse, VssError> {
		todo!("pending implementation.");
	}

	async fn delete(
		&self, user_token: String, request: DeleteObjectRequest,
	) -> Result<DeleteObjectResponse, VssError> {
		todo!("pending implementation.");
	}

	async fn list_key_versions(
		&self, user_token: String, request: ListKeyVersionsRequest,
	) -> Result<ListKeyVersionsResponse, VssError> {
		todo!("pending implementation.");
	}
}
