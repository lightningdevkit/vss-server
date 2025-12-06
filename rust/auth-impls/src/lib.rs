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

use api::auth::{AuthResponse, Authorizer};
use api::error::VssError;
use async_trait::async_trait;
use jsonwebtoken::{decode, Algorithm, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use jsonwebtoken::DecodingKey;

/// A JWT based authorizer, only allows requests with verified 'JsonWebToken' signed by the given
/// issuer key.
///
/// Refer: https://datatracker.ietf.org/doc/html/rfc7519
pub struct JWTAuthorizer {
	jwt_issuer_key: DecodingKey,
}

/// A set of Claims claimed by 'JsonWebToken'
///
/// Refer: https://datatracker.ietf.org/doc/html/rfc7519#section-4
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Claims {
	/// The "sub" (subject) claim identifies the principal that is the subject of the JWT.
	/// The claims in a JWT are statements about the subject. This can be used as user identifier.
	///
	/// Refer: https://datatracker.ietf.org/doc/html/rfc7519#section-4.1.2
	sub: String,
}

const BEARER_PREFIX: &str = "Bearer ";

impl JWTAuthorizer {
	/// Create new instance of [`JWTAuthorizer`]
	pub async fn new(jwt_issuer_key: DecodingKey) -> Self {
		Self { jwt_issuer_key }
	}
}

#[async_trait]
impl Authorizer for JWTAuthorizer {
	async fn verify(
		&self, headers_map: &HashMap<String, String>,
	) -> Result<AuthResponse, VssError> {
		let auth_header = headers_map
			.get("Authorization")
			.ok_or(VssError::AuthError("Authorization header not found.".to_string()))?;

		let token = auth_header
			.strip_prefix(BEARER_PREFIX)
			.ok_or(VssError::AuthError("Invalid token format.".to_string()))?;

		let claims =
			decode::<Claims>(token, &self.jwt_issuer_key, &Validation::new(Algorithm::RS256))
				.map_err(|e| VssError::AuthError(format!("Authentication failure. {}", e)))?
				.claims;

		Ok(AuthResponse { user_token: claims.sub })
	}
}

#[cfg(test)]
mod tests {
	use crate::JWTAuthorizer;
	use api::auth::Authorizer;
	use api::error::VssError;
	use jsonwebtoken::{encode, Algorithm, DecodingKey, EncodingKey, Header};
	use serde::{Deserialize, Serialize};
	use std::collections::HashMap;
	use std::time::SystemTime;

	#[derive(Deserialize, Serialize)]
	struct TestClaims {
		sub: String,
		iat: i64,
		nbf: i64,
		exp: i64,
	}

	#[tokio::test]
	async fn test_valid_jwt_token() -> Result<(), VssError> {
		let now =
			SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64;
		let user_id = "valid_user_id";
		let claims = TestClaims {
			sub: user_id.to_owned(),
			iat: now,
			nbf: now,
			exp: now + 30556889864403199,
		};

		let valid_encoding_key = EncodingKey::from_rsa_pem(
			"-----BEGIN PRIVATE KEY-----\
				MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQDKwakpT4j2L1v5\
				BlIA278TFoDrDiqJB0Vlpd5F6LPj2vWgN8AHAogVb2Ar+Q2eucv0fw/6lh+PuOpQ\
				n+CWaCoyy8GyFtsPYWHHK1JLSaGxuHpDFSGVqfKY9xJRTIoEPq/tbQIZSFLmW4eW\
				wIWfjKyUWTilq9wG0ZqnQNNRzzLPSP/GeZJBt2NaCbRrBsc3jy4i1E7dSEsA560b\
				4HOVYJHxixNrmmJXwqAmkb+vBhMZe67eVwKadbCOZt4OrXMUWsIMNWRogeQYmBG4\
				UgM9dofJTDkfYe8qU/3jJJu9MMtdZmPpPLMcQcNuy2qzgOC+6sH9siGL91DvMrcQ\
				hcvwpEGHAgMBAAECggEAZJZ5Fq6HkyLhrQRusFBUVeLnKDXJ8lsyGYCVafdNL3BU\
				RR0DXjbqTkAH5SjUkfc48N4MjlPl6oZhcIgwgk3BCZw+RtzB5rp4KLgcRo+L8UBF\
				H3yfQcGjQjHo235uRjbXTqGy1dokjnXAKZDvebzvbVVqHf7J1HQuFmW5sK9rVJvP\
				CstC7HqJL15iYTshObnlskB+bnhhBc3LA+UpwyRmvOxPd60XOSxLJ8PMvwki5Qsx\
				afFCOFpT17474199SxmZtnVpcan7xf9dET8AENTIg8iUAFzLIsl5YekyRAeXj0QW\
				p9ln6Sl/TsWF+0yJPbeZ1kmvk52MMW7G56SqWt3bAQKBgQDy9mi9hRyfpfBMGrrk\
				MFDAo1cUvkfuFfBLAfUE9HoEpnQYBqAVFRWCqy6vAa5WdNpVMCDhZkGrn1KDDd/n\
				ZE/26WBTL95BzXQIO3Laiqmifnio01K2zvjvJt7aGMQOFUEJj8Ts8hUTbRMXfmXz\
				wbueKeHmcvAUOXbZb5ylC/gkgQKBgQDVovBSib6FnJdv5Clxf1t4gyIbOYWTUPj3\
				nmkFguBpTLwprzkYjyhyhrGuRaFbcqOVNopgt4KC6enpLtaAMffXwduge+TDKqsS\
				X1o3OhSzpsya3TrWQMDXKszKTTlNogESOejHxj7LIzts4JmKJcRN4dEVEKhP/CxA\
				2b05YnJCBwKBgEiAuc7ceyc1GJlNXLodpOtnkuPwyHxG9bccdWauIf9jQL+usnS4\
				HvwoYzz8Tm8kXccQHq/EmRJC8BeFu2xMpgQzrngEj9mpGtgeDW8j8+02uoD+1u8Q\
				on6TZetFerQNKaRVz9k5gIqUgR8ArCHqjTdsninr4LLYVxwZz2/9O2aBAoGBAISQ\
				ziW5ebL5P3NcFmdqSv1WCeTw5bVLSqKE9tBHrS9KQXxwUbKuqr+eW0UzyfOwCFf/\
				9xAa726C7fYXbV0xJIUKs1k7Z/G/WVZWOuoILW5pM49pdigbGE6sLVXfY46L17RS\
				oOLOXoq4+xgNqtjxpIVbed1jb73qUh+PvX6NWy8jAoGBAOvE6mhHBig5YYdidAGG\
				kF2oYp06+JG5ZpOu+MFT34ZDbgTwxx3+yuzfxPyBS68RHFfz+vG4BqX3P+pDOJQS\
				FeGjkLHWEoW7ol5rh1D1ubhWf1MAVOd7O8vp9APnAwd11uraVky2xAVXvplgmSpT\
				vHSUrqBuEFZ5mIWJxwkGElKN\
				-----END PRIVATE KEY-----"
				.as_bytes(),
		)
		.expect("Failed to create Encoding Key.");

		let decoding_key = DecodingKey::from_rsa_pem(
			"-----BEGIN PUBLIC KEY-----\
			MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAysGpKU+I9i9b+QZSANu/\
			ExaA6w4qiQdFZaXeReiz49r1oDfABwKIFW9gK/kNnrnL9H8P+pYfj7jqUJ/glmgq\
			MsvBshbbD2FhxytSS0mhsbh6QxUhlanymPcSUUyKBD6v7W0CGUhS5luHlsCFn4ys\
			lFk4pavcBtGap0DTUc8yz0j/xnmSQbdjWgm0awbHN48uItRO3UhLAOetG+BzlWCR\
			8YsTa5piV8KgJpG/rwYTGXuu3lcCmnWwjmbeDq1zFFrCDDVkaIHkGJgRuFIDPXaH\
			yUw5H2HvKlP94ySbvTDLXWZj6TyzHEHDbstqs4DgvurB/bIhi/dQ7zK3EIXL8KRB\
			hwIDAQAB\
			-----END PUBLIC KEY-----"
				.as_bytes(),
		)
		.expect("Failed to create Decoding Key.");

		let jwt_authorizer = JWTAuthorizer::new(decoding_key).await;

		let valid_jwt_token =
			encode(&Header::new(Algorithm::RS256), &claims, &valid_encoding_key).unwrap();
		let mut headers_map: HashMap<String, String> = HashMap::new();
		let header_value = format!("Bearer {}", valid_jwt_token);
		headers_map.insert("Authorization".to_string(), header_value.clone());
		println!("headers_map: {:?}", headers_map);

		// JWT signed by valid key results in authenticated user.
		assert_eq!(jwt_authorizer.verify(&headers_map).await?.user_token, user_id);

		let invalid_encoding_key = EncodingKey::from_rsa_pem(
			"-----BEGIN PRIVATE KEY-----
			MIIEvwIBADANBgkqhkiG9w0BAQEFAASCBKkwggSlAgEAAoIBAQC77KWE/VUi7QTc\
			odlj5yRaawPO4z+Ik4c2r2W1BaivIn2dkeTYKT9cQUEcU3sP/i4bQ/DnSuOWAmmG\
			yaR4NvUvJyGxm6PSBf/kgzDbfvf/8sCi9OEpJEe/xYOhLFaPumtcJAB5mKrdaKsH\
			XBKJaxJInJsiA6eB67d6SESXG/q1H8f00VLxIAKLK32z5Uahuzc9HQvl4dya+dAI\
			Xcw0TJg+JoBIqv5ATlcoXKqguiAyQdG2nW5nRnArhvCl9blKjg26cjbhiJcVEZCf\
			z8vv56IEPhvYEtA8OaiP6vEquqA+vwNipKxqhLzfsjgqYMf18PtrftHjn7nkIvlW\
			RMnG4+IbAgMBAAECggEAXZf+171UKZDiWwBAxQDZmi6yNtf3TI4tSY8RmJa47IDB\
			DzkaQI5KgCf/xZvOLqjpTasI0Cj8MDoDVJ4Yy8aTVmim304kyPUz/RtZufgCi/ba\
			+k371gG7ukckx6DNe8fcsIc9tVHTx3HZvFCe6tHoyUE2AjrPsmUzfDOB9cB5nLrc\
			JFyKVRUwByeG76AgDJaYMq6cK53+GZih3F9e2exxdnlBuk11R2yJMr638yOfgYbY\
			9vzq49OvleLEH1AdAxkcNYuUiPNC7KUeS84MAn+Ok65WvSlyJC3IjVS+swv4p/SB\
			u0S38ljqisqr0qgfupEJJA/VQaXXo5NJDw48TDuEAQKBgQDuFt7sCoDyqm7XwzWf\
			f9t9VFnPrLjJbNF7ll2zNlzfArzwk6cDrps2sXoNY0r37ObAdK+awWYRDyoCXJCe\
			t1wP/leYMp8opn2axQVHSJCq8K2fZO3xRn98p6jy9Hub0l2r9EN6v3JGQmPffl03\
			qrtYvU8as1ppUXj8Rgw4EGOWRQKBgQDKD7LJ5l/GXotYdOW93y/AXKmEzUjfi1gN\
			QMxu4TxvK4Q5+CjALYtXb0swbOd7ThcYTU1vgD2Vf5t4z8L/0gSRssGxmMOw8UaS\
			lay3ONFPRUhffzCMB4wkaomt1km3t9J1LJJ8h8131x2604MrIKmPMIAU6wnikdNN\
			G5VXx6HM3wKBgQCBzqBdiuCA7WEfa8PJoTj23M1Wh7H7x8NyoSmW8tWxlNmURLwz\
			KrhfGmYT9IXEJDouxa+ULUtLk7vwq60Bi7C6243AYiEaVaN3hWF6WtrdB/lxROLh\
			v/Dz8qkPRTI7Y3dEsBk2TDiui7XN/SQvnHsmR5hgU1bAwvW2fS5eRrk1DQKBgQCf\
			Dq55ukwoNiJQtmxnA3puXULgFEzKE8FzZU/H9KuDA2lpzIwfg3qNkEFK1F9/s+AA\
			NFHBdNyFg1baSgnBIQyRuHo6l/trnPIlz4aPED3LvckTy2ZmxEYwIGFSoz2STjRw\
			Im8JcklujbqMZ5V4bJSs78vTK5WzcYE40H7GA5K9VwKBgQCMNL9R7GUGxfQaOxiI\
			4mjwus2eQ0fEodIXfU5XFppScHgtKhPWNWNfbrSICyFkfvGBBgQDLCZgt/fO+GAK\
			r0kIP0GD3KvsLVHsSTR6Fsnz+05HYUEwbc6ebjOegJu+ZO9C4MXnWIaiOzd6vxUz\
			UIOZiBd7mcNJ6ccxdZ39YIPTew==\
			-----END PRIVATE KEY-----"
				.as_bytes(),
		)
		.expect("Failed to create Encoding Key.");

		let invalid_jwt_token =
			encode(&Header::new(Algorithm::RS256), &claims, &invalid_encoding_key).unwrap();
		headers_map.insert("Authorization".to_string(), format!("Bearer {}", invalid_jwt_token));

		// JWT signed by invalid key results in AuthError.
		assert!(matches!(
			jwt_authorizer.verify(&headers_map).await.unwrap_err(),
			VssError::AuthError(_)
		));
		Ok(())
	}
}
