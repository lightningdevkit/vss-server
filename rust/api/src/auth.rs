use crate::error::VssError;
use async_trait::async_trait;
use std::collections::HashMap;
use std::string::ToString;

/// Response returned for [`Authorizer`] request if user is authenticated and authorized.
#[derive(Debug, Clone)]
pub struct AuthResponse {
	/// A `user_token` identifying the authenticated and authorized user.
	pub user_token: String,
}

/// Interface for authorizer that is run before executing every request.
#[async_trait]
pub trait Authorizer: Send + Sync {
	/// Verifies authentication and authorization based on request headers.
	/// Returns [`AuthResponse`] for an authenticated and authorized user or [`VssError::AuthError`]
	/// for an unauthorized request.
	async fn verify(&self, headers_map: &HashMap<String, String>)
		-> Result<AuthResponse, VssError>;
}

/// A no-operation authorizer, which lets any user-request go through.
pub struct NoopAuthorizer {}

const UNAUTHENTICATED_USER: &str = "unauth-user";

#[async_trait]
impl Authorizer for NoopAuthorizer {
	async fn verify(
		&self, _headers_map: &HashMap<String, String>,
	) -> Result<AuthResponse, VssError> {
		Ok(AuthResponse { user_token: UNAUTHENTICATED_USER.to_string() })
	}
}
