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

const BASE_PATH_PREFIX: &str = "/vss";

impl Service<Request<Incoming>> for VssService {
	type Response = Response<Full<Bytes>>;
	type Error = hyper::Error;
	type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

	fn call(&self, req: Request<Incoming>) -> Self::Future {
		let store = Arc::clone(&self.store);
		let authorizer = Arc::clone(&self.authorizer);
		let path = req.uri().path().to_owned();

		Box::pin(async move {
			let prefix_stripped_path = path.strip_prefix(BASE_PATH_PREFIX).unwrap_or("");

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
	store.get(user_token, request).await
}
async fn handle_put_object_request(
	store: Arc<dyn KvStore>, user_token: String, request: PutObjectRequest,
) -> Result<PutObjectResponse, VssError> {
	store.put(user_token, request).await
}
async fn handle_delete_object_request(
	store: Arc<dyn KvStore>, user_token: String, request: DeleteObjectRequest,
) -> Result<DeleteObjectResponse, VssError> {
	store.delete(user_token, request).await
}
async fn handle_list_object_request(
	store: Arc<dyn KvStore>, user_token: String, request: ListKeyVersionsRequest,
) -> Result<ListKeyVersionsResponse, VssError> {
	store.list_key_versions(user_token, request).await
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
		.map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
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
	let error_response = match e {
		VssError::NoSuchKeyError(msg) => ErrorResponse {
			error_code: ErrorCode::NoSuchKeyException.into(),
			message: msg.to_string(),
		},
		VssError::ConflictError(msg) => ErrorResponse {
			error_code: ErrorCode::ConflictException.into(),
			message: msg.to_string(),
		},
		VssError::InvalidRequestError(msg) => ErrorResponse {
			error_code: ErrorCode::InvalidRequestException.into(),
			message: msg.to_string(),
		},
		VssError::AuthError(msg) => {
			ErrorResponse { error_code: ErrorCode::AuthException.into(), message: msg.to_string() }
		},
		_ => ErrorResponse {
			error_code: ErrorCode::InternalServerException.into(),
			message: "Unknown Server Error occurred.".to_string(),
		},
	};
	Response::builder()
		.status(StatusCode::INTERNAL_SERVER_ERROR)
		.body(Full::new(Bytes::from(error_response.encode_to_vec())))
		// unwrap safety: body only errors when previous chained calls failed.
		.unwrap()
}
