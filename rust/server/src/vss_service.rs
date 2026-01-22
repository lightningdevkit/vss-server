use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::service::Service;
use hyper::{Request, Response, StatusCode};
use std::collections::HashMap;

use prost::Message;

use api::auth::Authorizer;
use api::error::VssError;
use api::kv_store::KvStore;
use api::types::{
	DeleteObjectRequest, DeleteObjectResponse, ErrorCode, ErrorResponse, GetObjectRequest,
	GetObjectResponse, ListKeyVersionsRequest, ListKeyVersionsResponse, PutObjectRequest,
	PutObjectResponse,
};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use log::{debug, trace};

use crate::util::KeyValueVecKeyPrinter;

#[derive(Clone)]
pub struct VssService {
	store: Arc<dyn KvStore>,
	authorizer: Arc<dyn Authorizer>,
}

impl VssService {
	pub(crate) fn new(store: Arc<dyn KvStore>, authorizer: Arc<dyn Authorizer>) -> Self {
		Self { store, authorizer }
	}
}

pub(crate) const BASE_PATH_PREFIX: &str = "/vss";

impl Service<Request<Incoming>> for VssService {
	type Response = Response<Full<Bytes>>;
	type Error = hyper::Error;
	type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

	fn call(&self, req: Request<Incoming>) -> Self::Future {
		let store = Arc::clone(&self.store);
		let authorizer = Arc::clone(&self.authorizer);
		let path = req.uri().path().to_owned();

		Box::pin(async move {
			let prefix_stripped_path = path.strip_prefix(BASE_PATH_PREFIX).unwrap_or_default();

			match prefix_stripped_path {
				"/getObject" => {
					handle_request(store, authorizer, req, handle_get_object_request).await
				},
				"/putObjects" => {
					handle_request(store, authorizer, req, handle_put_object_request).await
				},
				"/deleteObject" => {
					handle_request(store, authorizer, req, handle_delete_object_request).await
				},
				"/listKeyVersions" => {
					handle_request(store, authorizer, req, handle_list_object_request).await
				},
				_ => {
					let error_msg = "Invalid request path.".as_bytes();
					Ok(Response::builder()
						.status(StatusCode::BAD_REQUEST)
						.body(Full::new(Bytes::from(error_msg)))
						.unwrap())
				},
			}
		})
	}
}

async fn handle_get_object_request(
	store: Arc<dyn KvStore>, user_token: String, request: GetObjectRequest,
) -> Result<GetObjectResponse, VssError> {
	let request_id: u64 = rand::random();
	trace!("Handling GetObjectRequest {} for key {}.", request_id, request.key);
	let result = store.get(user_token, request).await;
	if let Err(ref e) = result {
		debug!("GetObjectRequest {} failed: {}", request_id, e);
	}
	result
}
async fn handle_put_object_request(
	store: Arc<dyn KvStore>, user_token: String, request: PutObjectRequest,
) -> Result<PutObjectResponse, VssError> {
	let request_id: u64 = rand::random();
	trace!(
		"Handling PutObjectRequest {} for transaction_items {} and delete_items {}.",
		request_id,
		KeyValueVecKeyPrinter(&request.transaction_items),
		KeyValueVecKeyPrinter(&request.delete_items),
	);
	let result = store.put(user_token, request).await;
	if let Err(ref e) = result {
		debug!("PutObjectRequest {} failed: {}", request_id, e);
	}
	result
}
async fn handle_delete_object_request(
	store: Arc<dyn KvStore>, user_token: String, request: DeleteObjectRequest,
) -> Result<DeleteObjectResponse, VssError> {
	let request_id: u64 = rand::random();
	trace!(
		"Handling DeleteObjectRequest {} for key {:?}",
		request_id,
		request.key_value.as_ref().map(|t| &t.key)
	);
	let result = store.delete(user_token, request).await;
	if let Err(ref e) = result {
		trace!("DeleteObjectRequest {} failed: {}", request_id, e);
	}
	result
}
async fn handle_list_object_request(
	store: Arc<dyn KvStore>, user_token: String, request: ListKeyVersionsRequest,
) -> Result<ListKeyVersionsResponse, VssError> {
	let request_id: u64 = rand::random();
	trace!(
		"Handling ListKeyVersionsRequest {} for key_prefix {:?}, page_size {:?}, page_token {:?}",
		request_id,
		request.key_prefix,
		request.page_size,
		request.page_token
	);
	let result = store.list_key_versions(user_token, request).await;
	if let Err(ref e) = result {
		debug!("ListKeyVersionsRequest {} failed: {}", request_id, e);
	}
	result
}
async fn handle_request<
	T: Message + Default,
	R: Message,
	F: FnOnce(Arc<dyn KvStore>, String, T) -> Fut + Send + 'static,
	Fut: Future<Output = Result<R, VssError>> + Send,
