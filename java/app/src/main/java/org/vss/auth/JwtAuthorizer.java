package org.vss.auth;

import com.auth0.jwt.JWT;
import com.auth0.jwt.algorithms.Algorithm;
import com.auth0.jwt.exceptions.JWTVerificationException;
import com.auth0.jwt.interfaces.DecodedJWT;
import com.auth0.jwt.interfaces.JWTVerifier;
import jakarta.ws.rs.core.HttpHeaders;
import org.vss.exception.AuthException;

import java.security.KeyFactory;
import java.security.PublicKey;
import java.security.interfaces.RSAPublicKey;
import java.security.spec.X509EncodedKeySpec;
import java.util.Base64;

// A JWT (https://datatracker.ietf.org/doc/html/rfc7519) based authorizer,
public class JwtAuthorizer implements Authorizer {

	private final PublicKey publicKey;
	private final JWTVerifier verifier;

	private static final String BEARER_PREFIX = "Bearer ";
	private static final int MAX_USER_TOKEN_LENGTH = 120;

	// `pemFormatRSAPublicKey` is RSA public key used by JWT Auth server for creating signed JWT tokens.
	// Refer to OpenSSL(https://docs.openssl.org/1.1.1/man1/rsa/) docs for generating valid key pairs.
	// Example:
	// * To generate private key, run : `openssl genpkey -algorithm RSA -out private_key.pem -pkeyopt rsa_keygen_bits:2048`
	// * To generate public key, run: `openssl rsa -pubout -in private_key.pem -out public_key.pem`
	public JwtAuthorizer(String pemFormatRSAPublicKey) throws Exception {
		this.publicKey = loadPublicKey(pemFormatRSAPublicKey);

		Algorithm algorithm = Algorithm.RSA256((RSAPublicKey) publicKey, null);
		this.verifier = JWT.require(algorithm).build();
	}

	@Override
	public AuthResponse verify(HttpHeaders headers) throws AuthException {

		try {
			String authorizationHeader = headers.getHeaderString(HttpHeaders.AUTHORIZATION);
			if (authorizationHeader == null || !authorizationHeader.startsWith(BEARER_PREFIX)) {
				throw new AuthException("Missing or invalid Authorization header.");
			}

			// Extract token by excluding BEARER_PREFIX.
			String token = authorizationHeader.substring(BEARER_PREFIX.length());

			DecodedJWT jwt = verifier.verify(token);

			// Extract the user identity from the token.
			String userToken = jwt.getSubject();

			if (userToken == null || userToken.isBlank()) {
				throw new AuthException("Invalid JWT token.");
			} else if (userToken.length() > MAX_USER_TOKEN_LENGTH) {
				throw new AuthException("UserToken is too long");
			}

			return new AuthResponse(userToken);

		} catch (JWTVerificationException e) {
			throw new AuthException("Invalid JWT token.");
		}
	}

	private PublicKey loadPublicKey(String pemFormatRSAPublicKey) throws Exception {
		String key = pemFormatRSAPublicKey
				.replaceAll("\\n", "")
				.replace("-----BEGIN PUBLIC KEY-----", "")
				.replace("-----END PUBLIC KEY-----", "");

		byte[] keyBytes = Base64.getDecoder().decode(key);

		X509EncodedKeySpec spec = new X509EncodedKeySpec(keyBytes);
		KeyFactory keyFactory = KeyFactory.getInstance("RSA");
		return keyFactory.generatePublic(spec);
	}
}
