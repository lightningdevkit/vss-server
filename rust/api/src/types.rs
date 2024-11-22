/// Request payload to be used for `GetObject` API call to server.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetObjectRequest {
	/// `store_id` is a keyspace identifier.
	/// Ref: <https://en.wikipedia.org/wiki/Keyspace_(distributed_data_store>)
	/// All APIs operate within a single `store_id`.
	/// It is up to clients to use single or multiple stores for their use-case.
	/// This can be used for client-isolation/ rate-limiting / throttling on the server-side.
	/// Authorization and billing can also be performed at the `store_id` level.
	#[prost(string, tag = "1")]
	pub store_id: ::prost::alloc::string::String,
	/// The key of the value to be fetched.
	///
	/// If the specified `key` does not exist, returns `ErrorCode.NO_SUCH_KEY_EXCEPTION` in the
	/// the `ErrorResponse`.
	///
	/// Consistency Guarantee:
	/// Get(read) operations against a `key` are consistent reads and will reflect all previous writes,
	/// since Put/Write provides read-after-write and read-after-update consistency guarantees.
	///
	/// Read Isolation:
	/// Get/Read operations against a `key` are ensured to have read-committed isolation.
	/// Ref: <https://en.wikipedia.org/wiki/Isolation_(database_systems>)#Read_committed
	#[prost(string, tag = "2")]
	pub key: ::prost::alloc::string::String,
}
/// Server response for `GetObject` API.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetObjectResponse {
	/// Fetched `value` and `version` along with the corresponding `key` in the request.
	#[prost(message, optional, tag = "2")]
	pub value: ::core::option::Option<KeyValue>,
}
/// Request payload to be used for `PutObject` API call to server.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PutObjectRequest {
	/// `store_id` is a keyspace identifier.
	/// Ref: <https://en.wikipedia.org/wiki/Keyspace_(distributed_data_store>)
	/// All APIs operate within a single `store_id`.
	/// It is up to clients to use single or multiple stores for their use-case.
	/// This can be used for client-isolation/ rate-limiting / throttling on the server-side.
	/// Authorization and billing can also be performed at the `store_id` level.
	#[prost(string, tag = "1")]
	pub store_id: ::prost::alloc::string::String,
	/// `global_version` is a sequence-number/version of the whole store. This can be used for versioning
	/// and ensures that multiple updates in case of multiple devices can only be done linearly, even
	/// if those updates did not directly conflict with each other based on keys/`transaction_items`.
	///
	/// If present, the write will only succeed if the current server-side `global_version` against
	/// the `store_id` is same as in the request.
	/// Clients are expected to store (client-side) the global version against `store_id`.
	/// The request must contain their client-side value of `global_version` if global versioning and
	/// conflict detection is desired.
	///
	/// For the first write of the store, global version should be '0'. If the write succeeds, clients
	/// must increment their global version (client-side) by 1.
	/// The server increments `global_version` (server-side) for every successful write, hence this
	/// client-side increment is required to ensure matching versions. This updated global version
	/// should be used in subsequent `PutObjectRequest`s for the store.
	///
	/// Requests with a conflicting version will fail with `CONFLICT_EXCEPTION` as ErrorCode.
	#[prost(int64, optional, tag = "2")]
	pub global_version: ::core::option::Option<i64>,
	/// Items to be written as a result of this `PutObjectRequest`.
	///
	/// In an item, each `key` is supplied with its corresponding `value` and `version`.
	/// Clients can choose to encrypt the keys client-side in order to obfuscate their usage patterns.
	/// If the write is successful, the previous `value` corresponding to the `key` will be overwritten.
	///
	/// Multiple items in `transaction_items` and `delete_items` of a single `PutObjectRequest` are written in
	/// a database-transaction in an all-or-nothing fashion.
	/// All Items in a single `PutObjectRequest` must have distinct keys.
	///
	/// Key-level versioning (Conditional Write):
	///    Clients are expected to store a `version` against every `key`.
	///    The write will succeed if the current DB version against the `key` is the same as in the request.
	///    When initiating a `PutObjectRequest`, the request should contain their client-side `version`
	///    for that key-value.
	///
	///    For the first write of any `key`, the `version` should be '0'. If the write succeeds, the client
	///    must increment their corresponding key versions (client-side) by 1.
	///    The server increments key versions (server-side) for every successful write, hence this
	///    client-side increment is required to ensure matching versions. These updated key versions should
	///    be used in subsequent `PutObjectRequest`s for the keys.
	///
	///    Requests with a conflicting/mismatched version will fail with `CONFLICT_EXCEPTION` as ErrorCode
	///    for conditional writes.
	///
	/// Skipping key-level versioning (Non-conditional Write):
	///    If you wish to skip key-level version checks, set the `version` against the `key` to '-1'.
	///    This will perform a non-conditional write query, after which the `version` against the `key`
	///    is reset to '1'. Hence, the next `PutObjectRequest` for the `key` can be either
	///    a non-conditional write or a conditional write with `version` set to `1`.
	///
	/// Considerations for transactions:
	/// Transaction writes of multiple items have a performance overhead, hence it is recommended to use
	/// them only if required by the client application to ensure logic/code correctness.
	/// That is, `transaction_items` are not a substitute for batch-write of multiple unrelated items.
	/// When a write of multiple unrelated items is desired, it is recommended to use separate
	/// `PutObjectRequest`s.
	///
	/// Consistency guarantee:
	/// All `PutObjectRequest`s are strongly consistent i.e. they provide read-after-write and
	/// read-after-update consistency guarantees.
	#[prost(message, repeated, tag = "3")]
	pub transaction_items: ::prost::alloc::vec::Vec<KeyValue>,
	/// Items to be deleted as a result of this `PutObjectRequest`.
	///
	/// Each item in the `delete_items` field consists of a `key` and its corresponding `version`.
	///
	/// Key-Level Versioning (Conditional Delete):
	///    The `version` is used to perform a version check before deleting the item.
	///    The delete will only succeed if the current database version against the `key` is the same as
	///    the `version` specified in the request.
	///
	/// Skipping key-level versioning (Non-conditional Delete):
	///    If you wish to skip key-level version checks, set the `version` against the `key` to '-1'.
	///    This will perform a non-conditional delete query.
	///
	/// Fails with `CONFLICT_EXCEPTION` as the ErrorCode if:
	///    * The requested item does not exist.
	///    * The requested item does exist but there is a version-number mismatch (in conditional delete)
	///      with the one in the database.
	///
	/// Multiple items in the `delete_items` field, along with the `transaction_items`, are written in a
	/// database transaction in an all-or-nothing fashion.
	///
	/// All items within a single `PutObjectRequest` must have distinct keys.
	#[prost(message, repeated, tag = "4")]
	pub delete_items: ::prost::alloc::vec::Vec<KeyValue>,
}
/// Server response for `PutObject` API.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PutObjectResponse {}
/// Request payload to be used for `DeleteObject` API call to server.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteObjectRequest {
	/// `store_id` is a keyspace identifier.
	/// Ref: <https://en.wikipedia.org/wiki/Keyspace_(distributed_data_store>)
	/// All APIs operate within a single `store_id`.
	/// It is up to clients to use single or multiple stores for their use-case.
	/// This can be used for client-isolation/ rate-limiting / throttling on the server-side.
	/// Authorization and billing can also be performed at the `store_id` level.
	#[prost(string, tag = "1")]
	pub store_id: ::prost::alloc::string::String,
	/// Item to be deleted as a result of this `DeleteObjectRequest`.
	///
	/// An item consists of a `key` and its corresponding `version`.
	///
	/// Key-level Versioning (Conditional Delete):
	///    The item is only deleted if the current database version against the `key` is the same as
	///    the `version` specified in the request.
	///
	/// Skipping key-level versioning (Non-conditional Delete):
	///    If you wish to skip key-level version checks, set the `version` against the `key` to '-1'.
	///    This will perform a non-conditional delete query.
	///
	/// This operation is idempotent, that is, multiple delete calls for the same item will not fail.
	///
	/// If the requested item does not exist, this operation will not fail.
	/// If you wish to perform stricter checks while deleting an item, consider using `PutObject` API.
	#[prost(message, optional, tag = "2")]
	pub key_value: ::core::option::Option<KeyValue>,
}
/// Server response for `DeleteObject` API.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteObjectResponse {}
/// Request payload to be used for `ListKeyVersions` API call to server.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListKeyVersionsRequest {
	/// `store_id` is a keyspace identifier.
	/// Ref: <https://en.wikipedia.org/wiki/Keyspace_(distributed_data_store>)
	/// All APIs operate within a single `store_id`.
	/// It is up to clients to use single or multiple stores for their use-case.
	/// This can be used for client-isolation/ rate-limiting / throttling on the server-side.
	/// Authorization and billing can also be performed at the `store_id` level.
	#[prost(string, tag = "1")]
	pub store_id: ::prost::alloc::string::String,
	/// A `key_prefix` is a string of characters at the beginning of the key. Prefixes can be used as
	/// a way to organize key-values in a similar way to directories.
	///
	/// If `key_prefix` is specified, the response results will be limited to those keys that begin with
	/// the specified prefix.
	///
	/// If no `key_prefix` is specified or it is empty (""), all the keys are eligible to be returned in
	/// the response.
	#[prost(string, optional, tag = "2")]
	pub key_prefix: ::core::option::Option<::prost::alloc::string::String>,
	/// `page_size` is used by clients to specify the maximum number of results that can be returned by
	/// the server.
	/// The server may further constrain the maximum number of results returned in a single page.
	/// If the `page_size` is 0 or not set, the server will decide the number of results to be returned.
	#[prost(int32, optional, tag = "3")]
	pub page_size: ::core::option::Option<i32>,
	/// `page_token` is a pagination token.
	///
	/// To query for the first page of `ListKeyVersions`, `page_token` must not be specified.
	///
	/// For subsequent pages, use the value that was returned as `next_page_token` in the previous
	/// page's `ListKeyVersionsResponse`.
	#[prost(string, optional, tag = "4")]
	pub page_token: ::core::option::Option<::prost::alloc::string::String>,
}
/// Server response for `ListKeyVersions` API.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListKeyVersionsResponse {
	/// Fetched keys and versions.
	/// Even though this API reuses the `KeyValue` struct, the `value` sub-field will not be set by the server.
	#[prost(message, repeated, tag = "1")]
	pub key_versions: ::prost::alloc::vec::Vec<KeyValue>,
	/// `next_page_token` is a pagination token, used to retrieve the next page of results.
	/// Use this value to query for next-page of paginated `ListKeyVersions` operation, by specifying
	/// this value as the `page_token` in the next request.
	///
	/// If `next_page_token` is empty (""), then the "last page" of results has been processed and
	/// there is no more data to be retrieved.
	///
	/// If `next_page_token` is not empty, it does not necessarily mean that there is more data in the
	/// result set. The only way to know when you have reached the end of the result set is when
	/// `next_page_token` is empty.
	///
	/// Caution: Clients must not assume a specific number of key_versions to be present in a page for
	/// paginated response.
	#[prost(string, optional, tag = "2")]
	pub next_page_token: ::core::option::Option<::prost::alloc::string::String>,
	/// `global_version` is a sequence-number/version of the whole store.
	///
	/// `global_version` is only returned in response for the first page of the `ListKeyVersionsResponse`
	/// and is guaranteed to be read before reading any key-versions.
	///
	/// In case of refreshing the complete key-version view on the client-side, correct usage for
	/// the returned `global_version` is as following:
	///    1. Read `global_version` from the first page of paginated response and save it as local variable.
	///    2. Update all the `key_versions` on client-side from all the pages of paginated response.
	///    3. Update `global_version` on client_side from the local variable saved in step-1.
	/// This ensures that on client-side, all current `key_versions` were stored at `global_version` or later.
	/// This guarantee is helpful for ensuring the versioning correctness if using the `global_version`
	/// in `PutObject` API and can help avoid the race conditions related to it.
	#[prost(int64, optional, tag = "3")]
	pub global_version: ::core::option::Option<i64>,
}
/// When HttpStatusCode is not ok (200), the response `content` contains a serialized `ErrorResponse`
/// with the relevant `ErrorCode` and `message`
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ErrorResponse {
	/// The error code uniquely identifying an error condition.
	/// It is meant to be read and understood programmatically by code that detects/handles errors by
	/// type.
	#[prost(enumeration = "ErrorCode", tag = "1")]
	pub error_code: i32,
	/// The error message containing a generic description of the error condition in English.
	/// It is intended for a human audience only and should not be parsed to extract any information
	/// programmatically. Client-side code may use it for logging only.
	#[prost(string, tag = "2")]
	pub message: ::prost::alloc::string::String,
}
/// Represents a key-value pair to be stored or retrieved.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct KeyValue {
	/// Key against which the value is stored.
	#[prost(string, tag = "1")]
	pub key: ::prost::alloc::string::String,
	/// Version field is used for key-level versioning.
	/// For first write of key, `version` should be '0'. If the write succeeds, clients must increment
	/// their corresponding key version (client-side) by 1.
	/// The server increments key version (server-side) for every successful write, hence this
	/// client-side increment is required to ensure matching versions. These updated key versions should
	/// be used in subsequent `PutObjectRequest`s for the keys.
	#[prost(int64, tag = "2")]
	pub version: i64,
	/// Object value in bytes which is stored (in put) and fetched (in get).
	/// Clients must encrypt the secret contents of this blob client-side before sending it over the
	/// wire to the server in order to preserve privacy and security.
	/// Clients may use a `Storable` object, serialize it and set it here.
	#[prost(bytes = "bytes", tag = "3")]
	pub value: ::prost::bytes::Bytes,
}
/// Represents a storable object that can be serialized and stored as `value` in `PutObjectRequest`.
/// Only provided as a helper object for ease of use by clients.
/// Clients MUST encrypt the `PlaintextBlob` before using it as `data` in `Storable`.
/// The server does not use or read anything from `Storable`, Clients may use its fields as
/// required.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Storable {
	/// Represents an encrypted and serialized `PlaintextBlob`. MUST encrypt the whole `PlaintextBlob`
	/// using client-side encryption before setting here.
	#[prost(bytes = "bytes", tag = "1")]
	pub data: ::prost::bytes::Bytes,
	/// Represents encryption related metadata
	#[prost(message, optional, tag = "2")]
	pub encryption_metadata: ::core::option::Option<EncryptionMetadata>,
}
/// Represents encryption related metadata
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EncryptionMetadata {
	/// The encryption algorithm used for encrypting the `PlaintextBlob`.
	#[prost(string, tag = "1")]
	pub cipher_format: ::prost::alloc::string::String,
	/// The nonce used for encryption. Nonce is a random or unique value used to ensure that the same
	/// plaintext results in different ciphertexts every time it is encrypted.
	#[prost(bytes = "bytes", tag = "2")]
	pub nonce: ::prost::bytes::Bytes,
	/// The authentication tag used for encryption. It provides integrity and authenticity assurance
	/// for the encrypted data.
	#[prost(bytes = "bytes", tag = "3")]
	pub tag: ::prost::bytes::Bytes,
}
/// Represents a data blob, which is encrypted, serialized and later used in `Storable.data`.
/// Since the whole `Storable.data` is client-side encrypted, the server cannot understand this.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PlaintextBlob {
	/// The unencrypted value.
	#[prost(bytes = "bytes", tag = "1")]
	pub value: ::prost::bytes::Bytes,
	/// The version of the value. Can be used by client to verify version integrity.
	#[prost(int64, tag = "2")]
	pub version: i64,
}
/// ErrorCodes to be used in `ErrorResponse`
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum ErrorCode {
	/// Default protobuf Enum value. Will not be used as `ErrorCode` by server.
	Unknown = 0,
	/// Used when the request contains mismatched version (either key or global)
	/// in `PutObjectRequest`. For more info refer `PutObjectRequest`.
	ConflictException = 1,
	/// Used in the following cases:
	///    - The request was missing a required argument.
	///    - The specified argument was invalid, incomplete or in the wrong format.
	///    - The request body of api cannot be deserialized into corresponding protobuf object.
	InvalidRequestException = 2,
	/// Used when an internal server error occurred, client is probably at no fault and can safely retry
	/// this error with exponential backoff.
	InternalServerException = 3,
	/// Used when the specified `key` in a `GetObjectRequest` does not exist.
	NoSuchKeyException = 4,
	/// Used when authentication fails or in case of an unauthorized request.
	AuthException = 5,
}
impl ErrorCode {
	/// String value of the enum field names used in the ProtoBuf definition.
	///
	/// The values are not transformed in any way and thus are considered stable
	/// (if the ProtoBuf definition does not change) and safe for programmatic use.
	pub fn as_str_name(&self) -> &'static str {
		match self {
			ErrorCode::Unknown => "UNKNOWN",
			ErrorCode::ConflictException => "CONFLICT_EXCEPTION",
			ErrorCode::InvalidRequestException => "INVALID_REQUEST_EXCEPTION",
			ErrorCode::InternalServerException => "INTERNAL_SERVER_EXCEPTION",
			ErrorCode::NoSuchKeyException => "NO_SUCH_KEY_EXCEPTION",
			ErrorCode::AuthException => "AUTH_EXCEPTION",
		}
	}
	/// Creates an enum from field names used in the ProtoBuf definition.
	pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
		match value {
			"UNKNOWN" => Some(Self::Unknown),
			"CONFLICT_EXCEPTION" => Some(Self::ConflictException),
			"INVALID_REQUEST_EXCEPTION" => Some(Self::InvalidRequestException),
			"INTERNAL_SERVER_EXCEPTION" => Some(Self::InternalServerException),
			"NO_SUCH_KEY_EXCEPTION" => Some(Self::NoSuchKeyException),
			"AUTH_EXCEPTION" => Some(Self::AuthException),
			_ => None,
		}
	}
}
