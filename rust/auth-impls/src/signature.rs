//! Hosts a VSS protocol compliant [`Authorizer`] implementation that requires that every request
//! come with a public key and proof of private key knowledge. Access is then granted to the user
//! defined by the public key.
//!
//! There is no specific restriction of who is allowed to store data in VSS using this
//! authentication scheme, only that each user is only allowed to store and access data for which
//! they have a corresponding private key. Thus, you must ensure new user accounts are
//! appropriately rate-limited or access to the VSS server is somehow limited.
//!
//! [`Authorizer`]: api::auth::Authorizer

use api::auth::{AuthResponse, Authorizer};
use api::error::VssError;
use async_trait::async_trait;
use bitcoin_hashes::HashEngine;
use std::collections::HashMap;
use std::time::SystemTime;

/// A 64-byte constant which, after appending the public key, is signed in order to prove knowledge
/// of the corresponding private key.
pub const SIGNING_CONSTANT: &'static [u8] =
	b"VSS Signature Authorizer Signing Salt Constant..................";

/// An authorizer that requires that every request come with a public key and proof of private key
/// knowledge. Access is then granted to the user defined by the public key.
///
/// The proof of private key knowledge takes the form of an ECDSA signature over the
/// [`SIGNING_CONSTANT`] followed by the public key followed by the current time since the UNIX
/// epoch, encoded as a string. It is expected to appear in the `Authorization` header, in the form
/// of the hex-encoded 33-byte secp256k1 public key in compressed form followed by the hex-encoded
/// 64-byte secp256k1 ECDSA signature followed by the signing time since the UNIX epoch, encoded as
/// a string.
///
/// The proof will not be valid if the provided time is more than an hour from now.
///
/// Because no rate-limiting of new user accounts is done, a higher-level service is required to
/// ensure requests are not triggering excess new user registrations.
pub struct SignatureValidatingAuthorizer;

#[async_trait]
impl Authorizer for SignatureValidatingAuthorizer {
	async fn verify(
		&self, headers_map: &HashMap<String, String>,
	) -> Result<AuthResponse, VssError> {
		let auth_header = headers_map
			.get("Authorization")
			.ok_or_else(|| VssError::AuthError("Authorization header not found.".to_string()))?;

		if auth_header.len() <= (33 + 64) * 2 {
			return Err(VssError::AuthError("Authorization header has wrong length".to_string()));
		}
		if !auth_header.is_ascii() {
			return Err(VssError::AuthError("Authorization header has bogus chars".to_string()));
		}

		let pubkey_hex = &auth_header[..33 * 2];
		let signat_hex = &auth_header[33 * 2..(33 + 64) * 2];
		let time_strng = &auth_header[(33 + 64) * 2..];

		let pubkey_bytes: [u8; 33] = hex_conservative::decode_to_array(pubkey_hex)
			.map_err(|_| VssError::AuthError("Authorization header is not hex".to_string()))?;
		let sig_bytes: [u8; 64] = hex_conservative::decode_to_array(signat_hex)
			.map_err(|_| VssError::AuthError("Authorization header is not hex".to_string()))?;
		let time: u64 = time_strng
			.parse()
			.map_err(|_| VssError::AuthError("Time is not an integer".to_string()))?;

		let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
		if now.as_secs() - 60 * 60 * 24 > time || now.as_secs() + 60 * 60 * 24 < time {
			return Err(VssError::AuthError("Time is too far from now".to_string()))?;
		}

		let pubkey = secp256k1::PublicKey::from_byte_array_compressed(pubkey_bytes)
			.map_err(|_| VssError::AuthError("Authorization header has bad pubkey".to_string()))?;
		let sig = secp256k1::ecdsa::Signature::from_compact(&sig_bytes)
			.map_err(|_| VssError::AuthError("Authorization header has bad sig".to_string()))?;

		let mut hash = bitcoin_hashes::Sha256::engine();
		hash.input(&SIGNING_CONSTANT);
		hash.input(&pubkey_bytes);
		hash.input(time_strng.as_bytes());
		let signed_hash = secp256k1::Message::from_digest(hash.finalize().to_byte_array());
		sig.verify(signed_hash, &pubkey)
			.map_err(|_| VssError::AuthError("Signature was invalid".to_string()))?;

		Ok(AuthResponse { user_token: pubkey_hex.to_owned() })
	}
}

#[cfg(test)]
mod tests {
	use crate::signature::{SignatureValidatingAuthorizer, SIGNING_CONSTANT};
	use api::auth::Authorizer;
	use api::error::VssError;
	use secp256k1::{Message, PublicKey, Secp256k1, SecretKey};
	use std::collections::HashMap;
	use std::fmt::Write;
	use std::time::SystemTime;

	fn build_token(now: u64) -> (String, PublicKey) {
		let secret_key = SecretKey::from_byte_array([42; 32]).unwrap();
		let pubkey = secret_key.public_key(secp256k1::SECP256K1);

		let mut bytes_to_sign = Vec::new();
		bytes_to_sign.extend_from_slice(SIGNING_CONSTANT);
		bytes_to_sign.extend_from_slice(&pubkey.serialize());
		bytes_to_sign.extend_from_slice(format!("{now}").as_bytes());
		let hash = bitcoin_hashes::Sha256::hash(&bytes_to_sign);
		let sig = secret_key.sign_ecdsa(Message::from_digest(hash.to_byte_array()));
		let mut sig_hex = String::with_capacity(64 * 2);
		for c in sig.serialize_compact() {
			write!(&mut sig_hex, "{:02x}", c).unwrap();
		}
		(format!("{pubkey:x}{sig_hex}{now}"), pubkey)
	}

	#[tokio::test]
	async fn test_sig() {
		let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
		let mut headers_map = HashMap::new();
		let auth = SignatureValidatingAuthorizer;

		// Test a valid signature
		let (token, pubkey) = build_token(now);
		headers_map.insert("Authorization".to_string(), token);
		assert_eq!(auth.verify(&headers_map).await.unwrap().user_token, format!("{pubkey:x}"));

		// Test a signature too far in the future
		let (token, _) = build_token(now + 60 * 60 * 24 + 10);
		headers_map.insert("Authorization".to_string(), token);
		assert!(matches!(auth.verify(&headers_map).await.unwrap_err(), VssError::AuthError(_)));

		// Test a signature too far in the past
		let (token, _) = build_token(now - 60 * 60 * 24 - 10);
		headers_map.insert("Authorization".to_string(), token);
		assert!(matches!(auth.verify(&headers_map).await.unwrap_err(), VssError::AuthError(_)));

		// Test a token with an invalid signature
		let (mut token, _) = build_token(now);
		token = token
			.chars()
			.enumerate()
			.map(|(idx, c)| if idx == 33 * 2 + 10 || idx == 33 * 2 + 11 { '0' } else { c })
			.collect();
		headers_map.insert("Authorization".to_string(), token);
		assert!(matches!(auth.verify(&headers_map).await.unwrap_err(), VssError::AuthError(_)));

		// Test a token with the wrong public key
		let (mut token, _) = build_token(now);
		token = token
			.chars()
			.enumerate()
			.map(|(idx, c)| if idx == 10 || idx == 11 { '0' } else { c })
			.collect();
		headers_map.insert("Authorization".to_string(), token);
		assert!(matches!(auth.verify(&headers_map).await.unwrap_err(), VssError::AuthError(_)));
	}
}
