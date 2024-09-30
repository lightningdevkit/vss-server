use crate::error::VssError;
use crate::types::{
	DeleteObjectRequest, DeleteObjectResponse, GetObjectRequest, GetObjectResponse,
	ListKeyVersionsRequest, ListKeyVersionsResponse, PutObjectRequest, PutObjectResponse,
};
use async_trait::async_trait;

pub(crate) const GLOBAL_VERSION_KEY: &str = "global_version";
pub(crate) const INITIAL_RECORD_VERSION: i32 = 1;

/// An interface that must be implemented by every backend implementation of VSS.
#[async_trait]
pub trait KvStore: Send + Sync {
	/// Retrieves an object based on the provided request and user token.
	async fn get(
		&self, user_token: String, request: GetObjectRequest,
	) -> Result<GetObjectResponse, VssError>;

	/// Stores an object with the provided request and user token.
	async fn put(
		&self, user_token: String, request: PutObjectRequest,
	) -> Result<PutObjectResponse, VssError>;

	/// Deletes an object based on the provided request and user token.
	async fn delete(
		&self, user_token: String, request: DeleteObjectRequest,
	) -> Result<DeleteObjectResponse, VssError>;

	/// Lists the versions of keys based on the provided request and user token.
	async fn list_key_versions(
		&self, user_token: String, request: ListKeyVersionsRequest,
	) -> Result<ListKeyVersionsResponse, VssError>;
}
