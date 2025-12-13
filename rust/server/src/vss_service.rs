use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::service::Service;
use hyper::{Request, Response, StatusCode};
use std::collections::HashMap;
use tracing::Instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;

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

use crate::tracing::extract_context;

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
		.map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or_default().to_string()))
		.collect::<HashMap<String, String>>();

	let parent_cx = extract_context(&headers_map);
	let (server_address, server_port) = parts
		.headers
		.get("host")
		.and_then(|v| v.to_str().ok())
		.map(|h| {
			let mut split = h.splitn(2, ':');
			let addr = split.next().map(|s| s.to_string());
			let port = split.next().and_then(|p| p.parse::<u16>().ok());
			(addr, port)
		})
		.unwrap_or((None, None));
	let span = tracing::info_span!(
		"vss.server.request",
		request_type = std::any::type_name::<T>().split("::").last().unwrap_or("unknown"),
		http.request.method = %parts.method,
		http.route = %parts.uri.path(),
		http.status_code = tracing::field::Empty,
		server.address = server_address,
		server.port = server_port,
	);
	let _ = span.set_parent(parent_cx);

	let user_token = match authorizer.verify(&headers_map).await {
		Ok(auth_response) => auth_response.user_token,
		Err(e) => return Ok(build_error_response(e)),
	};

	async move {
		// TODO: we should bound the amount of data we read to avoid allocating too much memory.
		let bytes = body.collect().await?.to_bytes();
		tracing::info!(payload_size = bytes.len());
		match T::decode(bytes) {
			Ok(request) => match handler(store.clone(), user_token, request).await {
				Ok(response) => {
					let status = StatusCode::OK;
					tracing::Span::current().record("http.status_code", status.as_u16());
					Ok(Response::builder()
						.body(Full::new(Bytes::from(response.encode_to_vec())))
						// unwrap safety: body only errors when previous chained calls failed.
						.unwrap())
				},
				Err(e) => {
					let response = build_error_response(e);
					tracing::Span::current().record("http.status_code", response.status().as_u16());
					Ok(response)
				},
			},
			Err(_) => {
				let status_code = StatusCode::BAD_REQUEST;
				tracing::Span::current().record("http.status_code", status_code.as_u16());
				Ok(Response::builder()
					.status(status_code)
					.body(Full::new(Bytes::from(b"Error parsing request".to_vec())))
					// unwrap safety: body only errors when previous chained calls failed.
					.unwrap())
			},
		}
	}
	.instrument(span)
	.await
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
