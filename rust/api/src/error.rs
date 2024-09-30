use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io;
/// When there is an error while writing to VSS storage, the response contains a relevant error code.
/// A mapping for VSS server error codes. Refer to [`ErrorResponse`] docs for more
/// information regarding each error code and corresponding use-cases.
///
/// [`ErrorResponse`]: crate::types::ErrorResponse
#[derive(Debug)]
pub enum VssError {
	/// Please refer to [`ErrorCode::NoSuchKeyException`].
	///
	/// [`ErrorCode::NoSuchKeyException`]: crate::types::ErrorCode::NoSuchKeyException
	NoSuchKeyError(String),

	/// Please refer to [`ErrorCode::InvalidRequestException`].
	///
	/// [`ErrorCode::InvalidRequestException`]: crate::types::ErrorCode::InvalidRequestException
	InvalidRequestError(String),

	/// Please refer to [`ErrorCode::ConflictException`].
	///
	/// [`ErrorCode::ConflictException`]: crate::types::ErrorCode::ConflictException
	ConflictError(String),

	/// Please refer to [`ErrorCode::AuthException`].
	///
	/// [`ErrorCode::AuthException`]: crate::types::ErrorCode::AuthException
	AuthError(String),

	/// Please refer to [`ErrorCode::InternalServerException`].
	///
	/// [`ErrorCode::InternalServerException`]: crate::types::ErrorCode::InternalServerException
	InternalServerError(String),
}

impl Display for VssError {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			VssError::NoSuchKeyError(message) => {
				write!(f, "Requested key does not exist: {}", message)
			},
			VssError::InvalidRequestError(message) => {
				write!(f, "Request sent to VSS was invalid: {}", message)
			},
			VssError::ConflictError(message) => {
				write!(f, "Version conflict in write operation: {}", message)
			},
			VssError::AuthError(message) => {
				write!(f, "Authentication or Authorization failure: {}", message)
			},
			VssError::InternalServerError(message) => {
				write!(f, "InternalServerError: {}", message)
			},
		}
	}
}

impl Error for VssError {}

impl From<io::Error> for VssError {
	fn from(err: io::Error) -> Self {
		VssError::InternalServerError(err.to_string())
	}
}