>(
	store: Arc<dyn KvStore>, authorizer: Arc<dyn Authorizer>, request: Request<Incoming>,
	handler: F,
) -> Result<<VssService as Service<Request<Incoming>>>::Response, hyper::Error> {
	let (parts, body) = request.into_parts();
	let headers_map = parts
		.headers
		.iter()
		.map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or_default().to_string()))
		.collect::<HashMap<String, String>>();

	let user_token = match authorizer.verify(&headers_map).await {
		Ok(auth_response) => auth_response.user_token,
		Err(e) => return Ok(build_error_response(e)),
	};
	// TODO: we should bound the amount of data we read to avoid allocating too much memory.
	let bytes = body.collect().await?.to_bytes();
	match T::decode(bytes) {
		Ok(request) => match handler(store.clone(), user_token, request).await {
			Ok(response) => Ok(Response::builder()
				.body(Full::new(Bytes::from(response.encode_to_vec())))
				// unwrap safety: body only errors when previous chained calls failed.
				.unwrap()),
			Err(e) => Ok(build_error_response(e)),
		},
		Err(_) => Ok(Response::builder()
			.status(StatusCode::BAD_REQUEST)
			.body(Full::new(Bytes::from(b"Error parsing request".to_vec())))
			// unwrap safety: body only errors when previous chained calls failed.
			.unwrap()),
	}
}

fn build_error_response(e: VssError) -> Response<Full<Bytes>> {
	let (status_code, error_response) = match e {
		VssError::NoSuchKeyError(msg) => {
			let status = StatusCode::NOT_FOUND;
			let error = ErrorResponse {
				error_code: ErrorCode::NoSuchKeyException.into(),
				message: msg.to_string(),
			};
			(status, error)
		},
		VssError::ConflictError(msg) => {
			let status = StatusCode::CONFLICT;
			let error = ErrorResponse {
				error_code: ErrorCode::ConflictException.into(),
				message: msg.to_string(),
			};
			(status, error)
		},
		VssError::InvalidRequestError(msg) => {
			let status = StatusCode::BAD_REQUEST;
			let error = ErrorResponse {
				error_code: ErrorCode::InvalidRequestException.into(),
				message: msg.to_string(),
			};
			(status, error)
		},
		VssError::AuthError(msg) => {
			let status = StatusCode::UNAUTHORIZED;
			let error = ErrorResponse {
				error_code: ErrorCode::AuthException.into(),
				message: msg.to_string(),
			};
			(status, error)
		},
		VssError::InternalServerError(_) => {
			let status = StatusCode::INTERNAL_SERVER_ERROR;
			let error = ErrorResponse {
				error_code: ErrorCode::InternalServerException.into(),
				message: "Unknown Server Error occurred.".to_string(),
			};
			(status, error)
		},
	};
	Response::builder()
		.status(status_code)
		.body(Full::new(Bytes::from(error_response.encode_to_vec())))
		// unwrap safety: body only errors when previous chained calls failed.
		.unwrap()
}
