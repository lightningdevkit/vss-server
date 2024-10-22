package org.vss.auth;

import jakarta.ws.rs.core.HttpHeaders;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.vss.exception.AuthException;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;

class JwtAuthorizerTest {

	private JwtAuthorizer jwtAuthorizer;
	private HttpHeaders headers;

	private static final String PUBLIC_KEY = "-----BEGIN PUBLIC KEY-----\n" +
			"MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAysGpKU+I9i9b+QZSANu/\n" +
			"ExaA6w4qiQdFZaXeReiz49r1oDfABwKIFW9gK/kNnrnL9H8P+pYfj7jqUJ/glmgq\n" +
			"MsvBshbbD2FhxytSS0mhsbh6QxUhlanymPcSUUyKBD6v7W0CGUhS5luHlsCFn4ys\n" +
			"lFk4pavcBtGap0DTUc8yz0j/xnmSQbdjWgm0awbHN48uItRO3UhLAOetG+BzlWCR\n" +
			"8YsTa5piV8KgJpG/rwYTGXuu3lcCmnWwjmbeDq1zFFrCDDVkaIHkGJgRuFIDPXaH\n" +
			"yUw5H2HvKlP94ySbvTDLXWZj6TyzHEHDbstqs4DgvurB/bIhi/dQ7zK3EIXL8KRB\n" +
			"hwIDAQAB\n" +
			"-----END PUBLIC KEY-----";

	private static final String VALID_AUTH_HEADER = "Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9." +
			"eyJzdWIiOiJ2YWxpZF91c2VyX2lkIiwiaWF0IjoxNzI5NjM0MjYwLCJuYmYiOjE3Mjk2MzQyNjAsImV4cCI6MzA1" +
			"NTY4OTE1OTQwMzc0NTl9.xBL5BYiv8B-ZN1bCuljuJ7dZeOPocVPPVwkeK_GH4lD5iQqD08zi93WuXw1c6NWWCK4" +
			"jn4ZssYrzSLLL5q3tAYbLKuhQ2-2A-e1HTasfvSnx_jCBUNApbIv3rM19M3rhRVRSxT2s2jI7dJFlM6E_bGMfj9w" +
			"uoZiT_amjIIPQJiRkDKcO2sXnD6eU_yx8EIhH_PemSX3kp9Sx9eTYqGbyCtLrs9jK7nr6GQ_1jc6ie03Uh2dsIzW" +
			"sZqGHh2n_WmdyURWEfwsMYFpepRLzm77dP9q78RgA8eDLZSLNW9ssJMYWY9DRkOZBFFuf4uy-uqC9MWS64DkJSAo" +
			"nH8Zof_tUiQ";

	private static final String VALID_USER_ID = "valid_user_id";

	@BeforeEach
	public void setUp() throws Exception {
		jwtAuthorizer = new JwtAuthorizer(PUBLIC_KEY);
		headers = mock(HttpHeaders.class);
	}

	@Test
	public void testValidJwtToken() {
		when(headers.getHeaderString(HttpHeaders.AUTHORIZATION)).thenReturn(VALID_AUTH_HEADER);

		AuthResponse authResponse = jwtAuthorizer.verify(headers);

		assertNotNull(authResponse);

		assertEquals(VALID_USER_ID, authResponse.getUserToken());
	}

	@Test
	public void testMissingAuthorizationHeader() {
		when(headers.getHeaderString(HttpHeaders.AUTHORIZATION)).thenReturn(null);

		assertThrows(AuthException.class, () -> jwtAuthorizer.verify(headers));
	}

	@Test
	public void testInvalidAuthorizationHeader() {
		when(headers.getHeaderString(HttpHeaders.AUTHORIZATION)).thenReturn("InvalidHeader");

		assertThrows(AuthException.class, () -> jwtAuthorizer.verify(headers));
	}

	@Test
	public void testInvalidJwtToken() {
		String invalidJwt = "Bearer invalid.jwt.token";
		when(headers.getHeaderString(HttpHeaders.AUTHORIZATION)).thenReturn(invalidJwt);

		assertThrows(AuthException.class, () -> jwtAuthorizer.verify(headers));
	}
}
